//! Глагол ∩ — течь. Голова тянется по вектору клаузы (по умолчанию вниз)
//! или к ближайшей материи-цели (стрелка+материя), плавно доворачивая и
//! слегка блуждая. `↷ M` отклоняет поток от материи M; о непрокрашиваемое
//! без обхода голова гибнет — мир учит, не прощает.

use rand::rngs::StdRng;
use rand::Rng;

use crate::event::Event;
use crate::plane::Plane;
use crate::rune::Clause;
use crate::tile::TileKind;

use super::matter;
use super::seed::{heading_of, Seed};

/// Насколько сильно поток доворачивает к курсу за шаг.
const PULL: f64 = 0.5;

pub fn act(
    seed: &mut Seed,
    plane: &mut Plane,
    rng: &mut StdRng,
    clause_idx: usize,
    clause: &Clause,
    events: &mut Vec<Event>,
) {
    let Some(i) = seed.pick_head(clause_idx) else { return };

    // курс: тяга к цели сильнее вектора
    let pos = (seed.heads[i].x, seed.heads[i].y);
    let target = seek_heading(plane, pos, clause)
        .unwrap_or_else(|| heading_of(clause.dir, 270.0));

    let own = seed.program.matter.tile();
    let head = &mut seed.heads[i];
    let delta = shortest_angle(target - head.heading_deg);
    head.heading_deg += delta * PULL + rng.gen_range(-12.0..12.0);

    let rad = head.heading_deg.to_radians();
    let nx = head.x + rad.cos();
    let ny = head.y - rad.sin();
    let (cx, cy) = (nx.floor() as i32, ny.floor() as i32);

    if !plane.in_bounds(cx, cy) {
        head.alive = false; // дотёк до края листа
        return;
    }
    let ahead = plane.get(cx, cy);
    if Some(ahead) == clause.avoid.map(|m| m.tile()) {
        // обходимая материя отклоняет поток
        head.heading_deg += if rng.gen_bool(0.5) { 45.0 } else { -45.0 };
        return;
    }
    if ahead != own && ahead != TileKind::Empty && !matter::can_paint(ahead, seed.program.matter)
    {
        head.alive = false; // упёрся в непрокрашиваемое без обхода
        return;
    }

    head.x = nx;
    head.y = ny;
    seed.stroke(plane, cx, cy, events);
}

/// Курс к ближайшей материи-цели, если она есть на листе.
pub fn seek_heading(plane: &Plane, from: (f64, f64), clause: &Clause) -> Option<f64> {
    let target = clause.seek?;
    let (tx, ty) = matter::nearest_pos(
        plane,
        (from.0.floor() as i32, from.1.floor() as i32),
        target.tile(),
    )?;
    let dx = tx as f64 + 0.5 - from.0;
    let dy = ty as f64 + 0.5 - from.1;
    Some((-dy).atan2(dx).to_degrees())
}

/// Кратчайший угол в (−180, 180].
pub fn shortest_angle(mut a: f64) -> f64 {
    while a > 180.0 {
        a -= 360.0;
    }
    while a <= -180.0 {
        a += 360.0;
    }
    a
}
