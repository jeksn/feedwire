//! FeedWire terminal client — a Ratatui frontend over feedwire-core.

mod app;
mod html;
mod ui;

use app::{App, Focus, RefreshState, SidebarEntry, View};
use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use feedwire_core::refresh::SharedDatabase;
use ratatui::DefaultTerminal;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;

/// Events driving the TUI event loop.
enum Event {
    Key(KeyEvent),
    RefreshProgress {
        done: usize,
        total: usize,
        feed_title: String,
    },
    RefreshDone {
        refreshed: usize,
        failed: usize,
    },
    Tick,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async_main())
}

async fn async_main() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = resolve_db_path()?;
    if !db_path.exists() {
        return Err(format!(
            "No database at {}. Launch the FeedWire app first, or pass --db <path>.",
            db_path.display()
        )
        .into());
    }

    let database = feedwire_core::Database::open(&db_path).await?;
    let db: SharedDatabase = std::sync::Arc::new(tokio::sync::Mutex::new(database));

    let (tx, mut rx) = mpsc::channel::<Event>(256);

    // Terminal input: crossterm's read() is blocking, so it lives on its own thread.
    let key_tx = tx.clone();
    std::thread::spawn(move || {
        loop {
            match crossterm::event::read() {
                Ok(CrosstermEvent::Key(key)) => {
                    if key_tx.blocking_send(Event::Key(key)).is_err() {
                        break;
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });

    // Periodic poll: detect writes by the GUI app via PRAGMA data_version.
    let tick_tx = tx.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            if tick_tx.send(Event::Tick).await.is_err() {
                break;
            }
        }
    });

    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal, db, &mut rx, &tx).await;
    ratatui::restore();
    result.map_err(|e| e.into())
}

async fn run_app(
    terminal: &mut DefaultTerminal,
    db: SharedDatabase,
    rx: &mut mpsc::Receiver<Event>,
    tx: &mpsc::Sender<Event>,
) -> Result<(), String> {
    let mut app = App::new(db).await?;

    while !app.quit {
        terminal
            .draw(|frame| ui::draw(frame, &mut app))
            .map_err(|e| e.to_string())?;

        let Some(event) = rx.recv().await else {
            break;
        };
        match event {
            Event::Key(key) => handle_key(&mut app, key, tx).await?,
            Event::RefreshProgress {
                done,
                total,
                feed_title,
            } => {
                app.refresh.done = done;
                app.refresh.total = total;
                app.refresh.last_feed = feed_title;
            }
            Event::RefreshDone { refreshed, failed } => {
                app.refresh = RefreshState::default();
                if failed > 0 && refreshed == 0 {
                    app.status = "Refresh failed".to_string();
                } else if failed > 0 {
                    app.status = format!("Refreshed {refreshed} feeds ({failed} failed)");
                } else {
                    app.status = format!("Refreshed {refreshed} feeds");
                }
                app.reload().await?;
            }
            Event::Tick => {
                app.poll_external_changes().await?;
            }
        }
    }

    Ok(())
}

async fn handle_key(app: &mut App, key: KeyEvent, tx: &mpsc::Sender<Event>) -> Result<(), String> {
    if key.kind != KeyEventKind::Press {
        return Ok(());
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.quit = true;
        return Ok(());
    }

    // Any key dismisses the help overlay.
    if app.show_help {
        app.show_help = false;
        return Ok(());
    }

    match key.code {
        KeyCode::Char('?') => app.show_help = true,
        KeyCode::Char('q') => app.quit = true,
        KeyCode::Tab | KeyCode::Char('l') | KeyCode::Right => app.cycle_focus(false),
        KeyCode::BackTab | KeyCode::Char('h') | KeyCode::Left => app.cycle_focus(true),
        KeyCode::Char('1') => select_view(app, View::All).await?,
        KeyCode::Char('2') => select_view(app, View::Unread).await?,
        KeyCode::Char('3') => select_view(app, View::Today).await?,
        KeyCode::Char('4') => select_view(app, View::Starred).await?,
        KeyCode::Char('b') => app.toggle_bookmark().await?,
        KeyCode::Char('m') => app.toggle_read().await?,
        KeyCode::Char('M') => app.mark_all_view_read().await?,
        KeyCode::Char('o') => app.open_in_browser().await?,
        KeyCode::Char('r') => refresh_selected_feed(app, tx).await?,
        KeyCode::Char('R') => refresh_all_feeds(app, tx).await?,
        KeyCode::Enter => match app.focus {
            Focus::Feeds => app.focus = Focus::Articles,
            Focus::Articles => app.open_article().await?,
            Focus::Content => {}
        },
        KeyCode::Esc => match app.focus {
            Focus::Content => app.focus = Focus::Articles,
            Focus::Articles => app.focus = Focus::Feeds,
            Focus::Feeds => app.status.clear(),
        },
        KeyCode::Char('j') | KeyCode::Down => list_move(app, 1).await?,
        KeyCode::Char('k') | KeyCode::Up => list_move(app, -1).await?,
        KeyCode::Char('g') => list_move(app, i64::MIN).await?,
        KeyCode::Char('G') | KeyCode::End => list_move(app, i64::MAX).await?,
        KeyCode::PageDown => list_move(app, 10).await?,
        KeyCode::PageUp => list_move(app, -10).await?,
        _ => {}
    }
    Ok(())
}

