//! Дрейф дерева — колода живёт своей жизнью. При тяге выражение карты может
//! чуть измениться; связность гарантирована по построению: структура списков
//! не ломается, материя-голова (личность карты) неприкосновенна. Узлы
//! перечисляются DFS — детерминизм.

use rand::rngs::StdRng;
use rand::Rng;

use super::{Expr, Family, Rune};

/// Один шаг дрейфа. Вернуть true, если дерево изменилось.
pub fn drift(expr: &mut Expr, rng: &mut StdRng) -> bool {
    for _ in 0..8 {
        let roll = rng.gen_range(0..9u32);
        let changed = match roll {
            // вес 4: заменить руну соседом того же семейства
            0..=3 => swap_rune(expr, rng),
            // вес 3: качнуть число
            4..=6 => nudge_num(expr, rng),
            // вес 1: переставить две соседние клаузы
            7 => swap_clauses(expr, rng),
            // вес 1: удвоить или снять модификатор
            _ => toggle_modifier(expr, rng),
        };
        if changed {
            return true;
        }
    }
    false
}

/// Собрать пути (DFS) до узлов, подходящих под предикат.
/// Пропускает голову верхнего списка (материю карты).
fn paths_where<F: Fn(&Expr) -> bool>(expr: &Expr, pred: &F) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    fn walk<F: Fn(&Expr) -> bool>(
        e: &Expr,
        path: &mut Vec<usize>,
        pred: &F,
        out: &mut Vec<Vec<usize>>,
        skip_head: bool,
    ) {
        if let Expr::List(items) = e {
            for (i, child) in items.iter().enumerate() {
                if skip_head && path.is_empty() && i == 0 {
                    continue; // материя-голова неприкосновенна
                }
                path.push(i);
                if pred(child) {
                    out.push(path.clone());
                }
                walk(child, path, pred, out, false);
                path.pop();
            }
        }
    }
    walk(expr, &mut Vec::new(), pred, &mut out, true);
    out
}

fn node_mut<'a>(expr: &'a mut Expr, path: &[usize]) -> Option<&'a mut Expr> {
    let mut cur = expr;
    for &i in path {
        match cur {
            Expr::List(items) => cur = items.get_mut(i)?,
            _ => return None,
        }
    }
    Some(cur)
}

/// Соседи руны — открытые руны того же семейства (кроме законных).
fn kin(r: Rune) -> Vec<Rune> {
    use Rune::*;
    let pool: &[Rune] = match r.family() {
        Family::Matter => &[Water, Wood, Stone, Meadow, Voidness, Hearth, Ruin],
        Family::Verb => &[Flow, Branch, Creep, Gnaw],
        Family::Aim => &[Up, Down, Left, Right],
        Family::Time => &[Burst, Abundant],
        _ => return Vec::new(),
    };
    pool.iter().copied().filter(|&k| k != r).collect()
}

fn swap_rune(expr: &mut Expr, rng: &mut StdRng) -> bool {
    let candidates = paths_where(expr, &|e| {
        matches!(e, Expr::Rune(r) if !kin(*r).is_empty())
    });
    if candidates.is_empty() {
        return false;
    }
    let path = &candidates[rng.gen_range(0..candidates.len())];
    if let Some(Expr::Rune(r)) = node_mut(expr, path) {
        let pool = kin(*r);
        *r = pool[rng.gen_range(0..pool.len())];
        return true;
    }
    false
}

fn nudge_num(expr: &mut Expr, rng: &mut StdRng) -> bool {
    let candidates = paths_where(expr, &|e| matches!(e, Expr::Num(_)));
    if candidates.is_empty() {
        return false;
    }
    let path = &candidates[rng.gen_range(0..candidates.len())];
    if let Some(Expr::Num(n)) = node_mut(expr, path) {
        let delta = ((*n as f32) * rng.gen_range(0.10..0.25)).max(1.0) as u32;
        *n = if rng.gen_bool(0.5) {
            n.saturating_add(delta)
        } else {
            n.saturating_sub(delta).max(1)
        };
        return true;
    }
    false
}

fn swap_clauses(expr: &mut Expr, rng: &mut StdRng) -> bool {
    if let Expr::List(items) = expr {
        let clause_idx: Vec<usize> = items
            .iter()
            .enumerate()
            .skip(1)
            .filter(|(_, e)| matches!(e, Expr::List(_)))
            .map(|(i, _)| i)
            .collect();
        if clause_idx.len() >= 2 {
            let k = rng.gen_range(0..clause_idx.len() - 1);
            items.swap(clause_idx[k], clause_idx[k + 1]);
            return true;
        }
    }
    false
}

fn toggle_modifier(expr: &mut Expr, rng: &mut StdRng) -> bool {
    // найти клаузу и добавить/убрать в ней + или !
    let clauses = paths_where(expr, &|e| matches!(e, Expr::List(_)));
    if clauses.is_empty() {
        return false;
    }
    let path = &clauses[rng.gen_range(0..clauses.len())];
    let add = rng.gen_bool(0.5);
    let rune = if rng.gen_bool(0.5) {
        Rune::Abundant
    } else {
        Rune::Burst
    };
    if let Some(Expr::List(items)) = node_mut(expr, path) {
        if add {
            items.push(Expr::Rune(rune));
            return true;
        }
        if let Some(pos) = items
            .iter()
            .position(|e| matches!(e, Expr::Rune(r) if *r == rune))
        {
            items.remove(pos);
            return true;
        }
    }
    false
}
