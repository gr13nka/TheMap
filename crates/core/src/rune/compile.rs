//! Компиляция выражения в программу посева — тяга как eval. Чистая функция:
//! ни RNG, ни мира; никогда не падает. Мусор игнорируется, слишком глубокое
//! обрезается, не-материя в голове — карта «ложится без следа» (обучающий
//! сигнал, не ошибка).
//!
//! Чтение позиционное, как в Лиспе:
//! - `(Материя …)` — верхний список; голова — субстанция карты;
//! - `(Глагол уточнения…)` — клауза-поведение;
//! - `(⏱ N …)` — ритм: клаузы внутри действуют пульсами; форма без клауз
//!   задаёт ритм всей карте;
//! - `(⌛ N)` — срок жизни; атомы верхнего уровня (`+`, `!`, стрелки) —
//!   дефолты, наследуемые всеми клаузами.

use serde::{Deserialize, Serialize};

use super::{Expr, Matter, Rune};

/// Срок жизни по умолчанию, тиков.
const DEFAULT_VITALITY: u32 = 120;
/// Предел глубины дерева — глубже смысл не читается.
const MAX_DEPTH: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verb {
    /// Материя без глаголов: лечь пятном и застыть.
    Still,
    Flow,
    Branch,
    Creep,
    Gnaw,
}

/// Кого позволено грызть.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Only {
    /// ♥ — только живое (луг, лес, очаг, тропа).
    Living,
    Matter(Matter),
}

/// Одно поведение посева.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clause {
    pub verb: Verb,
    /// Действий за тик (или за пульс — см. every).
    pub rate: f32,
    /// Вектор из стрелки; None — дефолт глагола (вода вниз, древо вверх).
    pub dir: Option<(i8, i8)>,
    /// Тяга к ближайшей материи (стрелка + материя).
    pub seek: Option<Matter>,
    /// Обход материи (↷ M): отклоняет головы, отводит кромку.
    pub avoid: Option<Matter>,
    /// Ограничение добычи для Gnaw.
    pub only: Option<Only>,
    /// Ритм: действовать пульсами раз в N тиков.
    pub every: Option<u32>,
    /// ! внутри ⏱ — пульс втрое сильнее.
    pub burst_pulse: bool,
    /// Число при Y — длина ствола (размах дерева).
    pub trunk: Option<u32>,
}

impl Clause {
    fn new(verb: Verb) -> Clause {
        Clause {
            verb,
            rate: 1.0,
            dir: None,
            seek: None,
            avoid: None,
            only: None,
            every: None,
            burst_pulse: false,
            trunk: None,
        }
    }
}

/// Скомпилированная карта: во что мир превратит тягу.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    pub matter: Matter,
    pub vitality: u32,
    pub clauses: Vec<Clause>,
}

impl Program {
    /// Ключ комбинации для атласа наблюдений (внутренний, в UI не показывается).
    pub fn combo_key(&self) -> String {
        let mut verbs: Vec<String> = self
            .clauses
            .iter()
            .map(|c| {
                let mut k = format!("{:?}", c.verb);
                if c.seek.is_some() {
                    k.push_str("→");
                }
                if c.only.is_some() {
                    k.push('♥');
                }
                if c.every.is_some() {
                    k.push('⏱');
                }
                k
            })
            .collect();
        verbs.sort_unstable();
        verbs.dedup();
        format!("{:?}+{}", self.matter, verbs.join("+"))
    }
}

/// Дефолты верхнего уровня, наследуемые клаузами.
#[derive(Default, Clone, Copy)]
struct Defaults {
    dir: Option<(i8, i8)>,
    seek: Option<Matter>,
    avoid: Option<Matter>,
    abundant: u32,
    burst: bool,
    every: Option<u32>,
    burst_pulse: bool,
}

pub fn compile(expr: &Expr) -> Option<Program> {
    let Expr::List(items) = expr else { return None };
    // голова верхнего списка — материя, иначе карта инертна
    let matter = match items.first() {
        Some(Expr::Rune(r)) => r.as_matter()?,
        _ => return None,
    };

    let mut vitality = DEFAULT_VITALITY;
    let mut defaults = Defaults::default();
    let mut clauses: Vec<Clause> = Vec::new();

    // первый проход: атомы-дефолты и (⌛ N) читаются раньше клауз
    for item in &items[1..] {
        match item {
            Expr::Rune(Rune::Abundant) => defaults.abundant += 1,
            Expr::Rune(Rune::Burst) => defaults.burst = true,
            Expr::Rune(r) => {
                if let Some(d) = r.as_dir() {
                    defaults.dir = Some(d);
                }
            }
            Expr::List(l) => {
                if let [Expr::Rune(Rune::Lifespan), Expr::Num(n), ..] = l.as_slice() {
                    vitality = (*n).max(1);
                }
            }
            _ => {}
        }
    }

    // второй проход: клаузы и формы ритма
    for item in &items[1..] {
        collect_clauses(item, &defaults, &mut clauses, 1);
    }

    if clauses.is_empty() {
        clauses.push(inherit(Clause::new(Verb::Still), &defaults));
    }
    if defaults.burst {
        vitality = (vitality / 3).max(1);
    }

    Some(Program {
        matter,
        vitality,
        clauses,
    })
}

