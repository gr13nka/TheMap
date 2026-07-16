//! Браузер колоды и стол крафта. Крафт — структурный редактор дерева рун:
//! курсор ходит по узлам, скобки сломать невозможно. Слева выражение,
//! снизу палитра открытых рун (цвет семейства — единственная подпись:
//! полная тьма), справа — зыбкое превью: мутное пятно, которое движется.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use palimpsest_core::TileKind;

use crate::app::App;
use crate::view::rune_text;

fn tile_color(app: &App, kind: TileKind) -> ratatui::style::Color {
    let pal = &app.pal;
    match kind {
        TileKind::Forest => pal.get("moss"),
        TileKind::Water => pal.get("water"),
        TileKind::Stone => pal.get("hills"),
        TileKind::Meadow => pal.get("meadow"),
        TileKind::Void => pal.get("void_edge_far"),
        TileKind::Hearth => pal.get("brick"),
        TileKind::Path => pal.get("ink_faded"),
        TileKind::Ruin => pal.get("ink_muted"),
        TileKind::Empty => pal.get("paper"),
    }
}

/// Браузер колоды: список карт, выбор — раскрыть на столе.
pub fn render_browse(app: &App, area: Rect, buf: &mut Buffer) {
    let pal = &app.pal;
    let paper = pal.get("paper");
    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(pal.get("frame")).bg(paper))
        .title(" колода Правителя ")
        .title_style(Style::default().fg(pal.get("ink_muted")).bg(paper))
        .style(Style::default().bg(paper));
    let inner = block.inner(area);
    block.render(area, buf);

    let names = app.world.deck.order();
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::styled(
        "сверху — то, что ляжет следующей тягой",
        Style::default().fg(pal.get("ink_muted")),
    ));
    lines.push(Line::raw(""));
    for (i, name) in names.iter().enumerate() {
        let stem = name.trim_end_matches(".md");
        let (fg, marker) = if i == app.browse_selected {
            (pal.get("gold"), "▊ ")
        } else {
            (pal.get("ink"), "  ")
        };
        lines.push(Line::styled(
            format!("{marker}{stem}"),
            Style::default().fg(fg),
        ));
    }
    lines.push(Line::raw(""));
    lines.push(Line::styled(
        format!("чистых листов: {}", app.legacy.blank_cards),
        Style::default().fg(if app.legacy.blank_cards > 0 {
            pal.get("gold")
        } else {
            pal.get("ink_muted")
        }),
    ));
    lines.push(Line::raw(""));
    lines.push(Line::styled(
        "[j/k] выбрать   [enter] раскрыть   [n] новая карта   [q] назад",
        Style::default().fg(pal.get("ink_muted")),
    ));

    Paragraph::new(lines)
        .style(Style::default().bg(paper))
        .render(inner, buf);
}

/// Стол крафта: выражение + палитра + превью.
pub fn render_craft(app: &App, area: Rect, buf: &mut Buffer) {
    let Some(craft) = &app.craft else { return };
    let pal = &app.pal;
    let paper = pal.get("paper");
    let gold = pal.get("gold");

    let title = format!(
        " {}{} ",
        craft.title,
        if craft.dirty { " · не вписано" } else { "" }
    );
    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(gold).bg(paper))
        .title(title)
        .title_style(Style::default().fg(gold).bg(paper))
        .style(Style::default().bg(paper));
    let inner = block.inner(area);
    block.render(area, buf);

    // --- выражение с курсором ---
    let mut expr_line = vec![Span::styled("  ", Style::default().bg(paper))];
    expr_line.extend(rune_text::spans(app, &craft.expr, Some(&craft.cursor)));
    let expr_y = inner.y + 2;
    render_line(buf, inner.x, expr_y, inner.width, Line::from(expr_line), paper);

    // --- палитра открытых рун ---
    let pal_y = expr_y + 3;
    let mut spans: Vec<Span> = vec![Span::styled("  ", Style::default().bg(paper))];
    for (i, r) in craft.palette.iter().enumerate() {
        let selected = i == craft.selected;
        let style = if selected {
            Style::default()
                .fg(rune_text::rune_color(app, *r))
                .bg(pal.get("frame"))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(rune_text::rune_color(app, *r)).bg(paper)
        };
        spans.push(Span::styled(
            format!("{}{}{}", if selected { '[' } else { ' ' }, r.ch(), if selected { ']' } else { ' ' }),
            style,
        ));
        spans.push(Span::styled(" ", Style::default().bg(paper)));
    }
    render_line(buf, inner.x, pal_y, inner.width, Line::from(spans), paper);

    // --- зыбкое превью: кадры листаются тиком анимации ---
    let pv_y = pal_y + 2;
    if craft.frames.is_empty() {
        render_line(
            buf,
            inner.x + 2,
            pv_y + 3,
            inner.width,
            Line::styled("пятно молчит", Style::default().fg(pal.get("ink_muted"))),
            paper,
        );
    } else {
        let frame = &craft.frames[app.anim_phase as usize % craft.frames.len()];
        for by in 0..(frame.h / 2) {
            for bx in 0..(frame.w / 2) {
                let mut kinds: Vec<TileKind> = Vec::with_capacity(4);
                for dy in 0..2 {
                    for dx in 0..2 {
                        let k = frame.get(bx * 2 + dx, by * 2 + dy);
                        if k != TileKind::Empty {
                            kinds.push(k);
                        }
                    }
                }
                let (ch, fg) = match kinds.len() {
                    0 => ('░', pal.get("frame")),
                    1 => ('░', tile_color(app, kinds[0])),
                    2 => ('▒', tile_color(app, kinds[0])),
                    3 => ('▓', tile_color(app, kinds[0])),
                    _ => ('█', tile_color(app, kinds[0])),
                };
                let x = inner.x + 2 + bx as u16;
                let y = pv_y + by as u16;
                if x < inner.right() && y < inner.bottom() {
                    if let Some(cell) = buf.cell_mut((x, y)) {
                        cell.set_char(ch);
                        cell.set_style(Style::default().fg(fg).bg(paper));
                    }
                }
            }
        }
    }

    // --- подсказки ---
    let help_y = inner.y + inner.height.saturating_sub(2);
    for (i, text) in [
        "[стрелки] по дереву: ←→ соседи · ↓ внутрь · ↑ наружу      [tab] руна палитры",
        "[пробел] положить/снять  [a] добавить рядом  [(] обернуть  [x] удалить  [0-9] число  [w] вписать  [q] назад",
    ]
    .iter()
    .enumerate()
    {
        render_line(
            buf,
            inner.x + 1,
            help_y + i as u16 - 1,
            inner.width.saturating_sub(2),
            Line::styled(*text, Style::default().fg(pal.get("ink_muted"))),
            paper,
        );
    }
}

fn render_line(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    max_w: u16,
    line: Line,
    bg: ratatui::style::Color,
) {
    let mut cx = x;
    for span in line.spans {
        for ch in span.content.chars() {
            if cx >= x + max_w {
                return;
            }
            if let Some(cell) = buf.cell_mut((cx, y)) {
                cell.set_char(ch);
                let style = if span.style.bg.is_none() {
                    span.style.bg(bg)
                } else {
                    span.style
                };
                cell.set_style(style);
            }
            cx += 1;
        }
    }
}
