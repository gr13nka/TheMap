//! Рунный язык — язык карт и законов мира. Грамматика в две строки,
//! в духе eval Бёрда:
//!
//! ```text
//! Expr ::= Atom | "(" Expr* ")"
//! Atom ::= Rune | Num
//! ```
//!
//! Смысл задаётся позицией, как в Лиспе: голова верхнего списка — материя
//! (субстанция карты), вложенный список с глаголом в голове — клауза-
//! поведение, `(⏱ N …)` — форма ритма. Один язык описывает и карты колоды,
//! и скрижали-законы мира — интерпретатор написан на языке, который он
//! интерпретирует; в этом красота.

pub mod compile;
pub mod mutate;
pub mod parse;
pub mod print;

use serde::{Deserialize, Serialize};

use crate::tile::TileKind;

pub use compile::{compile, Clause, Only, Program, Verb};
pub use parse::parse;
pub use print::pretty;

/// Материя — то, что ложится краской на бумагу. Живёт в рунном модуле:
/// материи — первые слова языка.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Matter {
    Water,
    Wood,
    Stone,
    Meadow,
    Voidness,
    Hearth,
    Ruin,
}

impl Matter {
    pub fn tile(self) -> TileKind {
        match self {
            Matter::Water => TileKind::Water,
            Matter::Wood => TileKind::Forest,
            Matter::Stone => TileKind::Stone,
            Matter::Meadow => TileKind::Meadow,
            Matter::Voidness => TileKind::Void,
            Matter::Hearth => TileKind::Hearth,
            Matter::Ruin => TileKind::Ruin,
        }
    }

    /// Слово для хроники: материя видна глазами, её можно называть.
    pub fn ru(self) -> &'static str {
        match self {
            Matter::Water => "вода",
            Matter::Wood => "древо",
            Matter::Stone => "камень",
            Matter::Meadow => "луг",
            Matter::Voidness => "пустота",
            Matter::Hearth => "очаг",
            Matter::Ruin => "руина",
        }
    }
}

/// Руна — односимвольный атом языка. Безымянна в интерфейсе: смысл
/// познаётся наблюдением. Все символы централизованы здесь — одна точка
/// замены, если глиф не рендерится у пользователя.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Rune {
    // --- материи ---
    Water,
    Wood,
    Stone,
    Meadow,
    Voidness,
    Hearth,
    Ruin,
    // --- глаголы ---
    Flow,
    Branch,
    Creep,
    Gnaw,
    // --- прицелы: стрелка — вектор; стрелка+материя — тяга к ней ---
    Up,
    Down,
    Left,
    #[serde(alias = "Seek")]
    Right,
    /// ↷ — прочь от материи / в обход её.
    Away,
    // --- время ---
    /// ⏱ — форма «каждые N тиков».
    Every,
    /// ⌛ — срок жизни / перевал возраста.
    Lifespan,
    /// ! — взрыв: втрое быстрее, втрое короче.
    Burst,
    /// + — обилие: темп +1.
    Abundant,
    // --- предикаты ---
    /// ♥ — только живое (луг, лес, очаг, тропа).
    Living,
    // --- законные руны: читаются скрижалями, в картах молчат ---
    /// ∈ — входит-из.
    EntersAt,
    /// ▢ — кромка листа.
    LeafEdge,
    /// ✶ — рождаться.
    BeBorn,
    /// ▲ — предел.
    Limit,
    /// ✺ — внутренний очаг распада.
    InnerBlight,
    /// ♡ — сердце (ритм тяги).
    Heart,
    /// ✋ — рука (бюджет жестов).
    Hand,
    /// ☾ — дрейф (мутация карт).
    DriftMoon,
    /// † — порог смерти.
    DeathMark,
    /// ▒ — стадия умирания.
    StageMid,
    /// █ — стадия смерти.
    StageFull,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Matter,
    Verb,
    Aim,
    Time,
    Predicate,
    Law,
}

impl Rune {
    pub fn ch(self) -> char {
        match self {
            Rune::Water => '~',
            Rune::Wood => '♠',
            Rune::Stone => '^',
            Rune::Meadow => ',',
            Rune::Voidness => '░',
            Rune::Hearth => '#',
            Rune::Ruin => '⌂',
            Rune::Flow => '∩',
            Rune::Branch => 'Y',
            Rune::Creep => '∴',
            Rune::Gnaw => '×',
            Rune::Up => '↑',
            Rune::Down => '↓',
            Rune::Left => '←',
            Rune::Right => '→',
            Rune::Away => '↷',
            Rune::Every => '⏱',
            Rune::Lifespan => '⌛',
            Rune::Burst => '!',
            Rune::Abundant => '+',
            Rune::Living => '♥',
            Rune::EntersAt => '∈',
            Rune::LeafEdge => '▢',
            Rune::BeBorn => '✶',
            Rune::Limit => '▲',
            Rune::InnerBlight => '✺',
            Rune::Heart => '♡',
            Rune::Hand => '✋',
            Rune::DriftMoon => '☾',
            Rune::DeathMark => '†',
            Rune::StageMid => '▒',
            Rune::StageFull => '█',
        }
    }

