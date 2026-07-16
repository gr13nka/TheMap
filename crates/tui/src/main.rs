//! Palimpsest TUI — живой тикающий мир. Пробел тянет карту (ритуал),
//! `p` — пауза, `1`/`4` — скорость, `q` — сохранить и выйти. Симуляция
//! идёт сама: посевы растут между тягами. После каждой тяги архивариус
//! дописывает History.md, а MAP.md перерисовывается для Obsidian.

mod app;
mod palette;
mod view;

use std::io::{self, Write};
use std::path::Path;
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use palimpsest_core::card::{self, Card};
use palimpsest_core::event::Event as WorldEvent;
use palimpsest_core::legacy::{Legacy, Observation};
use palimpsest_core::rune::Expr;
use palimpsest_core::tablet::{self, TabletSlot, Tablets};
use palimpsest_core::world::{DrawChoice, Gesture, MetaOp};
use palimpsest_core::{archivist, save, World};

use app::{App, Craft, CraftTarget, Mode, Speed, ANIM_TICK_MS, SIM_TICK_MS};
use palette::Palette;

const PLANE_W: i32 = 48;
const PLANE_H: i32 = 20;
const SEED: u64 = 1;

fn main() -> io::Result<()> {
    let root = std::env::current_dir()?;
    let deck_dir = root.join("Deck");
    let save_path = root.join("save.ron");

    if !deck_dir.is_dir() {
        eprintln!(
            "Не нашёл колоду: {}\nОжидаю папку Deck/ с картами (напр. forest.md).",
            deck_dir.display()
        );
        std::process::exit(1);
    }

    // Загрузить сейв, иначе — свежий мир.
    let world = if save_path.exists() {
        save::load(&save_path, &deck_dir).unwrap_or_else(|e| {
            eprintln!("Сейв не прочитался ({e}), начинаю заново.");
            World::new(deck_dir.clone(), PLANE_W, PLANE_H, SEED).expect("колода не собралась")
        })
    } else {
        World::new(deck_dir.clone(), PLANE_W, PLANE_H, SEED)?
    };

    let pal = Palette::load(&root.join("palette.ron"));
    let legacy_path = root.join("legacy.ron");
    let legacy = Legacy::load(&legacy_path);
    let mut app = App::new(
        world,
        pal,
        legacy,
        root.join("MAP.md"),
        root.join("History.md"),
        save_path,
        legacy_path,
        root.join("Chronicle"),
    );

    write_map(&app.map_path, &app.world)?;

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app);
    ratatui::restore();

    if let Err(e) = &result {
        eprintln!("Ошибка цикла: {e}");
    }

    // сохранить мир на выходе
    write_map(&app.map_path, &app.world)?;
    if let Err(e) = save::save(&app.world, &app.save_path) {
        eprintln!("Не удалось сохранить {}: {e}", app.save_path.display());
    } else {
        println!("Мир сохранён в {}", app.save_path.display());
    }
    println!("Карта: {}", app.map_path.display());
    println!("Хроника: {}", app.history_path.display());

    result
}

fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> io::Result<()> {
    let mut last_sim = Instant::now();
    let mut last_anim = Instant::now();

    loop {
        terminal.draw(|frame| view::draw(frame, app))?;

        // ждём ввод, но недолго: таймеры симуляции и анимации важнее
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    handle_key(app, key.code, key.modifiers)?;
                }
            }
        }
        if app.should_quit {
            return Ok(());
        }

        // тик симуляции: ядро о скоростях не знает — шагаем нужное число раз
        // (время идёт только в наблюдении и жестах)
        if matches!(app.mode, Mode::Observe | Mode::Intervene)
            && last_sim.elapsed() >= Duration::from_millis(SIM_TICK_MS)
        {
            last_sim = Instant::now();
            for _ in 0..app.speed.steps() {
                let events = app.world.step();
                digest_events(app, &events)?;
                if !matches!(app.mode, Mode::Observe | Mode::Intervene) {
                    break;
                }
            }
        }

        // тик анимации — независим от симуляции (мир дышит и на паузе)
        if last_anim.elapsed() >= Duration::from_millis(ANIM_TICK_MS) {
            last_anim = Instant::now();
            app.anim_phase = app.anim_phase.wrapping_add(1);
        }
    }
}

