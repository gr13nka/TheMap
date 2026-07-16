//! Состояние клиента: мир, скорость времени, хроника с метками времени
//! (для проявления строк), фаза анимации и ритуал тяги. Тик симуляции и
//! тик анимации независимы (см. AESTHETICS.md); ядро о скоростях не знает —
//! клиент просто зовёт `world.step()` нужное число раз.

use std::path::PathBuf;
use std::time::Instant;

use palimpsest_core::legacy::Legacy;
use palimpsest_core::plane::Plane;
use palimpsest_core::rune::{self, Expr, Family, Rune};
use palimpsest_core::tablet::TabletSlot;
use palimpsest_core::World;

use crate::palette::Palette;

/// Что раскрыто на столе крафта: карта колоды или скрижаль-закон.
#[derive(Debug, Clone)]
pub enum CraftTarget {
    Card(PathBuf),
    Tablet(TabletSlot),
}

/// Стол крафта — структурный редактор дерева рун. Курсор ходит по узлам
/// (путь индексов), операции не могут сломать скобки: синтаксических
/// ошибок не существует по построению.
pub struct Craft {
    pub target: CraftTarget,
    pub title: String,
    pub expr: Expr,
    /// Путь к выделенному узлу; пустой — выделено всё выражение.
    pub cursor: Vec<usize>,
    pub palette: Vec<Rune>,
    pub selected: usize,
    pub frames: Vec<Plane>,
    pub dirty: bool,
}

impl Craft {
    pub fn open(target: CraftTarget, title: String, expr: Expr, unlocked: &[Rune]) -> Craft {
        // палитра — только открытое, сгруппированное по семействам;
        // законные руны в палитре появляются, лишь когда открыты
        let mut palette: Vec<Rune> = Vec::new();
        for family in [
            Family::Matter,
            Family::Verb,
            Family::Aim,
            Family::Time,
            Family::Predicate,
            Family::Law,
        ] {
            palette.extend(unlocked.iter().copied().filter(|g| g.family() == family));
        }
        let frames = rune::preview(&expr);
        Craft {
            target,
            title,
            expr,
            cursor: Vec::new(),
            palette,
            selected: 0,
            frames,
            dirty: false,
        }
    }

    fn node(&self, path: &[usize]) -> Option<&Expr> {
        let mut cur = &self.expr;
        for &i in path {
            match cur {
                Expr::List(items) => cur = items.get(i)?,
                _ => return None,
            }
        }
        Some(cur)
    }

    fn node_mut(&mut self, path: &[usize]) -> Option<&mut Expr> {
        let mut cur = &mut self.expr;
        for &i in path {
            match cur {
                Expr::List(items) => cur = items.get_mut(i)?,
                _ => return None,
            }
        }
        Some(cur)
    }

    /// Навигация: к соседу (dx = ±1).
    pub fn sibling(&mut self, dx: i32) {
        let Some(&last) = self.cursor.last() else { return };
        let parent_path = &self.cursor[..self.cursor.len() - 1];
        let Some(Expr::List(items)) = self.node(parent_path) else { return };
        let n = items.len() as i32;
        let next = (last as i32 + dx).clamp(0, (n - 1).max(0)) as usize;
        *self.cursor.last_mut().unwrap() = next;
    }

    /// Вглубь: в первый элемент списка.
    pub fn descend(&mut self) {
        if let Some(Expr::List(items)) = self.node(&self.cursor.clone()) {
            if !items.is_empty() {
                self.cursor.push(0);
            }
        }
    }

    /// Наружу: к родителю.
    pub fn ascend(&mut self) {
        self.cursor.pop();
    }

    /// Положить выбранную руну: заменить атом под курсором; на пустом
    /// списке — вложить внутрь; повтор той же руны — стереть.
    pub fn place(&mut self) {
        let Some(r) = self.palette.get(self.selected).copied() else { return };
        let path = self.cursor.clone();
        let Some(node) = self.node_mut(&path) else { return };
        match node {
            Expr::Rune(cur) if *cur == r => {
                self.delete();
                return;
            }
            Expr::Rune(_) | Expr::Num(_) => *node = Expr::Rune(r),
            Expr::List(items) => {
                items.push(Expr::Rune(r));
                let idx = items.len() - 1;
                self.cursor.push(idx);
            }
        }
        self.refresh();
    }

