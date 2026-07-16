//! Карта — org-babel-документ: YAML-frontmatter + проза + исполняемый блок.
//! Рунная карта (`kind: rune`) несёт выражение в блоке ```rune …```;
//! движок читает frontmatter и блок, проза — для человека и наблюдений
//! Правителя (движок её не трогает). Скрижали (`kind: tablet`) — те же
//! карты, только законы.

use std::path::Path;

use crate::rune::{self, Expr};

#[derive(Debug, Clone)]
pub struct Card {
    pub name: String,
    pub kind: String,
    /// Операция мета-карты (`op: duplicate | destroy | shuffle`).
    pub op: Option<String>,
    /// Выбор Правителя при тяге (`choice: direction | site`).
    pub choice: Option<String>,
    /// Тело исполняемого блока (для `kind: sigil` — текстовая сетка узора).
    pub block: String,
    /// Всё тело после frontmatter — сохраняем для показа/будущего.
    pub prose: String,
}

impl Card {
    pub fn parse_file(path: &Path) -> std::io::Result<Card> {
        let text = std::fs::read_to_string(path)?;
        Ok(Card::parse(&text, path))
    }

    pub fn parse(text: &str, path: &Path) -> Card {
        let mut name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("card")
            .to_string();
        let mut kind = String::from("unknown");
        let mut op = None;
        let mut choice = None;

        // --- frontmatter между `---` … `---` ---
        let mut body = text;
        if let Some(rest) = text.strip_prefix("---") {
            if let Some(end) = rest.find("\n---") {
                let fm = &rest[..end];
                body = rest[end + 4..].trim_start_matches('\n');
                for line in fm.lines() {
                    if let Some((k, v)) = line.split_once(':') {
                        match k.trim() {
                            "name" => name = v.trim().to_string(),
                            "kind" => kind = v.trim().to_string(),
                            "op" => op = Some(v.trim().to_string()),
                            "choice" => choice = Some(v.trim().to_string()),
                            _ => {}
                        }
                    }
                }
            }
        }

        // скрижали несут тот же ```rune-блок, что и карты
        let block_lang = if kind == "tablet" { "rune" } else { kind.as_str() };
        let block = extract_block(body, block_lang).unwrap_or_default();

        Card {
            name,
            kind,
            op,
            choice,
            block,
            prose: body.to_string(),
        }
    }

    /// Выражение рунной карты или скрижали; None — блока нет или карта
    /// другого рода.
    pub fn expr(&self) -> Option<Expr> {
        if matches!(self.kind.as_str(), "rune" | "tablet") && !self.block.is_empty() {
            Some(rune::parse(&self.block))
        } else {
            None
        }
    }
}

/// Достаёт содержимое первого fenced-блока ```<lang> … ```.
fn extract_block(text: &str, lang: &str) -> Option<String> {
    let fence = format!("```{lang}");
    let start = text.find(&fence)?;
    let after = &text[start + fence.len()..];
    let content_start = after.find('\n')? + 1;
    let rest = &after[content_start..];
    let end = rest.find("```")?;
    Some(rest[..end].trim_end().to_string())
}

/// Переписать в .md-карте только ```rune-блок: frontmatter и проза —
/// территория человека, движок их не трогает. Нет блока — дописать в конец.
pub fn write_rune(path: &Path, expr: &Expr) -> std::io::Result<()> {
    let text = std::fs::read_to_string(path)?;
    let line = format!("{}\n", rune::pretty(expr));

    let new_text = match locate_block(&text, "rune") {
        Some((start, end)) => format!("{}{}{}", &text[..start], line, &text[end..]),
        None => format!("{}\n```rune\n{}```\n", text.trim_end(), line),
    };
    std::fs::write(path, new_text)
}

/// Границы содержимого первого блока ```<lang> (после заголовка, до ```).
fn locate_block(text: &str, lang: &str) -> Option<(usize, usize)> {
    let fence = format!("```{lang}");
    let start = text.find(&fence)?;
    let after_fence = start + fence.len();
    let content_start = after_fence + text[after_fence..].find('\n')? + 1;
    let content_end = content_start + text[content_start..].find("```")?;
    Some((content_start, content_end))
}
