use crate::db::{Database, FeedParser, FeedUpdate, ArticleUpdate};
use crate::opml;
use tauri::State;
use tauri_plugin_dialog::DialogExt;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type DbState = Arc<Mutex<Database>>;

#[tauri::command]
pub async fn add_feed(url: String, db: State<'_, DbState>) -> Result<crate::db::models::Feed, String> {
    println!("Adding feed: {}", url);
    
    let parser = FeedParser::new();
    
    // Validate URL
    FeedParser::validate_feed_url(&url).map_err(|e| {
        println!("URL validation failed: {}", e);
        e.to_string()
    })?;
    
    // Check if it's a YouTube channel and convert to RSS
    let final_url = if parser.is_youtube_channel(&url).await {
        println!("Detected YouTube channel, converting to RSS");
        parser.convert_youtube_to_rss(&url).await.map_err(|e| {
            println!("YouTube conversion failed: {}", e);
            e.to_string()
        })?
    } else {
        url.clone()
    };
    
    println!("Final URL to fetch: {}", final_url);
    
    // Check for duplicates — brief lock, released immediately after the check
    {
        let db_guard = db.lock().await;
        let existing_feeds = db_guard.get_feeds().await.map_err(|e| {
            println!("Failed to check existing feeds: {}", e);
            e.to_string()
        })?;
        for existing in existing_feeds {
            if existing.url == final_url {
                println!("Feed already exists: {}", existing.title);
                return Err("Feed already exists".to_string());
            }
        }
    } // lock released before any network I/O

    // Discover feed metadata — no lock held during the network call
    let new_feed = parser.discover_feed(&final_url).await.map_err(|e| {
        println!("Feed discovery failed: {}", e);
        e.to_string()
    })?;
    println!("Feed discovered: {}", new_feed.title);

    // Persist — brief lock just for the DB write
    let feed = {
        let db_guard = db.lock().await;
        let feed = db_guard.create_feed(new_feed).await.map_err(|e| {
            println!("Database save failed: {}", e);
            e.to_string()
        })?;
        println!("Feed saved to database with ID: {}", feed.id);
        feed
    };

    // Fetch initial articles after the lock is released to avoid deadlock
    if let Err(e) = fetch_feed_articles(feed.id.clone(), db.inner().clone()).await {
        println!("Failed to fetch initial articles: {}", e);
    }

    Ok(feed)
}