    /// Добавить атом-сиблинга после курсора.
    pub fn append_sibling(&mut self) {
        let Some(r) = self.palette.get(self.selected).copied() else { return };
        if self.cursor.is_empty() {
            // на корне: добавить в конец корневого списка
            if let Expr::List(items) = &mut self.expr {
                items.push(Expr::Rune(r));
                self.cursor = vec![items.len() - 1];
                self.refresh();
            }
            return;
        }
        let last = *self.cursor.last().unwrap();
        let parent_path = self.cursor[..self.cursor.len() - 1].to_vec();
        if let Some(Expr::List(items)) = self.node_mut(&parent_path) {
            items.insert(last + 1, Expr::Rune(r));
            *self.cursor.last_mut().unwrap() = last + 1;
            self.refresh();
        }
    }

    /// Обернуть узел под курсором в новый список.
    pub fn wrap(&mut self) {
        let path = self.cursor.clone();
        if let Some(node) = self.node_mut(&path) {
            let inner = std::mem::replace(node, Expr::List(Vec::new()));
            *node = Expr::List(vec![inner]);
            self.refresh();
        }
    }

    /// Удалить узел под курсором (корень — опустошить).
    pub fn delete(&mut self) {
        if self.cursor.is_empty() {
            self.expr = Expr::empty();
            self.refresh();
            return;
        }
        let last = *self.cursor.last().unwrap();
        let parent_path = self.cursor[..self.cursor.len() - 1].to_vec();
        if let Some(Expr::List(items)) = self.node_mut(&parent_path) {
            if last < items.len() {
                items.remove(last);
            }
            if items.is_empty() {
                self.cursor.pop();
            } else {
                *self.cursor.last_mut().unwrap() = last.min(items.len() - 1);
            }
            self.refresh();
        }
    }

    /// Цифра: копить число под курсором (не-число становится числом).
    pub fn digit(&mut self, d: u32) {
        let path = self.cursor.clone();
        if let Some(node) = self.node_mut(&path) {
            match node {
                Expr::Num(n) => *n = (*n * 10 + d).min(9999),
                _ => *node = Expr::Num(d),
            }
            self.refresh();
        }
    }

    fn refresh(&mut self) {
        self.frames = rune::preview(&self.expr);
        self.dirty = true;
    }
}

/// Интервал тика симуляции при скорости ×1, мс.
pub const SIM_TICK_MS: u64 = 500;
/// Интервал тика анимации (дыхание мира), мс.
pub const ANIM_TICK_MS: u64 = 400;
/// Сколько длится ритуал тяги, мс.
pub const RITUAL_MS: u64 = 900;

/// Режим клиента. Время мира идёт только в Observe и Intervene:
/// ритуалы, крафт и выборы останавливают мир.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Observe,
    DeathRitual,
    DeckBrowse,
    Craft,
    /// Ядро мира: четыре скрижали на одном экране.
    Core,
    /// Мета-карта ждёт выбора цели (дублировать/уничтожить).
    MetaChoice,
    /// Карта спрашивает направление («куда направить реку?»).
    DirectionChoice,
    /// Карта отдаёт точку посева руке Правителя.
    SiteChoice,
    /// Божественные жесты: курсор по карте, бюджет от тяги.
    Intervene,
    /// Атлас наблюдений: что рука уже засвидетельствовала.
    Atlas,
}

/// Сердцебиение: чего мир ждёт от Правителя прямо сейчас.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pulse {
    /// Мир живёт — наблюдай.
    Observing,
    /// Колода зовёт: пора тянуть (Скрижаль Сердца).
    Calling,
    /// Идёт ритуал тяги.
    Drawing,
    /// После тяги: время жестов, крафта, атласа.
    Aftermath,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Speed {
    Paused,
    X1,
    X4,
}

impl Speed {
    pub fn steps(self) -> u32 {
        match self {
            Speed::Paused => 0,
            Speed::X1 => 1,
            Speed::X4 => 4,
        }
    }

    /// Слова, не пиктограммы — как `-- ВСТАВКА --` в vim.
    pub fn label(self) -> &'static str {
        match self {
            Speed::Paused => "[пауза]",
            Speed::X1 => "скорость ×1",
            Speed::X4 => "скорость ×4",
        }
    }
}

