//! Скрижали — законы мира, записанные на языке карт. Интерпретатор,
//! написанный на языке, который он интерпретирует: пустота *грызёт* (×),
//! очаг *ветвится* (Y) и *лечит* (♥), сердце *пульсирует* (⏱). Мир исполняет
//! скрижали абсолютно честно: отсутствие клаузы = поведение не существует.
//! Выкинул укус — мир бессмертен; сломал скрижаль — закон молчит.
//! Никакой защиты от бога.

use std::path::{Path, PathBuf};

use crate::card::Card;
use crate::rune::{self, Expr, Matter, Rune};

/// Дефолтные скрижали — то, чем мир живёт, пока Правитель не вмешался.
pub const DEFAULT_ENTROPY: &str = "(░ (∈ ▢) (⏱ 2 ×) (⌛ 1200) (↷ ^ ⌂) (✺ 240))";
pub const DEFAULT_HEARTH: &str =
    "(# (✶ 48 , (→ ~ 5) (→ ♠ 8)) (Y 90 (, 3) (♠ 1) (▲ 8)) (♥ 2) (→ ⌂))";
pub const DEFAULT_HEART: &str = "(♡ (⏱ 90) (✋ 3) (☾ 12))";
pub const DEFAULT_THRESHOLD: &str = "(† (▒ 55) (█ 82) (⌛ 40))";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabletSlot {
    Entropy,
    Hearth,
    Heart,
    Threshold,
}

impl TabletSlot {
    pub const ALL: [TabletSlot; 4] = [
        TabletSlot::Entropy,
        TabletSlot::Hearth,
        TabletSlot::Heart,
        TabletSlot::Threshold,
    ];

    pub fn file(self) -> &'static str {
        match self {
            TabletSlot::Entropy => "entropy.md",
            TabletSlot::Hearth => "hearth.md",
            TabletSlot::Heart => "heart.md",
            TabletSlot::Threshold => "threshold.md",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            TabletSlot::Entropy => "Скрижаль Пустоты",
            TabletSlot::Hearth => "Скрижаль Очагов",
            TabletSlot::Heart => "Скрижаль Сердца",
            TabletSlot::Threshold => "Скрижаль Порога",
        }
    }

    fn default_expr(self) -> &'static str {
        match self {
            TabletSlot::Entropy => DEFAULT_ENTROPY,
            TabletSlot::Hearth => DEFAULT_HEARTH,
            TabletSlot::Heart => DEFAULT_HEART,
            TabletSlot::Threshold => DEFAULT_THRESHOLD,
        }
    }
}

/// Четыре выражения — весь интерпретатор мира.
#[derive(Debug, Clone)]
pub struct Tablets {
    pub entropy: Expr,
    pub hearth: Expr,
    pub heart: Expr,
    pub threshold: Expr,
}

impl Tablets {
    /// Папка скрижалей — рядом с колодой (законы старше листов).
    /// Если колода лежит не в каноничной `Deck/` — скрижали внутри неё
    /// (так у тестовых миров свои законы, не общие).
    pub fn dir_for(deck_dir: &Path) -> PathBuf {
        if deck_dir.file_name().and_then(|n| n.to_str()) == Some("Deck") {
            deck_dir
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join("Tablets")
        } else {
            deck_dir.join("Tablets")
        }
    }

    /// Первый запуск: если папки нет — создать её с дефолтами.
    /// Если папка есть, но файла нет — закон молчит (честно: его порвали).
    pub fn ensure_defaults(dir: &Path) -> std::io::Result<()> {
        if dir.is_dir() {
            return Ok(());
        }
        std::fs::create_dir_all(dir)?;
        for slot in TabletSlot::ALL {
            let body = format!(
                "---\nname: {}\nkind: tablet\n---\n\n# {}\n\nЗакон мира. Мир исполняет написанное буквально;\nпустая скрижаль — молчащий закон.\n\n```rune\n{}\n```\n",
                slot.title(),
                slot.title(),
                slot.default_expr()
            );
            std::fs::write(dir.join(slot.file()), body)?;
        }
        Ok(())
    }

    pub fn load(dir: &Path) -> Tablets {
        let read = |slot: TabletSlot| -> Expr {
            Card::parse_file(&dir.join(slot.file()))
                .ok()
                .and_then(|c| c.expr())
                .unwrap_or_else(Expr::empty)
        };
        Tablets {
            entropy: read(TabletSlot::Entropy),
            hearth: read(TabletSlot::Hearth),
            heart: read(TabletSlot::Heart),
            threshold: read(TabletSlot::Threshold),
        }
    }

