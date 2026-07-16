//! Гейт Фазы 2: сейв не меняет судьбу. Мир, сохранённый и загруженный,
//! проживает те же N тиков в тот же узор, что и мир, живший без перерыва —
//! RNG выводится из (seed, tick, id), состояние генератора не хранится.

use std::path::PathBuf;

use palimpsest_core::{save, World};

const FOREST: &str = "---\nname: Лес\nkind: rune\n---\n\n```rune\n(♠ (Y ↑ 5) (⌛ 140))\n```\n";
const RIVER: &str = "---\nname: Река\nkind: rune\n---\n\n```rune\n(~ (∩ ↓ ↷ ^) (⌛ 160))\n```\n";

fn temp_deck(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("palimpsest_deck_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("forest.md"), FOREST).unwrap();
    std::fs::write(dir.join("river.md"), RIVER).unwrap();
    dir
}

#[test]
fn save_load_preserves_the_future() {
    let dir = temp_deck("roundtrip");
    let save_path = dir.join("save.ron");

    let mut lived = World::new(dir.clone(), 48, 20, 7).unwrap();
    lived.draw(None).unwrap();
    for _ in 0..15 {
        lived.step();
    }
    lived.draw(None).unwrap();
    for _ in 0..15 {
        lived.step();
    }

    save::save(&lived, &save_path).unwrap();
    let mut reborn = save::load(&save_path, &dir).unwrap();

    assert_eq!(
        lived.plane.render_glyphs(),
        reborn.plane.render_glyphs(),
        "загруженный мир должен совпасть с сохранённым"
    );

    for _ in 0..60 {
        lived.step();
        reborn.step();
    }
    assert_eq!(
        lived.plane.render_glyphs(),
        reborn.plane.render_glyphs(),
        "будущее после загрузки должно совпасть с будущим без перерыва"
    );
    assert_eq!(lived.deck.order(), reborn.deck.order(), "порядок колоды пережил сейв");

    let _ = std::fs::remove_dir_all(&dir);
}
