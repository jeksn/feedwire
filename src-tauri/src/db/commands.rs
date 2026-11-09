use feedwire_core::models::{ArticleUpdate, NewFilterRule, Folder};
use feedwire_core::opml;
use feedwire_core::refresh;
use feedwire_core::{
    Database, FeedParser, SETTING_AUTO_REFRESH_INTERVAL, SETTING_SKIP_YOUTUBE_SHORTS,
};
use tauri::{State, AppHandle, Emitter};
use tauri_plugin_dialog::DialogExt;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, serde::Serialize)]
pub struct RefreshProgress {
    pub done: usize,
    pub total: usize,
    pub feed_title: String,
}

pub type DbState = Arc<Mutex<Database>>;

#[tauri::command]
pub async fn add_feed(url: String, db: State<'_, DbState>) -> Result<feedwire_core::models::Feed, String> {
    println!("Adding feed: {}", url);
    
    let parser = FeedParser::new();
    
    // Validate URL
    FeedParser::validate_feed_url(&url).map_err(|e| {
        println!("URL validation failed: {}", e);
        e.to_string()
    })?;
    
    // Check if it's a YouTube channel and convert to RSS
    let (final_url, yt_title) = if parser.is_youtube_channel(&url).await {
        println!("Detected YouTube channel, converting to RSS");
        parser.convert_youtube_to_rss(&url).await.map_err(|e| {
            println!("YouTube conversion failed: {}", e);
            e.to_string()
        })?
    } else {
        (url.clone(), None)
    };
    
    println!("Final URL to fetch: {}", final_url);
    
    // Check for active duplicates — brief lock, released before any network I/O
    {
        let db_guard = db.lock().await;
        if let Some(existing) = db_guard.get_feed_by_url(&final_url).await.map_err(|e| e.to_string())? {
            if existing.is_active {
                println!("Feed already exists: {}", existing.title);
                return Err("Feed already exists".to_string());
            }
        }
    }

    // Discover feed metadata — no lock held during the network call.
    // YouTube's RSS endpoint intermittently 404s for valid channels; in that
    // case save the subscription with best-effort metadata (title + avatar
    // scraped from the channel page) and let auto-refresh backfill articles
    // once the endpoint is reachable again.
    let new_feed = match parser.discover_feed(&final_url).await {
        Ok(f) => f,
        Err(e) if final_url.contains("youtube.com/feeds") => {
            println!("Feed discovery failed for YouTube feed ({}), saving with fallback metadata", e);
            parser.fallback_youtube_feed(&final_url, yt_title).await
        }
        Err(e) => {
            println!("Feed discovery failed: {}", e);
            return Err(e.to_string());
        }
    };
    println!("Feed discovered: {}", new_feed.title);

    // Persist — reactivates soft-deleted rows, inserts new ones
    let feed = {
        let db_guard = db.lock().await;
        let feed = db_guard.create_or_reactivate_feed(new_feed).await.map_err(|e| {
            println!("Database save failed: {}", e);
            e.to_string()
        })?;
        println!("Feed saved to database with ID: {}", feed.id);
        feed
    };

    // Fetch initial articles after the lock is released to avoid deadlock
    if let Err(e) = refresh::fetch_feed_articles(feed.id.clone(), db.inner().clone()).await {
        println!("Failed to fetch initial articles: {}", e);
    }

    Ok(feed)
}

#[tauri::command]
pub async fn get_feeds(db: State<'_, DbState>) -> Result<Vec<feedwire_core::models::Feed>, String> {
    let db = db.lock().await;
    let feeds = db.get_feeds().await.map_err(|e| e.to_string())?;
    Ok(feeds)
}

