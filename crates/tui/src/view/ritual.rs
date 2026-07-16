//! Ритуал смерти листа — второй спектакль после тяги, весь в золоте.
//! Поверх дотлевающей карты — эпилог архивариуса: что осталось от мира
//! (руины, хроника, знание) и слово свидетеля.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap};

use palimpsest_core::archivist;

use crate::app::App;

pub fn render(app: &App, area: Rect, buf: &mut Buffer) {
    let pal = &app.pal;
    let paper = pal.get("paper");
    let gold = pal.get("gold");

    // окно эпилога по центру
    let w = (area.width.saturating_sub(10)).min(64).max(30);
    let h = (app.epilogue.len() as u16 + 6).min(area.height.saturating_sub(2));
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let win = Rect::new(x, y, w, h);

    Clear.render(win, buf);
    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(gold).bg(paper))
        .title(format!(" лист {} умер ", archivist::roman(app.world.cycle.epoch)))
        .title_style(Style::default().fg(gold).bg(paper))
        .style(Style::default().bg(paper));
    let inner = block.inner(win);
    block.render(win, buf);

    let mut lines: Vec<Line> = app
        .epilogue
        .iter()
        .map(|l| Line::styled(l.clone(), Style::default().fg(pal.get("ink"))))
        .collect();
    lines.push(Line::raw(""));
    lines.push(Line::styled(
        "[n] развернуть новый лист   [q] уйти",
        Style::default().fg(pal.get("ink_muted")),
    ));

    Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .style(Style::default().bg(paper))
        .render(inner, buf);
}
