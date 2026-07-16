//! Сейв в RON — читаемый текст, приятно смотреть и править руками (brief).
//! Храним плоскость, посевы, порядок колоды, время и seed. Состояние RNG
//! не храним: оно детерминированно выводится из (seed, tick/draw_count).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::archivist::Archivist;
use crate::cycle::CycleState;
use crate::plane::Plane;
use crate::sim::entropy::EntropyState;
use crate::sim::seed::Seed;
use crate::sim::settlement::Settlement;
use crate::world::World;

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveState {
    pub plane: Plane,
    pub deck_order: Vec<String>,
    pub draw_count: u64,
    pub seed: u64,
    pub cursor: (i32, i32),
    #[serde(default)]
    pub tick: u64,
    #[serde(default)]
    pub seeds: Vec<Seed>,
    #[serde(default)]
    pub settlements: Vec<Settlement>,
    #[serde(default)]
    pub entropy: Option<EntropyState>,
    #[serde(default)]
    pub archivist: Archivist,
    #[serde(default)]
    pub cycle: Option<CycleState>,
    #[serde(default)]
    pub peak_filled: usize,
    #[serde(default)]
    pub hearths_founded: u32,
    #[serde(default)]
    pub last_draw_tick: u64,
}

impl SaveState {
    pub fn from_world(world: &World) -> SaveState {
        SaveState {
            plane: world.plane.clone(),
            deck_order: world.deck.order(),
            draw_count: world.draw_count,
            seed: world.seed,
            cursor: world.cursor,
            tick: world.tick,
            seeds: world.seeds.clone(),
            settlements: world.settlements.clone(),
            entropy: Some(world.entropy.clone()),
            archivist: world.archivist.clone(),
            cycle: Some(world.cycle.clone()),
            peak_filled: world.peak_filled,
            hearths_founded: world.hearths_founded,
            last_draw_tick: world.last_draw_tick,
        }
    }
}

pub fn save(world: &World, path: &Path) -> std::io::Result<()> {
    let state = SaveState::from_world(world);
    let text = ron::ser::to_string_pretty(&state, ron::ser::PrettyConfig::default())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    std::fs::write(path, text)
}

/// Загрузить мир из сейва; `deck_dir` нужен, чтобы восстановить пути карт.
pub fn load(path: &Path, deck_dir: &Path) -> std::io::Result<World> {
    let text = std::fs::read_to_string(path)?;
    let state: SaveState =
        ron::from_str(&text).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(World::from_parts(
        deck_dir.to_path_buf(),
        &state.deck_order,
        state.plane,
        state.seeds,
        state.settlements,
        state.entropy,
        state.archivist,
        state.cycle,
        (state.peak_filled, state.hearths_founded),
        state.tick,
        state.draw_count,
        state.last_draw_tick,
        state.seed,
        state.cursor,
    ))
}
