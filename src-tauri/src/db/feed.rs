use feed_rs::parser;
use reqwest::Client;
use url::Url;
use crate::db::models::{NewFeed, NewArticle};
use crate::db::DatabaseError;
use chrono::{DateTime, Utc, Timelike};

/// Derive an icon URL for a feed.
/// Priority: feed logo/icon field → Google favicon service for the feed's domain.
fn derive_icon_url(parsed_feed: &feed_rs::model::Feed, feed_url: &str) -> Option<String> {
    // 1. Use the logo or icon declared in the feed itself (often a high-res channel art or avatar)
    if let Some(logo) = &parsed_feed.logo {
        if !logo.uri.is_empty() {
            return Some(logo.uri.clone());
        }
    }
    if let Some(icon) = &parsed_feed.icon {
        if !icon.uri.is_empty() {
            return Some(icon.uri.clone());
        }
    }

    // 2. Fall back to Google's favicon service for the feed's domain
    if let Ok(parsed_url) = Url::parse(feed_url) {
        if let Some(domain) = parsed_url.domain() {
            return Some(format!(
                "https://www.google.com/s2/favicons?domain={}&sz=64",
                domain
            ));
        }
    }

    None
}

#[derive(Debug, thiserror::Error)]
pub enum FeedError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),
}

pub struct FeedParser {
    client: Client,
}

