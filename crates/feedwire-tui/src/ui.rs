//! Terminal rendering for FeedWire TUI.

use crate::app::{time_label, truncate, App, Focus, SidebarEntry, View};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, List, ListItem, ListState, Padding, Paragraph, Wrap};
use ratatui::Frame;

/// ASCII title, shown when the terminal is large enough (width >= logo
/// width, height >= 24). Falls back to the plain layout otherwise.
const LOGO: &[&str] = &[
    r#" ______   ______     ______     _____     __     __     __     ______     ______    "#,
    r#"/\  ___\ /\  ___\   /\  ___\   /\  __-.  /\ \  _ \ \   /\ \   /\  == \   /\  ___\ "#,
    r#"\ \  __\ \ \  __\   \ \  __\   \ \ \/\ \ \ \ \/ ".\ \  \ \ \  \ \  __<   \ \  __\ "#,
    r#" \ \_\    \ \_____\  \ \_____\  \ \____-  \ \__/".~\_\  \ \_\  \ \_\ \_\  \ \_____\"#,
    r#"  \/_/     \/_____/   \/_____/   \/____/   \/_/   \/_/   \/_/   \/_/ /_/   \/_____/ "#,
];

pub fn draw(f: &mut Frame, app: &mut App) {
    // One blank row of margin above and below the logo.
    let logo_height = if f.area().width >= logo_width() && f.area().height >= 24 {
        LOGO.len() as u16 + 2
    } else {
        0
    };
    let [logo_area, body, status] = Layout::vertical([
        Constraint::Length(logo_height),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(f.area());
    if logo_height > 0 {
        draw_logo(f, logo_area);
    }

    let [sidebar_area, list_area, content_area] = Layout::horizontal([
        Constraint::Length(36),
        Constraint::Percentage(42),
        Constraint::Min(30),
    ])
    .areas(body);

    draw_sidebar(f, app, sidebar_area);
    draw_article_list(f, app, list_area);
    draw_content(f, app, content_area);
    draw_status(f, app, status);

    if app.show_help {
        draw_help(f, f.area());
    }
}

fn logo_width() -> u16 {
    LOGO.iter()
        .map(|l| l.trim_end().chars().count())
        .max()
        .unwrap_or(0) as u16
}

fn draw_logo(f: &mut Frame, area: Rect) {
    let lines: Vec<Line> = std::iter::once(Line::from(""))
        .chain(
            LOGO.iter()
                .map(|l| Line::from(l.trim_end().to_string()).fg(Color::Cyan)),
        )
        .chain(std::iter::once(Line::from("")))
        .collect();
    let logo = Paragraph::new(lines);
    f.render_widget(logo, area);
}

fn focus_border<'a>(focused: bool) -> Block<'a> {
    let style = if focused {
        Style::new().fg(Color::Cyan)
    } else {
        Style::new().fg(Color::DarkGray)
    };
    Block::bordered().border_style(style).padding(Padding::horizontal(2))
}

fn draw_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let block = focus_border(app.focus == Focus::Feeds).title(" Feeds ");
    let inner_width = block.inner(area).width as usize;

    let items: Vec<ListItem> = app
        .entries
        .iter()
        .map(|entry| {
            let (label, count) = match entry {
                SidebarEntry::View(view) => (
                    view.title().to_string(),
                    match view {
                        View::All => None,
                        View::Unread => Some(app.unread_total),
                        View::Today => Some(app.today_total as i64),
                        View::Starred => Some(app.bookmark_total),
                    },
                ),
                SidebarEntry::Feed(feed) => (
                    feed.title.clone(),
                    app.unread_counts.get(&feed.id).copied(),
                ),
            };
            ListItem::new(sidebar_row(&label, count, inner_width))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.selected_entry));

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::new().reversed());
    f.render_stateful_widget(list, area, &mut state);
}

/// "Title    (12)" — title truncated to fit, count in a fixed right slot.
fn sidebar_row(title: &str, count: Option<i64>, width: usize) -> Line<'static> {
    let count_str = match count {
        Some(c) if c > 0 => format!("({c})"),
        _ => String::new(),
    };
    let count_width = 5usize;
    let title_slot = width.saturating_sub(count_width).max(1);
    let label = truncate(title, title_slot);
    format!("{label:<title_slot$}{count_str:>count_width$}").into()
}

