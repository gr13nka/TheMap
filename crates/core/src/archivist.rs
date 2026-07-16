//! Архивариус — рассказчик мира и его свидетель. Читает события симуляции
//! и складывает строки хроники: повествование, не лог (AESTHETICS.md).
//! У него есть пороги значимости и память: первая встреча материй — событие,
//! сотая — тишина. Хроника — единственный источник истины об эффектах
//! глифов: материю можно называть (она видна глазами), узоры — никогда.
//! Запись в History.md делает клиент: ядро остаётся headless.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::event::Event;
use crate::rune::Matter;
use crate::tile::TileKind;
use crate::world::DrawOutcome;

/// Кулдаун между жалобами на прорывы пустоты, тиков.
const BREACH_COOLDOWN: u64 = 300;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Archivist {
    /// Ключи уже рассказанных «первых встреч».
    seen: HashSet<String>,
    last_breach_tick: u64,
}

impl Archivist {
    /// Превратить события тика в строки хроники (сколько заслуживают слов).
    pub fn narrate(&mut self, events: &[Event], tick: u64) -> Vec<String> {
        let mut lines = Vec::new();
        for ev in events {
            match ev {
                Event::SeedDied { matter, placed, .. } => {
                    if *placed >= 8 {
                        lines.push(seed_epitaph(*matter));
                    }
                }
                Event::MatterClash { winner, loser, .. } => {
                    let key = format!("clash:{winner:?}:{loser:?}");
                    if self.seen.insert(key) {
                        lines.push(format!(
                            "Впервые {} — мир учится жестокости.",
                            clash_phrase(*winner, *loser)
                        ));
                    }
                }
                Event::SettlementFounded { pos, .. } => {
                    if self.seen.insert("first_hearth".into()) {
                        lines.push(format!(
                            "У воды затеплился первый очаг ({}, {}) — в мир пришли люди.",
                            pos.0, pos.1
                        ));
                    } else {
                        lines.push(format!("Ещё один очаг поднялся у ({}, {}).", pos.0, pos.1));
                    }
                }
                Event::SettlementGrew { pos, size, .. } => {
                    let word = match size {
                        2 => "двор окреп в деревню",
                        4 => "деревня разрослась в село",
                        _ => "село выросло в город",
                    };
                    lines.push(format!("У ({}, {}) {}.", pos.0, pos.1, word));
                }
                Event::SettlementDied { pos, .. } => {
                    lines.push(format!(
                        "Очаг у ({}, {}) погас; остались пепел и тропы, которые никуда не ведут.",
                        pos.0, pos.1
                    ));
                }
                Event::EntropyBreach { was, .. } => {
                    if tick.saturating_sub(self.last_breach_tick) >= BREACH_COOLDOWN {
                        self.last_breach_tick = tick;
                        lines.push(format!(
                            "Пустота прогрызла {} — в листе снова дыра.",
                            tile_accusative(*was)
                        ));
                    }
                }
                Event::WorldWaning => {
                    lines.push(
                        "Лист начал желтеть. Пустота теперь дышит глубже, и всё, что \
                         поднимется, поднимется ненадолго."
                            .to_string(),
                    );
                }
                Event::WorldDying => {
                    lines.push(
                        "Пустота обняла больше половины листа. Мир доживает — \
                         и я записываю быстрее, чем он исчезает."
                            .to_string(),
                    );
                }
                Event::WorldDead { .. } => {
                    // эпилог звучит в ритуале смерти, не в ленте
                }
            }
        }
        lines
    }
}

/// Вехи глубины: чем этот мир превзошёл предков. Тихие, не ачивки.
pub fn milestones(
    summary: &crate::cycle::CycleSummary,
    past: &[crate::cycle::CycleSummary],
) -> Vec<String> {
    if past.is_empty() {
        return vec!["Первый мир Правителя — с него начинается стопка.".to_string()];
    }
    let mut m = Vec::new();
    let name = roman(summary.epoch);
    if summary.ticks_lived > past.iter().map(|s| s.ticks_lived).max().unwrap_or(0) {
        m.push(format!("Мир {name} прожил дольше всех своих предков."));
    }
    if summary.peak_filled > past.iter().map(|s| s.peak_filled).max().unwrap_or(0) {
        m.push(format!("Мир {name} расцвёл гуще любого прежнего."));
    }
    if summary.hearths_founded > past.iter().map(|s| s.hearths_founded).max().unwrap_or(0) {
        m.push(format!("В мире {name} поднялось больше очагов, чем когда-либо."));
    }
    m
}

