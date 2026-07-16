//! Headless-проверка конвейера без терминала: собрать мир из ./Deck, стянуть
//! несколько карт и дать миру пожить между тягами. Тот же путь исполнения,
//! что и в TUI, минус crossterm.

use std::path::PathBuf;

use palimpsest_core::{archivist, World};

const TICKS_BETWEEN_DRAWS: u64 = 80;

fn main() -> std::io::Result<()> {
    let deck_dir = PathBuf::from("Deck");
    let mut world = World::new(deck_dir, 48, 20, 1)?;

    let draws: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);

    for _ in 0..draws {
        let outcome = world.draw(None)?;
        let line = archivist::chronicle_line(world.draw_count, &outcome);
        println!("{line}");
        for _ in 0..TICKS_BETWEEN_DRAWS {
            let events = world.step();
            for line in world.narrate(&events) {
                println!("{line}");
            }
        }
    }

    println!("\n--- MAP (тик {}) ---\n{}", world.tick, world.plane.render_glyphs());
    Ok(())
}
