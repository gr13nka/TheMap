//! Поселения — автономный закон мира, читаемый из Скрижали Очагов. Где
//! родиться, чем кормиться, до какого предела расти, что чинить и к чему
//! тянуться — всё руны, ничего зашитого. Молчит клауза — поведения нет:
//! без ✶ очаги не рождаются никогда, без ♥ никто не чинит распад.

use rand::rngs::StdRng;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::event::Event;
use crate::plane::Plane;
use crate::rune::Matter;
use crate::tablet::HearthLaw;
use crate::tile::TileKind;

use super::creep::NEIGHBORS4;
use super::matter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settlement {
    pub id: u64,
    pub pos: (i32, i32),
    pub size: u8,
    pub age: u32,
    pub alive: bool,
}

/// Один тик всех поселений + попытка рождения нового — по закону.
pub fn step(
    plane: &mut Plane,
    settlements: &mut Vec<Settlement>,
    next_id: &mut u64,
    rng: &mut StdRng,
    tick: u64,
    law: &HearthLaw,
    events: &mut Vec<Event>,
) {
    // --- рождение: только если закон велит рождаться ---
    if let Some(found) = &law.found {
        if tick % found.every == 0 {
            if let Some(pos) = find_site(plane, settlements, rng, law) {
                let id = *next_id;
                *next_id += 1;
                plane.set(pos.0, pos.1, TileKind::Hearth);
                lay_path(plane, pos);
                settlements.push(Settlement {
                    id,
                    pos,
                    size: 1,
                    age: 0,
                    alive: true,
                });
                events.push(Event::SettlementFounded { id, pos });
            }
        }
    }

    for s in settlements.iter_mut() {
        if !s.alive {
            continue;
        }
        s.age += 1;

        // очаг смыт или выеден — поселение погибло, остаётся руина
        if plane.get(s.pos.0, s.pos.1) != TileKind::Hearth {
            s.alive = false;
            for_each_in_radius(s.pos, 2, |x, y| {
                if plane.get(x, y) == TileKind::Hearth {
                    plane.set(x, y, TileKind::Ruin);
                }
            });
            events.push(Event::SettlementDied { id: s.id, pos: s.pos });
            continue;
        }

        // жизнь чинит истлевающий край — если закон дал ей руки
        if let Some(radius) = law.heal {
            heal_one(plane, s.pos, radius, rng);
        }

        // рост при прокорме — если закон велит расти
        if let Some(grow) = &law.grow {
            if s.age as u64 % grow.every == 0
                && s.size < grow.max_size
                && is_fed(plane, s.pos, grow)
            {
                if let Some((x, y)) = expansion_spot(plane, s.pos, rng) {
                    if matter::paint(plane, x, y, Matter::Hearth).is_some() {
                        s.size += 1;
                        if s.size == 2 || s.size == 4 || s.size == grow.max_size {
                            events.push(Event::SettlementGrew {
                                id: s.id,
                                pos: s.pos,
                                size: s.size,
                            });
                        }
                    }
                }
            }
        }
    }
    settlements.retain(|s| s.alive);
}

/// Детерминированный поиск места по правилу рождения.
fn find_site(
    plane: &Plane,
    settlements: &[Settlement],
    rng: &mut StdRng,
    law: &HearthLaw,
) -> Option<(i32, i32)> {
    let found = law.found.as_ref()?;
    for _ in 0..24 {
        let x = rng.gen_range(1..plane.w - 1);
        let y = rng.gen_range(1..plane.h - 1);
        let here = plane.get(x, y);
        let on_ok = here == TileKind::Empty
            || found.on.iter().any(|m| m.tile() == here);
        if !on_ok {
            continue;
        }
        // не теснить соседей
        if settlements.iter().any(|s| dist2(s.pos, (x, y)) < 36) {
            continue;
        }
        // всё требуемое — рядом
        let near_ok = found.near.iter().all(|(m, d2)| {
            matter::nearest_dist2(plane, (x, y), m.tile())
                .map(|d| d <= *d2)
                .unwrap_or(false)
        });
        if !near_ok {
            continue;
        }
        // наследие — плодородная почва: рядом с манящей материей селятся охотнее
        let lured = law
            .lure
            .and_then(|m| matter::nearest_dist2(plane, (x, y), m.tile()))
            .map(|d| d <= 16)
            .unwrap_or(false);
        if lured || rng.gen_bool(0.6) {
            return Some((x, y));
        }
    }
    None
}