/// Jump to a smart view (the four fixed sidebar entries).
async fn select_view(app: &mut App, view: View) -> Result<(), String> {
    if let Some(index) = app
        .entries
        .iter()
        .position(|e| matches!(e, SidebarEntry::View(v) if *v == view))
    {
        app.select_entry(index).await?;
    }
    Ok(())
}

/// Move within the focused pane by `delta`; sentinel min/max jumps to the ends.
async fn list_move(app: &mut App, delta: i64) -> Result<(), String> {
    if delta == i64::MIN {
        return jump(app, 0).await;
    }
    if delta == i64::MAX {
        let len = match app.focus {
            Focus::Feeds => app.entries.len(),
            Focus::Articles => app.articles.len(),
            Focus::Content => 0,
        };
        return jump(app, len.saturating_sub(1)).await;
    }
    if app.focus == Focus::Content {
        app.scroll_content(delta);
        return Ok(());
    }

    app.move_selection(delta);
    match app.focus {
        Focus::Feeds => {
            let index = app.selected_entry;
            app.select_entry(index).await?;
        }
        Focus::Articles => app.preview_selected_content(),
        Focus::Content => {}
    }
    Ok(())
}

async fn jump(app: &mut App, index: usize) -> Result<(), String> {
    if app.focus == Focus::Content {
        app.content_scroll = if index == 0 { 0 } else { u16::MAX };
        return Ok(());
    }
    let len = match app.focus {
        Focus::Feeds => app.entries.len(),
        Focus::Articles => app.articles.len(),
        Focus::Content => return Ok(()),
    };
    let index = index.min(len.saturating_sub(1));

    match app.focus {
        Focus::Feeds => app.select_entry(index).await?,
        Focus::Articles => {
            app.selected_article = index;
            app.content_scroll = 0;
            app.preview_selected_content();
        }
        Focus::Content => {}
    }
    Ok(())
}

/// Refresh the feed selected in the sidebar (no-op on smart views).
async fn refresh_selected_feed(app: &mut App, tx: &mpsc::Sender<Event>) -> Result<(), String> {
    let Some(SidebarEntry::Feed(feed)) = app.current_entry() else {
        app.status = "Select a feed to refresh it".into();
        return Ok(());
    };
    if app.refresh.active {
        app.status = "Refresh already running".into();
        return Ok(());
    }
    let feed_id = feed.id.clone();
    let feed_title = feed.title.clone();
    app.refresh = RefreshState {
        active: true,
        done: 0,
        total: 1,
        last_feed: feed_title.clone(),
    };
    app.status = format!("Refreshing {feed_title}");

    let db = app.db.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        let result = feedwire_core::refresh::refresh_feed(&db, &feed_id).await;
        let (refreshed, failed) = match result {
            Ok(_) => (1, 0),
            Err(e) => {
                eprintln!("Feed refresh failed: {e}");
                (0, 1)
            }
        };
        let _ = tx
            .send(Event::RefreshDone { refreshed, failed })
            .await;
    });
    Ok(())
}

/// Refresh every feed, streaming progress events into the status bar.
async fn refresh_all_feeds(app: &mut App, tx: &mpsc::Sender<Event>) -> Result<(), String> {
    if app.refresh.active {
        app.status = "Refresh already running".into();
        return Ok(());
    }
    app.refresh = RefreshState {
        active: true,
        ..RefreshState::default()
    };
    app.status.clear();

    // The core engine reports progress through a sync callback; relay it
    // through a standard channel + forwarder thread into the async loop.
    let (progress_tx, progress_rx) = std::sync::mpsc::channel::<(usize, usize, String)>();
    let forward_tx = tx.clone();
    std::thread::spawn(move || {
        while let Ok((done, total, feed_title)) = progress_rx.recv() {
            if forward_tx
                .blocking_send(Event::RefreshProgress {
                    done,
                    total,
                    feed_title,
                })
                .is_err()
            {
                break;
            }
        }
    });

    let db = app.db.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        let result = feedwire_core::refresh::refresh_all(db, move |done, total, feed_title| {
            let _ = progress_tx.send((done, total, feed_title.to_string()));
        })
        .await;
        let (refreshed, failed) = match result {
            Ok(ids) => (ids.len(), 0),
            Err(e) => {
                eprintln!("Refresh all failed: {e}");
                (0, 1)
            }
        };
        let _ = tx.send(Event::RefreshDone { refreshed, failed }).await;
    });
    Ok(())
}

/// Resolve the database path: `--db <path>` wins, else the macOS app data dir.
fn resolve_db_path() -> Result<PathBuf, String> {
    let mut args = std::env::args().skip(1);
    let mut explicit: Option<PathBuf> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--db" => {
                explicit = Some(args.next().map(PathBuf::from).ok_or(
                    "--db requires a path. Usage: feedwire-tui [--db <path>]",
                )?);
            }
            other => {
                return Err(format!(
                    "Unknown argument: {other}. Usage: feedwire-tui [--db <path>]"
                ));
            }
        }
    }
    if let Some(path) = explicit {
        return Ok(path);
    }

    let home = std::env::var("HOME").map_err(|_| "HOME is not set".to_string())?;
    let dir = PathBuf::from(home)
        .join("Library/Application Support")
        .join(feedwire_core::APP_IDENTIFIER);
    if dir.exists() {
        return Ok(dir.join(feedwire_core::DB_FILE_NAME));
    }
    Err(format!(
        "No FeedWire data directory at {}. Launch the FeedWire app once, or pass --db <path>.",
        dir.display()
    ))
}
