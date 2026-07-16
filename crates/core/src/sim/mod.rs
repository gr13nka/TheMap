//! Симуляция — жизнь посевов на тиках. Порядок жёстко фиксирован ради
//! детерминизма: посевы шагают в порядке id, RNG каждого действия выводится
//! из (world_seed, tick, соль) — мир воспроизводим и переживает save/load
//! без хранения состояния генератора.

pub mod branch;
pub mod creep;
pub mod entropy;
pub mod flow;
pub mod gnaw;
pub mod matter;
pub mod seed;
pub mod settlement;

use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::event::Event;
use crate::plane::Plane;
use seed::Seed;

/// Детерминированный RNG тика для системы с солью (id посева и т.п.).
pub fn tick_rng(world_seed: u64, tick: u64, salt: u64) -> StdRng {
    let mixed = world_seed
        ^ tick.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ salt.wrapping_mul(0xD1B5_4A32_D192_ED03);
    StdRng::seed_from_u64(mixed)
}

/// Один тик всех посевов; мёртвые убираются (краска их остаётся на бумаге).
pub fn step_seeds(
    plane: &mut Plane,
    seeds: &mut Vec<Seed>,
    world_seed: u64,
    tick: u64,
    events: &mut Vec<Event>,
) {
    for s in seeds.iter_mut() {
        if !s.alive {
            continue;
        }
        let mut rng = tick_rng(world_seed, tick, s.id);
        s.step(plane, &mut rng, events);
    }
    for s in seeds.iter().filter(|s| !s.alive) {
        events.push(Event::SeedDied {
            id: s.id,
            matter: s.program.matter,
            placed: s.placed,
        });
    }
    seeds.retain(|s| s.alive);
}
