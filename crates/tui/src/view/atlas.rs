//! Атлас — две вкладки. «Стопка миров» — одна растущая Вещь: строка на
//! каждый прожитый лист, вехи глубины. «Наблюдения» — знание, добытое
//! глазами: материя, эпоха, цитата из хроники. Ни одного имени руны:
//! что значит узор — Правитель по-прежнему выводит сам.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

use palimpsest_core::archivist;

use crate::app::App;

pub fn render(app: &App, area: Rect, buf: &mut Buffer) {
    let pal = &app.pal;
    let paper = pal.get("paper");
    let title = if app.atlas_tab == 0 {
        " атлас · стопка миров | наблюдения "
    } else {
        " атлас · миры | НАБЛЮДЕНИЯ "
    };
    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(pal.get("frame")).bg(paper))
        .title(title)
        .title_style(Style::default().fg(pal.get("ink_muted")).bg(paper))
        .style(Style::default().bg(paper));
    let inner = block.inner(area);
    block.render(area, buf);

    let mut lines: Vec<Line> = Vec::new();

    if app.atlas_tab == 0 {
        // --- стопка миров ---
        if app.legacy.summaries.is_empty() {
            lines.push(Line::styled(
                "Стопка пуста: ещё ни один мир не прожит до конца.",
                Style::default().fg(pal.get("ink_muted")),
            ));
            lines.push(Line::styled(
                "Когда лист умрёт, он ляжет сюда — и Вещь начнёт расти.",
                Style::default().fg(pal.get("ink_muted")),
            ));
        } else {
            let mut past: &[palimpsest_core::cycle::CycleSummary] = &[];
            let all = &app.legacy.summaries;
            for (i, s) in all.iter().enumerate().rev().skip(app.atlas_scroll) {
                past = &all[..i];
                lines.push(Line::styled(
                    format!(
                        "Мир {} · {} тиков · {} тяг · очагов {}",
                        archivist::roman(s.epoch),
                        s.ticks_lived,
                        s.draws,
                        s.hearths_founded
                    ),
                    Style::default().fg(pal.get("ink")),
                ));
                for m in archivist::milestones(s, past) {
                    lines.push(Line::styled(
                        format!("  {m}"),
                        Style::default().fg(pal.get("ochre")),
                    ));
                }
                lines.push(Line::raw(""));
            }
            let _ = past;
            lines.push(Line::styled(
                "Полные страницы — в Atlas/ (открывать в Obsidian).",
                Style::default().fg(pal.get("ink_muted")),
            ));
        }
    } else {
        // --- наблюдения ---
        if app.legacy.atlas.is_empty() {
            lines.push(Line::styled(
                "Пока пусто. Знание рождается из тяг: мир покажет — атлас запомнит.",
                Style::default().fg(pal.get("ink_muted")),
            ));
        } else {
            for obs in app.legacy.atlas.iter().rev().skip(app.atlas_scroll) {
                lines.push(Line::styled(
                    format!("{} · лист {}", obs.matter.ru(), archivist::roman(obs.epoch)),
                    Style::default().fg(pal.get("ink")),
                ));
                lines.push(Line::styled(
                    format!("  {}", obs.quote),
                    Style::default().fg(pal.get("ink_faded")),
                ));
                lines.push(Line::raw(""));
            }
        }
    }

    lines.push(Line::styled(
        "[tab] вкладка   [j/k] листать   [q] назад",
        Style::default().fg(pal.get("ink_muted")),
    ));

    Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .style(Style::default().bg(paper))
        .render(inner, buf);
}
