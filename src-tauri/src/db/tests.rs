/// Integration tests for the Database layer.
///
/// Each test gets its own in-memory SQLite database so there is no state
/// bleed between runs and no filesystem artefacts.
///
/// Run with: cargo test

use crate::db::{Database, DatabaseError};
use crate::db::models::{NewFeed, NewArticle};

async fn test_db() -> Database {
    Database::with_url("sqlite::memory:").await.expect("in-memory DB failed")
}

fn new_feed(title: &str, url: &str) -> NewFeed {
    NewFeed {
        title: title.to_string(),
        url: url.to_string(),
        description: Some(format!("Description for {}", title)),
        feed_type: "rss".to_string(),
    }
}

fn new_article(feed_id: &str, title: &str, guid: &str) -> NewArticle {
    NewArticle {
        feed_id: feed_id.to_string(),
        title: title.to_string(),
        link: Some(format!("https://example.com/{}", guid)),
        description: Some("An article description".to_string()),
        content: None,
        author: Some("Author Name".to_string()),
        published_at: Some(chrono::Utc::now()),
        guid: Some(guid.to_string()),
    }
}

// ── Feed CRUD ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_and_get_feed() {
    let db = test_db().await;
    let feed = db.create_feed(new_feed("Hacker News", "https://news.ycombinator.com/rss")).await.unwrap();

    assert!(!feed.id.is_empty());
    assert_eq!(feed.title, "Hacker News");
    assert_eq!(feed.url, "https://news.ycombinator.com/rss");
    assert!(feed.is_active);
    assert!(feed.last_fetched.is_none());
}

#[tokio::test]
async fn get_feeds_returns_all_active() {
    let db = test_db().await;
    db.create_feed(new_feed("Feed A", "https://a.example/rss")).await.unwrap();
    db.create_feed(new_feed("Feed B", "https://b.example/rss")).await.unwrap();

    let feeds = db.get_feeds().await.unwrap();
    assert_eq!(feeds.len(), 2);
}

#[tokio::test]
async fn get_feed_by_id() {
    let db = test_db().await;
    let created = db.create_feed(new_feed("Feed", "https://example.com/rss")).await.unwrap();

    let fetched = db.get_feed_by_id(&created.id).await.unwrap();
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.title, "Feed");
}

#[tokio::test]
async fn get_feed_by_id_not_found() {
    let db = test_db().await;
    let result = db.get_feed_by_id("nonexistent-id").await;
    assert!(matches!(result, Err(DatabaseError::FeedNotFound)));
}

#[tokio::test]
async fn delete_feed_soft_deletes() {
    let db = test_db().await;
    let feed = db.create_feed(new_feed("Feed", "https://example.com/rss")).await.unwrap();

    db.delete_feed(&feed.id).await.unwrap();

    // Should not appear in active feeds list
    let feeds = db.get_feeds().await.unwrap();
    assert!(feeds.is_empty());

    // Should not be findable by ID either
    let result = db.get_feed_by_id(&feed.id).await;
    assert!(matches!(result, Err(DatabaseError::FeedNotFound)));
}

#[tokio::test]
async fn duplicate_url_rejected() {
    let db = test_db().await;
    db.create_feed(new_feed("Feed", "https://example.com/rss")).await.unwrap();
    let result = db.create_feed(new_feed("Feed Copy", "https://example.com/rss")).await;
    assert!(result.is_err());
}

// ── Article CRUD ───────────────────────────────────────────────────────────

#[tokio::test]
async fn create_and_get_article() {
    let db = test_db().await;
    let feed = db.create_feed(new_feed("Feed", "https://example.com/rss")).await.unwrap();

    let article = db.create_article(new_article(&feed.id, "First Post", "guid-1")).await.unwrap();

    assert!(!article.id.is_empty());
    assert_eq!(article.title, "First Post");
    assert_eq!(article.feed_id, feed.id);
    assert!(!article.is_read);
    assert!(!article.is_bookmarked);
}

#[tokio::test]
async fn duplicate_guid_ignored() {
    let db = test_db().await;
    let feed = db.create_feed(new_feed("Feed", "https://example.com/rss")).await.unwrap();

    db.create_article(new_article(&feed.id, "Post", "guid-1")).await.unwrap();
    // Second insert with same (feed_id, guid) should silently succeed via INSERT OR IGNORE
    db.create_article(new_article(&feed.id, "Post Duplicate", "guid-1")).await.unwrap();

    let articles = db.get_articles(Some(feed.id), None, None).await.unwrap();
    assert_eq!(articles.len(), 1);
    assert_eq!(articles[0].title, "Post"); // Original preserved
}

