use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Feed {
    pub id: String,
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub feed_type: String, // "rss" or "atom"
    pub last_fetched: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Article {
    pub id: String,
    pub feed_id: String,
    pub title: String,
    pub link: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub author: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_read: bool,
    pub is_bookmarked: bool,
    pub guid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewFeed {
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub feed_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewArticle {
    pub feed_id: String,
    pub title: String,
    pub link: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub author: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub guid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedUpdate {
    pub title: Option<String>,
    pub description: Option<String>,
    pub last_fetched: Option<DateTime<Utc>>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleUpdate {
    pub title: Option<String>,
    pub link: Option<String>,
    pub description: Option<String>,
    pub content: Option<String>,
    pub author: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub is_read: Option<bool>,
    pub is_bookmarked: Option<bool>,
}