fn draw_article_list(f: &mut Frame, app: &App, area: Rect) {
    let title = current_view_title(app);
    let block = focus_border(app.focus == Focus::Articles).title(format!(" {title} "));
    let inner_width = block.inner(area).width as usize;

    let items: Vec<ListItem> = app
        .articles
        .iter()
        .map(|article| {
            let star = if article.is_bookmarked { "★" } else { " " };
            let time = format!("{:>5} ", time_label(article.published_at));
            let title = truncate(&article.title, inner_width.saturating_sub(9));
            let mut spans = vec![
                Span::styled(format!("{star} "), Style::new().fg(Color::Yellow)),
                Span::styled(time, Style::new().fg(Color::DarkGray)),
            ];
            let style = if article.is_read {
                Style::new().fg(Color::DarkGray)
            } else {
                Style::new().fg(Color::White).add_modifier(Modifier::BOLD)
            };
            spans.push(Span::styled(format!(" {title}"), style));
            ListItem::new(Line::from(spans))
        })
        .collect();

    let mut state = ListState::default();
    state.select(if app.articles.is_empty() { None } else { Some(app.selected_article) });

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::new().reversed());
    f.render_stateful_widget(list, area, &mut state);
}

fn current_view_title(app: &App) -> String {
    match app.current_entry() {
        Some(SidebarEntry::View(view)) => view.title().to_string(),
        Some(SidebarEntry::Feed(feed)) => truncate(&feed.title, 24),
        None => "Articles".to_string(),
    }
}

fn draw_content(f: &mut Frame, app: &App, area: Rect) {
    // The reading pane gets extra vertical padding for comfortable reading.
    let block = focus_border(app.focus == Focus::Content).padding(Padding::new(2, 1, 2, 1));
    let inner = block.inner(area);

    let paragraph = match app.selected_article_ref() {
        Some(article) if app.content_source.as_deref() == Some(article.id.as_str()) => {
            let mut lines: Vec<Line> = Vec::with_capacity(app.content_lines.len());
            lines.push(Line::from(article.title.clone()).bold().fg(Color::White));
            for line in &app.content_lines {
                let style = if article.link.as_deref() == Some(line.as_str()) {
                    Style::new().fg(Color::Cyan)
                } else {
                    Style::new().fg(Color::Gray)
                };
                lines.push(Line::from(line.clone()).style(style));
            }

            // Clamp the scroll to the estimated rendered length.
            let available = app.content_lines.len().saturating_sub(inner.height as usize) as u16;
            let scroll = app.content_scroll.min(available + 2);

            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .scroll((scroll, 0))
                .block(block)
        }
        _ => Paragraph::new("Select an article to read it").block(block),
    };
    f.render_widget(paragraph, area);
}

fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let left = if app.refresh.active {
        let width = 12usize;
        let filled = if app.refresh.total == 0 {
            0
        } else {
            width * app.refresh.done / app.refresh.total.max(app.refresh.done)
        };
        let bar = "█".repeat(filled) + &"░".repeat(width.saturating_sub(filled));
        format!(
            " Refreshing {} {} {}/{}",
            truncate(&app.refresh.last_feed, 18), bar, app.refresh.done, app.refresh.total
        )
    } else if !app.status.is_empty() {
        format!(" {}", app.status)
    } else {
        " Ready".to_string()
    };

    let unread = app.unread_total;
    let right = format!("{unread} unread · ?: help · q: quit");

    let right_width = right.chars().count() as u16;
    let [left_area, right_area] = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(right_width + 1),
    ])
    .areas(area);

    let left_max = left_area.width.saturating_sub(1) as usize;
    f.render_widget(
        Paragraph::new(Span::styled(truncate(&left, left_max), Style::new().fg(Color::Gray))),
        left_area,
    );
    f.render_widget(
        Paragraph::new(Span::styled(right, Style::new().fg(Color::DarkGray))),
        right_area,
    );
}

