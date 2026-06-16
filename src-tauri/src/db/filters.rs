/// Article filtering logic.
///
/// Filters are applied when articles are fetched (initial import and refresh).
/// An article is dropped if ANY enabled filter matches it.

use crate::db::models::{FilterRule, NewArticle};

/// Returns true if the article should be kept, false if it should be dropped.
pub fn should_keep(
    article: &NewArticle,
    skip_youtube_shorts: bool,
    rules: &[FilterRule],
) -> bool {
    // Built-in: skip YouTube Shorts.
    // Shorts always have a link of the form https://www.youtube.com/shorts/<id>
    // We restrict the check to youtube.com to avoid false positives on other sites.
    if skip_youtube_shorts {
        if let Some(link) = &article.link {
            let lower = link.to_lowercase();
            if lower.contains("youtube.com") && lower.contains("/shorts/") {
                return false;
            }
        }
    }

    // User-defined rules — substring match, case-insensitive.
    for rule in rules {
        if !rule.enabled {
            continue;
        }
        let pattern = rule.pattern.to_lowercase();
        let matched = match rule.field.as_str() {
            "url" => article
                .link
                .as_deref()
                .map(|l| l.to_lowercase().contains(&pattern))
                .unwrap_or(false),
            "title" => article
                .title
                .to_lowercase()
                .contains(&pattern),
            _ => false,
        };
        if matched {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn article(link: Option<&str>, title: &str) -> NewArticle {
        NewArticle {
            feed_id: "feed1".into(),
            title: title.into(),
            link: link.map(String::from),
            description: None,
            content: None,
            author: None,
            published_at: Some(Utc::now()),
            guid: None,
        }
    }

    fn rule(pattern: &str, field: &str) -> FilterRule {
        FilterRule {
            id: "r1".into(),
            pattern: pattern.into(),
            field: field.into(),
            enabled: true,
            created_at: Utc::now(),
        }
    }

    // ── YouTube Shorts built-in ────────────────────────────────────────────

    #[test]
    fn shorts_dropped_when_toggle_on() {
        let a = article(Some("https://www.youtube.com/shorts/abc123"), "A Short");
        assert!(!should_keep(&a, true, &[]));
    }

    #[test]
    fn shorts_kept_when_toggle_off() {
        let a = article(Some("https://www.youtube.com/shorts/abc123"), "A Short");
        assert!(should_keep(&a, false, &[]));
    }

    #[test]
    fn regular_youtube_video_kept() {
        let a = article(Some("https://www.youtube.com/watch?v=abc123"), "A Video");
        assert!(should_keep(&a, true, &[]));
    }

    #[test]
    fn non_youtube_url_with_shorts_kept() {
        // "shorts" in a non-youtube URL should NOT be filtered by the toggle
        let a = article(Some("https://example.com/shorts/foo"), "A post");
        assert!(should_keep(&a, true, &[]));
    }

    // ── User-defined URL rules ─────────────────────────────────────────────

    #[test]
    fn url_rule_drops_matching_article() {
        let a = article(Some("https://example.com/clips/abc"), "A Clip");
        let r = rule("/clips/", "url");
        assert!(!should_keep(&a, false, &[r]));
    }

    #[test]
    fn url_rule_case_insensitive() {
        let a = article(Some("https://example.com/CLIPS/abc"), "A Clip");
        let r = rule("/clips/", "url");
        assert!(!should_keep(&a, false, &[r]));
    }

    #[test]
    fn url_rule_no_match_keeps_article() {
        let a = article(Some("https://example.com/posts/123"), "A Post");
        let r = rule("/clips/", "url");
        assert!(should_keep(&a, false, &[r]));
    }

    #[test]
    fn disabled_rule_does_not_filter() {
        let a = article(Some("https://example.com/clips/abc"), "A Clip");
        let mut r = rule("/clips/", "url");
        r.enabled = false;
        assert!(should_keep(&a, false, &[r]));
    }

    // ── Title rules ────────────────────────────────────────────────────────

    #[test]
    fn title_rule_drops_matching_article() {
        let a = article(Some("https://example.com/post"), "#shorts");
        let r = rule("#shorts", "title");
        assert!(!should_keep(&a, false, &[r]));
    }

    #[test]
    fn title_rule_no_match_keeps_article() {
        let a = article(Some("https://example.com/post"), "A normal title");
        let r = rule("#shorts", "title");
        assert!(should_keep(&a, false, &[r]));
    }
}
