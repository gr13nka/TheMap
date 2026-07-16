//! Мир — состояние симуляции: плоскость, посевы, колода, время, seed.
//! Два глагола: `draw()` — тяга (компилировать сигил верхней карты, родить
//! посев) и `step()` — один тик (посевы живут). Пауза и скорость в ядре
//! не существуют: клиент зовёт `step()` 0/1/4 раза за интервал. RNG тяги
//! выводится из (seed, draw_count), RNG тика — из (seed, tick, id посева):
//! мир воспроизводим и переживает save/load.

use std::path::{Path, PathBuf};

use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;

use crate::archivist::Archivist;
use crate::card::{self, Card};
use crate::cycle::{CycleState, CycleSummary, Phase};
use crate::deck::Deck;
use crate::event::Event;
use crate::legacy::Legacy;
use crate::plane::Plane;
use crate::rune::{self, Matter};
use crate::sim::entropy::EntropyState;
use crate::sim::settlement::{self, Settlement};
use crate::sim::{self, seed::Seed};
use crate::tablet::{Laws, Tablets};
use crate::tile::{Tile, TileKind};

/// Соли RNG подсистем тика — чтобы потоки случайности не пересекались.
const SALT_SETTLEMENT: u64 = 0x5E77;
const SALT_ENTROPY: u64 = 0xE472;

/// Операция мета-карты над колодой.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaOp {
    Duplicate,
    Destroy,
    Shuffle,
}

/// Решение Правителя, принятое при тяге (карты с `choice:`).
#[derive(Debug, Clone, Copy)]
pub enum DrawChoice {
    /// Куда направить главную манеру («куда направить реку?»).
    Direction((i8, i8)),
    /// Куда посеять.
    Site((i32, i32)),
}

/// Божественный жест — точечное ручное вмешательство на карте.
#[derive(Debug, Clone, Copy)]
pub enum Gesture {
    /// Закрасить клетку материей (рука сильнее матрицы — как гуашь Грецингера).
    Paint(Matter),
    /// Стереть до чистой бумаги.
    Erase,
    /// Снять стадию распада — спасти от пустоты.
    Mend,
}

// Жесты, дрейф, ритм зова — не константы: их диктует Скрижаль Сердца.

/// Итог одной тяги — для архивариуса и клиента.
#[derive(Debug, Clone)]
pub struct DrawOutcome {
    pub card_name: String,
    /// Материя, проступившая на бумаге; None — карта легла без следа.
    pub matter: Option<Matter>,
    pub origin: (i32, i32),
    /// Мета-карта: Shuffle уже применён; Duplicate/Destroy ждут выбора цели
    /// (клиент зовёт `apply_meta`).
    pub meta: Option<MetaOp>,
    /// Узор карты дрейфнул сам при этой тяге.
    pub mutated: bool,
    /// Ключ комбинации (материя+манеры) — для атласа наблюдений.
    pub combo: Option<String>,
}

pub struct World {
    pub plane: Plane,
    pub seeds: Vec<Seed>,
    pub settlements: Vec<Settlement>,
    pub entropy: EntropyState,
    pub archivist: Archivist,
    pub cycle: CycleState,
    /// Скрижали — законы мира как выражения (правятся крафтом).
    pub tablets: Tablets,
    /// Скомпилированные законы (производные от скрижалей).
    pub laws: Laws,
    pub deck: Deck,
    pub deck_dir: PathBuf,
    pub tick: u64,
    pub draw_count: u64,
    /// Тик последней тяги — от него считает зов Скрижаль Сердца.
    pub last_draw_tick: u64,
    pub seed: u64,
    pub cursor: (i32, i32),
    /// Лучший час мира — максимум занятых клеток за жизнь.
    pub peak_filled: usize,
    /// Сколько очагов поднялось за жизнь листа.
    pub hearths_founded: u32,
    /// Остаток божественных жестов до следующей тяги.
    pub gestures: u8,
    next_seed_id: u64,
    next_settlement_id: u64,
}

impl World {
    pub fn new(deck_dir: PathBuf, w: i32, h: i32, seed: u64) -> std::io::Result<World> {
        World::with_epoch(deck_dir, w, h, seed, 1)
    }

