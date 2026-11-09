/// Feed refresh engine shared by the GUI and the TUI.
///
/// Extracted from the former Tauri command layer. Progress reporting is a
/// plain callback so each frontend can surface it its own way (Tauri events
/// for the GUI, a status-bar line for the TUI).

use crate::filters::should_keep;
use crate::models::{Article, FeedUpdate};
use crate::{Database, FeedParser, SETTING_SKIP_YOUTUBE_SHORTS};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};

/// Database handle shared across tasks; matches the GUI's `DbState`.
pub type SharedDatabase = Arc<Mutex<Database>>;

const REFRESH_CONCURRENCY: usize = 10;

/// Fetch and persist the initial articles for a newly added or imported feed.
/// Caps the first batch at 15 articles and backfills the feed icon when needed.
pub async fn fetch_feed_articles(
    feed_id: String,
    db: SharedDatabase,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Read feed URL, existing icon_url, + filter settings — brief lock
    let (feed_url, existing_icon_url, skip_shorts, rules) = {
        let db_guard = db.lock().await;
        let feed = db_guard.get_feed_by_id(&feed_id).await?;
        let skip_shorts = db_guard
            .get_setting(SETTING_SKIP_YOUTUBE_SHORTS)
            .await?
            .map(|v| v == "true")
            .unwrap_or(false);
        let rules = db_guard.get_filter_rules().await?;
        (feed.url, feed.icon_url, skip_shorts, rules)
    };

    // Network call — no lock held
    const MAX_INITIAL_ARTICLES: usize = 15;
    let parser = FeedParser::new();
    let (new_feed_meta, articles) = parser.fetch_feed(&feed_url).await?;

    // Apply filters before persisting
    let filtered: Vec<_> = articles
        .into_iter()
        .filter(|a| should_keep(a, skip_shorts, &rules))
        .take(MAX_INITIAL_ARTICLES)
        .collect();

    // Backfill icon_url if:
    // - it was never set (NULL), or
    // - it's the generic YouTube favicon fallback (meaning the real channel avatar
    //   wasn't scraped yet, e.g. feeds added before fetch_youtube_avatar was added)
    let needs_icon_backfill = existing_icon_url.as_deref()
        .map(|u| u.is_empty() || (u.contains("google.com/s2/favicons") && u.contains("youtube.com")))
        .unwrap_or(true); // None → needs backfill
    let icon_url = if needs_icon_backfill { new_feed_meta.icon_url } else { None };

    // Write articles — lock held only for DB operations
    let db_guard = db.lock().await;
    for mut article in filtered {
        article.feed_id = feed_id.clone();
        db_guard.create_article(article).await?;
    }
    let update = FeedUpdate {
        last_fetched: Some(chrono::Utc::now()),
        icon_url,
        ..Default::default()
    };
    db_guard.update_feed(&feed_id, update).await?;

    Ok(())
}

/// Refresh a single feed, persisting every new article (no cap, unlike the
/// initial import). Returns the articles saved in this refresh.
pub async fn refresh_feed(
    db: &SharedDatabase,
    feed_id: &str,
) -> Result<Vec<Article>, Box<dyn std::error::Error + Send + Sync>> {
    // Fetch feed URL + filter settings — brief lock, released before any network I/O
    let (feed_url, skip_shorts, rules) = {
        let db_guard = db.lock().await;
        let url = db_guard.get_feed_by_id(feed_id).await?.url;
        let skip_shorts = db_guard
            .get_setting(SETTING_SKIP_YOUTUBE_SHORTS)
            .await?
            .map(|v| v == "true")
            .unwrap_or(false);
        let rules = db_guard.get_filter_rules().await?;
        (url, skip_shorts, rules)
    };

    // Network call — no lock held
    let parser = FeedParser::new();
    let (_, articles) = parser.fetch_feed(&feed_url).await?;

    // Apply filters
    let filtered: Vec<_> = articles
        .into_iter()
        .filter(|a| should_keep(a, skip_shorts, &rules))
        .collect();

    // Persist — lock held only for DB writes
    let db_guard = db.lock().await;
    let mut saved_articles = Vec::new();
    for mut article in filtered {
        article.feed_id = feed_id.to_string();
        let saved = db_guard.create_article(article).await?;
        saved_articles.push(saved);
    }

    let update = FeedUpdate {
        last_fetched: Some(chrono::Utc::now()),
        ..Default::default()
    };
    db_guard.update_feed(feed_id, update).await?;

    Ok(saved_articles)
}

/// Refresh all feeds concurrently, reporting progress through `on_progress`
/// as each feed finishes. Returns the ids of the feeds that refreshed
/// successfully; failures are logged and skipped, mirroring the GUI behaviour.
pub async fn refresh_all<P>(
    db: SharedDatabase,
    on_progress: P,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>>
where
    P: Fn(usize, usize, &str) + Send + Sync + 'static,
{
    let on_progress = Arc::new(on_progress);

    // Collect feed list — brief lock
    let feeds = {
        let db_guard = db.lock().await;
        db_guard.get_feeds().await?
    };

    let total = feeds.len();
    let sem = Arc::new(Semaphore::new(REFRESH_CONCURRENCY));
    // Atomic counter so concurrent tasks can report progress in order
    let done_counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let refreshed: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    let mut handles = Vec::with_capacity(feeds.len());

    for feed in feeds {
        let sem = sem.clone();
        let db = db.clone();
        let on_progress = on_progress.clone();
        let done_counter = done_counter.clone();
        let refreshed = refreshed.clone();

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await;
            let ok = fetch_feed_articles(feed.id.clone(), db).await.is_ok();
            if !ok {
                eprintln!("Failed to refresh feed {}", feed.id);
            }
            let done = done_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            if ok {
                refreshed.lock().await.push(feed.id.clone());
            }
            on_progress(done, total, &feed.title);
        });
        handles.push(handle);
    }

    for h in handles {
        let _ = h.await;
    }

    let result = refreshed.lock().await.clone();
    Ok(result)
}