    pub fn expr(&self, slot: TabletSlot) -> &Expr {
        match slot {
            TabletSlot::Entropy => &self.entropy,
            TabletSlot::Hearth => &self.hearth,
            TabletSlot::Heart => &self.heart,
            TabletSlot::Threshold => &self.threshold,
        }
    }

    pub fn laws(&self) -> Laws {
        Laws {
            entropy: read_entropy(&self.entropy),
            hearth: read_hearth(&self.hearth),
            heart: read_heart(&self.heart),
            threshold: read_threshold(&self.threshold),
        }
    }
}

/// Скомпилированные законы. Все поля — Option: молчание закона —
/// отсутствие поведения, без магических дефолтов-подпорок.
#[derive(Debug, Clone, Default)]
pub struct Laws {
    pub entropy: EntropyLaw,
    pub hearth: HearthLaw,
    pub heart: HeartLaw,
    pub threshold: ThresholdLaw,
}

#[derive(Debug, Clone, Default)]
pub struct EntropyLaw {
    /// (⏱ N × …): каждые N тиков — K укусов (K = сколько ×).
    pub bite: Option<(u32, u32)>,
    /// (⌛ N): перевал возраста; давление растёт ×(1+(t/N)²).
    pub half_life: Option<u64>,
    /// (↷ M …): материи, держащиеся вдвое дольше.
    pub resists: Vec<Matter>,
    /// (✺ N): внутренние очаги распада после перевала.
    pub inner_blight: Option<u64>,
    /// (∈ ▢): пустота входит с кромок листа.
    pub enters_at_edges: bool,
}

#[derive(Debug, Clone, Default)]
pub struct FoundRule {
    pub every: u64,
    /// На каких материях родится очаг (плюс чистая бумага).
    pub on: Vec<Matter>,
    /// Что должно быть рядом: (материя, квадрат расстояния).
    pub near: Vec<(Matter, i64)>,
}

#[derive(Debug, Clone, Default)]
pub struct GrowRule {
    pub every: u64,
    /// Прокорм: (материя, сколько в округе).
    pub food: Vec<(Matter, usize)>,
    /// (▲ N): предел размера.
    pub max_size: u8,
}

#[derive(Debug, Clone, Default)]
pub struct HearthLaw {
    pub found: Option<FoundRule>,
    pub grow: Option<GrowRule>,
    /// (♥ r): чинит распад в радиусе r.
    pub heal: Option<i32>,
    /// (→ M): к чему тянутся новые очаги.
    pub lure: Option<Matter>,
}

