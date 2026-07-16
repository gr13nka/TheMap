//! Общий рендер рунных выражений: скобки окрашены по глубине, материи —
//! краской своего тайла (обучение без слов), глаголы — чернилами,
//! модификаторы — блёклыми, узел под курсором — инверсией. Один рендер
//! для крафта, экрана ядра и браузера колоды.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

use palimpsest_core::rune::{Expr, Family, Rune};

use crate::app::App;

/// Цвет руны — по семейству и материи; никаких подписей.
pub fn rune_color(app: &App, r: Rune) -> Color {
    let pal = &app.pal;
    match r {
        Rune::Water => pal.get("water"),
        Rune::Wood => pal.get("moss"),
        Rune::Stone => pal.get("hills"),
        Rune::Meadow => pal.get("meadow"),
        Rune::Voidness => pal.get("void_edge_near"),
        Rune::Hearth => pal.get("brick"),
        Rune::Ruin => pal.get("ink_muted"),
        _ => match r.family() {
            Family::Verb => pal.get("ink"),
            Family::Aim => pal.get("ink_faded"),
            Family::Time | Family::Predicate => pal.get("ochre"),
            _ => pal.get("ink_muted"),
        },
    }
}

/// Цвет скобки по глубине — три приглушённых тона по кругу.
fn paren_color(app: &App, depth: usize) -> Color {
    let pal = &app.pal;
    match depth % 3 {
        0 => pal.get("ink_faded"),
        1 => pal.get("ochre"),
        _ => pal.get("water_deep"),
    }
}

/// Выражение в спаны; `cursor` — путь выделенного узла (весь его текст
/// инвертируется). Пустой путь — выделено всё выражение.
pub fn spans(app: &App, expr: &Expr, cursor: Option<&[usize]>) -> Vec<Span<'static>> {
    let mut out = Vec::new();
    walk(app, expr, cursor, &mut Vec::new(), 0, &mut out);
    out
}

fn walk(
    app: &App,
    expr: &Expr,
    cursor: Option<&[usize]>,
    path: &mut Vec<usize>,
    depth: usize,
    out: &mut Vec<Span<'static>>,
) {
    let selected = cursor.map(|c| c == path.as_slice()).unwrap_or(false);
    let invert = |mut style: Style| -> Style {
        if selected {
            style = style
                .bg(app.pal.get("frame"))
                .add_modifier(Modifier::BOLD);
        }
        style.bg(if selected {
            app.pal.get("frame")
        } else {
            app.pal.get("paper")
        })
    };

    match expr {
        Expr::Rune(r) => {
            out.push(Span::styled(
                r.ch().to_string(),
                invert(Style::default().fg(rune_color(app, *r))),
            ));
        }
        Expr::Num(n) => {
            out.push(Span::styled(
                n.to_string(),
                invert(Style::default().fg(app.pal.get("ink_muted"))),
            ));
        }
        Expr::List(items) => {
            // выделенный список инвертируется целиком — включая содержимое
            let inner_cursor = if selected { None } else { cursor };
            let paren = if selected {
                invert(Style::default().fg(app.pal.get("gold")))
            } else {
                Style::default()
                    .fg(paren_color(app, depth))
                    .bg(app.pal.get("paper"))
            };
            out.push(Span::styled("(", paren));
            for (i, child) in items.iter().enumerate() {
                if i > 0 {
                    out.push(Span::styled(
                        " ",
                        Style::default().bg(if selected {
                            app.pal.get("frame")
                        } else {
                            app.pal.get("paper")
                        }),
                    ));
                }
                path.push(i);
                if selected {
                    // содержимое выделенного узла — тоже с инверсией
                    walk_selected(app, child, depth + 1, out);
                } else {
                    walk(app, child, inner_cursor, path, depth + 1, out);
                }
                path.pop();
            }
            out.push(Span::styled(")", paren));
        }
    }
}

/// Поддерево внутри выделенного узла: те же цвета, инвертированный фон.
fn walk_selected(app: &App, expr: &Expr, depth: usize, out: &mut Vec<Span<'static>>) {
    let bg = app.pal.get("frame");
    match expr {
        Expr::Rune(r) => out.push(Span::styled(
            r.ch().to_string(),
            Style::default().fg(rune_color(app, *r)).bg(bg),
        )),
        Expr::Num(n) => out.push(Span::styled(
            n.to_string(),
            Style::default().fg(app.pal.get("ink_muted")).bg(bg),
        )),
        Expr::List(items) => {
            let paren = Style::default().fg(paren_color(app, depth)).bg(bg);
            out.push(Span::styled("(", paren));
            for (i, child) in items.iter().enumerate() {
                if i > 0 {
                    out.push(Span::styled(" ", Style::default().bg(bg)));
                }
                walk_selected(app, child, depth + 1, out);
            }
            out.push(Span::styled(")", paren));
        }
    }
}