/// Разобрать элемент верхнего уровня (или содержимое формы ⏱).
fn collect_clauses(item: &Expr, defaults: &Defaults, out: &mut Vec<Clause>, depth: usize) {
    if depth > MAX_DEPTH {
        return;
    }
    let Expr::List(l) = item else { return };
    match l.first() {
        // клауза с глаголом в голове
        Some(Expr::Rune(r)) if verb_of(*r).is_some() => {
            let mut clause = Clause::new(verb_of(*r).unwrap());
            read_clause_args(&l[1..], &mut clause);
            out.push(inherit(clause, defaults));
        }
        // форма ритма: (⏱ N …)
        Some(Expr::Rune(Rune::Every)) => {
            let mut n = None;
            let mut inner_burst = false;
            let mut had_clauses = false;
            for e in &l[1..] {
                match e {
                    Expr::Num(v) => n = Some((*v).max(1)),
                    Expr::Rune(Rune::Burst) => inner_burst = true,
                    Expr::List(_) => {
                        had_clauses = true;
                        let mut d = *defaults;
                        d.every = n;
                        d.burst_pulse = inner_burst;
                        collect_clauses(e, &d, out, depth + 1);
                    }
                    _ => {}
                }
            }
            // форма без клауз — ритм всей карты: догоняем уже собранные
            // и будущее наследие (клаузы после формы читаются со старыми
            // дефолтами, поэтому правим задним числом)
            if !had_clauses {
                if let Some(n) = n {
                    for c in out.iter_mut() {
                        if c.every.is_none() {
                            c.every = Some(n);
                            c.burst_pulse = inner_burst;
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

/// Прочитать уточнения клаузы слева направо.
fn read_clause_args(args: &[Expr], clause: &mut Clause) {
    let mut i = 0;
    while i < args.len() {
        match &args[i] {
            Expr::Rune(r) => {
                if let Some(d) = r.as_dir() {
                    // стрелка + материя = тяга к ней; стрелка одна = вектор
                    if let Some(Expr::Rune(m)) = args.get(i + 1) {
                        if let Some(matter) = m.as_matter() {
                            clause.seek = Some(matter);
                            i += 2;
                            continue;
                        }
                    }
                    clause.dir = Some(d);
                } else {
                    match r {
                        Rune::Away => {
                            if let Some(Expr::Rune(m)) = args.get(i + 1) {
                                if let Some(matter) = m.as_matter() {
                                    clause.avoid = Some(matter);
                                    i += 2;
                                    continue;
                                }
                            }
                        }
                        Rune::Living => clause.only = Some(Only::Living),
                        Rune::Abundant => clause.rate += 1.0,
                        Rune::Burst => clause.rate *= 3.0,
                        _ => {
                            // материя как аргумент глагола — добыча (для ×)
                            if let Some(matter) = r.as_matter() {
                                clause.only = Some(Only::Matter(matter));
                            }
                        }
                    }
                }
            }
            Expr::Num(n) => {
                if clause.verb == Verb::Branch {
                    clause.trunk = Some((*n).max(1));
                }
            }
            Expr::List(_) => {} // вложение внутри клаузы пока молчит
        }
        i += 1;
    }
}

/// Наследование дефолтов верхнего уровня (дефолт слабее локального).
fn inherit(mut clause: Clause, d: &Defaults) -> Clause {
    if clause.dir.is_none() {
        clause.dir = d.dir;
    }
    if clause.seek.is_none() {
        clause.seek = d.seek;
    }
    if clause.avoid.is_none() {
        clause.avoid = d.avoid;
    }
    clause.rate += d.abundant as f32;
    if d.burst {
        clause.rate *= 3.0;
    }
    if clause.every.is_none() {
        clause.every = d.every;
        clause.burst_pulse = d.burst_pulse;
    }
    clause
}

fn verb_of(r: Rune) -> Option<Verb> {
    Some(match r {
        Rune::Flow => Verb::Flow,
        Rune::Branch => Verb::Branch,
        Rune::Creep => Verb::Creep,
        Rune::Gnaw => Verb::Gnaw,
        _ => return None,
    })
}
