// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;
mod opml;

use db::{Database, commands::DbState};
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Initialize database - use tokio::main runtime instead of creating a new one
            let app_handle = app.handle().clone();
            tauri::async_runtime::block_on(async move {
                match Database::new().await {
                    Ok(database) => {
                        app_handle.manage(Arc::new(Mutex::new(database)) as DbState);
                        Ok(())
                    }
                    Err(e) => {
                        eprintln!("Failed to initialize database: {:?}", e);
                        Err(Box::new(e) as Box<dyn std::error::Error>)
                    }
                }
            })
        })
        .invoke_handler(tauri::generate_handler![
            db::commands::add_feed,
            db::commands::get_feeds,
            db::commands::delete_feed,
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