/// Эпилог мёртвого листа — свидетельство архивариуса о том, что осталось.
/// Три формы наследия: руины (материя), хроника (рассказ), знание (рука).
pub fn epilogue(
    summary: &crate::cycle::CycleSummary,
    ruins: usize,
    new_glyphs: usize,
    milestones: &[String],
) -> Vec<String> {
    let mut lines = vec![
        format!(
            "Лист {} прожил {} тиков и {} тяг; пустота забрала его весь.",
            roman(summary.epoch),
            summary.ticks_lived,
            summary.draws
        ),
        format!(
            "В лучший час на бумаге жило {} мазков краски; очагов поднялось {}.",
            summary.peak_filled, summary.hearths_founded
        ),
    ];
    lines.extend(milestones.iter().cloned());
    if ruins > 0 {
        lines.push(format!(
            "Следующий лист получит {} руин в наследство — новые очаги любят вставать на пепле старых.",
            ruins
        ));
    } else {
        lines.push("Руин не осталось: этот мир ушёл бесследно. Кроме этих строк.".to_string());
    }
    if new_glyphs > 0 {
        lines.push(format!(
            "Рука Правителя стала богаче: смерть мира открыла ей {} новых знаков.",
            new_glyphs
        ));
    }
    lines.push(
        "Смерть мира дарит чистый лист: новое слово рождается только из прожитой жизни."
            .to_string(),
    );
    lines.push(String::new());
    lines.push(
        "Миры уходят. Остаётся тот, кто помнит, тот, кто тянет снова, — \
         и эта хроника. Я записал всё."
            .to_string(),
    );
    // кода свидетеля — изредка, детерминированно от эпохи: вопрос «зачем»
    // держится открытым, не закрывается
    if summary.epoch % 3 == 0 {
        lines.push(String::new());
        lines.push(
            "Зачем Правитель тянет снова? Я не знаю. Он не может остановиться. \
             Возможно, это и есть ответ."
                .to_string(),
        );
    }
    lines
}

/// Номер листа — римской цифрой, как эпохи на полях рукописи.
pub fn roman(mut n: u32) -> String {
    if n == 0 {
        return "0".to_string();
    }
    let mut out = String::new();
    for (v, s) in [
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ] {
        while n >= v {
            out.push_str(s);
            n -= v;
        }
    }
    out
}

/// Строка хроники по исходу тяги — для History.md.
pub fn chronicle_line(draw_count: u64, outcome: &DrawOutcome) -> String {
    match outcome.matter {
        None => format!(
            "Тяга {}. «{}» — карта легла, но на бумаге ни следа.",
            draw_count, outcome.card_name
        ),
        Some(matter) => format!(
            "Тяга {}. «{}» — из ({}, {}) проступает {}.",
            draw_count,
            outcome.card_name,
            outcome.origin.0,
            outcome.origin.1,
            matter.ru()
        ),
    }
}

/// Эпитафия посеву: материя дожила свой век, краска осталась.
fn seed_epitaph(matter: Matter) -> String {
    match matter {
        Matter::Water => "Река дотекла до своего предела и замерла.".to_string(),
        Matter::Wood => "Древо доросло до последней ветви и застыло.".to_string(),
        Matter::Stone => "Гора улеглась и больше не двинется.".to_string(),
        Matter::Meadow => "Луг отцвёл и перестал шириться.".to_string(),
        Matter::Voidness => "Язва пустоты насытилась и уснула.".to_string(),
        Matter::Hearth => "Очажная искра догорела.".to_string(),
        Matter::Ruin => "Пепел лёг и затих.".to_string(),
    }
}

/// «вода пожрала луг» — с согласованием рода.
fn clash_phrase(winner: Matter, loser: TileKind) -> String {
    let verb = match winner {
        Matter::Water | Matter::Voidness | Matter::Ruin => "пожрала",
        Matter::Wood => "пожрало",
        Matter::Stone | Matter::Meadow | Matter::Hearth => "пожрал",
    };
    format!("{} {} {}", winner.ru(), verb, tile_accusative(loser))
}

/// Тайл в винительном падеже — для фраз про пожирание.
fn tile_accusative(kind: TileKind) -> &'static str {
    match kind {
        TileKind::Empty => "бумагу",
        TileKind::Forest => "лес",
        TileKind::Water => "воду",
        TileKind::Stone => "камень",
        TileKind::Meadow => "луг",
        TileKind::Void => "пустоту",
        TileKind::Hearth => "очаг",
        TileKind::Path => "тропу",
        TileKind::Ruin => "руину",
    }
}
