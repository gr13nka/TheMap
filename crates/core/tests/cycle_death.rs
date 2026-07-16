//! Гейт Фазы 4: лист доживает до смерти, наследие переходит в следующий —
//! руины детерминированы и проступают на новом листе, колода переживает мир,
//! смерть открывает пустоту.

use std::path::PathBuf;

use palimpsest_core::cycle::Phase;
use palimpsest_core::event::Event;
use palimpsest_core::legacy::Legacy;
use palimpsest_core::rune::Rune;
use palimpsest_core::tile::TileKind;
use palimpsest_core::World;

const FOREST: &str = "---\nname: Лес\nkind: rune\n---\n\n```rune\n(♠ (Y ↑ 5) (⌛ 140))\n```\n";
const RIVER: &str = "---\nname: Река\nkind: rune\n---\n\n```rune\n(~ (∩ ↓ ↷ ^) (⌛ 160))\n```\n";
const MEADOW: &str = "---\nname: Луг\nkind: rune\n---\n\n```rune\n(, (∴ +) (⌛ 120))\n```\n";

fn temp_deck(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("palimpsest_cycle_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("forest.md"), FOREST).unwrap();
    std::fs::write(dir.join("river.md"), RIVER).unwrap();
    std::fs::write(dir.join("meadow.md"), MEADOW).unwrap();
    dir
}

/// Прожить лист до смерти; вернуть мир и его наследие.
fn live_and_die(dir: &PathBuf) -> (World, Legacy) {
    let mut world = World::new(dir.clone(), 48, 20, 11).unwrap();
    let mut legacy = Legacy::default();
    for t in 0..12_000u64 {
        if t % 40 == 0 && world.cycle.phase != Phase::Dead {
            world.draw(None).unwrap();
        }
        let events = world.step();
        legacy.witness(&events);
        if let Some(Event::WorldDead { summary }) = events
            .iter()
            .find(|e| matches!(e, Event::WorldDead { .. }))
        {
            legacy.absorb_death(&world, summary.clone());
            return (world, legacy);
        }
    }
    panic!("мир не умер за 12000 тиков — энтропия сломана");
}

#[test]
fn world_dies_and_leaves_legacy() {
    let dir = temp_deck("death");
    let (world, legacy) = live_and_die(&dir);

    assert_eq!(world.cycle.phase, Phase::Dead);
    let summary = legacy.summaries.last().expect("итог цикла записан");
    assert!(summary.ticks_lived > 500, "мир прожил подозрительно мало");
    assert!(summary.peak_filled > 50, "мир так и не расцвёл");

    // смерть открывает пустоту — Правитель получает оружие врага
    assert!(legacy.is_unlocked(Rune::Voidness));
    assert!(legacy.is_unlocked(Rune::Burst), "тяг было много — вспышка открыта");
    // …и право читать законы: законные руны открыты первой смертью
    assert!(legacy.is_unlocked(Rune::Heart), "смерть даёт право на законы");
    assert!(legacy.is_unlocked(Rune::StageFull));
    // …и бумагу: чистый лист для нового слова
    assert_eq!(legacy.blank_cards, 1, "смерть дарит чистый лист");
    assert_eq!(legacy.epoch, 2, "эпоха перевернулась");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ruins_survive_into_next_sheet() {
    let dir = temp_deck("ruins");
    let (_world, legacy) = live_and_die(&dir);

    let next = World::new_epoch(dir.clone(), 48, 20, 11, &legacy).unwrap();
    assert_eq!(next.cycle.epoch, 2);
    assert_eq!(next.tick, 0);

    // руины прошлого проступают на свежем листе ровно по легаси
    let on_sheet = next.plane.filled();
    assert_eq!(
        on_sheet,
        legacy.ruins.len(),
        "на новом листе должно быть ровно столько следов, сколько в наследии"
    );

    // и это именно руины/камень, слегка изъеденные
    for ruin in &legacy.ruins {
        let kind = next.plane.get(ruin.pos.0, ruin.pos.1);
        assert!(matches!(kind, TileKind::Ruin | TileKind::Stone));
        let decay = next.plane.tiles.get(&ruin.pos).unwrap().decay;
        assert!(decay > 0, "руины должны быть тронуты временем");
    }

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn deaths_are_deterministic() {
    // колода пересоздаётся между жизнями: тяги мутируют .md-файлы карт,
    // и вторая жизнь на той же колоде — уже другая история (так задумано)
    let dir = temp_deck("determ");
    let (_w1, l1) = live_and_die(&dir);
    let dir = temp_deck("determ");
    let (_w2, l2) = live_and_die(&dir);
    let r1: Vec<_> = l1.ruins.iter().map(|r| r.pos).collect();
    let r2: Vec<_> = l2.ruins.iter().map(|r| r.pos).collect();
    assert_eq!(r1, r2, "руины двух одинаковых жизней разошлись");
    let _ = std::fs::remove_dir_all(&dir);
}
