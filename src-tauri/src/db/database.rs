use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use std::str::FromStr;
use crate::db::models::*;

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Feed not found")]
    FeedNotFound,
    #[error("Article not found")]
    ArticleNotFound,
    #[error("Feed already exists")]
    DuplicateUrl,
}

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Open the database at a specific filesystem path.
    /// The parent directory must already exist.
    pub async fn open(path: &std::path::Path) -> Result<Self, DatabaseError> {
        let url = format!("sqlite:{}", path.display());
        Self::with_url(&url).await
    }

    /// Open using a raw SQLite connection string (used by tests with `sqlite::memory:`).
    pub async fn with_url(url: &str) -> Result<Self, DatabaseError> {
        let connect_options = SqliteConnectOptions::from_str(url)?
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(connect_options).await?;

        let db = Self { pool };
        db.migrate().await?;

        Ok(db)
    }

    async fn migrate(&self) -> Result<(), DatabaseError> {
        // Create feeds table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS feeds (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                url TEXT NOT NULL UNIQUE,
                description TEXT,
                feed_type TEXT NOT NULL,
                last_fetched TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                is_active BOOLEAN NOT NULL DEFAULT 1
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create articles table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS articles (
                id TEXT PRIMARY KEY,
                feed_id TEXT NOT NULL,
                title TEXT NOT NULL,
                link TEXT,
                description TEXT,
                content TEXT,
                author TEXT,
                published_at TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                is_read BOOLEAN NOT NULL DEFAULT 0,
                is_bookmarked BOOLEAN NOT NULL DEFAULT 0,
                guid TEXT,
                FOREIGN KEY (feed_id) REFERENCES feeds (id) ON DELETE CASCADE,
                UNIQUE(feed_id, guid)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_articles_feed_id ON articles (feed_id)")
            .execute(&self.pool)
            .await?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_articles_published_at ON articles (published_at)")
            .execute(&self.pool)
            .await?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_articles_is_read ON articles (is_read)")
            .execute(&self.pool)
            .await?;
        
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_articles_is_bookmarked ON articles (is_bookmarked)")
            .execute(&self.pool)
            .await?;

        // Filter rules table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS filter_rules (
                id TEXT PRIMARY KEY,
                pattern TEXT NOT NULL,
                field TEXT NOT NULL DEFAULT 'url',
                enabled BOOLEAN NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // App settings key/value store
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // Feed operations
    pub async fn create_feed(&self, feed: NewFeed) -> Result<Feed, DatabaseError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();
        
        let feed = Feed {
            id: id.clone(),
            title: feed.title,
            url: feed.url,
            description: feed.description,
            feed_type: feed.feed_type,
            last_fetched: None,
            created_at: now,
            updated_at: now,
            is_active: true,
            latest_article_at: None,
        };

        sqlx::query(
            r#"
            INSERT INTO feeds (id, title, url, description, feed_type, last_fetched, created_at, updated_at, is_active)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&feed.id)
        .bind(&feed.title)
        .bind(&feed.url)
        .bind(&feed.description)
        .bind(&feed.feed_type)
        .bind(&feed.last_fetched)
        .bind(&feed.created_at)
        .bind(&feed.updated_at)
        .bind(feed.is_active)
        .execute(&self.pool)
        .await?;

        Ok(feed)
    }

    pub async fn get_feeds(&self) -> Result<Vec<Feed>, DatabaseError> {
        let feeds = sqlx::query_as::<_, Feed>(
            r#"
            SELECT
                f.id, f.title, f.url, f.description, f.feed_type,
                f.last_fetched, f.created_at, f.updated_at, f.is_active,
                MAX(a.published_at) AS latest_article_at
            FROM feeds f
            LEFT JOIN articles a ON a.feed_id = f.id
            WHERE f.is_active = 1
            GROUP BY f.id
            ORDER BY f.title
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(feeds)
    }

    pub async fn get_feed_by_id(&self, id: &str) -> Result<Feed, DatabaseError> {
        let feed = sqlx::query_as::<_, Feed>(
            r#"
            SELECT
                f.id, f.title, f.url, f.description, f.feed_type,
                f.last_fetched, f.created_at, f.updated_at, f.is_active,
                MAX(a.published_at) AS latest_article_at
            FROM feeds f
            LEFT JOIN articles a ON a.feed_id = f.id
            WHERE f.id = ? AND f.is_active = 1
            GROUP BY f.id
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        feed.ok_or(DatabaseError::FeedNotFound)
    }

    pub async fn update_feed(&self, id: &str, update: FeedUpdate) -> Result<Feed, DatabaseError> {
        let now = chrono::Utc::now().to_rfc3339();

        let mut set_clauses = vec!["updated_at = ?".to_string()];
        let mut params: Vec<String> = vec![now];  // slot 1 = updated_at

        if let Some(v) = update.title        { set_clauses.push("title = ?".into());        params.push(v); }
        if let Some(v) = update.description  { set_clauses.push("description = ?".into());  params.push(v); }
        if let Some(v) = update.last_fetched { set_clauses.push("last_fetched = ?".into()); params.push(v.to_rfc3339()); }
        if let Some(v) = update.is_active    { set_clauses.push("is_active = ?".into());    params.push(if v { "1" } else { "0" }.into()); }

        let query = format!("UPDATE feeds SET {} WHERE id = ?", set_clauses.join(", "));
        params.push(id.to_string());

        let mut q = sqlx::query(&query);
        for param in params {
            q = q.bind(param);
        }
        q.execute(&self.pool).await?;

        self.get_feed_by_id(id).await
    }

    /// Look up a feed by URL regardless of its active/inactive state.
    pub async fn get_feed_by_url(&self, url: &str) -> Result<Option<Feed>, DatabaseError> {
        let feed = sqlx::query_as::<_, Feed>(
            r#"
            SELECT
                f.id, f.title, f.url, f.description, f.feed_type,
                f.last_fetched, f.created_at, f.updated_at, f.is_active,
                MAX(a.published_at) AS latest_article_at
            FROM feeds f
            LEFT JOIN articles a ON a.feed_id = f.id
            WHERE f.url = ?
            GROUP BY f.id
            LIMIT 1
            "#,
        )
        .bind(url)
        .fetch_optional(&self.pool)
        .await?;

        Ok(feed)
    }

    /// Insert a new feed, or reactivate and update an existing (soft-deleted) one
    /// with the same URL.  Returns the final feed row either way.
    pub async fn create_or_reactivate_feed(&self, feed: NewFeed) -> Result<Feed, DatabaseError> {
        // Check whether a row with this URL already exists (active or not)
        if let Some(existing) = self.get_feed_by_url(&feed.url).await? {
            if existing.is_active {
                // Already active — treat as duplicate
                return Err(DatabaseError::DuplicateUrl);
            }
            // Reactivate the existing row with fresh metadata
            let now = chrono::Utc::now().to_rfc3339();
            sqlx::query(
                "UPDATE feeds SET title = ?, description = ?, is_active = 1, updated_at = ? WHERE id = ?"
            )
            .bind(&feed.title)
            .bind(&feed.description)
            .bind(&now)
            .bind(&existing.id)
            .execute(&self.pool)
            .await?;

            return self.get_feed_by_id(&existing.id).await;
        }

        // No existing row — regular insert
        self.create_feed(feed).await
    }

    pub async fn delete_feed(&self, id: &str) -> Result<(), DatabaseError> {
        sqlx::query("UPDATE feeds SET is_active = 0 WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn delete_all_feeds(&self) -> Result<usize, DatabaseError> {
        let result = sqlx::query("UPDATE feeds SET is_active = 0 WHERE is_active = 1")
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() as usize)
    }

    // Article operations
    pub async fn create_article(&self, article: NewArticle) -> Result<Article, DatabaseError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();
        
        let article = Article {
            id: id.clone(),
            feed_id: article.feed_id,
            title: article.title,
            link: article.link,
            description: article.description,
            content: article.content,
            author: article.author,
            published_at: article.published_at,
            created_at: now,
            updated_at: now,
            is_read: false,
            is_bookmarked: false,
            guid: article.guid,
        };

        sqlx::query(
            r#"
            INSERT OR IGNORE INTO articles (id, feed_id, title, link, description, content, author, published_at, created_at, updated_at, is_read, is_bookmarked, guid)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&article.id)
        .bind(&article.feed_id)
        .bind(&article.title)
        .bind(&article.link)
        .bind(&article.description)
        .bind(&article.content)
        .bind(&article.author)
        .bind(&article.published_at)
        .bind(&article.created_at)
        .bind(&article.updated_at)
        .bind(article.is_read)
        .bind(article.is_bookmarked)
        .bind(&article.guid)
        .execute(&self.pool)
        .await?;

        Ok(article)
    }

    pub async fn get_articles(&self, feed_id: Option<String>, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<Article>, DatabaseError> {
        let mut query = String::from("SELECT * FROM articles WHERE 1=1");
        let mut params = vec![];
        
        if let Some(feed_id) = feed_id {
            query.push_str(" AND feed_id = ?");
            params.push(feed_id);
        }
        
        query.push_str(" ORDER BY published_at DESC, created_at DESC");
        
        if let Some(limit) = limit {
            query.push_str(" LIMIT ?");
            params.push(limit.to_string());
        }
        
        if let Some(offset) = offset {
            query.push_str(" OFFSET ?");
            params.push(offset.to_string());
        }
        
        let mut query_builder = sqlx::query_as::<_, Article>(&query);
        for param in params {
            query_builder = query_builder.bind(param);
        }
        
        let articles = query_builder.fetch_all(&self.pool).await?;
        Ok(articles)
    }

    pub async fn get_article_by_id(&self, id: &str) -> Result<Article, DatabaseError> {
        let article = sqlx::query_as::<_, Article>(
            "SELECT * FROM articles WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        article.ok_or(DatabaseError::ArticleNotFound)
    }

    pub async fn update_article(&self, id: &str, update: ArticleUpdate) -> Result<Article, DatabaseError> {
        let now = chrono::Utc::now().to_rfc3339();

        // Build SET clauses; updated_at is always set and must be bound first
        let mut set_clauses = vec!["updated_at = ?".to_string()];
        let mut params: Vec<String> = vec![now];  // slot 1 = updated_at

        if let Some(v) = update.title        { set_clauses.push("title = ?".into());       params.push(v); }
        if let Some(v) = update.link         { set_clauses.push("link = ?".into());        params.push(v); }
        if let Some(v) = update.description  { set_clauses.push("description = ?".into()); params.push(v); }
        if let Some(v) = update.content      { set_clauses.push("content = ?".into());     params.push(v); }
        if let Some(v) = update.author       { set_clauses.push("author = ?".into());      params.push(v); }
        if let Some(v) = update.published_at { set_clauses.push("published_at = ?".into()); params.push(v.to_rfc3339()); }
        if let Some(v) = update.is_read      { set_clauses.push("is_read = ?".into());     params.push(if v { "1" } else { "0" }.into()); }
        if let Some(v) = update.is_bookmarked { set_clauses.push("is_bookmarked = ?".into()); params.push(if v { "1" } else { "0" }.into()); }

        let query = format!("UPDATE articles SET {} WHERE id = ?", set_clauses.join(", "));
        params.push(id.to_string());  // last slot = WHERE id

        let mut q = sqlx::query(&query);
        for param in params {
            q = q.bind(param);
        }
        q.execute(&self.pool).await?;

        self.get_article_by_id(id).await
    }

    pub async fn get_bookmarked_articles(&self) -> Result<Vec<Article>, DatabaseError> {
        let articles = sqlx::query_as::<_, Article>(
            "SELECT * FROM articles WHERE is_bookmarked = 1 ORDER BY published_at DESC, created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(articles)
    }

    pub async fn mark_all_read(&self, feed_id: Option<&str>) -> Result<(), DatabaseError> {
        match feed_id {
            Some(id) => {
                sqlx::query("UPDATE articles SET is_read = 1 WHERE feed_id = ? AND is_read = 0")
                    .bind(id)
                    .execute(&self.pool)
                    .await?;
            }
            None => {
                sqlx::query("UPDATE articles SET is_read = 1 WHERE is_read = 0")
                    .execute(&self.pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn get_unread_articles(&self) -> Result<Vec<Article>, DatabaseError> {
        let articles = sqlx::query_as::<_, Article>(
            "SELECT * FROM articles WHERE is_read = 0 ORDER BY published_at DESC, created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(articles)
    }

    pub async fn get_unread_count(&self, feed_id: Option<String>) -> Result<i64, DatabaseError> {
        let mut query = String::from("SELECT COUNT(*) as count FROM articles WHERE is_read = 0");
        let mut params = vec![];
        
        if let Some(feed_id) = feed_id {
            query.push_str(" AND feed_id = ?");
            params.push(feed_id);
        }
        
        let mut query_builder = sqlx::query_scalar(&query);
        for param in params {
            query_builder = query_builder.bind(param);
        }
        
        let count: Option<i64> = query_builder.fetch_one(&self.pool).await?;
        Ok(count.unwrap_or(0))
    }

    // ── Filter rules ─────────────────────────────────────────────────────────

    pub async fn get_filter_rules(&self) -> Result<Vec<FilterRule>, DatabaseError> {
        let rules = sqlx::query_as::<_, FilterRule>(
            "SELECT * FROM filter_rules ORDER BY created_at ASC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rules)
    }

    pub async fn add_filter_rule(&self, rule: NewFilterRule) -> Result<FilterRule, DatabaseError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let rule = FilterRule {
            id: id.clone(),
            pattern: rule.pattern,
            field: rule.field,
            enabled: true,
            created_at: now,
        };

        sqlx::query(
            "INSERT INTO filter_rules (id, pattern, field, enabled, created_at) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(&rule.id)
        .bind(&rule.pattern)
        .bind(&rule.field)
        .bind(rule.enabled)
        .bind(&rule.created_at)
        .execute(&self.pool)
        .await?;

        Ok(rule)
    }

    pub async fn update_filter_rule_enabled(&self, id: &str, enabled: bool) -> Result<(), DatabaseError> {
        sqlx::query("UPDATE filter_rules SET enabled = ? WHERE id = ?")
            .bind(enabled)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_filter_rule(&self, id: &str) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM filter_rules WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── App settings ─────────────────────────────────────────────────────────

    pub async fn get_setting(&self, key: &str) -> Result<Option<String>, DatabaseError> {
        let row = sqlx::query_as::<_, AppSetting>(
            "SELECT key, value FROM app_settings WHERE key = ?"
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.value))
    }

    pub async fn set_setting(&self, key: &str, value: &str) -> Result<(), DatabaseError> {
        sqlx::query(
            "INSERT INTO app_settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value"
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}