fn handle_key(app: &mut App, code: KeyCode, mods: KeyModifiers) -> io::Result<()> {
    if code == KeyCode::Char('c') && mods.contains(KeyModifiers::CONTROL) {
        app.should_quit = true;
        return Ok(());
    }
    match app.mode {
        Mode::DeathRitual => match code {
            KeyCode::Char('n') | KeyCode::Char(' ') | KeyCode::Enter => rebirth(app)?,
            KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
            _ => {}
        },
        Mode::DeckBrowse => handle_browse_key(app, code),
        Mode::Craft => handle_craft_key(app, code)?,
        Mode::Core => handle_core_key(app, code),
        Mode::MetaChoice => handle_meta_key(app, code)?,
        Mode::DirectionChoice => handle_direction_key(app, code)?,
        Mode::SiteChoice => handle_site_key(app, code)?,
        Mode::Intervene => handle_intervene_key(app, code)?,
        Mode::Atlas => match code {
            KeyCode::Char('j') | KeyCode::Down => app.atlas_scroll += 1,
            KeyCode::Char('k') | KeyCode::Up => {
                app.atlas_scroll = app.atlas_scroll.saturating_sub(1)
            }
            KeyCode::Tab => {
                app.atlas_tab = 1 - app.atlas_tab;
                app.atlas_scroll = 0;
            }
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('a') => app.mode = Mode::Observe,
            _ => {}
        },
        Mode::Observe => match code {
            // тяга — ритуал; карта с choice: сперва спрашивает Правителя
            KeyCode::Char(' ') | KeyCode::Char('d') => {
                match app.world.peek_top().and_then(|c| c.choice) {
                    Some(ref c) if c == "direction" => app.mode = Mode::DirectionChoice,
                    Some(ref c) if c == "site" => {
                        app.hand = (app.world.plane.w / 2, app.world.plane.h / 2);
                        app.mode = Mode::SiteChoice;
                    }
                    _ => perform_draw(app, None)?,
                }
            }
            KeyCode::Char('c') => {
                app.browse_selected = 0;
                app.mode = Mode::DeckBrowse;
            }
            KeyCode::Char('t') => {
                app.core_selected = 0;
                app.mode = Mode::Core;
            }
            KeyCode::Char('i') => {
                app.hand = (app.world.plane.w / 2, app.world.plane.h / 2);
                app.mode = Mode::Intervene;
            }
            KeyCode::Char('a') => {
                app.atlas_scroll = 0;
                app.mode = Mode::Atlas;
            }
            KeyCode::Char('p') => app.toggle_pause(),
            KeyCode::Char('1') => app.speed = Speed::X1,
            KeyCode::Char('4') => app.speed = Speed::X4,
            KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
            _ => {}
        },
    }
    Ok(())
}