    fn with_epoch(
        deck_dir: PathBuf,
        w: i32,
        h: i32,
        seed: u64,
        epoch: u32,
    ) -> std::io::Result<World> {
        let deck = Deck::from_dir(&deck_dir)?;
        let plane = Plane::new(w, h);
        let entropy = EntropyState::new();
        let tablets_dir = Tablets::dir_for(&deck_dir);
        Tablets::ensure_defaults(&tablets_dir)?;
        let tablets = Tablets::load(&tablets_dir);
        let laws = tablets.laws();
        let gestures = laws.heart.gestures.unwrap_or(0);
        Ok(World {
            plane,
            seeds: Vec::new(),
            settlements: Vec::new(),
            entropy,
            archivist: Archivist::default(),
            cycle: CycleState::new(epoch),
            deck,
            deck_dir,
            tick: 0,
            draw_count: 0,
            last_draw_tick: 0,
            seed,
            cursor: (w / 2, h / 2),
            peak_filled: 0,
            hearths_founded: 0,
            gestures,
            tablets,
            laws,
            next_seed_id: 0,
            next_settlement_id: 0,
        })
    }

    /// Перечитать скрижали с диска и перекомпилировать законы —
    /// правка ядра действует на живом мире немедленно.
    pub fn reload_tablets(&mut self) {
        let dir = Tablets::dir_for(&self.deck_dir);
        self.tablets = Tablets::load(&dir);
        self.laws = self.tablets.laws();
    }

    /// Зовёт ли колода: спустя N тиков после тяги (Скрижаль Сердца).
    /// Скрижаль молчит — колода не зовёт никогда.
    pub fn deck_calls(&self) -> bool {
        self.laws
            .heart
            .call_every
            .map(|n| self.tick.saturating_sub(self.last_draw_tick) >= n)
            .unwrap_or(false)
    }

