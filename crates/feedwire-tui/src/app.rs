//! TUI application state and user actions.

use crate::html::html_to_text;
use chrono::{DateTime, Local, Utc};
use feedwire_core::models::{Article, Feed};
use feedwire_core::refresh::SharedDatabase;
use std::collections::HashMap;

/// Smart views shown above the per-feed list in the sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    All,
    Unread,
    Today,
    Starred,
}

impl View {
    pub fn title(self) -> &'static str {
        match self {
            View::All => "All Articles",
            View::Unread => "Unread",
            View::Today => "Today",
            View::Starred => "Bookmarks",
        }
    }
}

/// Which pane currently owns j/k navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Feeds,
    Articles,
    Content,
}

/// One row in the sidebar: either a smart view or a single feed.
#[derive(Debug, Clone)]
pub enum SidebarEntry {
    View(View),
    Feed(Feed),
}

/// A completed or in-flight refresh-all operation.
#[derive(Debug, Default)]
pub struct RefreshState {
    pub done: usize,
    pub total: usize,
    pub last_feed: String,
    pub active: bool,
}

pub struct App {
    pub db: SharedDatabase,
    pub entries: Vec<SidebarEntry>,
    pub unread_counts: HashMap<String, i64>,
    pub unread_total: i64,
    pub bookmark_total: i64,
    pub today_total: usize,
    pub articles: Vec<Article>,
    pub selected_entry: usize,
    pub selected_article: usize,
    pub focus: Focus,
    pub content_scroll: u16,
    pub content_lines: Vec<String>,
    pub content_source: Option<String>,
    pub status: String,
    pub refresh: RefreshState,
    pub last_data_version: Option<i64>,
    pub show_help: bool,
    pub quit: bool,
}

/// Articles fetched per view/feed.
const ARTICLE_LIMIT: i64 = 500;

impl App {
    pub async fn new(db: SharedDatabase) -> Result<Self, String> {
        let mut app = App {
            db,
            entries: Vec::new(),
            unread_counts: HashMap::new(),
            unread_total: 0,
            bookmark_total: 0,
            today_total: 0,
            articles: Vec::new(),
            selected_entry: 0,
            selected_article: 0,
            focus: Focus::Feeds,
            content_scroll: 0,
            content_lines: Vec::new(),
            content_source: None,
            status: String::new(),
            refresh: RefreshState::default(),
            last_data_version: None,
            show_help: false,
            quit: false,
        };
        app.reload().await?;
        Ok(app)
    }

    /// Reload everything that the current selection depends on.
    /// Called on startup, after actions, and when data_version changes.
    pub async fn reload(&mut self) -> Result<(), String> {
        let db = self.db.clone();
        let guard = db.lock().await;

        let feeds = guard.get_feeds().await.map_err(|e| e.to_string())?;
        let unread_counts = guard.get_unread_counts().await.map_err(|e| e.to_string())?;
        let unread_total = guard.get_unread_count(None).await.map_err(|e| e.to_string())?;
        let bookmark_total = guard.get_bookmark_count().await.map_err(|e| e.to_string())?;
        let today_total = guard.get_today_articles().await.map_err(|e| e.to_string())?.len();
        let version = guard.data_version().await.map_err(|e| e.to_string())?;
        let view_articles = match self.current_entry() {
            Some(SidebarEntry::Feed(feed)) => {
                guard
                    .get_articles(Some(feed.id.clone()), Some(ARTICLE_LIMIT), None)
                    .await
                    .map_err(|e| e.to_string())?
            }
            Some(SidebarEntry::View(View::All)) => {
                guard
                    .get_articles(None, Some(ARTICLE_LIMIT), None)
                    .await
                    .map_err(|e| e.to_string())?
            }
            Some(SidebarEntry::View(View::Unread)) => {
                guard.get_unread_articles().await.map_err(|e| e.to_string())?
            }
            Some(SidebarEntry::View(View::Today)) => {
                guard.get_today_articles().await.map_err(|e| e.to_string())?
            }
            Some(SidebarEntry::View(View::Starred)) => {
                guard
                    .get_bookmarked_articles(None)
                    .await
                    .map_err(|e| e.to_string())?
            }
            None => Vec::new(),
        };
        drop(guard);

        self.entries = std::iter::once(SidebarEntry::View(View::All))
            .chain([SidebarEntry::View(View::Unread), SidebarEntry::View(View::Today), SidebarEntry::View(View::Starred)])
            .chain(feeds.into_iter().map(SidebarEntry::Feed))
            .collect();
        self.unread_counts = unread_counts;
        self.unread_total = unread_total;
        self.bookmark_total = bookmark_total;
        self.today_total = today_total;
        self.last_data_version = Some(version);

        self.articles = view_articles;
        self.selected_article = self.selected_article.min(self.articles.len().saturating_sub(1));
        if self.selected_entry >= self.entries.len() {
            self.selected_entry = 0;
        }

        // Re-render the content pane when a different article is now selected.
        let selected_id = self.articles.get(self.selected_article).map(|a| a.id.clone());
        let displayed = self.content_source.clone();
        if selected_id.is_some() && selected_id != displayed {
            let article = self.articles.get(self.selected_article).cloned();
            if let Some(article) = article {
                self.render_article_content(&article);
            }
        }

        Ok(())
    }

