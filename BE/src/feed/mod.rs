use feed_rs::parser;
use reqwest::Client;
use chrono::Utc;
use uuid::Uuid;

use crate::db::{Database, Article, Feed};
use crate::web_feed;

/// Result of fetching and parsing a single RSS feed.
#[derive(Debug, serde::Serialize)]
pub struct FetchResult {
    pub feed_id: i64,
    pub feed_name: String,
    pub articles_found: usize,
    pub articles_new: usize,
    pub error: Option<String>,
    pub auto_paused: bool,
}

/// Strips all HTML tags from a string and decodes HTML entities (e.g. &amp; → &, &#32; → space).
/// Used for cleaning feed-provided text fields (title, author, summary).
fn strip_html_and_decode(input: &str) -> String {
    // First pass: remove all HTML tags using ammonia (allow nothing)
    let stripped = ammonia::Builder::new()
        .tags(std::collections::HashSet::new())
        .clean(input)
        .to_string();

    // Second pass: decode HTML entities
    let decoded = decode_html_entities(&stripped);

    // Third pass: fix double-encoded UTF-8 (e.g. "giÃ " → "già")
    let fixed = fix_double_utf8(&decoded);

    fixed.trim().to_string()
}

/// Decodes common HTML entities and numeric character references.
fn decode_html_entities(input: &str) -> String {
    let mut result = input.to_string();

    // Numeric character references (decimal): &#32; &#160; etc.
    while let Some(start) = result.find("&#") {
        if let Some(end) = result[start..].find(';') {
            let entity = &result[start..start + end + 1];
            let num_str = &entity[2..entity.len() - 1];
            if let Ok(code_point) = num_str.parse::<u32>()
                && let Some(ch) = char::from_u32(code_point) {
                result = result.replace(entity, &ch.to_string());
                continue;
            }
            // If we can't parse it, skip past this entity to avoid infinite loop
            break;
        } else {
            break;
        }
    }

    // Named entities
    result = result.replace("&amp;", "&");
    result = result.replace("&lt;", "<");
    result = result.replace("&gt;", ">");
    result = result.replace("&quot;", "\"");
    result = result.replace("&apos;", "'");
    result = result.replace("&#x27;", "'");
    result = result.replace("&#x2F;", "/");
    result = result.replace("&nbsp;", " ");
    result = result.replace("&laquo;", "«");
    result = result.replace("&raquo;", "»");
    result = result.replace("&ndash;", "–");
    result = result.replace("&mdash;", "—");
    result = result.replace("&hellip;", "…");
    result = result.replace("&rsquo;", "'");
    result = result.replace("&lsquo;", "'");
    result = result.replace("&rdquo;", "\u{201D}");
    result = result.replace("&ldquo;", "\u{201C}");
    result = result.replace("&eacute;", "é");
    result = result.replace("&egrave;", "è");
    result = result.replace("&agrave;", "à");
    result = result.replace("&igrave;", "ì");
    result = result.replace("&ograve;", "ò");
    result = result.replace("&ugrave;", "ù");
    result = result.replace("&Eacute;", "É");
    result = result.replace("&Egrave;", "È");
    result = result.replace("&Agrave;", "À");
    result = result.replace("&ccedil;", "ç");
    result = result.replace("&ntilde;", "ñ");
    result = result.replace("&uuml;", "ü");
    result = result.replace("&ouml;", "ö");
    result = result.replace("&auml;", "ä");

    result
}

/// Fixes double-encoded UTF-8 text.
/// Detects patterns like "Ã¨" (which should be "è") and corrects them.
fn fix_double_utf8(input: &str) -> String {
    // Check if the text contains typical double-encoding markers
    if !input.contains('\u{00C3}') && !input.contains('\u{00C2}') {
        return input.to_string();
    }

    // Convert each char to its Latin-1 byte value.
    // If any char is outside Latin-1 range, abort and return original.
    let mut bytes: Vec<u8> = Vec::with_capacity(input.len());
    for c in input.chars() {
        let code = c as u32;
        if code > 255 {
            // Contains characters outside Latin-1 — not a double-encoding issue
            return input.to_string();
        }
        bytes.push(code as u8);
    }

    // Try to interpret the bytes as UTF-8
    match String::from_utf8(bytes) {
        Ok(fixed) if !fixed.contains('\u{FFFD}') => fixed,
        _ => input.to_string(),
    }
}

