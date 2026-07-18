//! Web feed module.
//! Monitors web pages without RSS feeds by extracting article links
//! via an external JS rendering service. Links matching the pattern
//! `domain/yyyy/mm/dd/slug` are treated as feed entries.

use chrono::Utc;
use reqwest::Client;
use scraper::{Html, Selector};
use uuid::Uuid;

use crate::config::encryption;
use crate::config::WebRenderingProvider;
use crate::db::{Article, Database, Feed};

/// Result of scanning a single web feed page during refresh.
#[derive(Debug)]
pub struct WebFeedRefreshResult {
    pub feed_id: i64,
    pub feed_name: String,
    pub articles_found: usize,
    pub articles_new: usize,
    pub error: Option<String>,
    pub auto_paused: bool,
}

/// Extracted link from a web page scan.
#[derive(Debug, Clone)]
pub struct ExtractedLink {
    pub url: String,
    pub title: String,
    pub date: String,
}

// ─── Core flow methods ────────────────────────────────────────────────

/// Fetches the rendered HTML of a web page using the configured rendering API,
/// then extracts article links matching the date-based URL pattern.
/// Returns the list of extracted links or an error message.
pub async fn scan_web_page(
    client: &Client,
    provider: &WebRenderingProvider,
    api_key: &str,
    page_url: &str,
    pattern: &str,
) -> Result<Vec<ExtractedLink>, String> {
    let render_url = provider.render_url(api_key, page_url);

    let response = client
        .get(&render_url)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await
        // without_url() strips the request URL (which contains the API key)
        // from the error message before it reaches logs or the frontend
        .map_err(|e| format!("Rendering API request failed: {}", e.without_url()))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Rendering API returned {}: {}", status, body));
    }

    let html = response
        .text()
        .await
        .map_err(|e| format!("Failed to read rendered HTML: {}", e))?;

    Ok(extract_links_from_html(&html, pattern))
}

/// Tests a web page scan: calls the rendering API and checks if at least
/// one article link is found. Returns the extracted domain pattern on success.
pub async fn test_web_page(
    client: &Client,
    provider: &WebRenderingProvider,
    api_key: &str,
    page_url: &str,
) -> Result<(String, usize), String> {
    let pattern = extract_domain_pattern(page_url)?;
    let links = scan_web_page(client, provider, api_key, page_url, &pattern).await?;

    if links.is_empty() {
        return Err("web_feed_no_articles".to_string());
    }

    Ok((pattern, links.len()))
}

/// Tests if the web rendering API key is valid by making a lightweight request.
pub async fn test_rendering_api(
    client: &Client,
    provider: &WebRenderingProvider,
    api_key: &str,
) -> Result<(), String> {
    let test_url = provider.test_url(api_key);

    let response = client
        .get(&test_url)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        // without_url() strips the request URL (which contains the API key)
        .map_err(|e| format!("API test request failed: {}", e.without_url()))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!("API test failed ({}): {}", status, body))
    }
}

/// Refreshes all active web feeds: fetches rendered HTML, extracts new links,
/// and inserts them as articles. Runs separately from RSS feed refresh.
pub async fn refresh_web_feeds(db: &Database) -> Vec<WebFeedRefreshResult> {
    let (provider, api_key) = match load_rendering_config(db) {
        Some(config) => config,
        None => return vec![],
    };

    let feeds = match db.get_active_web_feeds() {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("Failed to load web feeds: {}", e);
            return vec![];
        }
    };

    if feeds.is_empty() {
        return vec![];
    }

    let client = Client::new();
    let mut results = Vec::new();

    for feed in &feeds {
        let result = refresh_single_web_feed(&client, &provider, &api_key, feed, db).await;
        results.push(result);
    }

    results
}

// ─── Standalone methods ───────────────────────────────────────────────

