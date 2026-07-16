//! Мета-карты правят колоду (не мир), жесты тратят бюджет тяги, выбор
//! направления действительно поворачивает закон. (Дрейф дерева проверяется
//! в rune_lang.rs.)

use std::path::PathBuf;

use palimpsest_core::rune::Matter;
use palimpsest_core::tile::TileKind;
use palimpsest_core::world::{DrawChoice, Gesture, MetaOp};
use palimpsest_core::World;

const FOREST: &str = "---\nname: Лес\nkind: rune\n---\n\n```rune\n(♠ (Y ↑ 5) (⌛ 140))\n```\n";
const RIVER: &str = "---\nname: Река\nkind: rune\nchoice: direction\n---\n\n```rune\n(~ (∩ ↓ ↷ ^) (⌛ 160))\n```\n";
const ECHO: &str = "---\nname: Эхо\nkind: meta\nop: duplicate\n---\n";
const OBLIVION: &str = "---\nname: Забвение\nkind: meta\nop: destroy\n---\n";
const WHIRL: &str = "---\nname: Смерч\nkind: meta\nop: shuffle\n---\n";

fn temp_deck(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("palimpsest_meta_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a_echo.md"), ECHO).unwrap();
    std::fs::write(dir.join("forest.md"), FOREST).unwrap();
    std::fs::write(dir.join("oblivion.md"), OBLIVION).unwrap();
    std::fs::write(dir.join("river.md"), RIVER).unwrap();
    std::fs::write(dir.join("whirl.md"), WHIRL).unwrap();
    dir
}

#[test]
fn meta_cards_rule_the_deck() {
    let dir = temp_deck("ops");
    let mut world = World::new(dir.clone(), 48, 20, 5).unwrap();
    // верхняя карта — a_echo (порядок по имени)
    let outcome = world.draw(None).unwrap();
    assert_eq!(outcome.meta, Some(MetaOp::Duplicate));
    assert!(outcome.matter.is_none(), "мета-карта не трогает мир");

    // раздвоить лес (после тяги echo ушла в низ: forest теперь index 0)
    let before = world.deck.order().len();
    let target = world
        .deck
        .order()
        .iter()
        .position(|n| n == "forest.md")
        .unwrap();
    let name = world.apply_meta(MetaOp::Duplicate, target).unwrap();
    assert_eq!(name, "Лес");
    assert_eq!(world.deck.order().len(), before + 1);
    assert!(dir.join("forest-2.md").exists(), "двойник живёт своим файлом");
    assert_eq!(world.deck.order()[1], "forest-2.md", "двойник лёг под верх");

    // предать забвению двойника: файл уходит в могильник, не удаляется
    let target = world
        .deck
        .order()
        .iter()
        .position(|n| n == "forest-2.md")
        .unwrap();
    world.apply_meta(MetaOp::Destroy, target).unwrap();
    assert!(!dir.join("forest-2.md").exists());
    assert!(dir.join("graveyard/forest-2.md").exists(), "забвение бережёт файл");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn shuffle_is_deterministic() {
    let dir = temp_deck("shuffle");
    let order = |seed: u64| {
        let mut w = World::new(dir.clone(), 48, 20, seed).unwrap();
        // тянем до смерча (он перемешает колоду сам)
        for _ in 0..5 {
            let o = w.draw(None).unwrap();
            if o.meta == Some(MetaOp::Shuffle) {
                break;
            }
        }
        w.deck.order()
    };
    assert_eq!(order(9), order(9), "один seed — одна перетасовка");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn direction_choice_turns_the_law() {
    let dir = temp_deck("dir");
    // тянуть реку с выбором «вправо» и без выбора — русла разные
    let run = |choice: Option<DrawChoice>| {
        let mut w = World::new(dir.clone(), 48, 20, 5).unwrap();
        loop {
            let top_is_river = w
                .peek_top()
                .map(|c| c.name == "Река")
                .unwrap_or(false);
            if top_is_river {
                w.draw(choice).unwrap();
                break;
            }
            w.draw(None).unwrap();
        }
        for _ in 0..60 {
            w.step();
        }
        w.plane.render_glyphs()
    };
    let east = run(Some(DrawChoice::Direction((1, 0))));
    let fate = run(None);
    assert_ne!(east, fate, "выбор направления должен менять русло");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn gestures_spend_the_budget() {
    let dir = temp_deck("hand");
    let mut world = World::new(dir.clone(), 48, 20, 5).unwrap();
    // бюджет жестов — руна ✋ в Скрижали Сердца (дефолт 3)
    assert_eq!(world.gestures, 3);

    assert!(world.gesture(Gesture::Paint(Matter::Stone), (10, 10)));
    assert_eq!(world.plane.get(10, 10), TileKind::Stone);
    assert!(world.gesture(Gesture::Erase, (10, 10)));
    assert_eq!(world.plane.get(10, 10), TileKind::Empty);
    assert!(world.gesture(Gesture::Paint(Matter::Water), (11, 10)));
    assert_eq!(world.gestures, 0, "три жеста — бюджет исчерпан");
    assert!(
        !world.gesture(Gesture::Paint(Matter::Water), (12, 10)),
        "без бюджета рука не действует"
    );

    world.draw(None).unwrap();
    assert_eq!(world.gestures, 3, "тяга пополняет жесты");
    let _ = std::fs::remove_dir_all(&dir);
}
