//! Гейт Фазы 3: полный тик мира (посевы → поселения → энтропия) детерминирован
//! — два одинаковых мира проживают одинаковую историю вплоть до событий,
//! и энтропия действительно убивает: к старости лист заметно изъеден.

use std::path::PathBuf;

use palimpsest_core::tile::TileKind;
use palimpsest_core::World;

const FOREST: &str = "---\nname: Лес\nkind: rune\n---\n\n```rune\n(♠ (Y ↑ 5) (⌛ 140))\n```\n";
const RIVER: &str = "---\nname: Река\nkind: rune\n---\n\n```rune\n(~ (∩ ↓ ↷ ^) (⌛ 160))\n```\n";
const MEADOW: &str = "---\nname: Луг\nkind: rune\n---\n\n```rune\n(, (∴ +) (⌛ 120))\n```\n";

fn temp_deck(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("palimpsest_world_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("forest.md"), FOREST).unwrap();
    std::fs::write(dir.join("river.md"), RIVER).unwrap();
    std::fs::write(dir.join("meadow.md"), MEADOW).unwrap();
    dir
}

/// Прожить историю: тяга каждые 40 тиков, всего `ticks`; вернуть след.
fn live(dir: &PathBuf, ticks: u64) -> (String, String) {
    let mut world = World::new(dir.clone(), 48, 20, 11).unwrap();
    let mut trace = String::new();
    for t in 0..ticks {
        if t % 40 == 0 {
            world.draw(None).unwrap();
        }
        let events = world.step();
        for e in &events {
            trace.push_str(&format!("{}:{:?}\n", world.tick, e));
        }
    }
    (world.plane.render_glyphs(), trace)
}

#[test]
fn full_tick_is_deterministic() {
    // колода пересоздаётся между жизнями: тяги мутируют .md-файлы карт
    let dir = temp_deck("determinism");
    let (map_a, trace_a) = live(&dir, 500);
    let dir = temp_deck("determinism");
    let (map_b, trace_b) = live(&dir, 500);
    assert_eq!(map_a, map_b, "плоскость двух одинаковых миров разошлась");
    assert_eq!(trace_a, trace_b, "история событий двух одинаковых миров разошлась");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn entropy_eats_the_sheet() {
    let dir = temp_deck("entropy");
    let mut world = World::new(dir.clone(), 48, 20, 11).unwrap();
    for t in 0..3000u64 {
        if t % 40 == 0 {
            world.draw(None).unwrap();
        }
        world.step();
    }
    let holes = world.plane.count(TileKind::Void);
    let total = (world.plane.w * world.plane.h) as usize;
    assert!(
        holes > total / 4,
        "к тику 3000 пустота должна была выесть заметную часть листа, а дыр лишь {holes}/{total}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
