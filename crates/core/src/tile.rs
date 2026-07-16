//! Тайл плоскости. `Empty` — бумага листа; остальное — краска материй.
//! `decay` — стадия истлевания под пустотой (0 = целый, 1..=3 — ░▒▓):
//! пустота выедает тайлы по стадиям, а не стирает (см. AESTHETICS.md).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TileKind {
    Empty,
    Forest,
    Water,
    Stone,
    Meadow,
    Void,
    Hearth,
    /// Тропа поселений.
    Path,
    /// Мёртвый очаг — пепел; след погибшей жизни (и прошлых миров).
    Ruin,
}

impl TileKind {
    /// Имя материи словом (для будущих карт/конфигов) → вид тайла.
    pub fn from_word(word: &str) -> TileKind {
        match word.trim() {
            "forest" => TileKind::Forest,
            "water" => TileKind::Water,
            "stone" => TileKind::Stone,
            "meadow" => TileKind::Meadow,
            "void" => TileKind::Void,
            "hearth" => TileKind::Hearth,
            "path" => TileKind::Path,
            "ruin" => TileKind::Ruin,
            _ => TileKind::Empty,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tile {
    pub kind: TileKind,
    /// Стадия истлевания 0..=3; старые сейвы без поля читаются как целые.
    #[serde(default)]
    pub decay: u8,
}

impl Tile {
    pub fn new(kind: TileKind) -> Self {
        Tile { kind, decay: 0 }
    }
}
