//! Разбор рунного текста. Парсер толерантен как бумага: неизвестные символы
//! молча пропускаются, незакрытые скобки закрываются на конце, лишние `)`
//! игнорируются. Кривая карта не роняет мир — она просто значит меньше,
//! чем хотела.

use super::{Expr, Rune};

pub fn parse(text: &str) -> Expr {
    let mut stack: Vec<Vec<Expr>> = vec![Vec::new()];
    let mut digits = String::new();

    let flush_num = |stack: &mut Vec<Vec<Expr>>, digits: &mut String| {
        if !digits.is_empty() {
            if let Ok(n) = digits.parse::<u32>() {
                if let Some(top) = stack.last_mut() {
                    top.push(Expr::Num(n));
                }
            }
            digits.clear();
        }
    };

    for c in text.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
            continue;
        }
        flush_num(&mut stack, &mut digits);
        match c {
            '(' => stack.push(Vec::new()),
            ')' => {
                if stack.len() > 1 {
                    let done = stack.pop().unwrap();
                    stack.last_mut().unwrap().push(Expr::List(done));
                }
                // лишняя `)` на верхнем уровне — молча мимо
            }
            _ => {
                if let Some(r) = Rune::from_char(c) {
                    stack.last_mut().unwrap().push(Expr::Rune(r));
                }
                // пробелы и мусор — молча мимо
            }
        }
    }
    flush_num(&mut stack, &mut digits);

    // незакрытые скобки закрываются сами
    while stack.len() > 1 {
        let done = stack.pop().unwrap();
        stack.last_mut().unwrap().push(Expr::List(done));
    }

    let mut top = stack.pop().unwrap();
    // одинокое выражение — оно и есть карта; несколько — обернём
    if top.len() == 1 {
        top.pop().unwrap()
    } else {
        Expr::List(top)
    }
}
