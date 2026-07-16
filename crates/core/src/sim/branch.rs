//! Глагол Y — ветвиться. Черепашка делает шаг за действие (дерево растёт
//! на глазах), сегмент несёт запас хода: исчерпался — развилка на две ветви
//! покороче; веточка тоньше предела — умирает. Размах задаёт число при Y
//! (или телесность), а не кромка листа. `↷ M` отклоняет ветвь; о прочее
//! непрокрашиваемое ветвь ломается.

use rand::rngs::StdRng;
use rand::Rng;

use crate::event::Event;
use crate::plane::Plane;
use crate::rune::Clause;
use crate::tile::TileKind;

use super::flow::{seek_heading, shortest_angle};
use super::matter;
use super::seed::{heading_of, Head, Seed};

/// Во сколько раз ветвь короче родителя.
const SHRINK: f64 = 0.72;
/// Сегмент короче этого — веточка кончилась.
const MIN_SEGMENT: f64 = 1.6;
/// Угол расхождения ветвей на развилке, градусы.
const SPLIT_ANGLE: f64 = 34.0;
/// Редкая внеплановая развилка — неровность живого дерева.
const EXTRA_SPLIT_P: f64 = 0.06;
/// Предохранитель от взрыва числа голов.
const MAX_HEADS: usize = 48;

pub fn act(
    seed: &mut Seed,
    plane: &mut Plane,
    rng: &mut StdRng,
    clause_idx: usize,
    clause: &Clause,
    events: &mut Vec<Event>,
) {
    let Some(i) = seed.pick_head(clause_idx) else { return };

    let pos = (seed.heads[i].x, seed.heads[i].y);
    let target = seek_heading(plane, pos, clause)
        .unwrap_or_else(|| heading_of(clause.dir, 90.0));

    let own = seed.program.matter.tile();
    let head = &mut seed.heads[i];
    // слабая тяга к курсу + блуждание ветви
    let delta = shortest_angle(target - head.heading_deg);
    head.heading_deg += delta * 0.15 + rng.gen_range(-14.0..14.0);

    let rad = head.heading_deg.to_radians();
    let nx = head.x + rad.cos();
    let ny = head.y - rad.sin();
    let (cx, cy) = (nx.floor() as i32, ny.floor() as i32);

    if !plane.in_bounds(cx, cy) {
        head.alive = false;
        return;
    }
    let ahead = plane.get(cx, cy);
    if Some(ahead) == clause.avoid.map(|m| m.tile()) {
        head.heading_deg += if rng.gen_bool(0.5) { 45.0 } else { -45.0 };
        return;
    }
    if ahead != own && ahead != TileKind::Empty && !matter::can_paint(ahead, seed.program.matter)
    {
        head.alive = false; // ветвь сломалась о чужое
        return;
    }

    head.x = nx;
    head.y = ny;
    head.fuel -= 1.0;
    seed.stroke(plane, cx, cy, events);

    let spent = seed.heads[i].fuel <= 0.0;
    let extra = rng.gen_bool(EXTRA_SPLIT_P);
    if spent || extra {
        split(seed, i, rng);
    }
}

/// Развилка: голова становится одной ветвью, рождается вторая; обе короче.
fn split(seed: &mut Seed, i: usize, rng: &mut StdRng) {
    let parent = seed.heads[i].clone();
    let child_segment = parent.segment * SHRINK;
    if child_segment < MIN_SEGMENT {
        seed.heads[i].alive = false; // веточка дошла до своего предела
        return;
    }
    let side = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };
    let spread = SPLIT_ANGLE + rng.gen_range(-8.0..8.0);

    let h = &mut seed.heads[i];
    h.heading_deg = parent.heading_deg + side * spread;
    h.fuel = child_segment;
    h.segment = child_segment;

    let alive = seed.heads.iter().filter(|h| h.alive).count();
    if alive < MAX_HEADS {
        seed.heads.push(Head {
            x: parent.x,
            y: parent.y,
            heading_deg: parent.heading_deg - side * spread,
            alive: true,
            clause: parent.clause,
            fuel: child_segment,
            segment: child_segment,
        });
    }
}