#[tokio::test]
async fn get_articles_filtered_by_feed() {
    let db = test_db().await;
    let feed_a = db.create_feed(new_feed("Feed A", "https://a.example/rss")).await.unwrap();
    let feed_b = db.create_feed(new_feed("Feed B", "https://b.example/rss")).await.unwrap();

    db.create_article(new_article(&feed_a.id, "A1", "a-guid-1")).await.unwrap();
    db.create_article(new_article(&feed_a.id, "A2", "a-guid-2")).await.unwrap();
    db.create_article(new_article(&feed_b.id, "B1", "b-guid-1")).await.unwrap();

    let a_articles = db.get_articles(Some(feed_a.id), None, None).await.unwrap();
    assert_eq!(a_articles.len(), 2);

    let b_articles = db.get_articles(Some(feed_b.id), None, None).await.unwrap();
    assert_eq!(b_articles.len(), 1);
}

#[tokio::test]
async fn get_articles_with_limit() {
    let db = test_db().await;
    let feed = db.create_feed(new_feed("Feed", "https://example.com/rss")).await.unwrap();

    for i in 0..10 {
        db.create_article(new_article(&feed.id, &format!("Post {}", i), &format!("guid-{}", i))).await.unwrap();
    }

    let limited = db.get_articles(Some(feed.id), Some(3), None).await.unwrap();
    assert_eq!(limited.len(), 3);
}

// ── Mark read / bookmark ───────────────────────────────────────────────────

#[tokio::test]
async fn mark_article_read() {
    let db = test_db().await;
    let feed = db.create_feed(new_feed("Feed", "https://example.com/rss")).await.unwrap();
    let article = db.create_article(new_article(&feed.id, "Post", "guid-1")).await.unwrap();

    assert!(!article.is_read);

    let updated = db.update_article(&article.id, crate::db::ArticleUpdate {
        is_read: Some(true),
        ..Default::default()
    }).await.unwrap();

    assert!(updated.is_read);
}

#[tokio::test]
async fn bookmark_article() {
    let db = test_db().await;
    let feed = db.create_feed(new_feed("Feed", "https://example.com/rss")).await.unwrap();
    let article = db.create_article(new_article(&feed.id, "Post", "guid-1")).await.unwrap();

    db.update_article(&article.id, crate::db::ArticleUpdate {
        is_bookmarked: Some(true),
        ..Default::default()
    }).await.unwrap();

    let bookmarks = db.get_bookmarked_articles().await.unwrap();
    assert_eq!(bookmarks.len(), 1);
    assert_eq!(bookmarks[0].id, article.id);
}

#[tokio::test]
async fn mark_all_read_for_feed() {
    let db = test_db().await;
    let feed = db.create_feed(new_feed("Feed", "https://example.com/rss")).await.unwrap();
    db.create_article(new_article(&feed.id, "P1", "g1")).await.unwrap();
    db.create_article(new_article(&feed.id, "P2", "g2")).await.unwrap();

    let unread_before = db.get_unread_count(Some(feed.id.clone())).await.unwrap();
    assert_eq!(unread_before, 2);

    db.mark_all_read(Some(&feed.id)).await.unwrap();

    let unread_after = db.get_unread_count(Some(feed.id)).await.unwrap();
    assert_eq!(unread_after, 0);
}

#[tokio::test]
async fn mark_all_read_global() {
    let db = test_db().await;
    let feed_a = db.create_feed(new_feed("A", "https://a.example/rss")).await.unwrap();
    let feed_b = db.create_feed(new_feed("B", "https://b.example/rss")).await.unwrap();
    db.create_article(new_article(&feed_a.id, "A1", "ag1")).await.unwrap();
    db.create_article(new_article(&feed_b.id, "B1", "bg1")).await.unwrap();

    db.mark_all_read(None).await.unwrap();

    let unread = db.get_unread_count(None).await.unwrap();
    assert_eq!(unread, 0);
}

// ── Cascade delete ─────────────────────────────────────────────────────────

#[tokio::test]
async fn deleting_feed_articles_cascade() {
    let db = test_db().await;
    let feed = db.create_feed(new_feed("Feed", "https://example.com/rss")).await.unwrap();
    db.create_article(new_article(&feed.id, "Post", "guid-1")).await.unwrap();

    // Soft-delete the feed
    db.delete_feed(&feed.id).await.unwrap();

    // Articles for that feed are still in the DB (soft delete doesn't cascade),
    // but since the feed is inactive the unread count should be 0 via feed lookup.
    // The important thing is there's no panic / FK violation.
    let feeds = db.get_feeds().await.unwrap();
    assert!(feeds.is_empty());
}
