//! Панель колоды: рубашка, ритуал тяги и сердцебиение. Золото — только
//! здесь и в курсоре: дефицит делает его церемониальным. Когда Скрижаль
//! Сердца велит — рубашка пульсирует золотом: колода зовёт. После тяги
//! панель словами говорит, что доступно руке.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::app::{App, Pulse, RITUAL_MS};

pub fn render(app: &App, area: Rect, buf: &mut Buffer) {
    let pal = &app.pal;
    let paper = pal.get("paper");
    let gold = pal.get("gold");
    let muted = pal.get("ink_muted");
    let pulse = app.pulse();

    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(pal.get("frame")).bg(paper))
        .title(" колода ")
        .title_style(Style::default().fg(muted).bg(paper))
        .style(Style::default().bg(paper));
    let inner = block.inner(area);
    block.render(area, buf);

    let mut lines: Vec<Line> = Vec::new();
    match &app.ritual {
        // ритуал: имя проступает посимвольно, всё в золоте
        Some((name, at)) if app.ritual_active() => {
            let t = at.elapsed().as_millis() as f64 / RITUAL_MS as f64;
            let shown = ((name.chars().count() as f64) * (t * 2.0).min(1.0)) as usize;
            let partial: String = name.chars().take(shown.max(1)).collect();
            lines.push(Line::styled("┌──────────┐", Style::default().fg(gold)));
            lines.push(Line::styled(
                format!("  {partial}"),
                Style::default().fg(gold),
            ));
            lines.push(Line::styled("└──────────┘", Style::default().fg(gold)));
        }
        _ => {
            // рубашка: при зове дышит золотом (пульс — тиком анимации)
            let back_fg = match pulse {
                Pulse::Calling if app.anim_phase % 2 == 0 => gold,
                Pulse::Calling => muted,
                _ => gold,
            };
            lines.push(Line::styled("▞▚▞▚▞▚▞▚▞▚", Style::default().fg(back_fg)));
            match pulse {
                Pulse::Calling => {
                    lines.push(Line::styled(
                        "колода зовёт — тяни".to_string(),
                        Style::default().fg(gold),
                    ));
                }
                Pulse::Aftermath => {
                    lines.push(Line::styled(
                        format!("рука Правителя: жесты ×{}", app.world.gestures),
                        Style::default().fg(pal.get("ink")),
                    ));
                    lines.push(Line::styled(
                        "[i] жесты  [c] крафт  [a] атлас",
                        Style::default().fg(muted),
                    ));
                }
                _ => {
                    lines.push(Line::styled(
                        format!("верхняя: {}", app.world.top_card_name()),
                        Style::default().fg(muted),
                    ));
                    lines.push(Line::styled(
                        format!("тяг: {}", app.world.draw_count),
                        Style::default().fg(muted),
                    ));
                }
            }
        }
    }

    Paragraph::new(lines)
        .style(Style::default().bg(paper))
        .render(inner, buf);
}