/// Refreshes a single web feed: scans the page and inserts new articles.
async fn refresh_single_web_feed(
    client: &Client,
    provider: &WebRenderingProvider,
    api_key: &str,
    feed: &Feed,
    db: &Database,
) -> WebFeedRefreshResult {
    let links = match scan_web_page(client, provider, api_key, &feed.url, &feed.web_feed_pattern).await {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!("Web feed '{}' scan failed: {}", feed.site_name, e);

            let is_timeout = e.contains("timed out") || e.contains("timeout")
                || e.contains("connect") || e.contains("Connection");
            if is_timeout {
                if let Err(pause_err) = db.set_feed_paused(feed.id, true) {
                    tracing::error!("Failed to auto-pause web feed {}: {}", feed.id, pause_err);
                } else {
                    if crate::is_system_italian() {
                        tracing::info!("Web feed '{}' messo in pausa automaticamente per errore di connessione", feed.site_name);
                    } else {
                        tracing::info!("Web feed '{}' auto-paused due to connection error", feed.site_name);
                    }
                }
            }

            return WebFeedRefreshResult {
                feed_id: feed.id,
                feed_name: feed.site_name.clone(),
                articles_found: 0,
                articles_new: 0,
                error: Some(e),
                auto_paused: is_timeout,
            };
        }
    };

    // Read the retention setting: links whose URL date is already beyond the
    // retention window must not be inserted, otherwise they would be counted
    // as new and then immediately deleted by the purge that runs right after
    // the web feed refresh (and re-inserted again on every following scan).
    let retention_hours = db
        .get_setting("retention_hours")
        .ok()
        .flatten()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(24);

    let found = links.len();
    let mut new_count = 0;
    let now = Utc::now().to_rfc3339();

    for link in &links {

        // Skip articles already expired at insertion time. published_at is
        // built as URL date + current clock time, so the age in whole days
        // (times 24) is exactly how far published_at is behind now.
        if let Ok(link_date) = chrono::NaiveDate::parse_from_str(&link.date, "%Y-%m-%d") {
            let age_days = (Utc::now().date_naive() - link_date).num_days();
            if age_days * 24 >= retention_hours {
                continue;
            }
        }

        let article = Article {
            id: Uuid::new_v4().to_string(),
            feed_id: feed.id,
            url: link.url.clone(),
            title: link.title.clone(),
            author: String::new(),
            published_at: Some(format!("{}T{}", link.date, Utc::now().format("%H:%M:%S+00:00"))),
            full_text: String::new(),
            summary: String::new(),
            summary_source: "none".to_string(),
            is_favorite: false,
            fetched_at: now.clone(),
            newsletter_html: String::new(),
        };

        match db.insert_article(&article) {
            Ok(true) => new_count += 1,
            Ok(false) => {}
            Err(e) => {
                tracing::warn!("Failed to insert web feed article '{}': {}", link.url, e);
            }
        }
    }

    if crate::is_system_italian() {
        tracing::info!(
            "Web feed '{}': {} link trovati, {} nuovi articoli salvati",
            feed.site_name, found, new_count
        );
    } else {
        tracing::info!(
            "Web feed '{}': {} links found, {} new articles saved",
            feed.site_name, found, new_count
        );
    }

    WebFeedRefreshResult {
        feed_id: feed.id,
        feed_name: feed.site_name.clone(),
        articles_found: found,
        articles_new: new_count,
        error: None,
        auto_paused: false,
    }
}

// ─── Helper methods ───────────────────────────────────────────────────

/// Loads and decrypts the web rendering API configuration from the database.
/// Returns None if no configuration is saved or decryption fails.
fn load_rendering_config(db: &Database) -> Option<(WebRenderingProvider, String)> {
    let (provider_str, encrypted_key, iv) = db.get_web_rendering_config().ok()??;

    if encrypted_key.is_empty() {
        return None;
    }

    let api_key = encryption::decrypt_api_key(&encrypted_key, &iv).ok()?;
    let provider = WebRenderingProvider::from_str(&provider_str)?;

    Some((provider, api_key))
}

/// Extracts the base domain from a URL to build the date-based pattern.
/// e.g. "https://www.news.online/c/science/" → "www.news.online"
fn extract_domain_pattern(url: &str) -> Result<String, String> {
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .ok_or_else(|| "Invalid URL: missing http(s) scheme".to_string())?;

    let domain = without_scheme
        .split('/')
        .next()
        .ok_or_else(|| "Invalid URL: cannot extract domain".to_string())?;

    if domain.is_empty() {
        return Err("Invalid URL: empty domain".to_string());
    }

    Ok(domain.to_string())
}