    pub fn from_char(c: char) -> Option<Rune> {
        Some(match c {
            '~' => Rune::Water,
            '♠' => Rune::Wood,
            '^' => Rune::Stone,
            ',' => Rune::Meadow,
            '░' => Rune::Voidness,
            '#' => Rune::Hearth,
            '⌂' => Rune::Ruin,
            '∩' => Rune::Flow,
            'Y' => Rune::Branch,
            '∴' => Rune::Creep,
            '×' => Rune::Gnaw,
            '↑' => Rune::Up,
            '↓' => Rune::Down,
            '←' => Rune::Left,
            '→' => Rune::Right,
            '↷' => Rune::Away,
            '⏱' => Rune::Every,
            '⌛' => Rune::Lifespan,
            '!' => Rune::Burst,
            '+' => Rune::Abundant,
            '♥' => Rune::Living,
            '∈' => Rune::EntersAt,
            '▢' => Rune::LeafEdge,
            '✶' => Rune::BeBorn,
            '▲' => Rune::Limit,
            '✺' => Rune::InnerBlight,
            '♡' => Rune::Heart,
            '✋' => Rune::Hand,
            '☾' => Rune::DriftMoon,
            '†' => Rune::DeathMark,
            '▒' => Rune::StageMid,
            '█' => Rune::StageFull,
            _ => return None,
        })
    }

    pub fn family(self) -> Family {
        match self {
            Rune::Water | Rune::Wood | Rune::Stone | Rune::Meadow | Rune::Voidness
            | Rune::Hearth | Rune::Ruin => Family::Matter,
            Rune::Flow | Rune::Branch | Rune::Creep | Rune::Gnaw => Family::Verb,
            Rune::Up | Rune::Down | Rune::Left | Rune::Right | Rune::Away => Family::Aim,
            Rune::Every | Rune::Lifespan | Rune::Burst | Rune::Abundant => Family::Time,
            Rune::Living => Family::Predicate,
            _ => Family::Law,
        }
    }

    pub fn as_matter(self) -> Option<Matter> {
        Some(match self {
            Rune::Water => Matter::Water,
            Rune::Wood => Matter::Wood,
            Rune::Stone => Matter::Stone,
            Rune::Meadow => Matter::Meadow,
            Rune::Voidness => Matter::Voidness,
            Rune::Hearth => Matter::Hearth,
            Rune::Ruin => Matter::Ruin,
            _ => return None,
        })
    }

    /// Вектор стрелки; None — не стрелка. Экранный y растёт вниз.
    pub fn as_dir(self) -> Option<(i8, i8)> {
        Some(match self {
            Rune::Up => (0, -1),
            Rune::Down => (0, 1),
            Rune::Left => (-1, 0),
            Rune::Right => (1, 0),
            _ => return None,
        })
    }
}

/// Выражение языка. Всё, что есть у карты и у закона, — одно такое дерево.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    Rune(Rune),
    Num(u32),
    List(Vec<Expr>),
}

impl Expr {
    pub fn empty() -> Expr {
        Expr::List(Vec::new())
    }
}

/// Сколько тиков живёт песочница превью (ритмы должны успеть пульснуть).
const PREVIEW_TICKS: u64 = 96;
const PREVIEW_FRAME_EVERY: u64 = 16;
const PREVIEW_SEED: u64 = 0xC0FF_EE;

/// Зыбкое превью для крафта: прогнать выражение в песочнице и отдать кадры.
/// Клиент сжимает их в мутное цветовое пятно — намёк, не описание.
/// Инертное выражение — пустое пятно, и это честный ответ.
pub fn preview(expr: &Expr) -> Vec<crate::plane::Plane> {
    let Some(program) = compile(expr) else {
        return Vec::new();
    };
    let mut plane = crate::plane::Plane::new(32, 22);
    let mut seeds = vec![crate::sim::seed::Seed::spawn(
        0,
        program,
        (16, 14),
        &mut plane,
    )];
    let mut events = Vec::new();
    let mut frames = Vec::new();
    for t in 1..=PREVIEW_TICKS {
        crate::sim::step_seeds(&mut plane, &mut seeds, PREVIEW_SEED, t, &mut events);
        if t % PREVIEW_FRAME_EVERY == 0 {
            frames.push(plane.clone());
        }
    }
    frames
}