#[tauri::command]
pub async fn delete_feed(feed_id: String, db: State<'_, DbState>) -> Result<(), String> {
    let db = db.lock().await;
    db.delete_feed(&feed_id).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn delete_all_feeds(db: State<'_, DbState>) -> Result<usize, String> {
    let db = db.lock().await;
    db.delete_all_feeds().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn archive_feed(feed_id: String, db: State<'_, DbState>) -> Result<(), String> {
    let db = db.lock().await;
    db.archive_feed(&feed_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn unarchive_feed(feed_id: String, db: State<'_, DbState>) -> Result<(), String> {
    let db = db.lock().await;
    db.unarchive_feed(&feed_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_archived_feeds(db: State<'_, DbState>) -> Result<Vec<feedwire_core::models::Feed>, String> {
    let db = db.lock().await;
    let feeds = db.get_archived_feeds().await.map_err(|e| e.to_string())?;
    Ok(feeds)
}

#[tauri::command]
pub async fn get_articles(
    feed_id: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    db: State<'_, DbState>
) -> Result<Vec<feedwire_core::models::Article>, String> {
    let db = db.lock().await;
    let articles = db.get_articles(feed_id, limit, offset).await.map_err(|e| e.to_string())?;
    Ok(articles)
}

#[tauri::command]
pub async fn get_article(article_id: String, db: State<'_, DbState>) -> Result<feedwire_core::models::Article, String> {
    let db = db.lock().await;
    let article = db.get_article_by_id(&article_id).await.map_err(|e| e.to_string())?;
    Ok(article)
}

#[tauri::command]
pub async fn mark_article_read(article_id: String, is_read: bool, db: State<'_, DbState>) -> Result<feedwire_core::models::Article, String> {
    let db = db.lock().await;
    let update = ArticleUpdate {
        is_read: Some(is_read),
        ..Default::default()
    };
    let article = db.update_article(&article_id, update).await.map_err(|e| e.to_string())?;
    Ok(article)
}

#[tauri::command]
pub async fn toggle_bookmark(article_id: String, db: State<'_, DbState>) -> Result<feedwire_core::models::Article, String> {
    let db = db.lock().await;
    
    // Get current article to check bookmark status
    let current_article = db.get_article_by_id(&article_id).await.map_err(|e| e.to_string())?;
    let new_bookmark_status = !current_article.is_bookmarked;
    
    let update = ArticleUpdate {
        is_bookmarked: Some(new_bookmark_status),
        ..Default::default()
    };
    let article = db.update_article(&article_id, update).await.map_err(|e| e.to_string())?;
    Ok(article)
}

#[tauri::command]
pub async fn get_bookmarked_articles(feed_id: Option<String>, db: State<'_, DbState>) -> Result<Vec<feedwire_core::models::Article>, String> {
    let db = db.lock().await;
    let articles = db.get_bookmarked_articles(feed_id.as_deref()).await.map_err(|e| e.to_string())?;
    Ok(articles)
}

#[tauri::command]
pub async fn get_bookmark_count(db: State<'_, DbState>) -> Result<i64, String> {
    let db = db.lock().await;
    let count = db.get_bookmark_count().await.map_err(|e| e.to_string())?;
    Ok(count)
}

#[tauri::command]
pub async fn get_bookmark_counts(db: State<'_, DbState>) -> Result<std::collections::HashMap<String, i64>, String> {
    let db = db.lock().await;
    let counts = db.get_bookmark_counts().await.map_err(|e| e.to_string())?;
    Ok(counts)
}

#[tauri::command]
pub async fn get_unread_articles(db: State<'_, DbState>) -> Result<Vec<feedwire_core::models::Article>, String> {
    let db = db.lock().await;
    let articles = db.get_unread_articles().await.map_err(|e| e.to_string())?;
    Ok(articles)
}

#[tauri::command]
pub async fn get_today_articles(db: State<'_, DbState>) -> Result<Vec<feedwire_core::models::Article>, String> {
    let db = db.lock().await;
    let articles = db.get_today_articles().await.map_err(|e| e.to_string())?;
    Ok(articles)
}

#[tauri::command]
pub async fn mark_all_read(feed_id: Option<String>, db: State<'_, DbState>) -> Result<(), String> {
    let db = db.lock().await;
    db.mark_all_read(feed_id.as_deref()).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn refresh_feed(feed_id: String, db: State<'_, DbState>) -> Result<Vec<feedwire_core::models::Article>, String> {
    let db = db.inner().clone();
    refresh::refresh_feed(&db, &feed_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn refresh_all_feeds(app: AppHandle, db: State<'_, DbState>) -> Result<Vec<String>, String> {
    refresh_all_feeds_inner(app, db.inner().clone()).await.map_err(|e| e.to_string())
}

/// Refreshes every feed through the shared core engine, forwarding progress to
/// the frontend as a Tauri event. Called by the command above and by the
/// background refresh timer in `lib.rs`.
pub async fn refresh_all_feeds_inner(
    app: AppHandle,
    db: DbState,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    let app_for_events = app.clone();
    refresh::refresh_all(db, move |done, total, feed_title| {
        let _ = app_for_events.emit(
            "refresh-progress",
            RefreshProgress { done, total, feed_title: feed_title.to_string() },
        );
    })
    .await
}

#[tauri::command]
pub async fn get_unread_count(feed_id: Option<String>, db: State<'_, DbState>) -> Result<i64, String> {
    let db = db.lock().await;
    let count = db.get_unread_count(feed_id).await.map_err(|e| e.to_string())?;
    Ok(count)
}

// ── Folder commands ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_folders(db: State<'_, DbState>) -> Result<Vec<Folder>, String> {
    let db_guard = db.lock().await;
    db_guard.get_folders().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_folder(name: String, db: State<'_, DbState>) -> Result<Folder, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Folder name cannot be empty".to_string());
    }
    let db_guard = db.lock().await;
    db_guard.create_folder(&name).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rename_folder(folder_id: String, name: String, db: State<'_, DbState>) -> Result<Folder, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Folder name cannot be empty".to_string());
    }
    let db_guard = db.lock().await;
    db_guard.rename_folder(&folder_id, &name).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_folder(folder_id: String, db: State<'_, DbState>) -> Result<(), String> {
    let db_guard = db.lock().await;
    db_guard.delete_folder(&folder_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_feed_folder(
    feed_id: String,
    folder_id: Option<String>,
    db: State<'_, DbState>,
) -> Result<feedwire_core::models::Feed, String> {
    let db_guard = db.lock().await;
    db_guard
        .set_feed_folder(&feed_id, folder_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}

// ── Filter rule commands ──────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_filter_settings(db: State<'_, DbState>) -> Result<feedwire_core::models::FilterSettings, String> {
    let db_guard = db.lock().await;
    let skip_youtube_shorts = db_guard
        .get_setting(SETTING_SKIP_YOUTUBE_SHORTS)
        .await
        .map_err(|e| e.to_string())?
        .map(|v| v == "true")
        .unwrap_or(false);
    let rules = db_guard.get_filter_rules().await.map_err(|e| e.to_string())?;
    Ok(feedwire_core::models::FilterSettings { skip_youtube_shorts, rules })
}

#[tauri::command]
pub async fn set_skip_youtube_shorts(enabled: bool, db: State<'_, DbState>) -> Result<(), String> {
    let db_guard = db.lock().await;
    db_guard
        .set_setting(SETTING_SKIP_YOUTUBE_SHORTS, if enabled { "true" } else { "false" })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_filter_rule(pattern: String, field: String, db: State<'_, DbState>) -> Result<feedwire_core::models::FilterRule, String> {
    // Validate field
    if field != "url" && field != "title" {
        return Err(format!("Invalid field '{}': must be 'url' or 'title'", field));
    }
    if pattern.trim().is_empty() {
        return Err("Pattern cannot be empty".to_string());
    }
    let db_guard = db.lock().await;
    db_guard
        .add_filter_rule(NewFilterRule { pattern, field })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_filter_rule_enabled(rule_id: String, enabled: bool, db: State<'_, DbState>) -> Result<(), String> {
    let db_guard = db.lock().await;
    db_guard
        .update_filter_rule_enabled(&rule_id, enabled)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_filter_rule(rule_id: String, db: State<'_, DbState>) -> Result<(), String> {
    let db_guard = db.lock().await;
    db_guard
        .delete_filter_rule(&rule_id)
        .await
        .map_err(|e| e.to_string())
}

/// Returns the configured auto-refresh interval in minutes.
/// Unset defaults to 30; an explicit 0 disables background refresh.
#[tauri::command]
pub async fn get_auto_refresh_interval(db: State<'_, DbState>) -> Result<u64, String> {
    let db_guard = db.lock().await;
    let val = db_guard
        .get_setting(SETTING_AUTO_REFRESH_INTERVAL)
        .await
        .map_err(|e| e.to_string())?
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(30);
    Ok(val)
}

/// Sets the auto-refresh interval (0 = disabled, otherwise minutes between refreshes).
#[tauri::command]
pub async fn set_auto_refresh_interval(minutes: u64, db: State<'_, DbState>) -> Result<(), String> {
    let db_guard = db.lock().await;
    db_guard
        .set_setting(SETTING_AUTO_REFRESH_INTERVAL, &minutes.to_string())
        .await
        .map_err(|e| e.to_string())
}

/// Export all feeds as an OPML 2.0 file — opens a native save dialog.
#[tauri::command]
pub async fn export_opml(app: tauri::AppHandle, db: State<'_, DbState>) -> Result<String, String> {
    // Collect feeds
    let feeds: Vec<(String, String)> = {
        let db_guard = db.lock().await;
        db_guard
            .get_feeds()
            .await
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|f| (f.title, f.url))
            .collect()
    };

    if feeds.is_empty() {
        return Err("No feeds to export".to_string());
    }

    let xml = opml::export_opml(&feeds);

    // Open native save dialog
    let path = app
        .dialog()
        .file()
        .set_title("Export OPML")
        .set_file_name("feedwire-subscriptions.opml")
        .add_filter("OPML Files", &["opml", "xml"])
        .blocking_save_file();

    match path {
        Some(fp) => {
            let path_buf = fp.as_path()
                .ok_or_else(|| "Invalid save path".to_string())?
                .to_path_buf();
            std::fs::write(&path_buf, xml).map_err(|e| e.to_string())?;
            Ok(format!("Exported {} feeds to {}", feeds.len(), path_buf.display()))
        }
        None => Err("Export cancelled".to_string()),
    }
}

#[derive(serde::Serialize)]
pub struct ImportResult {
    pub added: usize,
    pub skipped: usize,
    pub failed: Vec<FailedFeed>,
}

#[derive(serde::Serialize)]
pub struct FailedFeed {
    pub url: String,
    pub title: String,
    pub reason: String,
}

/// Import feeds from an OPML file — opens a native open dialog, then adds each
/// feed URL (skipping duplicates and failed ones gracefully).
#[tauri::command]
pub async fn import_opml(app: tauri::AppHandle, db: State<'_, DbState>) -> Result<ImportResult, String> {
    // Open native file picker
    let path = app
        .dialog()
        .file()
        .set_title("Import OPML")
        .add_filter("OPML Files", &["opml", "xml"])
        .blocking_pick_file();

    let file_path = match path {
        Some(fp) => fp.as_path()
            .ok_or_else(|| "Invalid file path".to_string())?
            .to_path_buf(),
        None => return Err("Import cancelled".to_string()),
    };

    let xml = std::fs::read_to_string(&file_path).map_err(|e| e.to_string())?;
    let opml_feeds = opml::import_opml(&xml).map_err(|e| e.to_string())?;

    if opml_feeds.is_empty() {
        return Err("No feeds found in OPML file".to_string());
    }

    let parser = FeedParser::new();
    let mut added = 0usize;
    let mut skipped = 0usize;
    let mut failed: Vec<FailedFeed> = Vec::new();

    for opml_feed in opml_feeds {
        // Skip only if an active feed with this URL already exists
        let already_active = {
            let db_guard = db.lock().await;
            db_guard.get_feed_by_url(&opml_feed.url).await
                .ok()
                .flatten()
                .map(|f| f.is_active)
                .unwrap_or(false)
        };

        if already_active {
            skipped += 1;
            continue;
        }

        // Validate and try to discover the feed
        let (final_url, yt_title) = if parser.is_youtube_channel(&opml_feed.url).await {
            match parser.convert_youtube_to_rss(&opml_feed.url).await {
                Ok(u) => u,
                Err(e) => {
                    eprintln!("OPML import: YouTube conversion failed for {}: {}", opml_feed.url, e);
                    failed.push(FailedFeed {
                        url: opml_feed.url.clone(),
                        title: opml_feed.title.clone(),
                        reason: format!("Could not convert YouTube URL: {}", e),
                    });
                    continue;
                }
            }
        } else {
            (opml_feed.url.clone(), None)
        };

        // Discover feed metadata — no lock held during network I/O.
        // YouTube feeds are saved with fallback metadata when the RSS
        // endpoint is unreachable (intermittent 404s); auto-refresh
        // backfills articles later.
        let new_feed = match parser.discover_feed(&final_url).await {
            Ok(f) => f,
            Err(e) if final_url.contains("youtube.com/feeds") => {
                println!("OPML import: discovery failed for {} ({}), saving with fallback metadata", final_url, e);
                let title_hint = if opml_feed.title.trim().is_empty() { yt_title } else { Some(opml_feed.title.clone()) };
                parser.fallback_youtube_feed(&final_url, title_hint).await
            }
            Err(e) => {
                eprintln!("OPML import: discovery failed for {}: {}", final_url, e);
                failed.push(FailedFeed {
                    url: final_url.clone(),
                    title: opml_feed.title.clone(),
                    reason: format!("Could not fetch feed: {}", e),
                });
                continue;
            }
        };

        // Persist — reactivates soft-deleted rows, inserts new ones.
        // Race-safe: create_or_reactivate_feed returns DuplicateUrl if
        // another concurrent insert beat us to it.
        let feed = {
            let db_guard = db.lock().await;
            match db_guard.create_or_reactivate_feed(new_feed).await {
                Ok(f) => f,
                Err(feedwire_core::DatabaseError::DuplicateUrl) => {
                    skipped += 1;
                    continue;
                }
                Err(e) => {
                    eprintln!("OPML import: DB save failed for {}: {}", final_url, e);
                    failed.push(FailedFeed {
                        url: final_url.clone(),
                        title: opml_feed.title.clone(),
                        reason: format!("Database error: {}", e),
                    });
                    continue;
                }
            }
        };

        // Fetch initial articles in background (non-fatal if this fails)
        if let Err(e) = refresh::fetch_feed_articles(feed.id.clone(), db.inner().clone()).await {
            eprintln!("OPML import: initial article fetch failed for {}: {}", feed.id, e);
        }

        added += 1;
    }

    Ok(ImportResult { added, skipped, failed })
}