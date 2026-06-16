use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// ── Filter rules ────────────────────────────────────────────────────────────

/// Which article field the pattern is matched against.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
pub enum FilterField {
    #[serde(rename = "url")]
    Url,
    #[serde(rename = "title")]
    Title,
}

impl std::fmt::Display for FilterField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FilterField::Url => write!(f, "url"),
            FilterField::Title => write!(f, "title"),
        }
    }
}

impl std::str::FromStr for FilterField {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "url" => Ok(FilterField::Url),
            "title" => Ok(FilterField::Title),
            _ => Err(format!("Unknown filter field: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FilterRule {
    pub id: String,
    pub pattern: String,
    pub field: String,   // stored as text, matches FilterField display
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewFilterRule {
    pub pattern: String,
    pub field: String,
}

// ── App settings ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AppSetting {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterSettings {
    pub skip_youtube_shorts: bool,
    pub rules: Vec<FilterRule>,
}

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