/// Fixes encoding declaration in RSS feed XML.
/// Handles two cases:
/// 1. Feed declares ISO-8859-x but bytes are actually UTF-8 (e.g. FeedBurner proxy)
/// 2. Feed declares ISO-8859-x and bytes are actually in that encoding
/// In both cases, we convert to valid UTF-8 and update the XML declaration.
fn fix_feed_encoding(bytes: &[u8]) -> Vec<u8> {
    let header_len = std::cmp::min(bytes.len(), 200);
    let header = String::from_utf8_lossy(&bytes[..header_len]).to_lowercase();

    if !header.contains("iso-8859") && !header.contains("latin") && !header.contains("windows-1252") {
        return bytes.to_vec();
    }

    // Try interpreting as UTF-8 first
    let text = if let Ok(s) = String::from_utf8(bytes.to_vec()) {
        // Bytes are valid UTF-8 — just fix the declaration
        s
    } else {
        // Bytes are NOT valid UTF-8 — convert from Latin-1 to UTF-8
        bytes.iter().map(|&b| b as char).collect::<String>()
    };

    // Replace the encoding declaration so feed-rs treats it as UTF-8
    let fixed = text
        .replace("encoding=\"ISO-8859-15\"", "encoding=\"UTF-8\"")
        .replace("encoding=\"ISO-8859-1\"", "encoding=\"UTF-8\"")
        .replace("encoding=\"iso-8859-15\"", "encoding=\"UTF-8\"")
        .replace("encoding=\"iso-8859-1\"", "encoding=\"UTF-8\"")
        .replace("encoding=\"windows-1252\"", "encoding=\"UTF-8\"")
        .replace("encoding='ISO-8859-15'", "encoding='UTF-8'")
        .replace("encoding='ISO-8859-1'", "encoding='UTF-8'")
        .replace("encoding='iso-8859-15'", "encoding='UTF-8'")
        .replace("encoding='iso-8859-1'", "encoding='UTF-8'")
        .replace("encoding='windows-1252'", "encoding='UTF-8'");

    fixed.into_bytes()
}

/// Fetches a single RSS feed and returns only the new articles.
/// `start_from` is the earliest date to accept articles from —
/// anything older is skipped to avoid unnecessary downloads.
async fn fetch_single_feed(
    client: &Client,
    feed: &Feed,
    start_from: chrono::DateTime<Utc>,
) -> Result<Vec<Article>, String> {
    // Fetch the raw RSS/Atom XML
    let response = client
        .get(&feed.url)
        .header("User-Agent", "MeigRSS/0.1 (RSS Aggregator)")
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("HTTP request failed for {}: {}", feed.url, e))?;

    let body = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response body from {}: {}", feed.url, e))?;

    // Some feeds declare a non-UTF-8 encoding in the XML header (e.g. ISO-8859-15)
    // but the HTTP server serves them as UTF-8, causing mojibake.
    // Detect and fix this by checking the XML declaration and re-encoding if needed.
    let body = fix_feed_encoding(&body);

    // Parse the feed using feed-rs
    let parsed = parser::parse(&body[..])
        .map_err(|e| format!("Failed to parse feed {}: {}", feed.url, e))?;

    let now = Utc::now().to_rfc3339();
    let mut articles = Vec::new();

    for entry in parsed.entries {
        // Extract the article URL (required, skip entries without one)
        let article_url = match entry.links.first() {
            Some(link) => link.href.clone(),
            None => continue,
        };

        // Strip HTML tags from title and decode HTML entities
        let title = entry
            .title
            .map(|t| strip_html_and_decode(&t.content))
            .unwrap_or_default();

        // Strip HTML tags from author name and decode HTML entities
        let author = entry
            .authors
            .first()
            .map(|a| strip_html_and_decode(&a.name))
            .unwrap_or_default();

        // Extract publication date
        let published_at = entry
            .published
            .or(entry.updated)
            .map(|d| d.to_rfc3339());

        // Skip articles older than the start_from date
        if let Some(ref pub_date) = published_at
            && let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(pub_date)
            && parsed < start_from {
            continue;
        }

        // Extract the feed-provided summary if available.
        // This may be used directly instead of calling the LLM.
        let summary_text = entry
            .summary
            .map(|s| s.content)
            .unwrap_or_default();

        // Determine if the feed summary is good enough to use as-is.
        // Criteria: longer than 150 chars and not just the title repeated.
        let (summary, summary_source) = evaluate_feed_summary(
            &summary_text,
            &title,
            feed.force_ai_summary,
        );

        articles.push(Article {
            id: Uuid::new_v4().to_string(),
            feed_id: feed.id,
            url: article_url,
            title,
            author,
            published_at,
            full_text: String::new(),
            summary,
            summary_source,
            is_favorite: false,
            fetched_at: now.clone(),
            newsletter_html: String::new(),
        });
    }

    Ok(articles)
}