    /// Check whether another process (the GUI app) changed the database.
    pub async fn poll_external_changes(&mut self) -> Result<bool, String> {
        let version = {
            let guard = self.db.lock().await;
            guard.data_version().await.map_err(|e| e.to_string())?
        };
        if self.last_data_version != Some(version) {
            self.reload().await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn current_entry(&self) -> Option<&SidebarEntry> {
        self.entries.get(self.selected_entry)
    }

    pub fn selected_article_ref(&self) -> Option<&Article> {
        self.articles.get(self.selected_article)
    }

    /// Sidebar selection changed: refetch articles and reset the cursor.
    /// The content pane re-renders via `reload`.
    pub async fn select_entry(&mut self, index: usize) -> Result<(), String> {
        if index >= self.entries.len() {
            return Ok(());
        }
        self.selected_entry = index;
        self.selected_article = 0;
        self.content_scroll = 0;
        self.reload().await?;
        Ok(())
    }

    /// Render the content pane for the article under the cursor without
    /// marking it read. Called as the user moves through the list.
    pub fn preview_selected_content(&mut self) {
        let article = self.articles.get(self.selected_article).cloned();
        if let Some(article) = article {
            self.render_article_content(&article);
            self.content_scroll = 0;
        }
    }

    pub fn move_selection(&mut self, delta: i64) {
        let len = match self.focus {
            Focus::Feeds => self.entries.len(),
            Focus::Articles => self.articles.len(),
            Focus::Content => return,
        };
        if len == 0 {
            return;
        }
        let current = match self.focus {
            Focus::Feeds => &mut self.selected_entry,
            Focus::Articles => &mut self.selected_article,
            Focus::Content => return,
        };
        let new = (*current as i64 + delta).rem_euclid(len as i64) as usize;
        *current = new;

        // Selecting an entry in the sidebar swaps the article list.
        if self.focus == Focus::Feeds {
            self.content_scroll = 0;
            self.content_lines.clear();
            self.content_source = None;
        }
        if self.focus == Focus::Articles {
            self.content_scroll = 0;
        }
    }

    pub fn cycle_focus(&mut self, backwards: bool) {
        self.focus = match (self.focus, backwards) {
            (Focus::Feeds, false) => Focus::Articles,
            (Focus::Articles, false) => Focus::Content,
            (Focus::Content, false) => Focus::Feeds,
            (Focus::Articles, true) => Focus::Feeds,
            (Focus::Content, true) => Focus::Articles,
            (Focus::Feeds, true) => Focus::Content,
        };
    }

    /// Open the selected article: render its body, switch focus to the
    /// content pane, and mark it read.
    pub async fn open_article(&mut self) -> Result<(), String> {
        let Some(article) = self.articles.get(self.selected_article).cloned() else {
            return Ok(());
        };
        let id = article.id.clone();
        let was_read = article.is_read;
        self.render_article_content(&article);
        if !was_read {
            self.set_read(&id, true).await?;
        }
        self.focus = Focus::Content;
        self.content_scroll = 0;
        Ok(())
    }

    /// Build the plain-text body for the content pane.
    fn render_article_content(&mut self, article: &Article) {
        let body_html = article
            .content
            .as_deref()
            .filter(|c| !c.trim().is_empty())
            .or(article.description.as_deref())
            .unwrap_or("");
        let body = html_to_text(body_html);
        let mut lines = Vec::new();
        if let Some(author) = &article.author {
            lines.push(format!("By {author}"));
        }
        if let Some(published) = &article.published_at {
            lines.push(time_label_full(*published));
        }
        if let Some(link) = &article.link {
            lines.push(link.clone());
        }
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.extend(body.lines().map(str::to_string));
        self.content_lines = lines;
        self.content_source = Some(article.id.clone());
    }

    pub async fn set_read(&mut self, article_id: &str, read: bool) -> Result<(), String> {
        use feedwire_core::models::ArticleUpdate;
        let update = ArticleUpdate {
            is_read: Some(read),
            ..Default::default()
        };
        {
            let guard = self.db.lock().await;
            guard
                .update_article(article_id, update)
                .await
                .map_err(|e| e.to_string())?;
        }
        if let Some(article) = self
            .articles
            .iter_mut()
            .find(|a| a.id == article_id)
        {
            article.is_read = read;
        }
        self.reload().await?;
        Ok(())
    }

    pub async fn toggle_read(&mut self) -> Result<(), String> {
        let Some(article) = self.selected_article_ref() else {
            return Ok(());
        };
        let (id, target) = (article.id.clone(), !article.is_read);
        self.set_read(&id, target).await?;
        self.status = if target {
            "Marked read".into()
        } else {
            "Marked unread".into()
        };
        Ok(())
    }

    pub async fn toggle_bookmark(&mut self) -> Result<(), String> {
        let Some(article) = self.selected_article_ref() else {
            return Ok(());
        };
        let (id, target) = (article.id.clone(), !article.is_bookmarked);
        let result = {
            let guard = self.db.lock().await;
            guard
                .update_article(
                    &id,
                    feedwire_core::models::ArticleUpdate {
                        is_bookmarked: Some(target),
                        ..Default::default()
                    },
                )
                .await
        };
        result.map_err(|e| e.to_string())?;
        self.reload().await?;
        self.status = if target {
            "Starred".into()
        } else {
            "Unstarred".into()
        };
        Ok(())
    }

    pub async fn mark_all_view_read(&mut self) -> Result<(), String> {
        let action = |app: &App| -> Option<(String, Option<String>)> {
            match app.current_entry() {
                Some(SidebarEntry::Feed(feed)) => Some(("feed".into(), Some(feed.id.clone()))),
                Some(SidebarEntry::View(_)) => Some(("all".into(), None)),
                None => None,
            }
        };
        let Some((scope, feed_id)) = action(self) else {
            return Ok(());
        };
        {
            let guard = self.db.lock().await;
            guard.mark_all_read(feed_id.as_deref()).await.map_err(|e| e.to_string())?;
        }
        self.reload().await?;
        self.status = if scope == "feed" {
            "Feed marked read".into()
        } else {
            "All visible articles marked read".into()
        };
        Ok(())
    }

    pub async fn open_in_browser(&mut self) -> Result<(), String> {
        let Some(link) = self.selected_article_ref().and_then(|a| a.link.clone()) else {
            self.status = "Article has no link".into();
            return Ok(());
        };
        std::process::Command::new("open")
            .arg(&link)
            .spawn()
            .map_err(|e| format!("Failed to open link: {e}"))?;
        self.status = format!("Opened {}", truncate(&link, 40));
        Ok(())
    }

    pub fn scroll_content(&mut self, delta: i64) {
        if delta < 0 {
            self.content_scroll = self
                .content_scroll
                .saturating_sub(delta.unsigned_abs() as u16);
        } else {
            self.content_scroll = self.content_scroll.saturating_add(delta as u16);
        }
    }
}

/// "14:32" today, "Mar 12" otherwise, "" for unknown dates.
pub fn time_label(published: Option<DateTime<Utc>>) -> String {
    let Some(published) = published else {
        return "no date".into();
    };
    let local = published.with_timezone(&Local);
    let today = Local::now().date_naive();
    if local.date_naive() == today {
        local.format("%H:%M").to_string()
    } else {
        local.format("%b %d").to_string()
    }
}

/// "Wed, 12 Mar 2025 14:32" — used in the article header.
pub fn time_label_full(published: DateTime<Utc>) -> String {
    published
        .with_timezone(&Local)
        .format("%a, %d %b %Y %H:%M")
        .to_string()
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short_strings_untouched() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long_strings_ellipsized() {
        let out = truncate("abcdefghij", 5);
        assert_eq!(out.chars().count(), 5);
        assert!(out.ends_with('…'));
    }

    #[test]
    fn time_label_unknown_date() {
        assert_eq!(time_label(None), "no date");
    }

    #[tokio::test]
    async fn focus_cycles_forward() {
        let db = feedwire_core::Database::with_url("sqlite::memory:")
            .await
            .expect("in-memory DB failed");
        let mut app = App {
            db: std::sync::Arc::new(tokio::sync::Mutex::new(db)),
            entries: vec![],
            unread_counts: HashMap::new(),
            unread_total: 0,
            bookmark_total: 0,
            today_total: 0,
            articles: vec![],
            selected_entry: 0,
            selected_article: 0,
            focus: Focus::Feeds,
            content_scroll: 0,
            content_lines: vec![],
            content_source: None,
            status: String::new(),
            refresh: RefreshState::default(),
            last_data_version: None,
            show_help: false,
            quit: false,
        };
        app.cycle_focus(false);
        assert_eq!(app.focus, Focus::Articles);
        app.cycle_focus(false);
        assert_eq!(app.focus, Focus::Content);
        app.cycle_focus(false);
        assert_eq!(app.focus, Focus::Feeds);
        app.cycle_focus(true);
        assert_eq!(app.focus, Focus::Content);
    }
}
