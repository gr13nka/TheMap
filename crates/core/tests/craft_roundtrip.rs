//! Превью детерминировано и честно; запись выражения в .md не трогает
//! ни frontmatter, ни прозу, и переживает круг parse→write→parse.

use std::path::Path;

use palimpsest_core::card::{self, Card};
use palimpsest_core::rune::{self, Expr, Rune};

const CARD: &str = "---\nname: Лес\nkind: rune\n---\n\n# Карта Леса\n\nПроза Правителя — движок её не трогает.\n\n```rune\n(♠ (Y ↑ 5) (⌛ 140))\n```\n\nПостскриптум после блока.\n";

#[test]
fn preview_is_deterministic_and_alive() {
    let expr = Card::parse(CARD, Path::new("forest.md")).expr().unwrap();
    let a = rune::preview(&expr);
    let b = rune::preview(&expr);
    assert_eq!(a.len(), b.len());
    assert!(!a.is_empty(), "живое выражение должно давать кадры");
    for (fa, fb) in a.iter().zip(&b) {
        assert_eq!(fa.render_glyphs(), fb.render_glyphs(), "превью обязано быть детерминированным");
    }
    // пятно растёт: последний кадр гуще первого
    assert!(a.last().unwrap().filled() > a.first().unwrap().filled());
}

#[test]
fn inert_expr_gives_empty_preview() {
    assert!(rune::preview(&Expr::empty()).is_empty(), "инертное выражение — пустое пятно");
}

#[test]
fn write_rune_touches_only_the_block() {
    let dir = std::env::temp_dir().join(format!("palimpsest_craft_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("forest.md");
    std::fs::write(&path, CARD).unwrap();

    // переложить руну: ↑ становится →
    let mut expr = Card::parse_file(&path).unwrap().expr().unwrap();
    if let Expr::List(items) = &mut expr {
        if let Some(Expr::List(clause)) = items.get_mut(1) {
            clause[1] = Expr::Rune(Rune::Right);
        }
    }
    card::write_rune(&path, &expr).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("name: Лес"), "frontmatter не тронут");
    assert!(text.contains("Проза Правителя"), "проза не тронута");
    assert!(text.contains("Постскриптум после блока."), "хвост не тронут");
    assert!(text.contains("(♠ (Y → 5) (⌛ 140))"), "блок переписан");

    // parse → write → parse — неподвижная точка
    let reread = Card::parse_file(&path).unwrap().expr().unwrap();
    assert_eq!(reread, expr, "перечитанное выражение должно совпасть с записанным");
    card::write_rune(&path, &reread).unwrap();
    let text2 = std::fs::read_to_string(&path).unwrap();
    assert_eq!(text, text2, "повторная запись не должна менять файл");

    let _ = std::fs::remove_dir_all(&dir);
}