impl FeedParser {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("FeedWire/0.1.0 (RSS Reader)")
                .gzip(true)
                .build()
                .unwrap_or_default(),
        }
    }

    pub async fn discover_feed(&self, url: &str) -> Result<NewFeed, FeedError> {
        let response = self.client.get(url).send().await?;
        let content = response.text().await?;

        let parsed_feed = parser::parse(content.as_bytes())
            .map_err(|e| FeedError::Parse(e.to_string()))?;

        let feed_type = match parsed_feed.feed_type {
            feed_rs::model::FeedType::Atom => "atom",
            feed_rs::model::FeedType::JSON => "json",
            feed_rs::model::FeedType::RSS0 | feed_rs::model::FeedType::RSS1 | feed_rs::model::FeedType::RSS2 => "rss",
        };

        let icon_url = derive_icon_url(&parsed_feed, url);

        Ok(NewFeed {
            title: parsed_feed.title.map(|t| t.content).unwrap_or_else(|| "Untitled Feed".to_string()),
            url: url.to_string(),
            description: parsed_feed.description.map(|d| d.content),
            feed_type: feed_type.to_string(),
            icon_url,
        })
    }

    pub async fn fetch_feed(&self, url: &str) -> Result<(NewFeed, Vec<NewArticle>), FeedError> {
        let response = self.client.get(url).send().await?;
        let content = response.text().await?;

        let parsed_feed = parser::parse(content.as_bytes())
            .map_err(|e| FeedError::Parse(e.to_string()))?;

        let feed_type = match parsed_feed.feed_type {
            feed_rs::model::FeedType::Atom => "atom",
            feed_rs::model::FeedType::JSON => "json",
            feed_rs::model::FeedType::RSS0 | feed_rs::model::FeedType::RSS1 | feed_rs::model::FeedType::RSS2 => "rss",
        };

        let icon_url = derive_icon_url(&parsed_feed, url);

        let new_feed = NewFeed {
            title: parsed_feed.title.map(|t| t.content).unwrap_or_else(|| "Untitled Feed".to_string()),
            url: url.to_string(),
            description: parsed_feed.description.map(|d| d.content),
            feed_type: feed_type.to_string(),
            icon_url,
        };

        let mut articles = Vec::new();
        for entry in parsed_feed.entries {
            let published_at = entry.published.or(entry.updated).map(|dt| {
                DateTime::<Utc>::from_timestamp(dt.timestamp(), dt.nanosecond() as u32)
                    .unwrap_or_else(|| Utc::now())
            });

            let content = entry.content.and_then(|c| {
                if let Some(body) = c.body {
                    if !body.is_empty() {
                        Some(body)
                    } else if let Some(src) = c.src {
                        Some(src.href)
                    } else {
                        None
                    }
                } else if let Some(src) = c.src {
                    Some(src.href)
                } else {
                    None
                }
            });

            let description = entry.summary.map(|s| s.content);

            let article = NewArticle {
                feed_id: String::new(), // Will be set by the caller
                title: entry.title.map(|t| t.content).unwrap_or_else(|| "Untitled Article".to_string()),
                link: entry.links.first().map(|l| l.href.clone()),
                description,
                content,
                author: entry.authors.first().map(|a| a.name.clone()),
                published_at,
                guid: Some(entry.id),
            };

            articles.push(article);
        }

        Ok((new_feed, articles))
    }

    pub async fn is_youtube_channel(&self, url: &str) -> bool {
        if let Ok(parsed_url) = Url::parse(url) {
            if let Some(domain) = parsed_url.domain() {
                return domain.contains("youtube.com") || domain.contains("youtu.be");
            }
        }
        false
    }

    pub async fn convert_youtube_to_rss(&self, url: &str) -> Result<String, FeedError> {
        let parsed_url = Url::parse(url)?;

        if let Some(domain) = parsed_url.domain() {
            if domain.contains("youtube.com") {
                // Already an RSS feed URL — return as-is.
                // Handles: /feeds/videos.xml?channel_id=... and /feeds/videos.xml?user=...
                if parsed_url.path().starts_with("/feeds/") {
                    println!("URL is already a YouTube RSS feed: {}", url);
                    return Ok(url.to_string());
                }

                if let Some(path) = parsed_url.path_segments() {
                    let path_segments: Vec<_> = path.collect();

                    for (i, segment) in path_segments.iter().enumerate() {
                        if *segment == "channel" {
                            if let Some(channel_id) = path_segments.get(i + 1) {
                                let rss_url = format!("https://www.youtube.com/feeds/videos.xml?channel_id={}", channel_id);
                                println!("Converted YouTube channel URL to RSS: {}", rss_url);
                                return Ok(rss_url);
                            }
                        } else if segment.starts_with("@") {
                            let username = segment.trim_start_matches('@');
                            return self.resolve_youtube_custom_url(username).await;
                        } else if *segment == "c" {
                            if let Some(channel_name) = path_segments.get(i + 1) {
                                return self.resolve_youtube_custom_url(channel_name).await;
                            }
                        }
                    }
                }

                // Try extracting a channel_id from query params directly
                // e.g. a bare https://www.youtube.com?channel_id=UCxxxx URL
                for (key, val) in parsed_url.query_pairs() {
                    if key == "channel_id" && !val.is_empty() {
                        let rss_url = format!("https://www.youtube.com/feeds/videos.xml?channel_id={}", val);
                        println!("Converted YouTube channel_id query param to RSS: {}", rss_url);
                        return Ok(rss_url);
                    }
                }
            }
        }

        Err(FeedError::Parse("Not a valid YouTube channel URL. Please use the format: https://www.youtube.com/channel/CHANNEL_ID".to_string()))
    }

    async fn resolve_youtube_custom_url(&self, custom_name: &str) -> Result<String, FeedError> {
        println!("Attempting to resolve YouTube custom URL: @{}", custom_name);

        // Method 1: Try the ?user= parameter (works for some older channels)
        let user_rss = format!("https://www.youtube.com/feeds/videos.xml?user={}", custom_name);
        if let Ok(resp) = self.client.get(&user_rss).send().await {
            if resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                if body.contains("<feed") || body.contains("<rss") {
                    println!("Resolved via ?user= parameter");
                    return Ok(user_rss);
                }
            }
        }

        // Method 2: Fetch the channel page and extract the channel ID from the HTML.
        // The gzip feature on reqwest ensures we can decompress YouTube's response.
        let channel_url = format!("https://www.youtube.com/@{}", custom_name);
        println!("Fetching channel page: {}", channel_url);

        let response = self.client
            .get(&channel_url)
            .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .header("Accept-Language", "en-US,en;q=0.9")
            .send()
            .await?;

        let html = response.text().await?;
        println!("Got {} bytes from channel page", html.len());

        // Channel IDs are always "UC" followed by exactly 22 base64url characters (24 chars total)
        let re = regex::Regex::new(r"UC[0-9A-Za-z_-]{22}").unwrap();
        if let Some(m) = re.find(&html) {
            let channel_id = m.as_str();
            let rss_url = format!("https://www.youtube.com/feeds/videos.xml?channel_id={}", channel_id);
            println!("Found channel ID: {} -> {}", channel_id, rss_url);
            return Ok(rss_url);
        }

        Err(FeedError::Parse(format!(
            "Could not resolve YouTube channel @{}. Try using the direct channel URL: https://www.youtube.com/channel/CHANNEL_ID",
            custom_name
        )))
    }

    pub fn validate_feed_url(url: &str) -> Result<(), FeedError> {
        let parsed_url = Url::parse(url)?;

        if !matches!(parsed_url.scheme(), "http" | "https") {
            return Err(FeedError::Parse("URL must use HTTP or HTTPS scheme".to_string()));
        }

        Ok(())
    }
}