/// Совершить тягу и переварить её исход: хроника, атлас, мета-карты, мутация.
fn perform_draw(app: &mut App, choice: Option<DrawChoice>) -> io::Result<()> {
    let outcome = app.world.draw(choice)?;
    app.ritual = Some((outcome.card_name.clone(), Instant::now()));
    // послетяжье: время жестов и крафта, панель колоды подскажет
    app.aftermath_until = app.world.tick + 40;

    // мета-карта: ритуал колоды
    if let Some(op) = outcome.meta {
        let line = match op {
            MetaOp::Shuffle => format!(
                "Тяга {}. «{}» — колода перемешалась; никто не знает, что теперь сверху.",
                app.world.draw_count, outcome.card_name
            ),
            MetaOp::Duplicate | MetaOp::Destroy => {
                app.pending_meta = Some(op);
                app.meta_selected = 0;
                app.mode = Mode::MetaChoice;
                format!(
                    "Тяга {}. «{}» — колода замерла и ждёт руки Правителя.",
                    app.world.draw_count, outcome.card_name
                )
            }
        };
        append_history(&app.history_path, &line)?;
        app.push_line(line);
        return Ok(());
    }

    let line = archivist::chronicle_line(app.world.draw_count, &outcome);
    append_history(&app.history_path, &line)?;
    write_map(&app.map_path, &app.world)?;
    app.push_line(line.clone());

    // колода живёт: узор дрейфнул сам
    if outcome.mutated {
        let m = format!(
            "Карта «{}» легла иначе, чем помнил Правитель, — глифы сдвинулись сами.",
            outcome.card_name
        );
        append_history(&app.history_path, &m)?;
        app.push_line(m);
    }

    // атлас: первая встреча комбинации — страница знания
    if let (Some(combo), Some(matter)) = (outcome.combo, outcome.matter) {
        if !app.legacy.atlas.iter().any(|o| o.combo == combo) {
            app.legacy.atlas.push(Observation {
                matter,
                epoch: app.world.cycle.epoch,
                quote: line,
                combo,
            });
            app.legacy.save(&app.legacy_path)?;
        }
    }
    Ok(())
}

/// Модал мета-карты: выбрать цель и исполнить.
fn handle_meta_key(app: &mut App, code: KeyCode) -> io::Result<()> {
    let count = app.world.deck.order().len();
    match code {
        KeyCode::Char('j') | KeyCode::Down => {
            if count > 0 {
                app.meta_selected = (app.meta_selected + 1) % count;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if count > 0 {
                app.meta_selected = (app.meta_selected + count - 1) % count;
            }
        }
        KeyCode::Enter => {
            if let Some(op) = app.pending_meta.take() {
                let name = app.world.apply_meta(op, app.meta_selected)?;
                let line = match op {
                    MetaOp::Duplicate => format!(
                        "Карта «{name}» раздвоилась; двойник лёг под верх колоды."
                    ),
                    MetaOp::Destroy => format!(
                        "Карта «{name}» ушла в могильник. Колода стала легче — и беднее."
                    ),
                    MetaOp::Shuffle => String::new(),
                };
                if !line.is_empty() {
                    append_history(&app.history_path, &line)?;
                    app.push_line(line);
                }
            }
            app.mode = Mode::Observe;
        }
        KeyCode::Char('q') | KeyCode::Esc => {
            app.pending_meta = None;
            app.mode = Mode::Observe;
            let line = "Правитель отвёл руку — колода осталась как есть.".to_string();
            append_history(&app.history_path, &line)?;
            app.push_line(line);
        }
        _ => {}
    }
    Ok(())
}

/// Модал направления: стрелка — сразу тяга с выбором.
fn handle_direction_key(app: &mut App, code: KeyCode) -> io::Result<()> {
    let dir = match code {
        KeyCode::Up | KeyCode::Char('k') => Some((0i8, -1i8)),
        KeyCode::Down | KeyCode::Char('j') => Some((0, 1)),
        KeyCode::Left | KeyCode::Char('h') => Some((-1, 0)),
        KeyCode::Right | KeyCode::Char('l') => Some((1, 0)),
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = Mode::Observe;
            return perform_draw(app, None); // пусть решит случай
        }
        _ => None,
    };
    if let Some(dir) = dir {
        app.mode = Mode::Observe;
        perform_draw(app, Some(DrawChoice::Direction(dir)))?;
    }
    Ok(())
}