#[derive(Debug, Clone, Default)]
pub struct HeartLaw {
    /// (⏱ N): колода зовёт спустя N тиков после тяги.
    pub call_every: Option<u64>,
    /// (✋ N): бюджет жестов на тягу.
    pub gestures: Option<u8>,
    /// (☾ N): шанс дрейфа карты при тяге, %.
    pub drift_pct: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct ThresholdLaw {
    /// (▒ N): доля дыр, после которой мир доживает, %.
    pub dying_pct: Option<u32>,
    /// (█ N): доля дыр, после которой лист мёртв, %.
    pub dead_pct: Option<u32>,
    /// (⌛ N): грация — конец ещё виден N тиков.
    pub grace: Option<u64>,
}

/// Клаузы выражения (вложенные списки после головы).
fn clauses(expr: &Expr) -> impl Iterator<Item = &[Expr]> {
    let items: &[Expr] = match expr {
        Expr::List(l) => l,
        _ => &[],
    };
    items.iter().skip(1).filter_map(|e| match e {
        Expr::List(l) => Some(l.as_slice()),
        _ => None,
    })
}

fn first_num(items: &[Expr]) -> Option<u32> {
    items.iter().find_map(|e| match e {
        Expr::Num(n) => Some(*n),
        _ => None,
    })
}

fn matters_in(items: &[Expr]) -> Vec<Matter> {
    items
        .iter()
        .filter_map(|e| match e {
            Expr::Rune(r) => r.as_matter(),
            _ => None,
        })
        .collect()
}

fn read_entropy(expr: &Expr) -> EntropyLaw {
    let mut law = EntropyLaw::default();
    for c in clauses(expr) {
        match c.first() {
            Some(Expr::Rune(Rune::Every)) => {
                let n = first_num(c).unwrap_or(1).max(1);
                let bites = c
                    .iter()
                    .filter(|e| matches!(e, Expr::Rune(Rune::Gnaw)))
                    .count() as u32;
                if bites > 0 {
                    law.bite = Some((n, bites));
                }
            }
            Some(Expr::Rune(Rune::Lifespan)) => {
                law.half_life = first_num(c).map(|n| n.max(1) as u64);
            }
            Some(Expr::Rune(Rune::Away)) => law.resists = matters_in(c),
            Some(Expr::Rune(Rune::InnerBlight)) => {
                law.inner_blight = first_num(c).map(|n| n.max(1) as u64);
            }
            Some(Expr::Rune(Rune::EntersAt)) => {
                law.enters_at_edges = c
                    .iter()
                    .any(|e| matches!(e, Expr::Rune(Rune::LeafEdge)));
            }
            _ => {}
        }
    }
    law
}

fn read_hearth(expr: &Expr) -> HearthLaw {
    let mut law = HearthLaw::default();
    for c in clauses(expr) {
        match c.first() {
            Some(Expr::Rune(Rune::BeBorn)) => {
                let mut rule = FoundRule {
                    every: first_num(c).unwrap_or(48).max(1) as u64,
                    on: Vec::new(),
                    near: Vec::new(),
                };
                for e in c.iter().skip(1) {
                    match e {
                        Expr::Rune(r) => {
                            if let Some(m) = r.as_matter() {
                                rule.on.push(m);
                            }
                        }
                        Expr::List(inner) => {
                            // (→ M d): рядом с материей M не дальше d
                            if matches!(inner.first(), Some(Expr::Rune(Rune::Right))) {
                                if let (Some(m), Some(d)) =
                                    (matters_in(inner).first().copied(), first_num(inner))
                                {
                                    rule.near.push((m, (d as i64) * (d as i64)));
                                }
                            }
                        }
                        _ => {}
                    }
                }
                law.found = Some(rule);
            }
            Some(Expr::Rune(Rune::Branch)) => {
                let mut rule = GrowRule {
                    every: first_num(c).unwrap_or(90).max(1) as u64,
                    food: Vec::new(),
                    max_size: 8,
                };
                for e in c.iter().skip(1) {
                    if let Expr::List(inner) = e {
                        match inner.first() {
                            Some(Expr::Rune(Rune::Limit)) => {
                                rule.max_size = first_num(inner).unwrap_or(8).max(1) as u8;
                            }
                            Some(Expr::Rune(r)) => {
                                if let Some(m) = r.as_matter() {
                                    rule.food
                                        .push((m, first_num(inner).unwrap_or(1) as usize));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                law.grow = Some(rule);
            }
            Some(Expr::Rune(Rune::Living)) => {
                law.heal = first_num(c).map(|n| n as i32);
            }
            Some(Expr::Rune(Rune::Right)) => {
                law.lure = matters_in(c).first().copied();
            }
            _ => {}
        }
    }
    law
}

fn read_heart(expr: &Expr) -> HeartLaw {
    let mut law = HeartLaw::default();
    for c in clauses(expr) {
        match c.first() {
            Some(Expr::Rune(Rune::Every)) => {
                law.call_every = first_num(c).map(|n| n.max(1) as u64);
            }
            Some(Expr::Rune(Rune::Hand)) => {
                law.gestures = first_num(c).map(|n| n.min(9) as u8);
            }
            Some(Expr::Rune(Rune::DriftMoon)) => {
                law.drift_pct = first_num(c).map(|n| n.min(100));
            }
            _ => {}
        }
    }
    law
}

fn read_threshold(expr: &Expr) -> ThresholdLaw {
    let mut law = ThresholdLaw::default();
    for c in clauses(expr) {
        match c.first() {
            Some(Expr::Rune(Rune::StageMid)) => law.dying_pct = first_num(c).map(|n| n.min(100)),
            Some(Expr::Rune(Rune::StageFull)) => law.dead_pct = first_num(c).map(|n| n.min(100)),
            Some(Expr::Rune(Rune::Lifespan)) => law.grace = first_num(c).map(|n| n as u64),
            _ => {}
        }
    }
    law
}

/// Записать скрижаль обратно (тем же write_rune, что и карты).
pub fn write(dir: &Path, slot: TabletSlot, expr: &Expr) -> std::io::Result<()> {
    let path = dir.join(slot.file());
    if path.exists() {
        rune_write(&path, expr)
    } else {
        let body = format!(
            "---\nname: {}\nkind: tablet\n---\n\n```rune\n{}\n```\n",
            slot.title(),
            rune::pretty(expr)
        );
        std::fs::write(path, body)
    }
}

fn rune_write(path: &Path, expr: &Expr) -> std::io::Result<()> {
    crate::card::write_rune(path, expr)
}
