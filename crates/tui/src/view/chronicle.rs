//! Панель хроники. Строки проявляются, а не печатаются: свежая проходит
//! ступени яркости ink_muted → ink_faded → ink за ~0.8 с (AESTHETICS.md).
//! Никакого машинописного эффекта — он мешает следить за картой.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

use crate::app::App;

pub fn render(app: &App, area: Rect, buf: &mut Buffer) {
    let pal = &app.pal;
    let paper = pal.get("paper");
    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(pal.get("frame")).bg(paper))
        .title(" хроника ")
        .title_style(Style::default().fg(pal.get("ink_muted")).bg(paper))
        .style(Style::default().bg(paper));
    let inner = block.inner(area);
    block.render(area, buf);

    // последние строки, свежие снизу; хроника дышит пустыми строками
    let mut lines: Vec<Line> = Vec::new();
    let take = (inner.height as usize / 2).max(1);
    let start = app.chronicle.len().saturating_sub(take);
    for (text, born) in &app.chronicle[start..] {
        let ms = born.elapsed().as_millis();
        let fg = if ms < 300 {
            pal.get("ink_muted")
        } else if ms < 800 {
            pal.get("ink_faded")
        } else {
            pal.get("ink")
        };
        lines.push(Line::styled(text.clone(), Style::default().fg(fg)));
        lines.push(Line::raw(""));
    }

    // прижать свежее к низу панели
    let height = inner.height as usize;
    let skip = lines.len().saturating_sub(height);
    let visible: Vec<Line> = lines.into_iter().skip(skip).collect();

    Paragraph::new(visible)
        .wrap(Wrap { trim: false })
        .style(Style::default().bg(paper))
        .render(inner, buf);
}