/// Extracts article links from HTML that match the date-based URL pattern.
/// Pattern: links whose href contains `domain/yyyy/mm/dd/` followed by a slug.
/// Deduplicates by URL (same article may appear as image + text links).
fn extract_links_from_html(html: &str, domain_pattern: &str) -> Vec<ExtractedLink> {
    let document = Html::parse_document(html);
    let a_selector = match Selector::parse("a[href]") {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let domain_lower = domain_pattern.to_lowercase();
    // Strip "www." for comparison since URLs may or may not include it
    let domain_bare = domain_lower.strip_prefix("www.").unwrap_or(&domain_lower);

    let mut seen_urls = std::collections::HashSet::new();
    let mut links = Vec::new();

    for element in document.select(&a_selector) {
        let href = match element.value().attr("href") {
            Some(h) => h.trim(),
            None => continue,
        };

        // Filter: only match links pointing to the same domain
        if !href_matches_domain(href, domain_bare) {
            continue;
        }

        // Check if the href matches the date-based pattern
        if let Some(captures) = SimpleDateUrlMatcher.captures(href) {
            let url = normalize_url(href);

            // Deduplicate by URL
            if seen_urls.contains(&url) {
                continue;
            }
            seen_urls.insert(url.clone());

            let year = &captures[1];
            let month = &captures[2];
            let day = &captures[3];
            let date = format!("{}-{}-{}", year, month, day);

            // Extract title from the <a> tag text content
            let title = extract_link_title(&element);

            if !title.is_empty() {
                links.push(ExtractedLink { url, title, date });
            }
        }
    }

    links
}

/// Extracts a clean title from an anchor element.
/// Strips inner HTML tags, falls back to title/aria-label attributes.
fn extract_link_title(element: &scraper::ElementRef) -> String {
    // Collect text nodes from the anchor element
    let text: String = element.text().collect::<Vec<_>>().join(" ");
    let trimmed = strip_time_prefix_suffix(text.trim());

    if !trimmed.is_empty() && trimmed.len() > 3 {
        return trimmed;
    }

    // Fallback: title attribute
    if let Some(title) = element.value().attr("title") {
        let t = title.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }

    // Fallback: aria-label attribute
    if let Some(label) = element.value().attr("aria-label") {
        let l = label.trim();
        if !l.is_empty() {
            return l.to_string();
        }
    }

    String::new()
}

/// Checks if a URL's domain matches the expected domain (case-insensitive).
/// Strips "www." from both sides before comparing.
fn href_matches_domain(href: &str, domain_bare: &str) -> bool {
    let href_lower = href.to_lowercase();
    let after_scheme = href_lower
        .strip_prefix("https://")
        .or_else(|| href_lower.strip_prefix("http://"));

    let after_scheme = match after_scheme {
        Some(s) => s,
        None => return false,
    };

    let href_domain = after_scheme.split('/').next().unwrap_or("");
    let href_bare = href_domain
        .split(':').next().unwrap_or(href_domain) // strip port
        .strip_prefix("www.").unwrap_or(href_domain.split(':').next().unwrap_or(href_domain));

    href_bare == domain_bare
}

/// Strips leading or trailing time patterns (HH:MM or HH:MM:SS) from a title.
/// e.g. "08:32 Price of fuel increased" → "Price of fuel increased"
/// e.g. "Price of fuel increased 08:32" → "Price of fuel increased"
/// Also strips relative time expressions (e.g. "1 day ago", "3 ore fa", "il y a 2 heures",
/// "vor 3 Stunden", "hace 1 hora", "há 5 dias") in all supported UI languages
fn strip_time_prefix_suffix(title: &str) -> String {
    let s = title.trim();

    // Check leading HH:MM or HH:MM:SS
    if s.len() >= 5 {
        let bytes = s.as_bytes();
        if bytes[2] == b':' && bytes[0].is_ascii_digit() && bytes[1].is_ascii_digit()
            && bytes[3].is_ascii_digit() && bytes[4].is_ascii_digit()
        {
            // HH:MM:SS followed by space
            if s.len() >= 9 && bytes[5] == b':' && bytes[6].is_ascii_digit()
                && bytes[7].is_ascii_digit() && bytes[8] == b' '
            {
                return s[9..].trim().to_string();
            }
            // HH:MM followed by space
            if bytes[5] == b' ' {
                return s[6..].trim().to_string();
            }
        }
    }

    // Check trailing HH:MM or HH:MM:SS
    if s.len() >= 5 {
        let bytes = s.as_bytes();
        let len = bytes.len();

        // Trailing space + HH:MM
        if len >= 6 && bytes[len - 3] == b':' && bytes[len - 5].is_ascii_digit()
            && bytes[len - 4].is_ascii_digit() && bytes[len - 2].is_ascii_digit()
            && bytes[len - 1].is_ascii_digit() && bytes[len - 6] == b' '
        {
            // Check if it's actually HH:MM:SS
            if len >= 9 && bytes[len - 6] == b':' && bytes[len - 8].is_ascii_digit()
                && bytes[len - 7].is_ascii_digit() && bytes[len - 9] == b' '
            {
                return s[..len - 9].trim().to_string();
            }
            return s[..len - 6].trim().to_string();
        }
    }

    // Relative time patterns for all supported UI languages.
    // Two structural families exist among the supported languages:
    // - Suffix marker: "<N> <unit> <marker>" (IT "3 ore fa", EN "2 hours ago", PT-BR "3 horas atrás")
    // - Prefix marker: "<marker> <N> <unit>" (FR "il y a 3 heures", DE "vor 3 Stunden",
    //                                         ES "hace 3 horas", PT "há 3 horas")
    // Note: Italian and English remain the primary languages only for terminal
    // logs and handled error messages; title cleanup covers every UI language.
    //
    // The lowercased copy is used for case-insensitive matching, and byte
    // positions are reused on the original string. This is only safe when
    // lowercasing did not change the byte length (always true for the
    // Latin-script languages handled here; the guard protects against
    // exotic Unicode edge cases).
    let lower = s.to_lowercase();
    if lower.len() == s.len() {
        // Suffix-marker patterns: number + unit + trailing marker word
        let suffix_patterns = [
            // Italian
            "minuto fa", "minuti fa", "ora fa", "ore fa", "giorno fa", "giorni fa",
            // English
            "minute ago", "minutes ago", "hour ago", "hours ago", "day ago", "days ago",
            // Portuguese (Brazilian style, e.g. "3 horas atrás")
            "minuto atrás", "minutos atrás", "hora atrás", "horas atrás", "dia atrás", "dias atrás",
        ];

        for pat in suffix_patterns.iter() {
            if let Some(pos) = lower.find(pat) {
                let after = s[pos + pat.len()..].trim();
                if !after.is_empty() {
                    return after.to_string();
                }
                // If there's nothing after, check if the real title is before
                let before = s[..pos].trim();
                // Strip the leading number (e.g. "3 ")
                let before = before.trim_end_matches(|c: char| c.is_ascii_digit() || c == ' ');
                if !before.is_empty() {
                    return before.to_string();
                }
            }
        }

        // Prefix-marker patterns (FR "il y a", DE "vor", ES "hace", PT "há")
        if let Some(cleaned) = strip_prefix_marker_time(s, &lower) {
            return cleaned;
        }
    }

    s.to_string()
}

/// Handles relative time expressions where the marker precedes the number,
/// e.g. FR "il y a 3 heures", DE "vor 2 Stunden", ES "hace 1 hora", PT "há 5 dias".
/// `lower` must be the lowercased version of `s` with identical byte length.
/// Returns the cleaned title if a time expression was found and removed.
fn strip_prefix_marker_time(s: &str, lower: &str) -> Option<String> {
    // Markers include the trailing space so "vor" cannot match inside a word
    let markers = ["il y a ", "vor ", "hace ", "há "];
    // Time unit words for FR, DE, ES, PT (singular, plural and common abbreviations).
    // Matching a unit requires the full "<marker> <digits> <unit>" structure,
    // so short abbreviations like "h" are safe here.
    let units = [
        // Minutes
        "minutes", "minuten", "minutos", "minuto", "minute", "min",
        // Hours
        "heures", "heure", "stunden", "stunde", "horas", "hora", "h",
        // Days
        "jours", "jour", "tagen", "tage", "tag", "días", "día", "dias", "dia",
    ];

    for marker in markers.iter() {
        let mut search_from = 0;
        while let Some(rel) = lower[search_from..].find(marker) {
            let start = search_from + rel;

            // The marker must start at a word boundary
            let boundary_ok = start == 0
                || lower[..start].chars().next_back().map_or(true, |c| !c.is_alphanumeric());
            if !boundary_ok {
                search_from = start + marker.len();
                continue;
            }

            // Read the number right after the marker
            let bytes = lower.as_bytes();
            let num_start = start + marker.len();
            let mut idx = num_start;
            while idx < bytes.len() && bytes[idx].is_ascii_digit() {
                idx += 1;
            }
            if idx == num_start || idx >= bytes.len() || bytes[idx] != b' ' {
                search_from = start + marker.len();
                continue;
            }
            idx += 1; // skip the space after the number

            // Read the unit word (must end at a word boundary)
            let rest = &lower[idx..];
            let matched_unit = units.iter().find(|u| {
                rest.starts_with(*u)
                    && rest[u.len()..].chars().next().map_or(true, |c| !c.is_alphanumeric())
            });
            let Some(unit) = matched_unit else {
                search_from = start + marker.len();
                continue;
            };
            let end = idx + unit.len();

            // Prefer the text after the time expression, fall back to the text before
            let after = s[end..]
                .trim()
                .trim_start_matches(|c: char| c == ',' || c == ':' || c == '-' || c == '–')
                .trim();
            if !after.is_empty() {
                return Some(after.to_string());
            }
            let before = s[..start].trim();
            if !before.is_empty() {
                return Some(before.to_string());
            }
            search_from = end;
        }
    }

    None
}

/// Lightweight URL matcher for the `domain/yyyy/mm/dd/slug` pattern.
/// Avoids pulling in the `regex` crate as a dependency.
struct SimpleDateUrlMatcher;

/// Captured groups from a date-based URL match.
struct UrlCaptures<'a> {
    groups: [&'a str; 4], // year, month, day, slug
}

impl std::ops::Index<usize> for UrlCaptures<'_> {
    type Output = str;
    fn index(&self, index: usize) -> &Self::Output {
        self.groups[index - 1]
    }
}

