/// OPML 2.0 import and export utilities.
///
/// The OPML format is the de-facto interchange format for RSS feed lists —
/// used by every major reader (NetNewsWire, Reeder, Miniflux, Feedly, etc.).
/// Each feed is represented by an <outline> element with:
///   type="rss"   (sometimes "atom", but rss is conventional for both)
///   text="Feed title"
///   xmlUrl="https://example.com/feed.xml"
///   htmlUrl="https://example.com"  (optional)
///
/// We keep the implementation focused and dependency-free — just manual
/// XML building for export (simpler + faster than pulling in a full serialiser)
/// and quick-xml for import.

use quick_xml::events::Event;
use quick_xml::Reader;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct OpmlFeed {
    pub title: String,
    pub url: String,
}

/// Generate an OPML 2.0 document from a list of feeds.
pub fn export_opml(feeds: &[(String, String)]) -> String {
    // feeds: Vec<(title, feed_url)>
    let now = chrono::Utc::now().format("%a, %d %b %Y %H:%M:%S +0000").to_string();

    let mut xml = String::with_capacity(2048);
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<opml version=\"2.0\">\n");
    xml.push_str("  <head>\n");
    xml.push_str("    <title>FeedWire Subscriptions</title>\n");
    xml.push_str(&format!("    <dateCreated>{}</dateCreated>\n", now));
    xml.push_str("    <generator>FeedWire</generator>\n");
    xml.push_str("  </head>\n");
    xml.push_str("  <body>\n");

    for (title, url) in feeds {
        let escaped_title = escape_xml_attr(title);
        let escaped_url = escape_xml_attr(url);
        xml.push_str(&format!(
            "    <outline type=\"rss\" text=\"{}\" title=\"{}\" xmlUrl=\"{}\"/>\n",
            escaped_title, escaped_title, escaped_url
        ));
    }

    xml.push_str("  </body>\n");
    xml.push_str("</opml>\n");
    xml
}

/// Parse an OPML document and return a flat list of feed `(title, xmlUrl)` pairs.
/// Handles nested categories by flattening them.
pub fn import_opml(xml: &str) -> Result<Vec<OpmlFeed>, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut feeds = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) => {
                let name = e.name();
                let local = std::str::from_utf8(name.local_name().as_ref())
                    .unwrap_or("")
                    .to_ascii_lowercase();

                if local == "outline" {
                    let mut xml_url: Option<String> = None;
                    let mut text: Option<String> = None;
                    let mut title: Option<String> = None;
                    let mut outline_type: Option<String> = None;

                    for attr in e.attributes().flatten() {
                        let key = std::str::from_utf8(attr.key.local_name().as_ref())
                            .unwrap_or("")
                            .to_ascii_lowercase();
                        // quick-xml 0.37 uses decode_and_unescape_value(decoder)
                        let val = attr
                            .decode_and_unescape_value(reader.decoder())
                            .unwrap_or_default()
                            .to_string();

                        match key.as_str() {
                            "xmlurl" => xml_url = Some(val),
                            "text"   => text = Some(val),
                            "title"  => title = Some(val),
                            "type"   => outline_type = Some(val),
                            _ => {}
                        }
                    }

                    // Accept outlines that either have a type of "rss"/"atom"/"feed"
                    // or simply have an xmlUrl (many exporters omit the type).
                    let has_feed_type = outline_type.as_deref().map(|t| {
                        matches!(t.to_ascii_lowercase().as_str(), "rss" | "atom" | "feed")
                    }).unwrap_or(false);

                    if let Some(url) = xml_url {
                        if has_feed_type || !url.is_empty() {
                            let feed_title = title
                                .or(text)
                                .filter(|s| !s.is_empty())
                                .unwrap_or_else(|| url.clone());
                            feeds.push(OpmlFeed { title: feed_title, url });
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("OPML parse error: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(feeds)
}

/// Minimal XML attribute escaping.
fn escape_xml_attr(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('"', "&quot;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let feeds = vec![
            ("Hacker News".to_string(), "https://news.ycombinator.com/rss".to_string()),
            ("The Verge".to_string(), "https://www.theverge.com/rss/index.xml".to_string()),
        ];

        let opml = export_opml(&feeds);
        assert!(opml.contains("xmlUrl=\"https://news.ycombinator.com/rss\""));

        let imported = import_opml(&opml).unwrap();
        assert_eq!(imported.len(), 2);
        assert_eq!(imported[0].title, "Hacker News");
        assert_eq!(imported[1].url, "https://www.theverge.com/rss/index.xml");
    }

    #[test]
    fn import_no_type_attr() {
        let opml = r#"<?xml version="1.0"?>
        <opml version="2.0">
          <body>
            <outline text="Feed" xmlUrl="https://example.com/feed"/>
          </body>
        </opml>"#;
        let feeds = import_opml(opml).unwrap();
        assert_eq!(feeds.len(), 1);
    }
}
