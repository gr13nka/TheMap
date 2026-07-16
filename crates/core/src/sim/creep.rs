//! Глаголы ∴ и молчание — расползаться пятном. Кромка тела захватывает
//! пустых соседей; стрелка+материя тянет фронт к цели, `↷ M` отводит его
//! от материи M. Still (материя без глаголов) пользуется той же механикой,
//! но застывает, выложив бюджет (см. Seed::step).

use rand::rngs::StdRng;
use rand::Rng;

use crate::event::Event;
use crate::plane::Plane;
use crate::rune::Clause;
use crate::tile::TileKind;

use super::matter;
use super::seed::Seed;

pub const NEIGHBORS4: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

pub fn act(
    seed: &mut Seed,
    plane: &mut Plane,
    rng: &mut StdRng,
    clause: &Clause,
    events: &mut Vec<Event>,
) {
    grow(seed, plane, rng, clause, events, |kind| {
        kind == TileKind::Empty
    });
}

/// Одно действие роста от кромки: найти клетку кромки с подходящим соседом,
/// закрасить его. Клетки без кандидатов выбывают из кромки.
pub fn grow<F>(
    seed: &mut Seed,
    plane: &mut Plane,
    rng: &mut StdRng,
    clause: &Clause,
    events: &mut Vec<Event>,
    candidate: F,
) where
    F: Fn(TileKind) -> bool,
{
    for _ in 0..8 {
        if seed.frontier.is_empty() {
            return;
        }
        let idx = rng.gen_range(0..seed.frontier.len());
        let (cx, cy) = seed.frontier[idx];

        let mut cands: Vec<(i32, i32)> = NEIGHBORS4
            .iter()
            .map(|&(dx, dy)| (cx + dx, cy + dy))
            .filter(|&(x, y)| {
                plane.in_bounds(x, y)
                    && candidate(plane.get(x, y))
                    && matter::can_paint(plane.get(x, y), seed.program.matter)
            })
            .collect();

        // ↷ M: фронт не подползает к материи M ближе, чем стоит сейчас
        if let Some(avoid) = clause.avoid {
            if let Some(here) = matter::nearest_dist2(plane, (cx, cy), avoid.tile()) {
                cands.retain(|&c| {
                    matter::nearest_dist2(plane, c, avoid.tile())
                        .map(|d| d >= here)
                        .unwrap_or(true)
                });
            }
        }

        if cands.is_empty() {
            seed.frontier.swap_remove(idx);
            continue;
        }

        let pick = choose(plane, rng, clause, &cands);
        seed.stroke(plane, pick.0, pick.1, events);
        return;
    }
}

/// Выбор кандидата: тяга к цели (стрелка+материя), иначе жребий.
fn choose(
    plane: &Plane,
    rng: &mut StdRng,
    clause: &Clause,
    cands: &[(i32, i32)],
) -> (i32, i32) {
    if let Some(target) = clause.seek {
        let kind = target.tile();
        let best = cands
            .iter()
            .filter_map(|&c| matter::nearest_dist2(plane, c, kind).map(|d| (d, c)))
            .min_by_key(|&(d, _)| d);
        if let Some((_, c)) = best {
            return c;
        }
    }
    cands[rng.gen_range(0..cands.len())]
}
