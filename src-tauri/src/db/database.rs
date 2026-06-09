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
}

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new() -> Result<Self, DatabaseError> {
        let database_url = "sqlite:./feedwire.db";
        
        let connect_options = SqliteConnectOptions::from_str(database_url)?
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
            "SELECT * FROM feeds WHERE is_active = 1 ORDER BY title"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(feeds)
    }

    pub async fn get_feed_by_id(&self, id: &str) -> Result<Feed, DatabaseError> {
        let feed = sqlx::query_as::<_, Feed>(
            "SELECT * FROM feeds WHERE id = ? AND is_active = 1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        feed.ok_or(DatabaseError::FeedNotFound)
    }

    pub async fn update_feed(&self, id: &str, update: FeedUpdate) -> Result<Feed, DatabaseError> {
        let mut query = String::from("UPDATE feeds SET updated_at = ?");
        let mut params = vec![];
        
        if update.title.is_some() {
            query.push_str(", title = ?");
            params.push(update.title.unwrap());
        }
        if update.description.is_some() {
            query.push_str(", description = ?");
            params.push(update.description.unwrap());
        }
        if update.last_fetched.is_some() {
            query.push_str(", last_fetched = ?");
            params.push(update.last_fetched.unwrap().to_rfc3339());
        }
        if update.is_active.is_some() {
            query.push_str(", is_active = ?");
            params.push(if update.is_active.unwrap() { "1" } else { "0" }.to_string());
        }
        
        query.push_str(" WHERE id = ?");
        
        let now = chrono::Utc::now();
        params.push(now.to_rfc3339());
        params.push(id.to_string());
        
        let mut query_builder = sqlx::query(&query);
        for param in params {
            query_builder = query_builder.bind(param);
        }
        
        query_builder.execute(&self.pool).await?;
        
        self.get_feed_by_id(id).await
    }

    pub async fn delete_feed(&self, id: &str) -> Result<(), DatabaseError> {
        sqlx::query("UPDATE feeds SET is_active = 0 WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
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
            INSERT OR REPLACE INTO articles (id, feed_id, title, link, description, content, author, published_at, created_at, updated_at, is_read, is_bookmarked, guid)
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
        let mut query = String::from("UPDATE articles SET updated_at = ?");
        let mut params = vec![];
        
        if update.title.is_some() {
            query.push_str(", title = ?");
            params.push(update.title.unwrap());
        }
        if update.link.is_some() {
            query.push_str(", link = ?");
            params.push(update.link.unwrap());
        }
        if update.description.is_some() {
            query.push_str(", description = ?");
            params.push(update.description.unwrap());
        }
        if update.content.is_some() {
            query.push_str(", content = ?");
            params.push(update.content.unwrap());
        }
        if update.author.is_some() {
            query.push_str(", author = ?");
            params.push(update.author.unwrap());
        }
        if update.published_at.is_some() {
            query.push_str(", published_at = ?");
            params.push(update.published_at.unwrap().to_rfc3339());
        }
        if update.is_read.is_some() {
            query.push_str(", is_read = ?");
            params.push(if update.is_read.unwrap() { "1" } else { "0" }.to_string());
        }
        if update.is_bookmarked.is_some() {
            query.push_str(", is_bookmarked = ?");
            params.push(if update.is_bookmarked.unwrap() { "1" } else { "0" }.to_string());
        }
        
        query.push_str(" WHERE id = ?");
        
        let now = chrono::Utc::now();
        params.push(now.to_rfc3339());
        params.push(id.to_string());
        
        let mut query_builder = sqlx::query(&query);
        for param in params {
            query_builder = query_builder.bind(param);
        }
        
        query_builder.execute(&self.pool).await?;
        
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
}