/// Выбор точки посева рукой Правителя.
fn handle_site_key(app: &mut App, code: KeyCode) -> io::Result<()> {
    match code {
        KeyCode::Left | KeyCode::Char('h') => app.hand.0 = (app.hand.0 - 1).max(0),
        KeyCode::Right | KeyCode::Char('l') => {
            app.hand.0 = (app.hand.0 + 1).min(app.world.plane.w - 1)
        }
        KeyCode::Up | KeyCode::Char('k') => app.hand.1 = (app.hand.1 - 1).max(0),
        KeyCode::Down | KeyCode::Char('j') => {
            app.hand.1 = (app.hand.1 + 1).min(app.world.plane.h - 1)
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            app.mode = Mode::Observe;
            perform_draw(app, Some(DrawChoice::Site(app.hand)))?;
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            app.mode = Mode::Observe;
            perform_draw(app, None)?; // пусть решит случай
        }
        _ => {}
    }
    Ok(())
}

/// Божественные жесты: рука ходит по листу, бюджет — от тяги.
fn handle_intervene_key(app: &mut App, code: KeyCode) -> io::Result<()> {
    let matters = app.unlocked_matters();
    match code {
        KeyCode::Left | KeyCode::Char('h') => app.hand.0 = (app.hand.0 - 1).max(0),
        KeyCode::Right | KeyCode::Char('l') => {
            app.hand.0 = (app.hand.0 + 1).min(app.world.plane.w - 1)
        }
        KeyCode::Up | KeyCode::Char('k') => app.hand.1 = (app.hand.1 - 1).max(0),
        KeyCode::Down | KeyCode::Char('j') => {
            app.hand.1 = (app.hand.1 + 1).min(app.world.plane.h - 1)
        }
        KeyCode::Tab => {
            if !matters.is_empty() {
                app.hand_matter = (app.hand_matter + 1) % matters.len();
            }
        }
        KeyCode::Char(' ') => {
            if let Some(&m) = matters.get(app.hand_matter % matters.len().max(1)) {
                if app.world.gesture(Gesture::Paint(m), app.hand) {
                    let line = format!(
                        "Рука Правителя легла на ({}, {}) — там теперь {}.",
                        app.hand.0,
                        app.hand.1,
                        m.ru()
                    );
                    append_history(&app.history_path, &line)?;
                    app.push_line(line);
                }
            }
        }
        KeyCode::Char('x') => {
            if app.world.gesture(Gesture::Erase, app.hand) {
                let line = format!(
                    "Правитель стёр ({}, {}) до чистой бумаги.",
                    app.hand.0, app.hand.1
                );
                append_history(&app.history_path, &line)?;
                app.push_line(line);
            }
        }
        KeyCode::Char('s') => {
            if app.world.gesture(Gesture::Mend, app.hand) {
                let line = format!(
                    "Правитель отвёл пустоту от ({}, {}).",
                    app.hand.0, app.hand.1
                );
                append_history(&app.history_path, &line)?;
                app.push_line(line);
            }
        }
        KeyCode::Char('q') | KeyCode::Esc => app.mode = Mode::Observe,
        _ => {}
    }
    Ok(())
}

