//! Плоскость — разреженная сетка тайлов в `HashMap`. В коде нет глобального
//! размера мира как мировоззрения: `w`/`h` — лишь текущее окно рисования,
//! параметр, а не константа (см. PROJECT_BRIEF).
//!
//! ИНВАРИАНТ ДЕТЕРМИНИЗМА: порядок обхода `tiles` не определён, поэтому
//! HashMap НЕЛЬЗЯ итерировать для поведения (выбора клетки, порядка
//! действий). Разрешены только порядко-независимые агрегаты (min, count,
//! сумма) и явные упорядоченные списки (`frontier` посевов и т.п.).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::tile::{Tile, TileKind};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plane {
    pub tiles: HashMap<(i32, i32), Tile>,
    pub w: i32,
    pub h: i32,
}

impl Plane {
    pub fn new(w: i32, h: i32) -> Self {
        Plane {
            tiles: HashMap::new(),
            w,
            h,
        }
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h
    }

    pub fn set(&mut self, x: i32, y: i32, kind: TileKind) {
        if self.in_bounds(x, y) {
            self.tiles.insert((x, y), Tile::new(kind));
        }
    }

    pub fn get(&self, x: i32, y: i32) -> TileKind {
        self.tiles
            .get(&(x, y))
            .map(|t| t.kind)
            .unwrap_or(TileKind::Empty)
    }

    /// Глиф текущего тайла (плейсхолдеры из AESTHETICS.md).
    pub fn glyph(&self, x: i32, y: i32) -> char {
        match self.get(x, y) {
            TileKind::Empty => ' ',
            TileKind::Forest => '♠',
            TileKind::Water => '~',
            TileKind::Stone => '^',
            TileKind::Meadow => ',',
            TileKind::Void => '░',
            TileKind::Hearth => '#',
            TileKind::Path => '·',
            TileKind::Ruin => '#',
        }
    }

    /// Снимок плоскости глифами — для MAP.md и снапшот-тестов.
    pub fn render_glyphs(&self) -> String {
        let mut s = String::with_capacity(((self.w + 1) * self.h) as usize);
        for y in 0..self.h {
            for x in 0..self.w {
                s.push(self.glyph(x, y));
            }
            s.push('\n');
        }
        s
    }

    /// Сколько клеток занято (не бумага) — для хроники/тестов.
    pub fn filled(&self) -> usize {
        self.tiles
            .values()
            .filter(|t| t.kind != TileKind::Empty)
            .count()
    }

    /// Сколько клеток данного вида (порядко-независимый агрегат).
    pub fn count(&self, kind: TileKind) -> usize {
        self.tiles.values().filter(|t| t.kind == kind).count()
    }
}
