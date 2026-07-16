//! Гейт Фазы 3: скрижали исполняются абсолютно честно. Дефолтные — мир
//! живёт и умирает как прежде; вырванный укус — бессмертие; утроенный —
//! скорая смерть; молчащий Порог — мир официально не умирает. Никакой
//! защиты от бога.

use std::path::PathBuf;

use palimpsest_core::cycle::Phase;
use palimpsest_core::tablet::{TabletSlot, Tablets, DEFAULT_ENTROPY};
use palimpsest_core::tile::TileKind;
use palimpsest_core::World;

const FOREST: &str = "---\nname: Лес\nkind: rune\n---\n\n```rune\n(♠ (Y ↑ 5) (⌛ 140))\n```\n";
const MEADOW: &str = "---\nname: Луг\nkind: rune\n---\n\n```rune\n(, (∴ +) (⌛ 120))\n```\n";

/// Свой мир со своей скрижалью Пустоты (None — дефолт).
fn world_with_entropy(tag: &str, entropy_expr: Option<&str>) -> (World, PathBuf) {
    let dir = std::env::temp_dir().join(format!("palimpsest_tablet_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("forest.md"), FOREST).unwrap();
    std::fs::write(dir.join("meadow.md"), MEADOW).unwrap();
    // мир создаст Tablets/ с дефолтами; затем — наша правка закона
    let mut world = World::new(dir.clone(), 48, 20, 11).unwrap();
    if let Some(expr) = entropy_expr {
        let tdir = Tablets::dir_for(&dir);
        palimpsest_core::tablet::write(
            &tdir,
            TabletSlot::Entropy,
            &palimpsest_core::rune::parse(expr),
        )
        .unwrap();
        world.reload_tablets();
    }
    (world, dir)
}

/// Прожить N тиков с тягой каждые 40; вернуть число дыр.
fn live(world: &mut World, ticks: u64) -> usize {
    for t in 0..ticks {
        if t % 40 == 0 && world.cycle.phase != Phase::Dead {
            world.draw(None).unwrap();
        }
        world.step();
    }
    world.plane.count(TileKind::Void)
}

#[test]
fn default_tablets_keep_the_world_mortal() {
    let (mut world, dir) = world_with_entropy("default", None);
    let holes = live(&mut world, 3000);
    let total = (world.plane.w * world.plane.h) as usize;
    assert!(
        holes > total / 4,
        "с дефолтными скрижалями пустота должна была выесть заметную часть листа, а дыр {holes}/{total}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn torn_bite_makes_the_world_immortal() {
    // выкинуть (⏱ 2 ×) из Скрижали Пустоты — ни одного укуса, никогда
    let (mut world, dir) = world_with_entropy("immortal", Some("(░ (∈ ▢) (⌛ 1200))"));
    let holes = live(&mut world, 800);
    assert_eq!(holes, 0, "без укуса в законе мир бессмертен, а дыр {holes}");
    // и ни одна клетка даже не тронута распадом
    let decayed = world.plane.tiles.values().filter(|t| t.decay > 0).count();
    assert_eq!(decayed, 0, "распада нет вовсе");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn tripled_bite_burns_the_world_faster() {
    let death_tick = |expr: Option<&str>, tag: &str| -> u64 {
        let (mut world, dir) = world_with_entropy(tag, expr);
        let mut died_at = 0;
        for t in 0..12_000u64 {
            if t % 40 == 0 && world.cycle.phase != Phase::Dead {
                world.draw(None).unwrap();
            }
            world.step();
            if world.cycle.phase == Phase::Dead {
                died_at = world.tick;
                break;
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
        assert!(died_at > 0, "мир так и не умер ({tag})");
        died_at
    };
    let normal = death_tick(None, "normal");
    let fast = death_tick(Some("(░ (∈ ▢) (⏱ 1 × × ×) (⌛ 400) (✺ 120))"), "burning");
    assert!(
        fast * 2 < normal,
        "утроенный укус должен жечь заметно быстрее: дефолт {normal}, усиленный {fast}"
    );
}

#[test]
fn silent_threshold_means_no_official_death() {
    let (mut world, dir) = world_with_entropy("nothreshold", None);
    // порвать Скрижаль Порога: пустая — мир не умирает официально
    let tdir = Tablets::dir_for(&dir);
    palimpsest_core::tablet::write(
        &tdir,
        TabletSlot::Threshold,
        &palimpsest_core::rune::parse("(†)"),
    )
    .unwrap();
    world.reload_tablets();

    live(&mut world, 4000);
    assert_ne!(
        world.cycle.phase,
        Phase::Dead,
        "молчащий Порог — мир доживает в лохмотьях, но не умирает"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn default_expressions_parse_to_themselves() {
    // дефолтные скрижали — неподвижные точки принтера
    for text in [
        DEFAULT_ENTROPY,
        palimpsest_core::tablet::DEFAULT_HEARTH,
        palimpsest_core::tablet::DEFAULT_HEART,
        palimpsest_core::tablet::DEFAULT_THRESHOLD,
    ] {
        let e = palimpsest_core::rune::parse(text);
        assert_eq!(palimpsest_core::rune::pretty(&e), text, "канонический вид: {text}");
    }
}

#[test]
fn heart_law_governs_call_and_gestures() {
    let (mut world, dir) = world_with_entropy("heart", None);
    assert_eq!(world.gestures, 3, "✋ 3 из дефолтной Скрижали Сердца");
    assert!(!world.deck_calls(), "сразу после рождения колода молчит");
    for _ in 0..95 {
        world.step();
    }
    assert!(world.deck_calls(), "спустя ⏱ 90 тиков колода зовёт");
    world.draw(None).unwrap();
    assert!(!world.deck_calls(), "тяга перезаводит зов");
    let _ = std::fs::remove_dir_all(&dir);
}
