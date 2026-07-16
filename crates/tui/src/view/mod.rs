//! Сборка кадра: карта занимает максимум места, справа — колода и хроника,
//! внизу — vim-статус словами. Полная перерисовка каждый кадр (immediate
//! mode ratatui) — ровно то, что делал и голый crossterm, только с панелями.

pub mod atlas;
pub mod chronicle;
pub mod core;
pub mod craft;
pub mod deck;
pub mod map;
pub mod ritual;
pub mod rune_text;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Widget};
use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
    let paper = app.pal.get("paper");
    let area = frame.area();
    let buf = frame.buffer_mut();

    // фон всего кадра — бумага
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char(' ');
                cell.set_style(Style::default().bg(paper));
            }
        }
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(1)])
        .split(area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(app.world.plane.w as u16 + 2),
            Constraint::Length(34),
        ])
        .split(rows[0]);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(3)])
        .split(cols[1]);

    use crate::app::Mode;

    // браузер колоды, стол крафта, ядро и атлас замещают карту; хроника рядом
    match app.mode {
        Mode::DeckBrowse => craft::render_browse(app, cols[0], buf),
        Mode::Craft => craft::render_craft(app, cols[0], buf),
        Mode::Core => core::render(app, cols[0], buf),
        Mode::Atlas => atlas::render(app, cols[0], buf),
        _ => map::render(app, cols[0], buf),
    }
    deck::render(app, right[0], buf);
    chronicle::render(app, right[1], buf);

    // модальные ритуалы — поверх карты
    match app.mode {
        Mode::DeathRitual => ritual::render(app, cols[0], buf),
        Mode::MetaChoice => render_meta_modal(app, cols[0], buf),
        Mode::DirectionChoice => render_direction_modal(app, cols[0], buf),
        _ => {}
    }

    // статус-строка: слова, не пиктограммы; подсказки — по режиму
    let phase = match app.world.cycle.phase {
        palimpsest_core::cycle::Phase::Bloom => "лист цветёт",
        palimpsest_core::cycle::Phase::Wane => "лист желтеет",
        palimpsest_core::cycle::Phase::Dying => "лист умирает",
        palimpsest_core::cycle::Phase::Dead => "лист мёртв",
    };
    let hint = match app.mode {
        Mode::Observe => {
            "[пробел] тянуть  [i] жесты  [c] колода  [t] ядро  [a] атлас  [p] пауза  [1]/[4] скорость  [q] выйти"
        }
        Mode::Core => "[j/k] скрижаль  [enter] править  [q] назад",
        Mode::Intervene => {
            "[стрелки] рука  [tab] материя  [пробел] закрасить  [x] стереть  [s] спасти  [q] назад"
        }
        Mode::SiteChoice => "[стрелки] выбрать место посева  [enter] посеять  [esc] пусть решит случай",
        Mode::DirectionChoice => "[стрелки] куда направить  [esc] пусть решит случай",
        Mode::MetaChoice => "[j/k] цель  [enter] исполнить  [esc] отвести руку",
        _ => "",
    };
    let gestures = if app.mode == Mode::Intervene {
        let matters = app.unlocked_matters();
        let matter = matters
            .get(app.hand_matter % matters.len().max(1))
            .map(|m| m.ru())
            .unwrap_or("—");
        format!(" · жесты {} · в руке {}", app.world.gestures, matter)
    } else {
        String::new()
    };
    let status = format!(
        " {} · лист {} · {} · тик {} · тяга {}{}   {}",
        app.speed.label(),
        palimpsest_core::archivist::roman(app.world.cycle.epoch),
        phase,
        app.world.tick,
        app.world.draw_count,
        gestures,
        hint,
    );
    Paragraph::new(Line::styled(
        status,
        Style::default().fg(app.pal.get("ink_muted")).bg(paper),
    ))
    .render(rows[1], buf);
}

/// Модал мета-карты: выбор цели. Золото — это ритуал колоды.
fn render_meta_modal(app: &App, area: Rect, buf: &mut ratatui::buffer::Buffer) {
    use palimpsest_core::world::MetaOp;
    let title = match app.pending_meta {
        Some(MetaOp::Duplicate) => " какую карту раздвоить? ",
        Some(MetaOp::Destroy) => " какую карту предать забвению? ",
        _ => " колода ждёт ",
    };
    let names = app.world.deck.order();
    let lines: Vec<String> = names
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let stem = n.trim_end_matches(".md");
            if i == app.meta_selected {
                format!("▊ {stem}")
            } else {
                format!("  {stem}")
            }
        })
        .collect();
    render_modal(app, area, buf, title, &lines);
}

/// Модал выбора направления.
fn render_direction_modal(app: &App, area: Rect, buf: &mut ratatui::buffer::Buffer) {
    let lines = vec![
        "        ↑        ".to_string(),
        "   ←  русло  →   ".to_string(),
        "        ↓        ".to_string(),
    ];
    render_modal(app, area, buf, " куда направить? ", &lines);
}

/// Общий золотой модал по центру области.
fn render_modal(
    app: &App,
    area: Rect,
    buf: &mut ratatui::buffer::Buffer,
    title: &str,
    body: &[String],
) {
    use ratatui::widgets::{Block, Borders, Clear};
    let paper = app.pal.get("paper");
    let gold = app.pal.get("gold");
    let w = body
        .iter()
        .map(|l| l.chars().count() as u16)
        .max()
        .unwrap_or(10)
        .max(title.chars().count() as u16)
        + 6;
    let h = body.len() as u16 + 2;
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height.saturating_sub(h) / 2;
    let win = Rect::new(x, y, w.min(area.width), h.min(area.height));

    Clear.render(win, buf);
    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(gold).bg(paper))
        .title(title.to_string())
        .title_style(Style::default().fg(gold).bg(paper))
        .style(Style::default().bg(paper));
    let inner = block.inner(win);
    block.render(win, buf);

    let lines: Vec<Line> = body
        .iter()
        .map(|l| Line::styled(l.clone(), Style::default().fg(app.pal.get("ink"))))
        .collect();
    Paragraph::new(lines)
        .style(Style::default().bg(paper))
        .render(inner, buf);
}