/// Evaluates the feed-provided summary and always saves it if valid,
/// regardless of the force_ai flag. The force_ai flag only affects
/// the summary_source marker, not whether the text is stored.
fn evaluate_feed_summary(
    summary: &str,
    title: &str,
    force_ai: bool,
) -> (String, String) {
    // Strip ALL HTML tags for evaluation and storage
    let clean = strip_html_and_decode(summary);
    let clean = clean.trim();

    // Too short: probably just a teaser or subtitle
    if clean.len() < 150 {
        return (String::new(), "none".to_string());
    }

    // Too similar to title: the summary field is just repeating the title
    if is_too_similar_to_title(clean, title) {
        return (String::new(), "none".to_string());
    }

    // Summary is valid — always save the text.
    // If force_ai is on, mark source as "none" so the UI knows
    // it still needs an LLM summary, but keep the feed text as fallback.
    if force_ai {
        (clean.to_string(), "none".to_string())
    } else {
        (clean.to_string(), "feed".to_string())
    }
}

/// Checks if the summary is essentially the same as the title.
/// Uses a simple heuristic: if the summary starts with the title text
/// or if the length difference is very small, they're too similar.
fn is_too_similar_to_title(summary: &str, title: &str) -> bool {
    let summary_lower = summary.to_lowercase();
    let title_lower = title.to_lowercase();

    // Summary starts with the title
    if summary_lower.starts_with(&title_lower) {
        return true;
    }

    // Summary and title are nearly the same length (within 20 chars)
    // and share most content
    if (summary.len() as i64 - title.len() as i64).abs() < 20 {
        return true;
    }

    false
}