impl SimpleDateUrlMatcher {
    /// Attempts to match a URL against the `domain/yyyy/mm/dd/slug` pattern.
    /// Returns captured groups: [1]=year, [2]=month, [3]=day, [4]=slug.
    fn captures<'a>(&self, url: &'a str) -> Option<UrlCaptures<'a>> {
        // Find the path portion after the domain
        let path_start = url.find("://")?;
        let after_scheme = &url[path_start + 3..];
        // Skip past the domain
        let slash_pos = after_scheme.find('/')?;
        let path = &after_scheme[slash_pos..];
        // Ignore query string and fragment: article identity is in the path
        // only, and rendering APIs may attach variable tracking parameters
        let path = path.split(|c| c == '?' || c == '#').next().unwrap_or(path);

        // Look for /yyyy/mm/dd/slug pattern anywhere in the path
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        for window in segments.windows(4) {
            let year = window[0];
            let month = window[1];
            let day = window[2];
            let slug = window[3];

            // Validate year (4 digits, reasonable range)
            if year.len() != 4 || year.parse::<u16>().map_or(true, |y| y < 2000 || y > 2100) {
                continue;
            }
            // Validate month (2 digits, 01-12)
            if month.len() != 2 || month.parse::<u8>().map_or(true, |m| m < 1 || m > 12) {
                continue;
            }
            // Validate day (2 digits, 01-31)
            if day.len() != 2 || day.parse::<u8>().map_or(true, |d| d < 1 || d > 31) {
                continue;
            }
            // Validate slug (starts with alphanumeric, at least 2 chars)
            if slug.len() < 2 || !slug.chars().next().map_or(false, |c| c.is_alphanumeric()) {
                continue;
            }
            // Slug must contain only alphanumeric, hyphens, underscores
            if !slug.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
                continue;
            }

            return Some(UrlCaptures {
                groups: [year, month, day, slug],
            });
        }

        None
    }
}

/// Normalizes a URL by removing query strings, fragment identifiers and
/// trailing slashes. Web feed article identity lives entirely in the URL
/// path (`domain/yyyy/mm/dd/slug`), so variable tracking parameters
/// (e.g. "?ref=", "?utm_...") returned by different renders of the same
/// page must not create duplicate articles.
fn normalize_url(url: &str) -> String {
    let without_fragment = url.split('#').next().unwrap_or(url);
    let without_query = without_fragment.split('?').next().unwrap_or(without_fragment);
    let trimmed = without_query.trim_end_matches('/');
    trimmed.to_string()
}