//! Ядро мира — все четыре скрижали на одном экране. Весь интерпретатор,
//! малый и цельный, как десять строк eval: пустота грызёт, очаг ветвится,
//! сердце пульсирует, порог ждёт. Enter — править закон тем же крафтом;
//! мир исполнит написанное буквально.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use palimpsest_core::tablet::TabletSlot;

use crate::app::App;
use crate::view::rune_text;

pub fn render(app: &App, area: Rect, buf: &mut Buffer) {
    let pal = &app.pal;
    let paper = pal.get("paper");
    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(pal.get("frame")).bg(paper))
        .title(" ядро мира ")
        .title_style(Style::default().fg(pal.get("ink_muted")).bg(paper))
        .style(Style::default().bg(paper));
    let inner = block.inner(area);
    block.render(area, buf);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::styled(
        "четыре закона; мир исполняет написанное буквально",
        Style::default().fg(pal.get("ink_muted")),
    ));
    lines.push(Line::raw(""));

    for (i, slot) in TabletSlot::ALL.iter().enumerate() {
        let selected = i == app.core_selected;
        let marker = if selected { "▊ " } else { "  " };
        let name_fg = if selected {
            pal.get("gold")
        } else {
            pal.get("ink")
        };
        lines.push(Line::styled(
            format!("{marker}{}", slot.title()),
            Style::default().fg(name_fg),
        ));
        let mut spans = vec![Span::styled("    ", Style::default().bg(paper))];
        spans.extend(rune_text::spans(app, app.world.tablets.expr(*slot), None));
        lines.push(Line::from(spans));
        lines.push(Line::raw(""));
    }

    lines.push(Line::styled(
        "[j/k] выбрать   [enter] править закон   [q] назад",
        Style::default().fg(pal.get("ink_muted")),
    ));

    Paragraph::new(lines)
        .style(Style::default().bg(paper))
        .render(inner, buf);
}
