//! OPML 2.0 generation and parsing for feed import/export.
//!
//! OPML is the interchange format shared by virtually every feed reader, so it
//! only carries what all readers have in common: categories (as nested
//! outlines) and feeds. Favorites and application settings have no OPML
//! equivalent and remain exclusive to the JSON export format.

use std::collections::{HashMap, HashSet};

use chrono::Utc;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::db::{CategoryExport, FeedExport, FullExport};

/// Namespace for the few MeigRSS-specific attributes (web feed detection and
/// pause state). Other readers ignore namespaced attributes, while our own
/// import uses them to round-trip without losing information.
const MEIG_NS: &str = "https://meigrss.app/opml";

/// Builds an OPML 2.0 document from an export payload.
/// Categories become container outlines holding their feeds; feeds whose
/// category is not in the exported list belong to a default category, which
/// OPML represents as top-level outlines.
pub fn build_opml(export: &FullExport) -> String {
    let named: HashSet<&str> = export
        .categories
        .iter()
        .map(|c| c.name.as_str())
        .collect();

    // Newsletter feeds are internal containers with a "newsletter://" URL:
    // no reader can fetch them, and the JSON export already preserves them
    // together with their articles
    let feeds: Vec<&FeedExport> = export
        .feeds
        .iter()
        .filter(|f| !f.url.starts_with("newsletter://"))
        .collect();

    // Rough pre-allocation to avoid repeated reallocation on large exports
    let mut out = String::with_capacity(256 + export.feeds.len() * 200);

    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!("<opml version=\"2.0\" xmlns:meig=\"{}\">\n", MEIG_NS));
    out.push_str("  <head>\n");
    out.push_str("    <title>MeigRSS</title>\n");
    // The OPML spec dates are RFC 822
    out.push_str(&format!(
        "    <dateCreated>{}</dateCreated>\n",
        Utc::now().to_rfc2822()
    ));
    out.push_str("  </head>\n");
    out.push_str("  <body>\n");

    for feed in feeds
        .iter()
        .filter(|f| !named.contains(f.category_name.as_str()))
    {
        push_feed_outline(&mut out, feed, "    ");
    }

    for cat in &export.categories {
        out.push_str(&format!(
            "    <outline text=\"{0}\" title=\"{0}\">\n",
            escape_attr(&cat.name)
        ));
        for feed in feeds.iter().filter(|f| f.category_name == cat.name) {
            push_feed_outline(&mut out, feed, "      ");
        }
        out.push_str("    </outline>\n");
    }

    out.push_str("  </body>\n");
    out.push_str("</opml>\n");
    out
}

/// Parses an OPML document into the internal export structure, so the OPML and
/// JSON import paths converge on the same database write path.
/// Feeds found outside any container are assigned to `default_category`.
/// Nested folders resolve to their innermost name, since categories are flat.
pub fn parse_opml(xml: &str, default_category: &str) -> Result<FullExport, String> {
    let mut reader = Reader::from_str(xml);

    let mut categories: Vec<CategoryExport> = Vec::new();
    let mut feeds: Vec<FeedExport> = Vec::new();
    let mut seen_urls: HashSet<String> = HashSet::new();
    // Stack of enclosing folder names, so nested outlines resolve correctly
    let mut stack: Vec<String> = Vec::new();
    let mut in_body = false;

    loop {
        match reader.read_event() {
            // Container outline, or a feed outline that has children
            Ok(Event::Start(e)) => match local_name(e.name().as_ref()).as_str() {
                "body" => in_body = true,
                "outline" if in_body => {
                    let attrs = collect_attributes(&e);
                    let context = current_category(&stack, default_category);
                    match feed_from_attrs(&attrs, &context) {
                        Some(feed) => {
                            if seen_urls.insert(feed.url.clone()) {
                                feeds.push(feed);
                            }
                            // Keep the stack balanced with the closing tag
                            stack.push(context);
                        }
                        None => match folder_name(&attrs) {
                            Some(name) => {
                                register_category(&mut categories, &name);
                                stack.push(name);
                            }
                            // Unnamed container: children keep the current category
                            None => stack.push(context),
                        },
                    }
                }
                _ => {}
            },

            // Self-closing outline: a feed, or a folder with no feeds
            Ok(Event::Empty(e)) => {
                if in_body && local_name(e.name().as_ref()) == "outline" {
                    let attrs = collect_attributes(&e);
                    let context = current_category(&stack, default_category);
                    match feed_from_attrs(&attrs, &context) {
                        Some(feed) => {
                            if seen_urls.insert(feed.url.clone()) {
                                feeds.push(feed);
                            }
                        }
                        None => {
                            if let Some(name) = folder_name(&attrs) {
                                register_category(&mut categories, &name);
                            }
                        }
                    }
                }
            }

            Ok(Event::End(e)) => match local_name(e.name().as_ref()).as_str() {
                "outline" => {
                    stack.pop();
                }
                "body" => in_body = false,
                _ => {}
            },

            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("Invalid OPML file: {}", e)),
            _ => {}
        }
    }

    if feeds.is_empty() {
        return Err("The OPML file contains no feeds".to_string());
    }

    Ok(FullExport {
        categories,
        feeds,
        favorites: Vec::new(),
        settings: None,
    })
}