/// Прокорм по закону: каждой материи — не меньше её счёта в округе.
fn is_fed(plane: &Plane, pos: (i32, i32), grow: &crate::tablet::GrowRule) -> bool {
    grow.food.iter().all(|(m, need)| {
        let mut count = 0;
        for_each_in_radius(pos, 3, |x, y| {
            if plane.get(x, y) == m.tile() {
                count += 1;
            }
        });
        count >= *need
    })
}

/// Куда поставить новый очаг: первый подходящий сосед по кругу от жребия.
fn expansion_spot(plane: &Plane, pos: (i32, i32), rng: &mut StdRng) -> Option<(i32, i32)> {
    let start = rng.gen_range(0..NEIGHBORS4.len());
    for k in 0..NEIGHBORS4.len() {
        let (dx, dy) = NEIGHBORS4[(start + k) % NEIGHBORS4.len()];
        let (x, y) = (pos.0 + dx, pos.1 + dy);
        if plane.in_bounds(x, y)
            && matches!(plane.get(x, y), TileKind::Empty | TileKind::Meadow)
        {
            return Some((x, y));
        }
    }
    None
}

/// Тропа от очага к ближайшей воде — по клетке, прямой походкой.
fn lay_path(plane: &mut Plane, from: (i32, i32)) {
    let mut target: Option<(i32, i32)> = None;
    let mut best = i64::MAX;
    for_each_in_radius(from, 6, |x, y| {
        if plane.get(x, y) == TileKind::Water {
            let d = dist2(from, (x, y));
            if d < best {
                best = d;
                target = Some((x, y));
            }
        }
    });
    let Some((tx, ty)) = target else { return };

    let (mut x, mut y) = from;
    for _ in 0..12 {
        let dx = (tx - x).signum();
        let dy = (ty - y).signum();
        if (tx - x).abs() >= (ty - y).abs() {
            x += dx;
        } else {
            y += dy;
        }
        if !plane.in_bounds(x, y) || (x, y) == (tx, ty) {
            break;
        }
        if matches!(plane.get(x, y), TileKind::Empty | TileKind::Meadow) {
            plane.set(x, y, TileKind::Path);
        }
    }
}

/// Починить одну истлевающую клетку рядом (снять стадию распада).
fn heal_one(plane: &mut Plane, pos: (i32, i32), radius: i32, rng: &mut StdRng) {
    if rng.gen_bool(0.7) {
        return; // жизнь чинит, но не всесильна
    }
    let mut candidates: Vec<(i32, i32)> = Vec::new();
    for_each_in_radius(pos, radius, |x, y| {
        if let Some(t) = plane.tiles.get(&(x, y)) {
            if t.decay > 0 {
                candidates.push((x, y));
            }
        }
    });
    if candidates.is_empty() {
        return;
    }
    let (x, y) = candidates[rng.gen_range(0..candidates.len())];
    if let Some(t) = plane.tiles.get_mut(&(x, y)) {
        t.decay = t.decay.saturating_sub(1);
    }
}

/// Обойти квадрат радиуса r вокруг точки в фиксированном порядке
/// (порядок обхода — часть детерминизма, см. инвариант в plane.rs).
fn for_each_in_radius<F: FnMut(i32, i32)>(pos: (i32, i32), r: i32, mut f: F) {
    for y in (pos.1 - r)..=(pos.1 + r) {
        for x in (pos.0 - r)..=(pos.0 + r) {
            f(x, y);
        }
    }
}

fn dist2(a: (i32, i32), b: (i32, i32)) -> i64 {
    let dx = (a.0 - b.0) as i64;
    let dy = (a.1 - b.1) as i64;
    dx * dx + dy * dy
}