pub struct App {
    pub world: World,
    pub pal: Palette,
    pub legacy: Legacy,
    pub mode: Mode,
    /// Эпилог умершего листа — текст ритуала смерти.
    pub epilogue: Vec<String>,
    /// Выделение в браузере колоды.
    pub browse_selected: usize,
    /// Раскрытая на столе карта (режим Craft).
    pub craft: Option<Craft>,
    /// Мета-операция, ждущая выбора цели.
    pub pending_meta: Option<palimpsest_core::world::MetaOp>,
    /// Выделение в списке целей мета-карты.
    pub meta_selected: usize,
    /// Курсор жестов и выбора точки посева.
    pub hand: (i32, i32),
    /// Какая материя под рукой в режиме жестов (индекс в открытых материях).
    pub hand_matter: usize,
    /// Прокрутка атласа.
    pub atlas_scroll: usize,
    /// Вкладка атласа: 0 — стопка миров, 1 — наблюдения.
    pub atlas_tab: usize,
    pub speed: Speed,
    resume: Speed,
    /// Хроника с моментами появления строк — для проявления чернил.
    pub chronicle: Vec<(String, Instant)>,
    /// Счётчик тиков анимации; чётность и остатки задают фазы дыхания.
    pub anim_phase: u32,
    /// Идущий ритуал тяги: имя карты и момент начала.
    pub ritual: Option<(String, Instant)>,
    /// До какого тика мира длится послетяжье (время жестов).
    pub aftermath_until: u64,
    /// Выделение на экране ядра (индекс скрижали).
    pub core_selected: usize,
    pub should_quit: bool,
    pub map_path: PathBuf,
    pub history_path: PathBuf,
    pub save_path: PathBuf,
    pub legacy_path: PathBuf,
    pub chronicle_dir: PathBuf,
}

impl App {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        world: World,
        pal: Palette,
        legacy: Legacy,
        map_path: PathBuf,
        history_path: PathBuf,
        save_path: PathBuf,
        legacy_path: PathBuf,
        chronicle_dir: PathBuf,
    ) -> App {
        let mut app = App {
            world,
            pal,
            legacy,
            mode: Mode::Observe,
            epilogue: Vec::new(),
            browse_selected: 0,
            craft: None,
            pending_meta: None,
            meta_selected: 0,
            hand: (0, 0),
            hand_matter: 0,
            atlas_scroll: 0,
            atlas_tab: 0,
            speed: Speed::X1,
            resume: Speed::X1,
            chronicle: Vec::new(),
            anim_phase: 0,
            ritual: None,
            aftermath_until: 0,
            core_selected: 0,
            should_quit: false,
            map_path,
            history_path,
            save_path,
            legacy_path,
            chronicle_dir,
        };
        app.push_line("Правитель берёт колоду. Мир дышит; тяни карту.".to_string());
        app
    }

    pub fn push_line(&mut self, line: String) {
        self.chronicle.push((line, Instant::now()));
        // держим хвост — хроника целиком живёт в History.md
        if self.chronicle.len() > 200 {
            self.chronicle.remove(0);
        }
    }

    pub fn toggle_pause(&mut self) {
        if self.speed == Speed::Paused {
            self.speed = self.resume;
        } else {
            self.resume = self.speed;
            self.speed = Speed::Paused;
        }
    }

    pub fn ritual_active(&self) -> bool {
        self.ritual
            .as_ref()
            .map(|(_, at)| at.elapsed().as_millis() < RITUAL_MS as u128)
            .unwrap_or(false)
    }

    /// Сердцебиение: чего мир ждёт от Правителя прямо сейчас.
    pub fn pulse(&self) -> Pulse {
        if self.ritual_active() {
            Pulse::Drawing
        } else if self.world.tick < self.aftermath_until && self.world.gestures > 0 {
            Pulse::Aftermath
        } else if self.world.deck_calls() {
            Pulse::Calling
        } else {
            Pulse::Observing
        }
    }

    /// Материи, открытые руке Правителя (для жестов).
    pub fn unlocked_matters(&self) -> Vec<palimpsest_core::rune::Matter> {
        self.legacy
            .unlocked
            .iter()
            .filter_map(|g| g.as_matter())
            .collect()
    }
}