/// Appends a single feed as an OPML outline element.
fn push_feed_outline(out: &mut String, feed: &FeedExport, indent: &str) {
    let title = if !feed.title.is_empty() {
        &feed.title
    } else if !feed.site_name.is_empty() {
        &feed.site_name
    } else {
        &feed.url
    };

    out.push_str(indent);
    // type="rss" is the universal convention, used even for Atom feeds
    out.push_str(&format!(
        "<outline text=\"{0}\" title=\"{0}\" type=\"rss\" xmlUrl=\"{1}\"",
        escape_attr(title),
        escape_attr(&feed.url)
    ));
    if !feed.site_name.is_empty() {
        out.push_str(&format!(
            " meig:siteName=\"{}\"",
            escape_attr(&feed.site_name)
        ));
    }
    // Always written, so the file states the pause state explicitly instead
    // of relying on the importer's default
    out.push_str(&format!(" meig:isPaused=\"{}\"", feed.is_paused));
    if feed.is_web_feed {
        // The fetch mode only means something for web feeds, so it travels
        // together with the pattern
        out.push_str(&format!(
            " meig:isWebFeed=\"true\" meig:webFeedPattern=\"{}\" meig:isApiScraping=\"{}\"",
            escape_attr(&feed.web_feed_pattern),
            feed.is_api_scraping
        ));
    }
    out.push_str("/>\n");
}

/// Builds a feed from outline attributes, or returns None when the outline has
/// no `xmlUrl`, which marks it as a container rather than a feed.
fn feed_from_attrs(attrs: &HashMap<String, String>, category: &str) -> Option<FeedExport> {
    let url = attrs
        .get("xmlurl")
        .map(|u| u.trim())
        .filter(|u| !u.is_empty())?;

    let title = attrs
        .get("text")
        .or_else(|| attrs.get("title"))
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .unwrap_or(url);

    let site_name = attrs
        .get("sitename")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or(title);

    Some(FeedExport {
        url: url.to_string(),
        title: title.to_string(),
        site_name: site_name.to_string(),
        category_name: category.to_string(),
        // These flags have no OPML equivalent: imported feeds start with
        // defaults and can be configured afterwards
        force_ai_summary: false,
        enable_scraping: false,
        // Restored from the MeigRSS namespace when present, default otherwise,
        // so files from other readers import as active, non-web feeds
        is_paused: attrs.get("ispaused").map(|v| v == "true").unwrap_or(false),
        is_web_feed: attrs.get("iswebfeed").map(|v| v == "true").unwrap_or(false),
        web_feed_pattern: attrs
            .get("webfeedpattern")
            .map(|p| p.trim().to_string())
            .unwrap_or_default(),
            // Files from other readers never carry this, so those feeds import in
        // the free fetch mode
        is_api_scraping: attrs
            .get("isapiscraping")
            .map(|v| v == "true")
            .unwrap_or(false),
    })
}

/// Returns a container outline's display name, if it has one.
fn folder_name(attrs: &HashMap<String, String>) -> Option<String> {
    attrs
        .get("text")
        .or_else(|| attrs.get("title"))
        .map(|n| n.trim())
        .filter(|n| !n.is_empty())
        .map(|n| n.to_string())
}

/// Returns the category an outline belongs to: the innermost enclosing folder,
/// or the default category when at top level.
fn current_category(stack: &[String], default_category: &str) -> String {
    stack
        .last()
        .cloned()
        .unwrap_or_else(|| default_category.to_string())
}

/// Collects a category name unless it was already seen.
fn register_category(categories: &mut Vec<CategoryExport>, name: &str) {
    if !categories.iter().any(|c| c.name == name) {
        categories.push(CategoryExport {
            name: name.to_string(),
        });
    }
}

/// Collects outline attributes into a map keyed by lowercase local name.
/// OPML in the wild is inconsistent about capitalization, and the MeigRSS
/// attributes arrive namespace-prefixed, so both case and prefix are
/// normalized away before lookup.
fn collect_attributes(e: &BytesStart) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for attr in e.attributes().flatten() {
        let raw_key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
        let key = raw_key
            .rsplit(':')
            .next()
            .unwrap_or(raw_key.as_str())
            .to_ascii_lowercase();

        let raw_value = String::from_utf8_lossy(&attr.value).to_string();
        let value = match quick_xml::escape::unescape(&raw_value) {
            Ok(decoded) => decoded.to_string(),
            // Malformed entities: keep the raw text rather than dropping the feed
            Err(_) => raw_value,
        };

        map.insert(key, value);
    }
    map
}

/// Returns an element's local name in lowercase, ignoring any namespace prefix.
fn local_name(raw: &[u8]) -> String {
    let name = String::from_utf8_lossy(raw);
    name.rsplit(':')
        .next()
        .unwrap_or(name.as_ref())
        .to_ascii_lowercase()
}

/// Escapes a value for safe inclusion in an XML attribute.
/// Single pass, so there is no risk of double-escaping an ampersand.
fn escape_attr(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}