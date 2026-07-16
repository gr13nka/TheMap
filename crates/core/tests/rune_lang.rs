//! Гейт Фазы 1 шага 3: рунный язык. Парсер толерантен и обратим, компилятор
//! читает позицию как Лисп, эталонные карты дают узнаваемо разные существа,
//! качественный скачок (ритмы, цели, обход, добыча) работает, дрейф дерева
//! связен и детерминирован.

use palimpsest_core::plane::Plane;
use palimpsest_core::rune::{self, mutate, Expr, Matter, Only, Rune, Verb};
use palimpsest_core::sim::{self, seed::Seed};
use palimpsest_core::tile::TileKind;
use rand::rngs::StdRng;
use rand::SeedableRng;

const RIVER: &str = "(~ (∩ ↓ ↷ ^) (⌛ 160))";
const FOREST: &str = "(♠ (Y ↑ 5) (⌛ 140))";
const MOUNTAIN: &str = "(^ (⌛ 90))";
const MEADOW: &str = "(, (∴ +) (⌛ 120))";
const FLOOD: &str = "(~ (× ♥ !) (⌛ 60))";
const PLAGUE: &str = "(░ (× ♥ → #) (⏱ 40 !))";
const GEYSER: &str = "(~ (⏱ 30 (∩ ↑ !)))";
const RING: &str = "(♠ (∴ ↷ ~) (⌛ 100))";
const THIRST: &str = "(░ (× ~ → ~) (⌛ 80))";
const TEETH: &str = "(^ (Y ↓ 4) (⌛ 60))";

fn run_on(plane: &mut Plane, text: &str, origin: (i32, i32), ticks: u64) {
    let program = rune::compile(&rune::parse(text)).expect("выражение должно компилироваться");
    let mut seeds = vec![Seed::spawn(0, program, origin, plane)];
    let mut events = Vec::new();
    for t in 1..=ticks {
        sim::step_seeds(plane, &mut seeds, 7, t, &mut events);
    }
}

fn run(text: &str, origin: (i32, i32), ticks: u64) -> Plane {
    let mut plane = Plane::new(48, 20);
    run_on(&mut plane, text, origin, ticks);
    plane
}

// --- парсер и принтер ---

#[test]
fn parse_print_roundtrip() {
    for text in [RIVER, FOREST, MOUNTAIN, MEADOW, FLOOD, PLAGUE, GEYSER, RING, THIRST, TEETH] {
        let e = rune::parse(text);
        assert_eq!(rune::parse(&rune::pretty(&e)), e, "принтер и парсер — неподвижная точка: {text}");
    }
}

#[test]
fn parser_is_tolerant() {
    // мусор, незакрытые скобки, лишние ')' — бумага всё стерпит
    let e = rune::parse("(~ фыва (∩ ↓  ");
    assert_eq!(rune::pretty(&e), "(~ (∩ ↓))");
    let e = rune::parse(") ) (♠)");
    assert_eq!(rune::pretty(&e), "(♠)");
    // числа сливаются из цифр
    let e = rune::parse("(░ (⏱ 40 !))");
    assert!(matches!(
        e,
        Expr::List(ref l) if matches!(l[1], Expr::List(ref f) if f[1] == Expr::Num(40))
    ));
}

// --- компиляция читает позицию ---

#[test]
fn compile_reads_position_like_lisp() {
    let p = rune::compile(&rune::parse(PLAGUE)).unwrap();
    assert_eq!(p.matter, Matter::Voidness);
    assert_eq!(p.clauses.len(), 1);
    // (× ♥ → #): грызть живое, ползя к очагу
    assert_eq!(p.clauses[0].verb, Verb::Gnaw);
    assert_eq!(p.clauses[0].only, Some(Only::Living));
    assert_eq!(p.clauses[0].seek, Some(Matter::Hearth));
    // (⏱ 40 !) без клауз внутри — ритм всей карты
    assert_eq!(p.clauses[0].every, Some(40));
    assert!(p.clauses[0].burst_pulse);
}

#[test]
fn compile_reads_avoid_and_trunk() {
    let p = rune::compile(&rune::parse(RIVER)).unwrap();
    assert_eq!(p.clauses[0].avoid, Some(Matter::Stone));
    assert_eq!(p.clauses[0].dir, Some((0, 1)));
    assert_eq!(p.vitality, 160);

    let p = rune::compile(&rune::parse(FOREST)).unwrap();
    assert_eq!(p.clauses[0].trunk, Some(5));
}

#[test]
fn inert_expressions() {
    // не материя в голове — без следа
    assert!(rune::compile(&rune::parse("(∩ ↓)")).is_none());
    assert!(rune::compile(&rune::parse("()")).is_none());
    // материя без глаголов — пятно Still
    let p = rune::compile(&rune::parse(MOUNTAIN)).unwrap();
    assert_eq!(p.clauses[0].verb, Verb::Still);
}

// --- существа различимы и ведут себя по-своему ---

#[test]
fn worlds_are_deterministic() {
    for text in [RIVER, FOREST, MEADOW, GEYSER, TEETH] {
        let a = run(text, (24, 12), 90).render_glyphs();
        let b = run(text, (24, 12), 90).render_glyphs();
        assert_eq!(a, b, "один seed → один в один тот же мир: {text}");
    }
}

