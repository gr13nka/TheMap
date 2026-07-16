//! Панель карты: плоскость мира в гуаши. Мир дышит, а не мигает:
//! мерцание воды (~12% клеток), дрожь луга (~1%), курсор-пульс золотом —
//! всё от фазы анимации, детерминированно по (x, y, phase), без RNG.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, Widget};

use palimpsest_core::TileKind;

use crate::app::App;

/// Крошечный хеш клетки и фазы — стабильное «дыхание» без генератора.
fn breath(x: i32, y: i32, phase: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B9);
    h ^= (y as u32).wrapping_mul(0x85EB_CA6B);
    h ^= phase.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 16;
    h
}

pub fn render(app: &App, area: Rect, buf: &mut Buffer) {
    let pal = &app.pal;
    let paper = pal.get("paper");
    let frame = pal.get("frame");

    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(frame).bg(paper))
        .style(Style::default().bg(paper));
    let inner = block.inner(area);
    block.render(area, buf);

    let plane = &app.world.plane;
    for y in 0..plane.h.min(inner.height as i32) {
        for x in 0..plane.w.min(inner.width as i32) {
            let kind = plane.get(x, y);
            let b = breath(x, y, app.anim_phase);
            let (mut glyph, mut fg) = match kind {
                TileKind::Empty => (' ', paper),
                TileKind::Forest => ('♠', pal.get("moss")),
                TileKind::Water => ('~', pal.get("water")),
                TileKind::Stone => ('^', pal.get("hills")),
                TileKind::Meadow => (',', pal.get("meadow")),
                TileKind::Void => (' ', pal.get("void")),
                TileKind::Hearth => ('#', pal.get("brick")),
                TileKind::Path => ('·', pal.get("ink_faded")),
                TileKind::Ruin => ('#', pal.get("ink_muted")),
            };
            // дыра в бумаге — чернее фона
            if kind == TileKind::Void {
                if let Some(cell) = buf.cell_mut((inner.x + x as u16, inner.y + y as u16)) {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(pal.get("void")));
                }
                continue;
            }
            // истлевание: пустота выедает по стадиям — дизеринг как язык границ
            let decay = plane.tiles.get(&(x, y)).map(|t| t.decay).unwrap_or(0);
            if decay > 0 {
                glyph = match decay {
                    1 => '░',
                    2 => '▒',
                    _ => '▓',
                };
                fg = if decay == 1 {
                    pal.get("void_edge_near")
                } else {
                    pal.get("void_edge_far")
                };
            }
            // мерцание воды: ~ ⇄ ≈, цвет — в глубину
            if decay == 0 && kind == TileKind::Water && b % 8 == 0 {
                glyph = '≈';
                fg = pal.get("water_deep");
            }
            // дрожь травы: , ⇄ '
            if decay == 0 && kind == TileKind::Meadow && b % 97 == 0 {
                glyph = '\'';
            }
            let cell_x = inner.x + x as u16;
            let cell_y = inner.y + y as u16;
            if let Some(cell) = buf.cell_mut((cell_x, cell_y)) {
                cell.set_char(glyph);
                cell.set_style(Style::default().fg(fg).bg(paper));
            }
        }
    }

    // курсор-пульс: знак, что мир жив, даже когда время стоит.
    // В режимах руки (жесты, выбор места) курсор — рука Правителя, без мигания.
    let hand_mode = matches!(
        app.mode,
        crate::app::Mode::Intervene | crate::app::Mode::SiteChoice
    );
    let (cx, cy) = if hand_mode { app.hand } else { app.world.cursor };
    if cx >= 0
        && cy >= 0
        && cx < inner.width as i32
        && cy < inner.height as i32
        && (hand_mode || app.anim_phase % 2 == 0)
    {
        if let Some(cell) = buf.cell_mut((inner.x + cx as u16, inner.y + cy as u16)) {
            cell.set_char('▊');
            cell.set_style(Style::default().fg(app.pal.get("gold")).bg(paper));
        }
    }
}
