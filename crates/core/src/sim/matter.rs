//! Матрица материй — общий для мира закон «кто кого перекрашивает».
//! Она нарочно одна на весь мир, а не пер-сигильная: комбинаторика сигилов
//! остаётся осмысленной, потому что физика встречи материй предсказуема.

use crate::plane::Plane;
use crate::rune::Matter;
use crate::tile::TileKind;

/// Может ли материя лечь поверх текущего тайла.
pub fn can_paint(current: TileKind, incoming: Matter) -> bool {
    if current == TileKind::Empty {
        return true;
    }
    match incoming {
        // пустота ест всё — включая камень (медленнее её сдержит decay)
        Matter::Voidness => true,
        // вода точит луг, валит лес, топит очаги и тропы; камень её держит
        Matter::Water => matches!(
            current,
            TileKind::Meadow | TileKind::Forest | TileKind::Hearth | TileKind::Path
        ),
        // лес наступает на луг и затягивает тропы
        Matter::Wood => matches!(current, TileKind::Meadow | TileKind::Path),
        // луг, камень — только на чистую бумагу
        Matter::Meadow | Matter::Stone => false,
        // очаг ставится на луг (и на бумагу)
        Matter::Hearth => current == TileKind::Meadow,
        // руина — след, не сила: только на бумагу
        Matter::Ruin => false,
    }
}

/// Положить материю; вернуть прежний тайл, если краска легла
/// (`Some(Empty)` — на бумагу, `Some(прочее)` — материя пожрала материю).
pub fn paint(plane: &mut Plane, x: i32, y: i32, m: Matter) -> Option<TileKind> {
    if !plane.in_bounds(x, y) {
        return None;
    }
    let cur = plane.get(x, y);
    let tile = m.tile();
    if cur == tile || !can_paint(cur, m) {
        return None;
    }
    plane.set(x, y, tile);
    Some(cur)
}

/// Квадрат расстояния до ближайшего тайла вида `kind`. Обход HashMap здесь
/// допустим: min — порядко-независимый агрегат (инвариант в plane.rs).
pub fn nearest_dist2(plane: &Plane, from: (i32, i32), kind: TileKind) -> Option<i64> {
    plane
        .tiles
        .iter()
        .filter(|(_, t)| t.kind == kind)
        .map(|(&(x, y), _)| {
            let dx = (x - from.0) as i64;
            let dy = (y - from.1) as i64;
            dx * dx + dy * dy
        })
        .min()
}

/// Позиция ближайшего тайла вида `kind`. Min по кортежу (d², x, y) —
/// порядко-независим, значит детерминирован (инвариант в plane.rs).
pub fn nearest_pos(plane: &Plane, from: (i32, i32), kind: TileKind) -> Option<(i32, i32)> {
    plane
        .tiles
        .iter()
        .filter(|(_, t)| t.kind == kind)
        .map(|(&(x, y), _)| {
            let dx = (x - from.0) as i64;
            let dy = (y - from.1) as i64;
            (dx * dx + dy * dy, x, y)
        })
        .min()
        .map(|(_, x, y)| (x, y))
}
