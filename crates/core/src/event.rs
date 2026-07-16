//! События мира — сырьё для архивариуса (хроника) и будущих анлоков.
//! Симуляция сообщает, что случилось; кто и как об этом расскажет —
//! не её дело. Пороги значимости и кулдауны живут в архивариусе.

use serde::{Deserialize, Serialize};

use crate::cycle::CycleSummary;
use crate::rune::Matter;
use crate::tile::TileKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    /// Посев истлел — его краска остаётся на бумаге.
    SeedDied { id: u64, matter: Matter, placed: usize },
    /// Одна материя пожрала другую (вода съела луг и т.п.).
    MatterClash {
        winner: Matter,
        loser: TileKind,
        at: (i32, i32),
    },
    SettlementFounded { id: u64, pos: (i32, i32) },
    SettlementGrew { id: u64, pos: (i32, i32), size: u8 },
    SettlementDied { id: u64, pos: (i32, i32) },
    /// Пустота прогрызла живое насквозь.
    EntropyBreach { at: (i32, i32), was: TileKind },
    /// Лист начал вянуть (перевал энтропии).
    WorldWaning,
    /// Пустота обняла больше половины листа — мир доживает.
    WorldDying,
    /// Лист мёртв; эпилог у архивариуса, наследие — у Правителя.
    WorldDead { summary: CycleSummary },
}
