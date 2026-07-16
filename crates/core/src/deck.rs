//! Колода — очередь файлов карт из папки `Deck/`. Тяга снимает верхнюю карту
//! и кладёт её в низ: колода циклична, карты не теряются (дух Грецингера).

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct Deck {
    pub cards: VecDeque<PathBuf>,
}

impl Deck {
    /// Собрать колоду из `*.md` в папке, порядок — по имени файла.
    pub fn from_dir(dir: &Path) -> std::io::Result<Deck> {
        let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().map(|e| e == "md").unwrap_or(false))
            .collect();
        paths.sort();
        Ok(Deck {
            cards: paths.into(),
        })
    }

    /// Восстановить порядок из сейва: имена → пути в `dir` (пропуская пропавшие).
    pub fn from_order(dir: &Path, order: &[String]) -> Deck {
        let cards = order
            .iter()
            .map(|name| dir.join(name))
            .filter(|p| p.exists())
            .collect();
        Deck { cards }
    }

    pub fn top(&self) -> Option<&PathBuf> {
        self.cards.front()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// Снять верхнюю карту и положить её в низ (цикл).
    pub fn draw(&mut self) -> Option<PathBuf> {
        let card = self.cards.pop_front()?;
        self.cards.push_back(card.clone());
        Some(card)
    }

    /// Порядок колоды именами файлов — для сейва.
    pub fn order(&self) -> Vec<String> {
        self.cards
            .iter()
            .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(String::from))
            .collect()
    }

    /// Перетасовать (Fisher–Yates по данному RNG — детерминировано от тяги).
    pub fn shuffle(&mut self, rng: &mut rand::rngs::StdRng) {
        use rand::Rng;
        let mut v: Vec<PathBuf> = self.cards.drain(..).collect();
        for i in (1..v.len()).rev() {
            let j = rng.gen_range(0..=i);
            v.swap(i, j);
        }
        self.cards = v.into();
    }

    /// Положить карту сразу под верх колоды.
    pub fn insert_under_top(&mut self, path: PathBuf) {
        let at = 1.min(self.cards.len());
        self.cards.insert(at, path);
    }

    /// Убрать карту по индексу (для Забвения).
    pub fn remove(&mut self, index: usize) -> Option<PathBuf> {
        self.cards.remove(index)
    }
}