    /// Родить следующий лист: свежая бумага, seed из (base, epoch),
    /// руины прошлой эпохи уже проступают — слегка изъеденные, как память.
    pub fn new_epoch(
        deck_dir: PathBuf,
        w: i32,
        h: i32,
        base_seed: u64,
        legacy: &Legacy,
    ) -> std::io::Result<World> {
        let seed =
            base_seed ^ (legacy.epoch as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let mut world = World::with_epoch(deck_dir, w, h, seed, legacy.epoch)?;
        for ruin in &legacy.ruins {
            let (x, y) = ruin.pos;
            if world.plane.in_bounds(x, y) {
                let mut tile = Tile::new(ruin.kind);
                tile.decay = if ruin.kind == TileKind::Stone { 2 } else { 1 };
                world.plane.tiles.insert((x, y), tile);
            }
        }
        Ok(world)
    }

    /// Имя верхней карты колоды (для статус-строки).
    pub fn top_card_name(&self) -> String {
        self.deck
            .top()
            .and_then(|p| Card::parse_file(p).ok())
            .map(|c| c.name)
            .unwrap_or_else(|| "—".to_string())
    }

    /// Детерминированный RNG именно этой тяги.
    fn draw_rng(&self) -> StdRng {
        let mixed = self
            .seed
            .wrapping_add(self.draw_count.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        StdRng::seed_from_u64(mixed)
    }

    /// Один тик симуляции. Порядок жёсткий ради детерминизма:
    /// посевы → поселения → энтропия. Возвращает события для архивариуса.
    pub fn step(&mut self) -> Vec<Event> {
        self.tick += 1;
        let mut events = Vec::new();

        sim::step_seeds(
            &mut self.plane,
            &mut self.seeds,
            self.seed,
            self.tick,
            &mut events,
        );

        let hearth_law = self.laws.hearth.clone();
        let mut rng = sim::tick_rng(self.seed, self.tick, SALT_SETTLEMENT);
        settlement::step(
            &mut self.plane,
            &mut self.settlements,
            &mut self.next_settlement_id,
            &mut rng,
            self.tick,
            &hearth_law,
            &mut events,
        );

        let entropy_law = self.laws.entropy.clone();
        let mut rng = sim::tick_rng(self.seed, self.tick, SALT_ENTROPY);
        self.entropy
            .step(&mut self.plane, &mut rng, self.tick, &entropy_law, &mut events);

        let seen = events.clone();
        self.watch_cycle(&seen, &mut events);
        events
    }

    /// Следить за фазой цикла: расцвет → увядание → умирание → смерть.
    fn watch_cycle(&mut self, seen: &[Event], events: &mut Vec<Event>) {
        // статистика лучшего часа
        self.peak_filled = self.peak_filled.max(self.plane.filled());
        self.hearths_founded += seen
            .iter()
            .filter(|e| matches!(e, Event::SettlementFounded { .. }))
            .count() as u32;

        if self.cycle.phase == Phase::Dead {
            return;
        }
        if seen.iter().any(|e| matches!(e, Event::WorldWaning)) {
            self.cycle.phase = Phase::Wane;
        }

        // пороги — из Скрижали Порога; молчит — мир официально не умирает
        let area = (self.plane.w * self.plane.h) as f64;
        let void_share = self.plane.count(TileKind::Void) as f64 / area;
        let law = &self.laws.threshold;

        if let Some(dying) = law.dying_pct {
            if self.cycle.phase != Phase::Dying && void_share >= dying as f64 / 100.0 {
                self.cycle.phase = Phase::Dying;
                events.push(Event::WorldDying);
            }
        }

        if let Some(dead) = law.dead_pct {
            if self.cycle.doomed_at.is_none() && void_share >= dead as f64 / 100.0 {
                self.cycle.doomed_at = Some(self.tick);
            }
        }
        if let Some(doomed) = self.cycle.doomed_at {
            if self.tick >= doomed + law.grace.unwrap_or(0) {
                self.cycle.phase = Phase::Dead;
                events.push(Event::WorldDead {
                    summary: self.summary(),
                });
            }
        }
    }

    /// Итог прожитого — для эпилога и наследия.
    pub fn summary(&self) -> CycleSummary {
        CycleSummary {
            epoch: self.cycle.epoch,
            ticks_lived: self.tick,
            draws: self.draw_count,
            peak_filled: self.peak_filled,
            hearths_founded: self.hearths_founded,
        }
    }

    /// Отдать события архивариусу; вернуть строки хроники.
    pub fn narrate(&mut self, events: &[Event]) -> Vec<String> {
        self.archivist.narrate(events, self.tick)
    }

    /// Верхняя карта колоды целиком — клиент смотрит, нужен ли выбор
    /// (`choice:`) до самой тяги.
    pub fn peek_top(&self) -> Option<Card> {
        self.deck.top().and_then(|p| Card::parse_file(p).ok())
    }

    /// Тяга: снять верхнюю карту (она уходит в низ колоды) и исполнить её.
    /// Сигил может дрейфнуть (лёгкая мутация), компилируется и сеет объект;
    /// мета-карта действует на колоду. Тяга пополняет жесты Правителя.
    pub fn draw(&mut self, choice: Option<DrawChoice>) -> std::io::Result<DrawOutcome> {
        let mut outcome = DrawOutcome {
            card_name: "—".to_string(),
            matter: None,
            origin: self.cursor,
            meta: None,
            mutated: false,
            combo: None,
        };
        let Some(path) = self.deck.draw() else {
            return Ok(outcome);
        };
        let card = Card::parse_file(&path)?;
        outcome.card_name = card.name.clone();
        let mut rng = self.draw_rng();
        // тяга пополняет жесты и перезаводит зов — по Скрижали Сердца
        self.gestures = self.laws.heart.gestures.unwrap_or(0);
        self.last_draw_tick = self.tick;

        // --- мета-карта: закон колоды, не мира ---
        if card.kind == "meta" {
            let op = match card.op.as_deref() {
                Some("duplicate") => Some(MetaOp::Duplicate),
                Some("destroy") => Some(MetaOp::Destroy),
                Some("shuffle") => Some(MetaOp::Shuffle),
                _ => None,
            };
            if op == Some(MetaOp::Shuffle) {
                self.deck.shuffle(&mut rng);
            }
            outcome.meta = op;
            self.draw_count += 1;
            return Ok(outcome);
        }

        // --- рунная карта ---
        // точка посева: выбор Правителя или случай тяги
        let site = match choice {
            Some(DrawChoice::Site(pos)) => pos,
            _ => (
                rng.gen_range(3..(self.plane.w - 3).max(4)),
                rng.gen_range(2..(self.plane.h - 2).max(3)),
            ),
        };
        self.cursor = site;

        // лёгкая мутация: выражение может дрейфнуть прямо в .md
        // (шанс дрейфа — руна ☾ в Скрижали Сердца)
        let drift_p = self.laws.heart.drift_pct.unwrap_or(0) as f64 / 100.0;
        let mut expr = card.expr();
        if let Some(e) = expr.as_mut() {
            if drift_p > 0.0 && rng.gen_bool(drift_p) && rune::mutate::drift(e, &mut rng) {
                card::write_rune(&path, e)?;
                outcome.mutated = true;
            }
        }

        let mut program = expr.as_ref().and_then(rune::compile);
        // выбор направления подменяет стрелку первой клаузы
        if let (Some(DrawChoice::Direction(dir)), Some(p)) = (choice, program.as_mut()) {
            if let Some(clause) = p.clauses.first_mut() {
                clause.dir = Some(dir);
                clause.seek = None;
            }
        }

        if let Some(program) = program {
            outcome.matter = Some(program.matter);
            outcome.combo = Some(program.combo_key());
            let id = self.next_seed_id;
            self.next_seed_id += 1;
            self.seeds
                .push(Seed::spawn(id, program, self.cursor, &mut self.plane));
            outcome.origin = self.cursor;
        }

        self.draw_count += 1;
        Ok(outcome)
    }

    /// Применить решение Правителя по мета-карте (цель — индекс в колоде).
    /// Возвращает имя целевой карты.
    pub fn apply_meta(&mut self, op: MetaOp, target: usize) -> std::io::Result<String> {
        let Some(path) = self.deck.cards.get(target).cloned() else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "нет такой карты в колоде",
            ));
        };
        let name = Card::parse_file(&path)?.name;
        match op {
            MetaOp::Duplicate => {
                let copy = free_copy_path(&path);
                std::fs::copy(&path, &copy)?;
                self.deck.insert_under_top(copy);
            }
            MetaOp::Destroy => {
                // данные не удаляем — карта уходит в могильник
                let graveyard = self.deck_dir.join("graveyard");
                std::fs::create_dir_all(&graveyard)?;
                let dest = graveyard.join(path.file_name().unwrap_or_default());
                std::fs::rename(&path, &dest)?;
                self.deck.remove(target);
            }
            MetaOp::Shuffle => {}
        }
        Ok(name)
    }

    /// Божественный жест: потратить один из бюджета тяги.
    /// Возвращает false, если жестов не осталось или жест ничего не изменил.
    pub fn gesture(&mut self, g: Gesture, pos: (i32, i32)) -> bool {
        if self.gestures == 0 || !self.plane.in_bounds(pos.0, pos.1) {
            return false;
        }
        let done = match g {
            Gesture::Paint(m) => {
                // рука сильнее матрицы: краска ложится куда велено
                self.plane.set(pos.0, pos.1, m.tile());
                true
            }
            Gesture::Erase => {
                if self.plane.get(pos.0, pos.1) == TileKind::Empty {
                    false
                } else {
                    self.plane.set(pos.0, pos.1, TileKind::Empty);
                    true
                }
            }
            Gesture::Mend => match self.plane.tiles.get_mut(&pos) {
                Some(t) if t.decay > 0 => {
                    t.decay -= 1;
                    true
                }
                _ => false,
            },
        };
        if done {
            self.gestures -= 1;
        }
        done
    }

    /// Собрать мир из частей сейва (см. save.rs).
    #[allow(clippy::too_many_arguments)]
    pub fn from_parts(
        deck_dir: PathBuf,
        order: &[String],
        plane: Plane,
        seeds: Vec<Seed>,
        settlements: Vec<Settlement>,
        entropy: Option<EntropyState>,
        archivist: Archivist,
        cycle: Option<CycleState>,
        stats: (usize, u32),
        tick: u64,
        draw_count: u64,
        last_draw_tick: u64,
        seed: u64,
        cursor: (i32, i32),
    ) -> World {
        let deck = if order.is_empty() {
            Deck::from_dir(&deck_dir).unwrap_or_default()
        } else {
            Deck::from_order(&deck_dir, order)
        };
        let next_seed_id = seeds.iter().map(|s| s.id + 1).max().unwrap_or(0);
        let next_settlement_id = settlements.iter().map(|s| s.id + 1).max().unwrap_or(0);
        let entropy = entropy.unwrap_or_default();
        // законы — производные от файлов скрижалей, в сейве их нет
        let tablets_dir = Tablets::dir_for(&deck_dir);
        let _ = Tablets::ensure_defaults(&tablets_dir);
        let tablets = Tablets::load(&tablets_dir);
        let laws = tablets.laws();
        let gestures = laws.heart.gestures.unwrap_or(0);
        World {
            plane,
            seeds,
            settlements,
            entropy,
            archivist,
            cycle: cycle.unwrap_or_else(|| CycleState::new(1)),
            tablets,
            laws,
            deck,
            deck_dir,
            tick,
            draw_count,
            last_draw_tick,
            seed,
            cursor,
            peak_filled: stats.0,
            hearths_founded: stats.1,
            gestures,
            next_seed_id,
            next_settlement_id,
        }
    }

    pub fn deck_dir(&self) -> &Path {
        &self.deck_dir
    }
}

/// Свободное имя для копии карты: «лес-2.md», «лес-3.md»…
fn free_copy_path(path: &Path) -> PathBuf {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("card");
    let dir = path.parent().unwrap_or(Path::new("."));
    for n in 2..100 {
        let candidate = dir.join(format!("{stem}-{n}.md"));
        if !candidate.exists() {
            return candidate;
        }
    }
    dir.join(format!("{stem}-copy.md"))
}