/// Браузер колоды: выбрать карту и раскрыть её на столе.
fn handle_browse_key(app: &mut App, code: KeyCode) {
    let count = app.world.deck.order().len();
    match code {
        KeyCode::Char('j') | KeyCode::Down => {
            if count > 0 {
                app.browse_selected = (app.browse_selected + 1) % count;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if count > 0 {
                app.browse_selected = (app.browse_selected + count - 1) % count;
            }
        }
        KeyCode::Enter | KeyCode::Char('e') => {
            let names = app.world.deck.order();
            if let Some(name) = names.get(app.browse_selected) {
                let path = app.world.deck_dir().join(name);
                if let Ok(card) = Card::parse_file(&path) {
                    let expr = card.expr().unwrap_or_else(Expr::empty);
                    app.craft = Some(Craft::open(
                        CraftTarget::Card(path),
                        format!("карта «{}»", card.name),
                        expr,
                        &app.legacy.unlocked,
                    ));
                    app.mode = Mode::Craft;
                }
            }
        }
        // чистый лист: создать карту с нуля — если смерть подарила бумагу
        KeyCode::Char('n') => {
            if app.legacy.blank_cards == 0 {
                app.push_line(
                    "Чистых листов нет. Бумагу для новых слов дарит только смерть мира."
                        .to_string(),
                );
            } else if let Ok(path) = birth_blank_card(app) {
                app.legacy.blank_cards -= 1;
                let _ = app.legacy.save(&app.legacy_path);
                app.world.deck.insert_under_top(path.clone());
                let line = "Правитель разворачивает чистый лист — новое слово ждёт рун."
                    .to_string();
                let _ = append_history(&app.history_path, &line);
                app.push_line(line);
                app.craft = Some(Craft::open(
                    CraftTarget::Card(path),
                    "новая карта".to_string(),
                    Expr::empty(),
                    &app.legacy.unlocked,
                ));
                app.mode = Mode::Craft;
            }
        }
        KeyCode::Char('q') | KeyCode::Esc => app.mode = Mode::Observe,
        _ => {}
    }
}

/// Родить файл новой карты со свободным именем.
fn birth_blank_card(app: &App) -> io::Result<std::path::PathBuf> {
    let deck_dir = app.world.deck_dir();
    for n in 1..100 {
        let path = deck_dir.join(format!("word-{n}.md"));
        if !path.exists() {
            std::fs::write(
                &path,
                format!(
                    "---\nname: Слово {n}\nkind: rune\n---\n\n# Слово {n}\n\nКарта, рождённая с нуля — оплаченная целым прожитым миром.\n\n```rune\n()\n```\n"
                ),
            )?;
            return Ok(path);
        }
    }
    Err(io::Error::new(io::ErrorKind::Other, "нет свободного имени"))
}

/// Ядро мира: выбрать скрижаль и раскрыть её на столе крафта.
fn handle_core_key(app: &mut App, code: KeyCode) {
    let count = TabletSlot::ALL.len();
    match code {
        KeyCode::Char('j') | KeyCode::Down => {
            app.core_selected = (app.core_selected + 1) % count;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.core_selected = (app.core_selected + count - 1) % count;
        }
        KeyCode::Enter | KeyCode::Char('e') => {
            let slot = TabletSlot::ALL[app.core_selected];
            let expr = app.world.tablets.expr(slot).clone();
            app.craft = Some(Craft::open(
                CraftTarget::Tablet(slot),
                slot.title().to_string(),
                expr,
                &app.legacy.unlocked,
            ));
            app.mode = Mode::Craft;
        }
        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('t') => app.mode = Mode::Observe,
        _ => {}
    }
}

/// Стол крафта: ходить по дереву, перекладывать руны, вписать в карту/закон.
fn handle_craft_key(app: &mut App, code: KeyCode) -> io::Result<()> {
    let Some(craft) = app.craft.as_mut() else {
        app.mode = Mode::Observe;
        return Ok(());
    };
    match code {
        KeyCode::Left | KeyCode::Char('h') => craft.sibling(-1),
        KeyCode::Right | KeyCode::Char('l') => craft.sibling(1),
        KeyCode::Down | KeyCode::Char('j') => craft.descend(),
        KeyCode::Up | KeyCode::Char('k') => craft.ascend(),
        KeyCode::Tab | KeyCode::Char(']') => {
            if !craft.palette.is_empty() {
                craft.selected = (craft.selected + 1) % craft.palette.len();
            }
        }
        KeyCode::BackTab | KeyCode::Char('[') => {
            if !craft.palette.is_empty() {
                craft.selected =
                    (craft.selected + craft.palette.len() - 1) % craft.palette.len();
            }
        }
        KeyCode::Char(' ') | KeyCode::Enter => craft.place(),
        KeyCode::Char('a') => craft.append_sibling(),
        KeyCode::Char('(') => craft.wrap(),
        KeyCode::Char('x') | KeyCode::Delete => craft.delete(),
        KeyCode::Char(c) if c.is_ascii_digit() => {
            craft.digit(c.to_digit(10).unwrap_or(0));
        }
        KeyCode::Char('w') => {
            let line = match &craft.target {
                CraftTarget::Card(path) => {
                    card::write_rune(path, &craft.expr)?;
                    format!(
                        "Правитель переложил руны — {}. Что это изменит, покажет тяга.",
                        craft.title
                    )
                }
                CraftTarget::Tablet(slot) => {
                    let dir = Tablets::dir_for(app.world.deck_dir());
                    tablet::write(&dir, *slot, &craft.expr)?;
                    app.world.reload_tablets();
                    format!(
                        "Правитель переписал закон — {}. Мир подчинился немедленно.",
                        craft.title
                    )
                }
            };
            craft.dirty = false;
            append_history(&app.history_path, &line)?;
            app.push_line(line);
        }
        KeyCode::Char('q') | KeyCode::Esc => {
            let back = match &craft.target {
                CraftTarget::Card(_) => Mode::DeckBrowse,
                CraftTarget::Tablet(_) => Mode::Core,
            };
            app.craft = None;
            app.mode = back;
        }
        _ => {}
    }
    Ok(())
}

/// Переварить события тика: хроника, открытия глифов, смерть мира.
fn digest_events(app: &mut App, events: &[WorldEvent]) -> io::Result<()> {
    let lines = app.world.narrate(events);
    for line in lines {
        append_history(&app.history_path, &line)?;
        app.push_line(line);
    }

    // наблюдение окупается: рука Правителя прозревает новые знаки
    let newly = app.legacy.witness(events);
    if !newly.is_empty() {
        app.legacy.save(&app.legacy_path)?;
        let line = if newly.len() == 1 {
            "В палитре Правителя проступил новый знак.".to_string()
        } else {
            format!("В палитре Правителя проступило {} новых знаков.", newly.len())
        };
        append_history(&app.history_path, &line)?;
        app.push_line(line);
    }

    // смерть листа — ритуал, наследие, страница Атласа
    if let Some(WorldEvent::WorldDead { summary }) = events
        .iter()
        .find(|e| matches!(e, WorldEvent::WorldDead { .. }))
    {
        // вехи считаются до того, как итог ляжет в стопку
        let milestones = archivist::milestones(summary, &app.legacy.summaries);
        let death_glyphs = app.legacy.absorb_death(&app.world, summary.clone());
        let epilogue = archivist::epilogue(
            summary,
            app.legacy.ruins.len(),
            death_glyphs.len(),
            &milestones,
        );
        for line in &epilogue {
            append_history(&app.history_path, line)?;
        }
        app.legacy.save(&app.legacy_path)?;
        write_atlas_page(app, summary, &epilogue, &milestones)?;
        archive_history(app)?;
        write_map(&app.map_path, &app.world)?;
        app.epilogue = epilogue;
        app.mode = Mode::DeathRitual;
    }
    Ok(())
}

/// Лист в стопку: страница мира в Атласе + строка в оглавлении.
/// Атлас — одна растущая Вещь: миры умирают, стопка растёт.
fn write_atlas_page(
    app: &App,
    summary: &palimpsest_core::cycle::CycleSummary,
    epilogue: &[String],
    milestones: &[String],
) -> io::Result<()> {
    let atlas_dir = app
        .save_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("Atlas");
    std::fs::create_dir_all(&atlas_dir)?;
    let name = archivist::roman(summary.epoch);

    // страница мира: эпилог, финальный снимок, законы эпохи, колода
    let mut page = format!("# Мир {name}\n\n");
    for line in epilogue {
        if !line.is_empty() {
            page.push_str(&format!("{line}\n"));
        } else {
            page.push('\n');
        }
    }
    page.push_str("\n## Последний снимок листа\n\n```\n");
    page.push_str(&app.world.plane.render_glyphs());
    page.push_str("```\n\n## Законы, которыми он жил\n\n");
    for slot in palimpsest_core::tablet::TabletSlot::ALL {
        page.push_str(&format!(
            "- {}: `{}`\n",
            slot.title(),
            palimpsest_core::rune::pretty(app.world.tablets.expr(slot))
        ));
    }
    page.push_str("\n## Колода эпохи\n\n");
    for card in app.world.deck.order() {
        page.push_str(&format!("- [[{}]]\n", card.trim_end_matches(".md")));
    }
    std::fs::write(atlas_dir.join(format!("world-{:03}.md", summary.epoch)), page)?;

    // оглавление-стопка
    let atlas_index = atlas_dir.join("ATLAS.md");
    if !atlas_index.exists() {
        std::fs::write(
            &atlas_index,
            "# Атлас миров\n\nСтопка листов. Миры уходят — Вещь растёт.\n\n",
        )?;
    }
    let highlight = milestones.first().cloned().unwrap_or_default();
    let mut f = std::fs::OpenOptions::new().append(true).open(&atlas_index)?;
    writeln!(
        f,
        "- [[world-{:03}|Мир {}]] — {} тиков, {} тяг, очагов {}.{}",
        summary.epoch,
        name,
        summary.ticks_lived,
        summary.draws,
        summary.hearths_founded,
        if highlight.is_empty() {
            String::new()
        } else {
            format!(" {highlight}")
        }
    )?;
    Ok(())
}

/// Развернуть новый лист: руины прошлого уже на бумаге, колода та же.
fn rebirth(app: &mut App) -> io::Result<()> {
    let deck_dir = app.world.deck_dir().to_path_buf();
    let (w, h) = (app.world.plane.w, app.world.plane.h);
    app.world = World::new_epoch(deck_dir, w, h, SEED, &app.legacy)?;
    app.mode = Mode::Observe;
    app.speed = Speed::X1;
    app.epilogue.clear();
    save::save(&app.world, &app.save_path)?;
    write_map(&app.map_path, &app.world)?;
    let line = format!(
        "Лист {}. Правитель разворачивает свежую бумагу; сквозь неё проступают руины.",
        archivist::roman(app.world.cycle.epoch)
    );
    append_history(&app.history_path, &line)?;
    app.push_line(line);
    Ok(())
}

/// Убрать хронику умершего листа в архив эпох и начать новую.
fn archive_history(app: &App) -> io::Result<()> {
    std::fs::create_dir_all(&app.chronicle_dir)?;
    let dest = app
        .chronicle_dir
        .join(format!("epoch-{:03}.md", app.world.cycle.epoch));
    if app.history_path.exists() {
        std::fs::copy(&app.history_path, &dest)?;
    }
    std::fs::write(
        &app.history_path,
        format!(
            "# Хроника — лист {}\n\n",
            archivist::roman(app.world.cycle.epoch + 1)
        ),
    )
}

/// MAP.md — визуальный снимок плоскости, перезаписывается тягой и выходом.
fn write_map(path: &Path, world: &World) -> io::Result<()> {
    let glyphs = world.plane.render_glyphs();
    let body = format!(
        "# Карта\n\n_тяга {} · тик {} · занято {} клеток_\n\n```\n{}```\n",
        world.draw_count,
        world.tick,
        world.plane.filled(),
        glyphs
    );
    std::fs::write(path, body)
}

/// History.md — хроника архивариуса, дописывается строкой на тягу.
fn append_history(path: &Path, line: &str) -> io::Result<()> {
    if !path.exists() {
        std::fs::write(path, "# Хроника\n\n")?;
    }
    let mut f = std::fs::OpenOptions::new().append(true).open(path)?;
    writeln!(f, "{line}\n")
}
