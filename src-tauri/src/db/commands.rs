use crate::db::{Database, FeedParser, FeedUpdate, ArticleUpdate};
use tauri::State;
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
    
    // Check if feed already exists, then discover and save — all while holding the lock
    let feed = {
        let db_guard = db.lock().await;
        let existing_feeds = db_guard.get_feeds().await.map_err(|e| {
            println!("Failed to check existing feeds: {}", e);
            e.to_string()
        })?;

        for existing_feed in existing_feeds {
            if existing_feed.url == final_url {
                println!("Feed already exists: {}", existing_feed.title);
                return Err("Feed already exists".to_string());
            }
        }

        // Discover feed (network call — lock is held but that's fine, no re-entry here)
        let new_feed = parser.discover_feed(&final_url).await.map_err(|e| {
            println!("Feed discovery failed: {}", e);
            e.to_string()
        })?;

        println!("Feed discovered: {}", new_feed.title);

        let feed = db_guard.create_feed(new_feed).await.map_err(|e| {
            println!("Database save failed: {}", e);
            e.to_string()
        })?;

        println!("Feed saved to database with ID: {}", feed.id);
        feed
        // db_guard is dropped here
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
pub async fn refresh_feed(feed_id: String, db: State<'_, DbState>) -> Result<Vec<crate::db::models::Article>, String> {
    let db = db.lock().await;
    let feed = db.get_feed_by_id(&feed_id).await.map_err(|e| e.to_string())?;
    
    let parser = FeedParser::new();
    let (_, articles) = parser.fetch_feed(&feed.url).await.map_err(|e| e.to_string())?;
    
    let mut saved_articles = Vec::new();
    for mut article in articles {
        article.feed_id = feed_id.clone();
        let saved = db.create_article(article).await.map_err(|e| e.to_string())?;
        saved_articles.push(saved);
    }
    
    // Update feed's last_fetched timestamp
    let update = FeedUpdate {
        last_fetched: Some(chrono::Utc::now()),
        ..Default::default()
    };
    db.update_feed(&feed_id, update).await.map_err(|e| e.to_string())?;
    
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
    let db = db.lock().await;
    let feed = db.get_feed_by_id(&feed_id).await?;
    
    let parser = FeedParser::new();
    let (_, articles) = parser.fetch_feed(&feed.url).await?;
    
    for mut article in articles {
        article.feed_id = feed_id.clone();
        db.create_article(article).await?;
    }
    
    // Update feed's last_fetched timestamp
    let update = FeedUpdate {
        last_fetched: Some(chrono::Utc::now()),
        ..Default::default()
    };
    db.update_feed(&feed_id, update).await?;
    
    Ok(())
}

async fn refresh_feed_helper(feed_id: String, db: Arc<Mutex<Database>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    fetch_feed_articles(feed_id, db).await
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