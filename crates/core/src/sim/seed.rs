//! Посев — активный объект, рождённый тягой карты. Программа компилируется
//! в момент тяги и дальше не меняется (правка карты не трогает уже живущих).
//! Посев живёт на тиках: каждая клауза копит бюджет (ровно или пульсами —
//! ритм ⏱) и тратит его на действия; посев стареет и умирает — краска
//! остаётся на бумаге.

use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};

use crate::event::Event;
use crate::plane::Plane;
use crate::rune::{Program, Verb};
use crate::tile::TileKind;

use super::{branch, creep, flow, gnaw, matter};

/// Черепашья голова — орган роста клауз Flow и Branch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Head {
    pub x: f64,
    pub y: f64,
    /// Курс в градусах: 0 — вправо (+x), 90 — вверх (−y экрана).
    pub heading_deg: f64,
    pub alive: bool,
    /// Чья голова (индекс клаузы в программе).
    pub clause: usize,
    /// Остаток хода в текущем сегменте (Branch: кончился — развилка).
    #[serde(default)]
    pub fuel: f64,
    /// Длина текущего сегмента; у детей развилки — короче.
    #[serde(default)]
    pub segment: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Seed {
    pub id: u64,
    pub program: Program,
    pub origin: (i32, i32),
    pub heads: Vec<Head>,
    /// Кромка тела — клетки, от которых расползаются Creep/Gnaw/Still.
    pub frontier: Vec<(i32, i32)>,
    pub age: u32,
    pub placed: usize,
    pub alive: bool,
    /// Накопители дробного бюджета по клаузам.
    accs: Vec<f32>,
    /// Круговой выбор головы — одно действие двигает одну голову.
    next_head: usize,
}

/// Курс из вектора клаузы; None — курс по умолчанию для глагола.
pub fn heading_of(dir: Option<(i8, i8)>, default_deg: f64) -> f64 {
    match dir {
        None | Some((0, 0)) => default_deg,
        Some((dx, dy)) => (-(dy as f64)).atan2(dx as f64).to_degrees(),
    }
}

impl Seed {
    pub fn spawn(id: u64, program: Program, origin: (i32, i32), plane: &mut Plane) -> Seed {
        let mut heads = Vec::new();
        for (i, clause) in program.clauses.iter().enumerate() {
            match clause.verb {
                Verb::Flow => heads.push(Head {
                    x: origin.0 as f64 + 0.5,
                    y: origin.1 as f64 + 0.5,
                    heading_deg: heading_of(clause.dir, 270.0), // вода по умолчанию вниз
                    alive: true,
                    clause: i,
                    fuel: f64::INFINITY, // поток не знает усталости — его предел vitality
                    segment: f64::INFINITY,
                }),
                Verb::Branch => {
                    // ствол: размах дерева — руной (число при Y) или от телесности
                    let trunk = clause
                        .trunk
                        .map(|t| t as f64)
                        .unwrap_or((program.vitality as f64 / 12.0).clamp(3.0, 12.0));
                    heads.push(Head {
                        x: origin.0 as f64 + 0.5,
                        y: origin.1 as f64 + 0.5,
                        heading_deg: heading_of(clause.dir, 90.0), // древо вверх
                        alive: true,
                        clause: i,
                        fuel: trunk,
                        segment: trunk,
                    });
                }
                _ => {}
            }
        }
        let accs = vec![0.0; program.clauses.len()];
        let mut seed = Seed {
            id,
            program,
            origin,
            heads,
            frontier: vec![origin],
            age: 0,
            placed: 0,
            alive: true,
            accs,
            next_head: 0,
        };
        // первый мазок — самим фактом тяги
        if matter::paint(plane, origin.0, origin.1, seed.program.matter).is_some() {
            seed.placed = 1;
        }
        seed
    }

    /// Мазок кистью: положить материю, запомнить кромку, заметить
    /// столкновение материй (для хроники).
    pub fn stroke(
        &mut self,
        plane: &mut Plane,
        x: i32,
        y: i32,
        events: &mut Vec<Event>,
    ) -> bool {
        match matter::paint(plane, x, y, self.program.matter) {
            Some(prior) => {
                self.placed += 1;
                self.frontier.push((x, y));
                if prior != TileKind::Empty {
                    events.push(Event::MatterClash {
                        winner: self.program.matter,
                        loser: prior,
                        at: (x, y),
                    });
                }
                true
            }
            None => false,
        }
    }