#[test]
fn ten_etalons_are_ten_different_beings() {
    // хищникам нужен живой мир: луг и очаги; пьющим и кольцам — вода
    let mut living = Plane::new(48, 20);
    for y in 8..18 {
        for x in 10..38 {
            living.set(x, y, TileKind::Meadow);
        }
    }
    living.set(34, 10, TileKind::Hearth);
    living.set(35, 10, TileKind::Hearth);

    let mut watered = Plane::new(48, 20);
    for y in 4..18 {
        watered.set(28, y, TileKind::Water);
    }

    let mut renders = Vec::new();
    for text in [RIVER, FOREST, MOUNTAIN, MEADOW, GEYSER, TEETH] {
        renders.push(run(text, (24, 12), 90).render_glyphs());
    }
    for text in [FLOOD, PLAGUE] {
        let mut plane = living.clone();
        run_on(&mut plane, text, (20, 12), 90);
        renders.push(plane.render_glyphs());
    }
    for text in [RING, THIRST] {
        let mut plane = watered.clone();
        run_on(&mut plane, text, (22, 12), 90);
        renders.push(plane.render_glyphs());
    }
    for i in 0..renders.len() {
        for j in (i + 1)..renders.len() {
            assert_ne!(renders[i], renders[j], "эталоны {i} и {j} неотличимы");
        }
    }
}

#[test]
fn geyser_pulses() {
    // ритм: между пульсами тишина, на пульсе — струя
    let quiet = run(GEYSER, (24, 15), 29).filled();
    let after_pulse = run(GEYSER, (24, 15), 35).filled();
    let later = run(GEYSER, (24, 15), 65).filled();
    assert!(quiet <= 1, "до первого пульса гейзер молчит, а занято {quiet}");
    assert!(after_pulse > quiet, "первый пульс должен дать струю");
    assert!(later > after_pulse, "второй пульс должен добавить");
}

#[test]
fn plague_hunts_hearths_and_spares_stone() {
    let mut plane = Plane::new(48, 20);
    for y in 6..16 {
        for x in 8..40 {
            plane.set(x, y, TileKind::Meadow);
        }
    }
    for y in 9..12 {
        plane.set(36, y, TileKind::Hearth);
    }
    plane.set(20, 10, TileKind::Stone);
    run_on(&mut plane, PLAGUE, (12, 10), 200);

    let voids: Vec<(i32, i32)> = plane
        .tiles
        .iter()
        .filter(|(_, t)| t.kind == TileKind::Void)
        .map(|(&p, _)| p)
        .collect();
    assert!(!voids.is_empty(), "чума должна оставить след");
    let max_x = voids.iter().map(|&(x, _)| x).max().unwrap();
    assert!(max_x > 22, "фронт чумы должен ползти к очагам, а дополз до x={max_x}");
    assert_eq!(plane.get(20, 10), TileKind::Stone, "камень чуме не по зубам (× ♥)");
}

#[test]
fn ring_grove_keeps_off_water() {
    let mut plane = Plane::new(48, 20);
    for y in 8..13 {
        for x in 20..26 {
            plane.set(x, y, TileKind::Water);
        }
    }
    run_on(&mut plane, RING, (28, 10), 120);
    // лес не должен подступить к воде вплотную (клетки, смежные с водой, пусты от леса)
    let mut touching = 0;
    for (&(x, y), t) in plane.tiles.iter() {
        if t.kind == TileKind::Forest {
            for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                if plane.get(x + dx, y + dy) == TileKind::Water {
                    touching += 1;
                }
            }
        }
    }
    assert_eq!(touching, 0, "роща-кольцо не должна касаться воды, а касается в {touching} местах");
    assert!(plane.count(TileKind::Forest) > 30, "при этом роща должна вырасти");
}

#[test]
fn thirst_drinks_the_river_and_spares_meadow() {
    let mut plane = Plane::new(48, 20);
    for y in 4..18 {
        plane.set(30, y, TileKind::Water);
        plane.set(26, y, TileKind::Meadow);
    }
    let water_before = plane.count(TileKind::Water);
    let meadow_before = plane.count(TileKind::Meadow);
    run_on(&mut plane, THIRST, (29, 10), 200);
    let water_after = plane.count(TileKind::Water);
    assert!(
        water_after < water_before,
        "жажда должна выпить часть воды: было {water_before}, осталось {water_after}"
    );
    assert_eq!(
        plane.count(TileKind::Meadow),
        meadow_before,
        "луг жажде не по вкусу (× ~)"
    );
}

// --- дрейф дерева ---

#[test]
fn drift_is_deterministic_and_coherent() {
    let base = rune::parse(PLAGUE);
    let drift_once = |seed: u64| {
        let mut e = base.clone();
        let mut rng = StdRng::seed_from_u64(seed);
        let moved = mutate::drift(&mut e, &mut rng);
        (moved, e)
    };
    let (m1, e1) = drift_once(3);
    let (m2, e2) = drift_once(3);
    assert_eq!(m1, m2);
    assert_eq!(e1, e2, "дрейф обязан быть детерминированным");

    for seed in 0..30u64 {
        let (moved, e) = drift_once(seed);
        if !moved {
            continue;
        }
        // материя-голова неприкосновенна
        if let Expr::List(items) = &e {
            assert_eq!(items[0], Expr::Rune(Rune::Voidness), "личность карты не тронута (seed {seed})");
        }
        // структура цела: принтер↔парсер сходятся
        assert_eq!(rune::parse(&rune::pretty(&e)), e, "скобки не сломаны (seed {seed})");
        assert_ne!(e, base, "дрейф что-то изменил (seed {seed})");
    }
}
