//! Канонический принтер: одна строка, скобки и пробелы. Что печатается —
//! то и парсится обратно в то же дерево (неподвижная точка); эта строка
//! и пишется в ```rune-блок .md-карты.

use super::Expr;

pub fn pretty(expr: &Expr) -> String {
    match expr {
        Expr::Rune(r) => r.ch().to_string(),
        Expr::Num(n) => n.to_string(),
        Expr::List(items) => {
            let inner: Vec<String> = items.iter().map(pretty).collect();
            format!("({})", inner.join(" "))
        }
    }
}
