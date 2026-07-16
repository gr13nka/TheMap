//! Загрузка палитры «Гуашь на тёмной бумаге» из palette.ron. Каждый цвет
//! именован из мира; берём truecolor (hex). `core` глифов и цветов не знает —
//! маппинг живёт здесь, в клиенте.

use std::collections::HashMap;
use std::path::Path;

use ratatui::style::Color;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct PaletteFile {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    title: String,
    colors: Vec<ColorDef>,
}

#[derive(Debug, Deserialize)]
struct ColorDef {
    id: String,
    #[allow(dead_code)]
    ru: String,
    hex: String,
    #[allow(dead_code)]
    x256: u8,
}

pub struct Palette {
    by_id: HashMap<String, Color>,
}

impl Palette {
    /// Загрузить из файла; при любой беде — встроенный фолбэк, чтобы клиент
    /// всегда запускался.
    pub fn load(path: &Path) -> Palette {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| ron::from_str::<PaletteFile>(&text).ok())
            .map(|file| {
                let by_id = file
                    .colors
                    .into_iter()
                    .filter_map(|c| parse_hex(&c.hex).map(|col| (c.id, col)))
                    .collect();
                Palette { by_id }
            })
            .unwrap_or_else(Palette::fallback)
    }

    pub fn get(&self, id: &str) -> Color {
        self.by_id.get(id).copied().unwrap_or(Color::Reset)
    }

    fn fallback() -> Palette {
        let mut by_id = HashMap::new();
        by_id.insert("paper".to_string(), Color::Rgb(22, 19, 14));
        by_id.insert("void".to_string(), Color::Rgb(10, 9, 8));
        by_id.insert("void_edge_far".to_string(), Color::Rgb(42, 38, 32));
        by_id.insert("void_edge_near".to_string(), Color::Rgb(56, 51, 42));
        by_id.insert("frame".to_string(), Color::Rgb(44, 40, 32));
        by_id.insert("ink".to_string(), Color::Rgb(216, 208, 191));
        by_id.insert("ink_faded".to_string(), Color::Rgb(168, 159, 140));
        by_id.insert("ink_muted".to_string(), Color::Rgb(111, 104, 90));
        by_id.insert("water".to_string(), Color::Rgb(79, 168, 154));
        by_id.insert("water_deep".to_string(), Color::Rgb(61, 138, 126));
        by_id.insert("ochre".to_string(), Color::Rgb(201, 162, 75));
        by_id.insert("moss".to_string(), Color::Rgb(122, 143, 78));
        by_id.insert("meadow".to_string(), Color::Rgb(107, 122, 78));
        by_id.insert("hills".to_string(), Color::Rgb(141, 131, 113));
        by_id.insert("brick".to_string(), Color::Rgb(192, 96, 63));
        by_id.insert("gold".to_string(), Color::Rgb(224, 180, 88));
        Palette { by_id }
    }
}

fn parse_hex(hex: &str) -> Option<Color> {
    let h = hex.trim_start_matches('#');
    if h.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()?;
    let g = u8::from_str_radix(&h[2..4], 16).ok()?;
    let b = u8::from_str_radix(&h[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}
