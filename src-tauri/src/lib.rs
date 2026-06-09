// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;

use db::{Database, commands::DbState};
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Initialize database synchronously
            let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
            let database = rt.block_on(Database::new()).expect("Failed to initialize database");
            
            app.manage(Arc::new(Mutex::new(database)) as DbState);
            Ok(())
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
            db::commands::refresh_feed,
            db::commands::refresh_all_feeds,
            db::commands::get_unread_count,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