/// Fetches all configured feeds, parses entries, deduplicates by URL,
/// and saves new articles to the database.
/// Also triggers expired article cleanup.
pub async fn fetch_all_feeds(db: &Database) -> Vec<FetchResult> {
    let client = Client::new();

    // Quick connectivity check: try multiple sites with a short timeout.
    // If at least one responds, internet is available.
    // This avoids false negatives when a single site is down.
    let check_urls = [
        "https://www.google.com",
        "https://www.cloudflare.com",
        "https://www.apple.com",
    ];
    let mut connected = false;
    for url in &check_urls {
        if client
            .get(*url)
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await
            .is_ok()
        {
            connected = true;
            break;
        }
    }
    if !connected {
        if crate::is_system_italian() {
                tracing::error!("Nessuna connessione internet rilevata. Aggiornamento feed saltato.");
            } else {
                tracing::error!("No internet connection detected. Skipping feed update.");
            }
        return vec![FetchResult {
            feed_id: 0,
            feed_name: String::new(),
            articles_found: 0,
            articles_new: 0,
            error: Some("no_internet".to_string()),
            auto_paused: false,
        }];
    }

    // Save the current timestamp as the refresh marker.
    // Articles fetched after this point will be highlighted as "new" in the UI.
    let refresh_ts = Utc::now().to_rfc3339();
    let _ = db.set_setting("last_refresh_timestamp", &refresh_ts);
    
    let feeds = match db.get_feeds() {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("Failed to load feeds from DB: {}", e);
            return vec![];
        }
    };

    // Read retention setting for cutoff calculation and purge
    let retention = db
        .get_setting("retention_hours")
        .ok()
        .flatten()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(24);

    let retention_cutoff = Utc::now() - chrono::Duration::hours(retention);

    // Maximum number of feeds fetched in parallel. Bounded to avoid
    // saturating slow networks while still cutting total refresh time.
    const MAX_CONCURRENT_FETCHES: usize = 8;

    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_FETCHES));
    let mut join_set = tokio::task::JoinSet::new();

    for feed in &feeds {
        // Skip paused feeds
        if feed.is_paused {
            continue;
        }
        // Skip web feeds (they are refreshed separately via the rendering API)
        if feed.is_web_feed {
            continue;
        }
        // Determine the start date: the more recent between
        // the retention cutoff and the latest article already in DB
        let start_from = match db.get_latest_article_date(feed.id) {
            Ok(Some(date_str)) => {
                match chrono::DateTime::parse_from_rfc3339(&date_str) {
                    Ok(last_date) => {
                        let last_date_utc = last_date.with_timezone(&Utc);
                        // Use the older of the two dates to fill any gaps.
                        // This handles the case where retention was reduced
                        // (deleting old articles) and then increased again.
                        if last_date_utc < retention_cutoff {
                            retention_cutoff
                        } else {
                            // Check if there are gaps: if the oldest article in DB
                            // is newer than the cutoff, we need to backfill
                            let oldest_date = db.get_oldest_article_date(feed.id)
                                .ok()
                                .flatten()
                                .and_then(|d| chrono::DateTime::parse_from_rfc3339(&d).ok())
                                .map(|d| d.with_timezone(&Utc));

                            match oldest_date {
                                Some(oldest) if oldest > retention_cutoff => retention_cutoff,
                                _ => last_date_utc,
                            }
                        }
                    }
                    Err(_) => retention_cutoff,
                }
            }
            _ => retention_cutoff,
        };

        // Spawn a concurrent fetch task for this feed.
        // reqwest::Client is cheap to clone (shared connection pool inside).
        let client = client.clone();
        let feed = feed.clone();
        let semaphore = std::sync::Arc::clone(&semaphore);
        join_set.spawn(async move {
            // Limit the number of simultaneous HTTP fetches
            let _permit = semaphore.acquire_owned().await;
            let fetch_result = fetch_single_feed(&client, &feed, start_from).await;
            (feed, fetch_result)
        });
    }

    let mut results = Vec::new();

    // Collect results as tasks complete; DB inserts stay on this task
    // since Database uses a Mutex-guarded single connection.
    while let Some(joined) = join_set.join_next().await {
        let (feed, fetch_result) = match joined {
            Ok(v) => v,
            Err(e) => {
                tracing::error!("Feed fetch task panicked or was cancelled: {}", e);
                continue;
            }
        };

        let result = match fetch_result {
            Ok(articles) => {
                let found = articles.len();
                let mut new_count = 0;

                for article in &articles {
                    match db.insert_article(article) {
                        Ok(true) => new_count += 1,
                        Ok(false) => {}
                        Err(e) => {
                            tracing::warn!("Failed to insert article '{}': {}", article.url, e);
                        }
                    }
                }

                if crate::is_system_italian() {
                    tracing::info!(
                        "Feed '{}': {} articoli trovati, {} nuovi articoli salvati",
                        feed.site_name, found, new_count
                    );
                } else {
                    tracing::info!(
                        "Feed '{}': {} entries found, {} new articles saved",
                        feed.site_name, found, new_count
                    );
                }

                FetchResult {
                    feed_id: feed.id,
                    feed_name: feed.site_name.clone(),
                    articles_found: found,
                    articles_new: new_count,
                    error: None,
                    auto_paused: false,
                }
            }
            Err(e) => {
                tracing::warn!("Failed to fetch feed '{}': {}", feed.url, e);

                // Auto-pause feeds that timeout or fail to connect
                let is_timeout = e.contains("timed out") || e.contains("timeout")
                    || e.contains("connect") || e.contains("Connection");
                if is_timeout {
                    if let Err(pause_err) = db.set_feed_paused(feed.id, true) {
                        tracing::error!("Failed to auto-pause feed {}: {}", feed.id, pause_err);
                    } else {
                        if crate::is_system_italian() {
                            tracing::info!("Feed '{}' messo in pausa automaticamente per errore di connessione", feed.site_name);
                        } else {
                            tracing::info!("Feed '{}' auto-paused due to connection error", feed.site_name);
                        }
                    }
                }

                FetchResult {
                    feed_id: feed.id,
                    feed_name: feed.site_name.clone(),
                    articles_found: 0,
                    articles_new: 0,
                    error: Some(e),
                    auto_paused: is_timeout,
                }
            }
        };
        results.push(result);
    }

    // Refresh web feeds separately (they use a rendering API, not RSS)
    let web_results = web_feed::refresh_web_feeds(db).await;
    for wr in web_results {
        results.push(FetchResult {
            feed_id: wr.feed_id,
            feed_name: wr.feed_name,
            articles_found: wr.articles_found,
            articles_new: wr.articles_new,
            error: wr.error,
            auto_paused: wr.auto_paused,
        });
    }

    // Purge expired articles (except favorites) after fetching new ones
    if let Err(e) = db.purge_expired_articles(retention) {
        tracing::error!("Failed to purge expired articles: {}", e);
    }

    results
}

/// Tests a single feed URL to verify it's valid and parseable.
/// Used when adding a new feed from the UI.
pub async fn test_feed_url(url: &str) -> Result<(String, String), String> {
    let client = Client::new();

    let response = client
        .get(url)
        .header("User-Agent", "MeigRSS/0.1 (RSS Aggregator)")
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    let body = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    let body = fix_feed_encoding(&body);

    let parsed = parser::parse(&body[..])
        .map_err(|e| format!("Not a valid RSS/Atom feed: {}", e))?;

    // Return the feed title and site name for auto-population
    let title = parsed.title.map(|t| t.content).unwrap_or_default();

    // Use the feed title as the site name displayed in the UI.
    // Falls back to the domain extracted from the feed link if title is empty.
    let site_name = if title.is_empty() {
        parsed
            .links
            .first()
            .map(|l| {
                l.href
                    .trim_start_matches("https://")
                    .trim_start_matches("http://")
                    .trim_start_matches("www.")
                    .split('/')
                    .next()
                    .unwrap_or("")
                    .to_string()
            })
            .unwrap_or_default()
    } else {
        title.clone()
    };

    Ok((title, site_name))
}