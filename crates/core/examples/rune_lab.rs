//! Рунная лаборатория — самая дешёвая проверка семантики языка без терминала.
//! Принимает выражение или путь к .md-карте:
//!
//!   cargo run -p palimpsest-core --example rune_lab -- '(~ (∩ ↓) (↷ ^))' 120
//!   cargo run -p palimpsest-core --example rune_lab -- Deck/forest.md 120 [x y]
//!
//! Печатает канонический вид, программу (для глаз творца) и кадры плоскости.
//! Главная проверка руками: малое изменение выражения — связный дрейф
//! поведения, не хаос.

use std::path::PathBuf;

use palimpsest_core::card::Card;
use palimpsest_core::plane::Plane;
use palimpsest_core::rune::{self, Expr};
use palimpsest_core::sim::{self, seed::Seed};

fn main() -> std::io::Result<()> {
    let mut args = std::env::args().skip(1);
    let source = args.next().unwrap_or_else(|| "Deck/forest.md".into());
    let ticks: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(120);
    let ox: i32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(24);
    let oy: i32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(15);

    let expr: Expr = if source.trim_start().starts_with('(') {
        rune::parse(&source)
    } else {
        let card = Card::parse_file(&PathBuf::from(&source))?;
        match card.expr() {
            Some(e) => e,
            None => {
                eprintln!("«{}» — не рунная карта (kind: {}).", card.name, card.kind);
                std::process::exit(1);
            }
        }
    };

    println!("=== {} ===", rune::pretty(&expr));

    let Some(program) = rune::compile(&expr) else {
        println!("Выражение инертно: в голове не материя — карта ляжет без следа.");
        return Ok(());
    };
    println!("{program:#?}\n");

    let mut plane = Plane::new(48, 20);
    let origin = (ox, oy);
    let mut seeds = vec![Seed::spawn(0, program, origin, &mut plane)];

    let frame_every = (ticks / 6).max(1);
    let mut events = Vec::new();
    for t in 1..=ticks {
        sim::step_seeds(&mut plane, &mut seeds, 1, t, &mut events);
        if t % frame_every == 0 || t == ticks {
            println!(
                "--- тик {t} · занято {} · посев {} ---",
                plane.filled(),
                if seeds.is_empty() { "истлел" } else { "жив" }
            );
            print!("{}", plane.render_glyphs());
        }
    }
    Ok(())
}