#[tauri::command]
pub async fn get_feeds(db: State<'_, DbState>) -> Result<Vec<crate::db::models::Feed>, String> {
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
pub async fn get_articles(
    feed_id: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    db: State<'_, DbState>
) -> Result<Vec<crate::db::models::Article>, String> {
    let db = db.lock().await;
    let articles = db.get_articles(feed_id, limit, offset).await.map_err(|e| e.to_string())?;
    Ok(articles)
}

#[tauri::command]
pub async fn get_article(article_id: String, db: State<'_, DbState>) -> Result<crate::db::models::Article, String> {
    let db = db.lock().await;
    let article = db.get_article_by_id(&article_id).await.map_err(|e| e.to_string())?;
    Ok(article)
}

#[tauri::command]
pub async fn mark_article_read(article_id: String, is_read: bool, db: State<'_, DbState>) -> Result<crate::db::models::Article, String> {
    let db = db.lock().await;
    let update = ArticleUpdate {
        is_read: Some(is_read),
        ..Default::default()
    };
    let article = db.update_article(&article_id, update).await.map_err(|e| e.to_string())?;
    Ok(article)
}

#[tauri::command]
pub async fn toggle_bookmark(article_id: String, db: State<'_, DbState>) -> Result<crate::db::models::Article, String> {
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
pub async fn get_bookmarked_articles(db: State<'_, DbState>) -> Result<Vec<crate::db::models::Article>, String> {
    let db = db.lock().await;
    let articles = db.get_bookmarked_articles().await.map_err(|e| e.to_string())?;
    Ok(articles)
}

#[tauri::command]
pub async fn get_unread_articles(db: State<'_, DbState>) -> Result<Vec<crate::db::models::Article>, String> {
    let db = db.lock().await;
    let articles = db.get_unread_articles().await.map_err(|e| e.to_string())?;
    Ok(articles)
}

#[tauri::command]
pub async fn mark_all_read(feed_id: Option<String>, db: State<'_, DbState>) -> Result<(), String> {
    let db = db.lock().await;
    db.mark_all_read(feed_id.as_deref()).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn refresh_feed(feed_id: String, db: State<'_, DbState>) -> Result<Vec<crate::db::models::Article>, String> {
    // Fetch feed URL — brief lock, released before any network I/O
    let feed_url = {
        let db_guard = db.lock().await;
        db_guard.get_feed_by_id(&feed_id).await.map_err(|e| e.to_string())?.url
    };

    // Network call — no lock held
    let parser = FeedParser::new();
    let (_, articles) = parser.fetch_feed(&feed_url).await.map_err(|e| e.to_string())?;

    // Persist — lock held only for DB writes
    let db_guard = db.lock().await;
    let mut saved_articles = Vec::new();
    for mut article in articles {
        article.feed_id = feed_id.clone();
        let saved = db_guard.create_article(article).await.map_err(|e| e.to_string())?;
        saved_articles.push(saved);
    }

    let update = FeedUpdate {
        last_fetched: Some(chrono::Utc::now()),
        ..Default::default()
    };
    db_guard.update_feed(&feed_id, update).await.map_err(|e| e.to_string())?;

    Ok(saved_articles)
}

#[tauri::command]
pub async fn refresh_all_feeds(db: State<'_, DbState>) -> Result<Vec<String>, String> {
    // Collect feed list first, then drop the lock before refreshing each feed
    let feeds = {
        let db_guard = db.lock().await;
        db_guard.get_feeds().await.map_err(|e| e.to_string())?
        // db_guard dropped here
    };

    let mut refreshed_feeds = Vec::new();
    for feed in feeds {
        if let Err(e) = refresh_feed_helper(feed.id.clone(), db.inner().clone()).await {
            eprintln!("Failed to refresh feed {}: {}", feed.id, e);
        } else {
            refreshed_feeds.push(feed.id);
        }
    }

    Ok(refreshed_feeds)
}

#[tauri::command]
pub async fn get_unread_count(feed_id: Option<String>, db: State<'_, DbState>) -> Result<i64, String> {
    let db = db.lock().await;
    let count = db.get_unread_count(feed_id).await.map_err(|e| e.to_string())?;
    Ok(count)
}

async fn fetch_feed_articles(feed_id: String, db: Arc<Mutex<Database>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Read feed URL — brief lock
    let feed_url = {
        let db_guard = db.lock().await;
        db_guard.get_feed_by_id(&feed_id).await?.url
    };

    // Network call — no lock held
    const MAX_INITIAL_ARTICLES: usize = 15;
    let parser = FeedParser::new();
    let (_, articles) = parser.fetch_feed(&feed_url).await?;

    // Write articles — lock held only for DB operations
    let db_guard = db.lock().await;
    for mut article in articles.into_iter().take(MAX_INITIAL_ARTICLES) {
        article.feed_id = feed_id.clone();
        db_guard.create_article(article).await?;
    }
    let update = FeedUpdate {
        last_fetched: Some(chrono::Utc::now()),
        ..Default::default()
    };
    db_guard.update_feed(&feed_id, update).await?;

    Ok(())
}

async fn refresh_feed_helper(feed_id: String, db: Arc<Mutex<Database>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    fetch_feed_articles(feed_id, db).await
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

/// Import feeds from an OPML file — opens a native open dialog, then adds each
/// feed URL (skipping duplicates and failed ones gracefully).
#[tauri::command]
pub async fn import_opml(app: tauri::AppHandle, db: State<'_, DbState>) -> Result<String, String> {
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
    let mut imported = 0usize;
    let mut skipped = 0usize;
    let mut failed = 0usize;

    for opml_feed in opml_feeds {
        // Check duplicate by URL before attempting network fetch
        let already_exists = {
            let db_guard = db.lock().await;
            let existing = db_guard.get_feeds().await.unwrap_or_default();
            existing.iter().any(|f| f.url == opml_feed.url)
        };

        if already_exists {
            skipped += 1;
            continue;
        }

        // Validate and try to discover the feed
        let final_url = if parser.is_youtube_channel(&opml_feed.url).await {
            match parser.convert_youtube_to_rss(&opml_feed.url).await {
                Ok(u) => u,
                Err(e) => {
                    eprintln!("OPML import: YouTube conversion failed for {}: {}", opml_feed.url, e);
                    failed += 1;
                    continue;
                }
            }
        } else {
            opml_feed.url.clone()
        };

        // Discover feed metadata — no lock held during network I/O
        let new_feed = match parser.discover_feed(&final_url).await {
            Ok(f) => f,
            Err(e) => {
                eprintln!("OPML import: discovery failed for {}: {}", final_url, e);
                failed += 1;
                continue;
            }
        };

        // Persist — brief lock, re-check duplicate race-safely before writing
        let feed = {
            let db_guard = db.lock().await;
            let existing = db_guard.get_feeds().await.unwrap_or_default();
            if existing.iter().any(|f| f.url == final_url) {
                skipped += 1;
                continue;
            }
            match db_guard.create_feed(new_feed).await {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("OPML import: DB save failed for {}: {}", final_url, e);
                    failed += 1;
                    continue;
                }
            }
        };

        // Fetch initial articles in background (non-fatal if this fails)
        if let Err(e) = fetch_feed_articles(feed.id.clone(), db.inner().clone()).await {
            eprintln!("OPML import: initial article fetch failed for {}: {}", feed.id, e);
        }

        imported += 1;
    }

    Ok(format!(
        "Import complete: {} added, {} skipped (duplicates), {} failed",
        imported, skipped, failed
    ))
}

impl Default for FeedUpdate {
    fn default() -> Self {
        Self {
            title: None,
            description: None,
            last_fetched: None,
            is_active: None,
        }
    }
}

impl Default for ArticleUpdate {
    fn default() -> Self {
        Self {
            title: None,
            link: None,
            description: None,
            content: None,
            author: None,
            published_at: None,
            is_read: None,
            is_bookmarked: None,
        }
    }
}