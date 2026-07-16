//! Глагол × — грызть. Как ∴, но обращает только чужую непустую материю;
//! `♥` сужает добычу до живого, материя-аргумент — до неё одной.
//! Вода+(× ♥ !) — потоп; пустота+(∴ → #)+(× ♥) — чума.

use rand::rngs::StdRng;

use crate::event::Event;
use crate::plane::Plane;
use crate::rune::{Clause, Only};
use crate::tile::TileKind;

use super::creep;
use super::seed::Seed;

/// Живое — то, что растёт и горит: луг, лес, очаг, тропа.
pub fn is_living(kind: TileKind) -> bool {
    matches!(
        kind,
        TileKind::Meadow | TileKind::Forest | TileKind::Hearth | TileKind::Path
    )
}

pub fn act(
    seed: &mut Seed,
    plane: &mut Plane,
    rng: &mut StdRng,
    clause: &Clause,
    events: &mut Vec<Event>,
) {
    let own = seed.program.matter.tile();
    let only = clause.only;
    creep::grow(seed, plane, rng, clause, events, move |kind| {
        if kind == TileKind::Empty || kind == own {
            return false;
        }
        match only {
            Some(Only::Living) => is_living(kind),
            Some(Only::Matter(m)) => kind == m.tile(),
            None => true,
        }
    });
}
