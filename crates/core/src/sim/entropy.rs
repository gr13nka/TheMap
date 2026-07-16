//! Энтропия — смертность листа, читаемая из Скрижали Пустоты. Здесь нет
//! зашитых констант: сколько кусать, когда перевал, что щадить — всё руны.
//! Выкинул укус из скрижали — мир бессмертен; утроил — сгорит за минуты.
//! Пустота выедает тайлы по стадиям (decay 0→░→▒→▓→дыра).

use rand::rngs::StdRng;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::event::Event;
use crate::plane::Plane;
use crate::tablet::EntropyLaw;
use crate::tile::{Tile, TileKind};

use super::creep::NEIGHBORS4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntropyState {
    /// Фронт — клетки, которые пустота сейчас точит (кромки + края дыр).
    pub front: Vec<(i32, i32)>,
    /// Дробный накопитель укусов.
    acc: f32,
    /// Событие «лист вянет» уже прозвучало.
    waned: bool,
    /// Кромки уже засеяны во фронт (лениво: закон могли переписать).
    #[serde(default)]
    edges_seeded: bool,
}

impl EntropyState {
    pub fn new() -> EntropyState {
        EntropyState {
            front: Vec::new(),
            acc: 0.0,
            waned: false,
            edges_seeded: false,
        }
    }

    /// Давление распада: укусов за тик по закону.
    fn pressure(&self, tick: u64, law: &EntropyLaw) -> f32 {
        let Some((every, bites)) = law.bite else {
            return 0.0; // закон молчит — мир бессмертен
        };
        let base = bites as f32 / every.max(1) as f32;
        match law.half_life {
            Some(half) => {
                let t = tick as f32 / half as f32;
                base * (1.0 + t * t)
            }
            None => base, // без перевала давление не растёт
        }
    }

    /// Один тик энтропии — буквальное исполнение скрижали.
    pub fn step(
        &mut self,
        plane: &mut Plane,
        rng: &mut StdRng,
        tick: u64,
        law: &EntropyLaw,
        events: &mut Vec<Event>,
    ) {
        // пустота входит с кромок — если так велит закон
        if law.enters_at_edges && !self.edges_seeded {
            self.edges_seeded = true;
            for x in 0..plane.w {
                self.front.push((x, 0));
                self.front.push((x, plane.h - 1));
            }
            for y in 1..plane.h - 1 {
                self.front.push((0, y));
                self.front.push((plane.w - 1, y));
            }
        }

        if let Some(half) = law.half_life {
            if tick == half && !self.waned {
                self.waned = true;
                events.push(Event::WorldWaning);
            }
            // после перевала пустота рождается и внутри листа
            if let Some(blight) = law.inner_blight {
                if tick > half && tick % blight == 0 {
                    let x = rng.gen_range(0..plane.w);
                    let y = rng.gen_range(0..plane.h);
                    self.front.push((x, y));
                }
            }
        }

        self.acc += self.pressure(tick, law);
        let mut guard = 0;
        while self.acc >= 1.0 && guard < 64 {
            self.acc -= 1.0;
            guard += 1;
            self.gnaw_once(plane, rng, law, events);
        }
    }

    /// Один укус: клетка фронта стареет на стадию.
    fn gnaw_once(
        &mut self,
        plane: &mut Plane,
        rng: &mut StdRng,
        law: &EntropyLaw,
        events: &mut Vec<Event>,
    ) {
        for _ in 0..8 {
            if self.front.is_empty() {
                return;
            }
            let idx = rng.gen_range(0..self.front.len());
            let (x, y) = self.front[idx];

            if !plane.in_bounds(x, y) || plane.get(x, y) == TileKind::Void {
                self.front.swap_remove(idx);
                continue;
            }

            let tile = plane
                .tiles
                .entry((x, y))
                .or_insert_with(|| Tile::new(TileKind::Empty));
            // щадимые материи держатся вдвое дольше — половина укусов мимо
            let resists = law.resists.iter().any(|m| m.tile() == tile.kind);
            if resists && rng.gen_bool(0.5) {
                return;
            }

            if tile.decay < 3 {
                tile.decay += 1;
                return;
            }

            // тайл истлел насквозь — дыра в бумаге
            let was = tile.kind;
            tile.kind = TileKind::Void;
            tile.decay = 0;
            if !matches!(was, TileKind::Empty) {
                events.push(Event::EntropyBreach { at: (x, y), was });
            }
            self.front.swap_remove(idx);
            for (dx, dy) in NEIGHBORS4 {
                let (nx, ny) = (x + dx, y + dy);
                if plane.in_bounds(nx, ny) && plane.get(nx, ny) != TileKind::Void {
                    self.front.push((nx, ny));
                }
            }
            return;
        }
    }
}

impl Default for EntropyState {
    fn default() -> Self {
        EntropyState::new()
    }
}
