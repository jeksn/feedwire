// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;
mod opml;

use db::{Database, commands::DbState};
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{Listener, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Resolve the database path:
            //   - Debug builds: project-local `feedwire.db` (convenient for development)
            //   - Release builds: `~/Library/Application Support/<identifier>/feedwire.db`
            //     This is stable across app updates and won't change between builds.
            let db_path = if cfg!(debug_assertions) {
                std::path::PathBuf::from("feedwire.db")
            } else {
                let data_dir = app.path().app_data_dir()
                    .expect("Could not resolve app data directory");
                std::fs::create_dir_all(&data_dir)
                    .expect("Could not create app data directory");
                data_dir.join("feedwire.db")
            };

            println!("Database path: {}", db_path.display());

            // Initialize database
            let app_handle = app.handle().clone();
            tauri::async_runtime::block_on(async move {
                match Database::open(&db_path).await {
                    Ok(database) => {
                        app_handle.manage(Arc::new(Mutex::new(database)) as DbState);
                        Ok(())
                    }
                    Err(e) => {
                        eprintln!("Failed to initialize database: {:?}", e);
                        Err(Box::new(e) as Box<dyn std::error::Error>)
                    }
                }
            })?;

            // Show the window once the page has loaded. We listen for the
            // frontend-emitted "app-ready" event, with a fallback timer so
            // the window always appears even if the event is never fired.
            if let Some(window) = app.get_webview_window("main") {
                let window_for_event = window.clone();
                let window_for_timer = window.clone();

                // Fallback: show after 2s regardless
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                    let _ = window_for_timer.show();
                    let _ = window_for_timer.set_focus();
                });

                // Primary: show as soon as frontend signals it's ready
                window.listen("app-ready", move |_| {
                    let _ = window_for_event.show();
                    let _ = window_for_event.set_focus();
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            db::commands::add_feed,
            db::commands::get_feeds,
            db::commands::delete_feed,
            db::commands::delete_all_feeds,
            db::commands::get_articles,
            db::commands::get_article,
            db::commands::mark_article_read,
            db::commands::toggle_bookmark,
            db::commands::get_bookmarked_articles,
            db::commands::get_unread_articles,
            db::commands::mark_all_read,
            db::commands::refresh_feed,
            db::commands::refresh_all_feeds,
            db::commands::get_unread_count,
            db::commands::export_opml,
            db::commands::import_opml,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