impl Default for FeedParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── is_youtube_channel ──────────────────────────────────────────────────

    #[tokio::test]
    async fn youtube_channel_url_detected() {
        let parser = FeedParser::new();
        assert!(parser.is_youtube_channel("https://www.youtube.com/channel/UC1234").await);
        assert!(parser.is_youtube_channel("https://www.youtube.com/@SomeHandle").await);
        assert!(parser.is_youtube_channel("https://youtu.be/abc123").await);
        assert!(parser.is_youtube_channel("https://www.youtube.com/feeds/videos.xml?channel_id=UC123").await);
    }

    #[tokio::test]
    async fn non_youtube_url_not_detected() {
        let parser = FeedParser::new();
        assert!(!parser.is_youtube_channel("https://news.ycombinator.com/rss").await);
        assert!(!parser.is_youtube_channel("https://example.com/feed.xml").await);
    }

    // ── convert_youtube_to_rss ─────────────────────────────────────────────

    #[tokio::test]
    async fn already_rss_url_passthrough() {
        let parser = FeedParser::new();
        let url = "https://www.youtube.com/feeds/videos.xml?channel_id=UCxxxxxxxxxxxxxxxxxxxxxx";
        let result = parser.convert_youtube_to_rss(url).await.unwrap();
        assert_eq!(result, url);
    }

    #[tokio::test]
    async fn channel_path_converted() {
        let parser = FeedParser::new();
        let url = "https://www.youtube.com/channel/UCxxxxxxxxxxxxxxxxxxxxxx";
        let result = parser.convert_youtube_to_rss(url).await.unwrap();
        assert_eq!(
            result,
            "https://www.youtube.com/feeds/videos.xml?channel_id=UCxxxxxxxxxxxxxxxxxxxxxx"
        );
    }

    #[tokio::test]
    async fn invalid_url_returns_error() {
        let parser = FeedParser::new();
        let result = parser.convert_youtube_to_rss("not-a-url").await;
        assert!(result.is_err());
    }

    // ── validate_feed_url ──────────────────────────────────────────────────

    #[test]
    fn valid_http_url_accepted() {
        assert!(FeedParser::validate_feed_url("https://example.com/feed.xml").is_ok());
        assert!(FeedParser::validate_feed_url("http://example.com/rss").is_ok());
    }

    #[test]
    fn non_http_scheme_rejected() {
        assert!(FeedParser::validate_feed_url("ftp://example.com/feed").is_err());
        assert!(FeedParser::validate_feed_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn invalid_url_rejected() {
        assert!(FeedParser::validate_feed_url("not-a-url").is_err());
        assert!(FeedParser::validate_feed_url("").is_err());
    }
}