fn draw_help(f: &mut Frame, area: Rect) {
    let width = (area.width.min(46)).max(20);
    let height = area.height.min(18).max(8);
    let popup = centered(area, width, height);

    let hotkeys = vec![
        ("j / k", "Move up / down"),
        ("Tab / h / l", "Switch pane"),
        ("Enter", "Open article (marks read)"),
        ("b", "Toggle bookmark"),
        ("m", "Toggle read"),
        ("M", "Mark all in view read"),
        ("o", "Open link in browser"),
        ("r", "Refresh selected feed"),
        ("R", "Refresh all feeds"),
        ("1 – 4", "All / Unread / Today / Starred"),
        ("?", "Toggle this help"),
        ("q", "Quit"),
    ];
    let items: Vec<Line> = hotkeys
        .into_iter()
        .map(|(key, action)| {
            Line::from(vec![
                Span::styled(format!("{key:<12}"), Style::new().fg(Color::Cyan)),
                Span::styled(action, Style::new().fg(Color::Gray)),
            ])
        })
        .collect();

    f.render_widget(Clear, popup);
    let paragraph = Paragraph::new(items).block(
        Block::bordered()
            .title(" Help ")
            .border_style(Style::new().fg(Color::Yellow))
            .padding(Padding::new(3, 1, 3, 1)),
    );
    f.render_widget(paragraph, popup);
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect {
        x,
        y,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, Focus, RefreshState};
    use chrono::Utc;
    use feedwire_core::models::{Article, Feed};

    fn fake_feed(id: &str, title: &str) -> Feed {
        let now = Utc::now();
        Feed {
            id: id.into(),
            title: title.into(),
            url: format!("https://example.com/{id}"),
            description: None,
            feed_type: "rss".into(),
            last_fetched: None,
            created_at: now,
            updated_at: now,
            is_active: true,
            is_archived: false,
            folder_id: None,
            latest_article_at: None,
            icon_url: None,
        }
    }

    fn fake_article(id: &str, feed_id: &str, title: &str, read: bool) -> Article {
        let now = Utc::now();
        Article {
            id: id.into(),
            feed_id: feed_id.into(),
            title: title.into(),
            link: Some(format!("https://example.com/{id}")),
            description: Some("<p>The body of {title}.</p>".replace("{title}", title)),
            content: None,
            thumbnail_url: None,
            author: Some("A. Author".into()),
            published_at: Some(now),
            created_at: now,
            updated_at: now,
            is_read: read,
            is_bookmarked: false,
            guid: Some(id.into()),
        }
    }

    /// Draw the whole app into a TestBackend and return the rendered text.
    fn render(app: &mut App, width: u16, height: u16) -> String {
        let backend = ratatui::backend::TestBackend::new(width, height);
        let mut terminal = ratatui::Terminal::new(backend).expect("test terminal failed");
        terminal.draw(|f| draw(f, app)).expect("draw failed");
        let buffer = terminal.backend().buffer();
        buffer
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<Vec<_>>()
            .join("")
    }

    #[tokio::test]
    async fn draw_renders_feeds_articles_and_content() {
        let db = feedwire_core::Database::with_url("sqlite::memory:")
            .await
            .expect("in-memory DB failed");
        let mut app = App {
            db: std::sync::Arc::new(tokio::sync::Mutex::new(db)),
            entries: vec![
                SidebarEntry::View(View::All),
                SidebarEntry::View(View::Unread),
                SidebarEntry::Feed(fake_feed("f1", "Hacker News")),
            ],
            unread_counts: [("f1".to_string(), 3)].into_iter().collect(),
            unread_total: 3,
            bookmark_total: 1,
            today_total: 2,
            articles: vec![
                fake_article("a1", "f1", "First post", false),
                fake_article("a2", "f1", "Second post", true),
            ],
            selected_entry: 2,
            selected_article: 0,
            focus: Focus::Articles,
            content_scroll: 0,
            content_lines: vec!["By A. Author".into(), "The body of First post.".into()],
            content_source: Some("a1".into()),
            status: "Ready for tests".into(),
            refresh: RefreshState::default(),
            last_data_version: None,
            show_help: false,
            quit: false,
        };

        let rendered = render(&mut app, 120, 30);
        assert!(rendered.contains("______")); // ASCII logo header
        assert!(rendered.contains("Feeds"));
        assert!(rendered.contains("Hacker News"));
        assert!(rendered.contains("(3)"));
        assert!(rendered.contains("First post"));
        assert!(rendered.contains("The body of First post."));
        assert!(rendered.contains("3 unread"));
        assert!(!rendered.contains("Help")); // overlay closed

        // Small terminals must not panic; the logo is dropped instead of
        // clipping, and the help overlay still fits.
        app.show_help = true;
        let rendered = render(&mut app, 80, 20);
        assert!(!rendered.contains("______"));
        assert!(rendered.contains("Quit"));
    }
}

