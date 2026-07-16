//! Наследие — единственное, что переживает смерть мира. Три его формы:
//! руины (материя — проступят на следующем листе), хроника (рассказ —
//! архив эпох ведёт клиент) и знание Правителя (открытые глифы и атлас
//! наблюдений). Хранится в legacy.ron — одном кросс-цикловом файле.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::cycle::CycleSummary;
use crate::event::Event;
use crate::rune::{Matter, Rune};
use crate::tile::TileKind;
use crate::world::World;

/// След погибшего мира на новом листе.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ruin {
    pub pos: (i32, i32),
    pub kind: TileKind,
}

/// Засвидетельствованное наблюдение — страница атласа. Ключ комбинации
/// (`combo`) — внутренний, в интерфейсе не показывается никогда: знание
/// Правителя — это цитаты хроники, не имена.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub matter: Matter,
    pub epoch: u32,
    pub quote: String,
    #[serde(default)]
    pub combo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Legacy {
    /// Номер текущего листа (растёт при смерти мира).
    pub epoch: u32,
    pub unlocked: Vec<Rune>,
    pub atlas: Vec<Observation>,
    /// Руины последнего умершего мира — проступят на следующем листе.
    pub ruins: Vec<Ruin>,
    pub summaries: Vec<CycleSummary>,
    /// Чистые листы — право создать карту с нуля. Дарит только смерть
    /// мира: язык растёт через прожитые жизни.
    #[serde(default)]
    pub blank_cards: u8,
}

impl Default for Legacy {
    fn default() -> Legacy {
        Legacy {
            epoch: 1,
            // рука Правителя в начале: простые материи, мирные глаголы
            // и стрелки — базовая грамматика языка
            unlocked: vec![
                Rune::Water,
                Rune::Wood,
                Rune::Meadow,
                Rune::Creep,
                Rune::Branch,
                Rune::Abundant,
                Rune::Up,
                Rune::Down,
                Rune::Left,
                Rune::Right,
            ],
            atlas: Vec::new(),
            ruins: Vec::new(),
            summaries: Vec::new(),
            blank_cards: 0,
        }
    }
}

impl Legacy {
    pub fn load(path: &Path) -> Legacy {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| ron::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let text = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, text)
    }

    pub fn is_unlocked(&self, g: Rune) -> bool {
        self.unlocked.contains(&g)
    }

    fn unlock(&mut self, g: Rune, newly: &mut Vec<Rune>) {
        if !self.unlocked.contains(&g) {
            self.unlocked.push(g);
            newly.push(g);
        }
    }

    /// Засвидетельствовать события тика; вернуть свежеоткрытые глифы.
    /// Открытия — плод наблюдений: мир показал поведение, рука его запомнила.
    pub fn witness(&mut self, events: &[Event]) -> Vec<Rune> {
        let mut newly = Vec::new();
        for ev in events {
            match ev {
                Event::MatterClash { .. } => self.unlock(Rune::Gnaw, &mut newly),
                Event::WorldWaning => {
                    self.unlock(Rune::Stone, &mut newly);
                    self.unlock(Rune::Lifespan, &mut newly);
                }
                Event::SeedDied {
                    matter: Matter::Water,
                    ..
                } => self.unlock(Rune::Flow, &mut newly),
                Event::SettlementFounded { .. } => self.unlock(Rune::Hearth, &mut newly),
                Event::SettlementGrew { size, .. } if *size >= 4 => {
                    self.unlock(Rune::Away, &mut newly)
                }
                _ => {}
            }
        }
        newly
    }

    /// Принять смерть мира: собрать руины, записать итог, открыть эпоху,
    /// подарить чистый лист. Возвращает руны, открытые самой смертью.
    pub fn absorb_death(&mut self, world: &World, summary: CycleSummary) -> Vec<Rune> {
        let mut newly = Vec::new();
        // пережить первую смерть мира — получить оружие врага, руины
        // и право читать (и переписывать) законы: законные руны
        self.unlock(Rune::Voidness, &mut newly);
        self.unlock(Rune::Ruin, &mut newly);
        self.unlock(Rune::Living, &mut newly);
        for law_rune in [
            Rune::EntersAt,
            Rune::LeafEdge,
            Rune::BeBorn,
            Rune::Limit,
            Rune::InnerBlight,
            Rune::Heart,
            Rune::Hand,
            Rune::DriftMoon,
            Rune::DeathMark,
            Rune::StageMid,
            Rune::StageFull,
        ] {
            self.unlock(law_rune, &mut newly);
        }
        if summary.draws >= 10 {
            self.unlock(Rune::Burst, &mut newly);
            self.unlock(Rune::Every, &mut newly);
        }
        self.ruins = harvest_ruins(world);
        self.summaries.push(summary);
        self.epoch += 1;
        // смерть даёт бумагу: единственный источник карт с нуля
        self.blank_cards = self.blank_cards.saturating_add(1);
        newly
    }
}

/// Собрать руины с мёртвого листа: пепел очагов и остовы камня.
/// Порядок — сортировкой (обход HashMap не влияет на результат).
fn harvest_ruins(world: &World) -> Vec<Ruin> {
    let mut hearths: Vec<(i32, i32)> = Vec::new();
    let mut stones: Vec<(i32, i32)> = Vec::new();
    for (&pos, tile) in &world.plane.tiles {
        match tile.kind {
            TileKind::Hearth | TileKind::Ruin => hearths.push(pos),
            TileKind::Stone => stones.push(pos),
            _ => {}
        }
    }
    hearths.sort_unstable();
    stones.sort_unstable();

    let mut ruins: Vec<Ruin> = hearths
        .into_iter()
        .take(40)
        .map(|pos| Ruin {
            pos,
            kind: TileKind::Ruin,
        })
        .collect();
    // камень переживает миры прореженным — остовы, не хребты
    ruins.extend(stones.into_iter().step_by(3).take(40).map(|pos| Ruin {
        pos,
        kind: TileKind::Stone,
    }));
    ruins
}