    /// Один тик жизни. RNG уже посолен (world_seed, tick, id).
    pub fn step(&mut self, plane: &mut Plane, rng: &mut StdRng, events: &mut Vec<Event>) {
        self.age += 1;

        for i in 0..self.program.clauses.len() {
            let clause = self.program.clauses[i].clone();
            // ритм: без ⏱ бюджет капает ровно; с ⏱ — копится и бьёт пульсом
            match clause.every {
                None => self.accs[i] += clause.rate,
                Some(n) => {
                    let n = n.max(1);
                    if self.age % n == 0 {
                        let pulse = if clause.burst_pulse { 3.0 } else { 1.0 };
                        self.accs[i] += clause.rate * n as f32 * pulse;
                        // пульс возрождает исток: мёртвые головы клаузы
                        // рождаются заново в сердцевине (гейзер бьёт снова)
                        if matches!(clause.verb, Verb::Flow | Verb::Branch)
                            && !self.heads.iter().any(|h| h.alive && h.clause == i)
                        {
                            self.revive_head(i, &clause);
                        }
                    }
                }
            }
            // предохранитель: не больше 32 действий клаузы за тик
            let mut guard = 0;
            while self.accs[i] >= 1.0 && guard < 32 {
                self.accs[i] -= 1.0;
                guard += 1;
                match clause.verb {
                    Verb::Flow => flow::act(self, plane, rng, i, &clause, events),
                    Verb::Branch => branch::act(self, plane, rng, i, &clause, events),
                    Verb::Creep | Verb::Still => creep::act(self, plane, rng, &clause, events),
                    Verb::Gnaw => gnaw::act(self, plane, rng, &clause, events),
                }
            }
        }

        // --- смерть посева ---
        if self.age >= self.program.vitality {
            self.alive = false;
        }
        // чистое пятно (Still) застывает, выложив свой бюджет клеток
        let still_only = self
            .program
            .clauses
            .iter()
            .all(|c| matches!(c.verb, Verb::Still));
        if still_only && self.placed >= self.program.vitality as usize {
            self.alive = false;
        }
        // объект из одних голов умирает вместе с последней головой —
        // если только ритм не обещает возродить исток
        let has_area = self
            .program
            .clauses
            .iter()
            .any(|c| matches!(c.verb, Verb::Creep | Verb::Gnaw | Verb::Still));
        let has_rhythm = self.program.clauses.iter().any(|c| c.every.is_some());
        if !has_area
            && !has_rhythm
            && !self.heads.is_empty()
            && self.heads.iter().all(|h| !h.alive)
        {
            self.alive = false;
        }
    }

    /// Родить голову клаузы заново в сердцевине посева.
    fn revive_head(&mut self, clause_idx: usize, clause: &crate::rune::Clause) {
        let default_deg = if clause.verb == Verb::Branch { 90.0 } else { 270.0 };
        let trunk = clause
            .trunk
            .map(|t| t as f64)
            .unwrap_or((self.program.vitality as f64 / 12.0).clamp(3.0, 12.0));
        let (fuel, segment) = if clause.verb == Verb::Branch {
            (trunk, trunk)
        } else {
            (f64::INFINITY, f64::INFINITY)
        };
        self.heads.push(Head {
            x: self.origin.0 as f64 + 0.5,
            y: self.origin.1 as f64 + 0.5,
            heading_deg: heading_of(clause.dir, default_deg),
            alive: true,
            clause: clause_idx,
            fuel,
            segment,
        });
    }

    /// Круговой выбор живой головы данной клаузы; None — все мертвы.
    pub fn pick_head(&mut self, clause: usize) -> Option<usize> {
        let n = self.heads.len();
        for _ in 0..n {
            let i = self.next_head % n.max(1);
            self.next_head = self.next_head.wrapping_add(1);
            if self
                .heads
                .get(i)
                .map(|h| h.alive && h.clause == clause)
                .unwrap_or(false)
            {
                return Some(i);
            }
        }
        None
    }
}
