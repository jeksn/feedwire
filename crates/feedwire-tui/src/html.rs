//! Minimal HTML to plain-text conversion for terminal display.
//!
//! Feed content and descriptions arrive as HTML. For the terminal we don't
//! need rich rendering: block elements become line breaks, list items get a
//! bullet, everything else is stripped. Entities in common use are decoded.

/// Convert an HTML fragment into plain-text with newline-separated paragraphs.
pub fn html_to_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut chars = html.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '<' => {
                // Scan the whole tag, keeping only the leading name.
                let mut tag = String::new();
                let mut in_attrs = false;
                loop {
                    match chars.next() {
                        Some('>') | None => break,
                        Some(t) => {
                            if !in_attrs {
                                let is_name = t.is_ascii_alphanumeric()
                                    || (tag.is_empty() && (t == '/' || t == '!'));
                                if is_name {
                                    tag.push(t);
                                } else {
                                    in_attrs = true;
                                }
                            }
                        }
                    }
                }
                let tag = tag.trim_start_matches(['/', '!']).to_lowercase();
                match tag.as_str() {
                    "br" | "hr" => out.push('\n'),
                    "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "table" | "tr"
                    | "blockquote" => out.push('\n'),
                    "li" => out.push_str("\n• "),
                    _ => {}
                }
            }
            '&' => {
                // Lookahead up to 12 chars for an entity name.
                let mut ahead = chars.clone();
                let mut name = String::new();
                let mut consumed = 0usize;
                let mut terminated = false;
                for _ in 0..12 {
                    match ahead.next() {
                        Some(';') => {
                            consumed += 1;
                            terminated = true;
                            break;
                        }
                        Some(ch) => {
                            name.push(ch);
                            consumed += 1;
                        }
                        None => break,
                    }
                }
                if terminated && !name.is_empty() {
                    if let Some(decoded) = decode_entity(&name) {
                        out.push_str(&decoded);
                        for _ in 0..consumed {
                            chars.next();
                        }
                        continue;
                    }
                }
                out.push('&');
            }
            _ => out.push(c),
        }
    }

    // Collapse whitespace runs inside lines and squash multiple blank lines.
    let mut cleaned = String::with_capacity(out.len());
    let mut blank_run = 0;
    for line in out.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            blank_run += 1;
            if blank_run == 1 {
                cleaned.push('\n');
            }
        } else {
            blank_run = 0;
            let mut prev_space = false;
            for ch in trimmed.chars() {
                if ch.is_whitespace() {
                    if !prev_space {
                        cleaned.push(' ');
                    }
                    prev_space = true;
                } else {
                    cleaned.push(ch);
                    prev_space = false;
                }
            }
            cleaned.push('\n');
        }
    }
    cleaned.trim().to_string()
}

/// Decode a named or numeric entity body (without & and ;). Returns `None`
/// for unknown names so the literal text is kept.
fn decode_entity(name: &str) -> Option<String> {
    Some(match name {
        "amp" => "&".into(),
        "lt" => "<".into(),
        "gt" => ">".into(),
        "quot" => "\"".into(),
        "apos" | "rsquo" => "'".into(),
        "lsquo" => "'".into(),
        "ldquo" | "rdquo" => "\"".into(),
        "nbsp" => " ".into(),
        "hellip" | "mldr" => "...".into(),
        "mdash" | "ndash" => "-".into(),
        "middot" | "mdot" => "·".into(),
        "eacute" => "\u{e9}".into(),
        "egrave" => "\u{e8}".into(),
        "agrave" => "\u{e0}".into(),
        "ccedil" => "\u{e7}".into(),
        "ouml" => "\u{f6}".into(),
        "uuml" => "\u{fc}".into(),
        "auml" => "\u{e4}".into(),
        _ => {
            let code = if let Some(hex) = name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
                u32::from_str_radix(hex, 16).ok()
            } else {
                name.strip_prefix('#').and_then(|num| num.parse::<u32>().ok())
            };
            return code.and_then(char::from_u32).map(|ch| ch.to_string());
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_simple_markup() {
        assert_eq!(html_to_text("<p>Hello world</p>"), "Hello world");
    }

    #[test]
    fn tags_with_attributes_are_handled() {
        assert_eq!(html_to_text("<p class=\"x\">a</p>"), "a");
        assert_eq!(html_to_text("a<br/>b"), "a\nb");
    }

    #[test]
    fn block_tags_break_lines() {
        assert_eq!(html_to_text("<p>one</p><p>two</p>"), "one\n\ntwo");
        assert_eq!(html_to_text("a<br>b"), "a\nb");
    }

    #[test]
    fn list_items_get_bullets() {
        let text = html_to_text("<ul><li>a</li><li>b</li></ul>");
        assert!(text.contains("• a"));
        assert!(text.contains("• b"));
    }

    #[test]
    fn headings_break_lines() {
        assert_eq!(html_to_text("<h1>Title</h1>Body"), "Title\nBody");
    }

    #[test]
    fn entities_decode() {
        assert_eq!(html_to_text("a &amp; b"), "a & b");
        assert_eq!(html_to_text("&lt;tag&gt;"), "<tag>");
        assert_eq!(html_to_text("caf&#233;"), "caf\u{e9}");
        assert_eq!(html_to_text("caf&eacute;"), "caf\u{e9}");
        assert_eq!(html_to_text(&"caf\u{e9}".to_string()), "caf\u{e9}");
    }

    #[test]
    fn unknown_entity_left_alone() {
        assert_eq!(html_to_text("x &nosuch; y"), "x &nosuch; y");
    }

    #[test]
    fn ampersand_without_entity_survives() {
        assert_eq!(html_to_text("a & b"), "a & b");
    }

    #[test]
    fn blank_lines_collapse() {
        // Multiple blank lines collapse to one paragraph separator; leading
        // and trailing blanks are trimmed.
        assert_eq!(html_to_text("<p>a</p>\n\n\n\n\n<p>b</p>"), "a\n\nb");
        assert_eq!(html_to_text("a<br/><br/><br/>b"), "a\n\nb");
    }
}
