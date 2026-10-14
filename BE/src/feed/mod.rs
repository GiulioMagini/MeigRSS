use feed_rs::parser;
use reqwest::{Client, StatusCode, Url, redirect};
use chrono::Utc;
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::db::{Database, Article, Feed};
use crate::web_feed;

/// User-Agent sent when polling feeds. The version comes from Cargo at compile
/// time so it can never drift from the released one, and the contact URL lets
/// feed operators reach the project.
///
/// Feed polling must never pretend to be a browser: the Chrome User-Agent in
/// `scraper/mod.rs` covers on-demand article scraping, which is a different
/// case and stays as it is.
const FEED_USER_AGENT: &str = concat!(
    "MeigRSS/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/GiulioMagini/MeigRSS)"
);

/// Minimum length of feed-provided content before it is accepted as the full
/// article text. Publishers often repeat the same teaser in both the summary
/// and the content element: taking a teaser here would permanently block
/// scraping and leave the reader with a truncated article, so anything short
/// is ignored and the scraper keeps its job.
const FEED_CONTENT_MIN_LEN: usize = 500;

/// Result of fetching and parsing a single RSS feed.
#[derive(Debug, serde::Serialize)]
pub struct FetchResult {
    pub feed_id: i64,
    pub feed_name: String,
    pub articles_found: usize,
    pub articles_new: usize,
    pub error: Option<String>,
    /// i18n key explaining why the feed was paused, `None` when it was not.
    /// The frontend renders it next to the feed name in the auto-pause modal.
    pub pause_reason: Option<&'static str>,
    /// Seconds requested by a `429` through `Retry-After`, when it sent one.
    /// Left raw so the frontend can format it in the current language.
    pub retry_after_seconds: Option<u64>,
}

/// Why a feed was paused during this refresh.
enum PauseReason {
    /// `410`: the feed does not exist any more.
    Gone,
    /// `429`: polled too often, with the `Retry-After` delay in seconds when
    /// the server provided one.
    TooManyRequests(Option<u64>),
    /// Timeout or failed connection: the server could not be reached at all.
    Unreachable,
}

/// What the server answered to a conditional GET.
enum FeedOutcome {
    /// `200`: fresh content. Each validator is `Some` only when the server
    /// actually sent that header; `None` means the stored value must be left
    /// alone rather than erased.
    Updated {
        articles: Vec<Article>,
        etag: Option<String>,
        last_modified: Option<String>,
    },
    /// `304`: nothing changed, and the stored validators stay as they are.
    NotModified,
    /// `410` / `429` / timeout or failed connection: the feed gets paused.
    Paused(PauseReason),
}

/// Outcome of one feed fetch.
///
/// `canonical_url` sits beside the outcome instead of being one of its variants
/// because a redirect and the response body arrive together: reqwest follows
/// the chain internally and hands back only the final response, which may be a
/// `200` or a `304`. "Moved" is therefore not an alternative to "not modified".
struct FeedResponse {
    outcome: FeedOutcome,
    canonical_url: Option<String>,
}

/// State of one redirect chain while it is being followed.
struct RedirectTrack {
    /// Last URL reached through an unbroken sequence of `301` / `308`.
    candidate: Option<String>,
    /// Cleared by the first temporary redirect: with `A →301→ B →302→ C` the
    /// canonical URL is `B`, because the hop to `C` is not permanent.
    permanent_only: bool,
}

/// What a feed URL test produced, kept around so that adding the feed right
/// afterwards does not have to download and parse the same body a second time.
///
/// The body is already decoded by `fix_feed_encoding`, so it can be handed
/// straight to `parse_articles`.
pub struct TestedFeed {
    pub title: String,
    pub site_name: String,
    pub body: Vec<u8>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

/// Redirect chains in flight, keyed by the first URL of each chain.
///
/// A `std` mutex is the right one here: it is only held for the few statements
/// that touch the map, never across an `await`, and the closure required by
/// `Policy::custom` is synchronous and must be `Send + Sync + 'static`.
type RedirectTracker = Arc<Mutex<HashMap<String, RedirectTrack>>>;

/// Builds the client used for feed polling, whose redirect policy records where
/// a feed was permanently moved to.
///
/// A dedicated client is needed because this policy must not affect the clients
/// used for article scraping and web feeds, where following redirects silently
/// is the correct behaviour.
fn build_feed_client(tracker: RedirectTracker) -> Result<Client, String> {
    let policy = redirect::Policy::custom(move |attempt| {
        // Each chain is keyed by its first URL, which is the feed URL itself:
        // `feeds.url` is UNIQUE, so the key stays unambiguous even with several
        // feeds redirecting in parallel on the same client.
        if let Some(first) = attempt.previous().first() {
            let key = first.to_string();
            let status = attempt.status();
            let next = attempt.url().to_string();

            // A poisoned mutex only costs the URL rewrite, never the fetch,
            // so the error is swallowed instead of failing the request.
            if let Ok(mut map) = tracker.lock() {
                let track = map.entry(key).or_insert(RedirectTrack {
                    candidate: None,
                    permanent_only: true,
                });
                if track.permanent_only
                    && (status == StatusCode::MOVED_PERMANENTLY
                        || status == StatusCode::PERMANENT_REDIRECT)
                {
                    track.candidate = Some(next);
                } else {
                    track.permanent_only = false;
                }
            }
        }

        // Delegating the decision to the default policy keeps its 10-hop limit:
        // a custom policy is not loop-bounded on its own, as the documentation
        // of `Policy::custom` states explicitly.
        redirect::Policy::default().redirect(attempt)
    });

    Client::builder()
        .redirect(policy)
        .build()
        .map_err(|e| format!("Failed to build the feed HTTP client: {}", e))
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

/// Reads a response header as an owned string, keeping `None` when the header
/// is absent. That distinction decides whether a stored validator survives, so
/// the `Option` must never be flattened into an empty string along the way.
fn header_string(
    response: &reqwest::Response,
    name: reqwest::header::HeaderName,
) -> Option<String> {
    response.headers().get(name)?.to_str().ok().map(str::to_string)
}

/// Whether a request error means the server could not be reached at all
/// (timeout or failed connection), which auto-pauses the feed. Checked on the
/// error kind, since reqwest's message never names the cause.
pub(crate) fn is_unreachable(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect()
}

/// Parses `Retry-After`, allowed in two forms: a delta in seconds (`3600`) or
/// an HTTP-date (`Wed, 29 Jul 2026 14:00:00 GMT`), from which the delta is
/// computed. Dates already in the past collapse to zero.
///
/// Only the IMF-fixdate form is handled, which is what current servers send;
/// the two obsolete HTTP date formats are not, and fall back to no delay.
fn parse_retry_after(value: &str) -> Option<u64> {
    let value = value.trim();

    if let Ok(seconds) = value.parse::<u64>() {
        return Some(seconds);
    }

    let target = chrono::DateTime::parse_from_rfc2822(value).ok()?;
    let delta = target.with_timezone(&Utc) - Utc::now();
    Some(delta.num_seconds().max(0) as u64)
}

/// Formats a retry delay for terminal output: minutes up to two hours, hours
/// above that, since `Retry-After` values of several hours are not unusual.
/// Rounded up with a floor of one, so a short delay never reads as zero.
fn format_retry_delay(seconds: u64, italian: bool) -> String {
    let minutes = seconds.div_ceil(60).max(1);
    if minutes <= 120 {
        if italian {
            format!("{} minuti", minutes)
        } else {
            format!("{} minutes", minutes)
        }
    } else {
        let hours = minutes.div_ceil(60);
        if italian {
            format!("{} ore", hours)
        } else {
            format!("{} hours", hours)
        }
    }
}

/// Oldest date worth keeping, derived from the `retention_hours` setting.
/// Shared by the periodic refresh and by the import of a feed that was just
/// tested, so the two cannot drift apart on the default value.
fn retention_cutoff(db: &Database) -> chrono::DateTime<Utc> {
    let hours = db
        .get_setting("retention_hours")
        .ok()
        .flatten()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(24);

    Utc::now() - chrono::Duration::hours(hours)
}

/// Fetches a single RSS feed with a conditional GET and reports what the server
/// answered. `start_from` is the earliest date to accept articles from —
/// anything older is skipped to avoid unnecessary work.
async fn fetch_single_feed(
    client: &Client,
    feed: &Feed,
    start_from: chrono::DateTime<Utc>,
    tracker: &RedirectTracker,
) -> Result<FeedResponse, String> {
    // The redirect policy keys chains by the URL as reqwest normalized it, so
    // the same normalization is applied here to find the entry again.
    let tracker_key = Url::parse(&feed.url)
        .map(|u| u.to_string())
        .unwrap_or_else(|_| feed.url.clone());

    let mut request = client
        .get(&feed.url)
        .header("User-Agent", FEED_USER_AGENT)
        // Same accept header of miniflux
        .header(
            "Accept",
            "application/xml,application/atom+xml,application/rss+xml,application/rdf+xml,application/feed+json,text/html,*/*;q=0.9",
        )
        .timeout(Duration::from_secs(30));

    // Conditional headers. Both values go back exactly as they arrived: they
    // are opaque strings, so quotes and the weak `W/` prefix stay in place.
    if !feed.etag.is_empty() {
        request = request.header("If-None-Match", &feed.etag);
    }
    if !feed.last_modified.is_empty() {
        request = request.header("If-Modified-Since", &feed.last_modified);
    }
    // Accept-Encoding is deliberately not set: reqwest adds it and decompresses
    // the body only as long as the request does not already carry one.

    let response = request.send().await;

    // The tracking entry is consumed here, on success and on failure alike, so
    // the shared map cannot grow across refreshes.
    let canonical_url = tracker
        .lock()
        .ok()
        .and_then(|mut map| map.remove(&tracker_key))
        .and_then(|track| track.candidate);

    // An unreachable server is reported as an outcome rather than an error,
    // because the feed gets paused exactly as for a 410 or a 429
    let response = match response {
        Ok(response) => response,
        Err(e) if is_unreachable(&e) => {
            return Ok(FeedResponse {
                outcome: FeedOutcome::Paused(PauseReason::Unreachable),
                canonical_url: None,
            });
        }
        Err(e) => return Err(format!("HTTP request failed for {}: {}", feed.url, e)),
    };

    let status = response.status();

    // 304: nothing changed, and the validators we sent are still valid.
    if status == StatusCode::NOT_MODIFIED {
        return Ok(FeedResponse {
            outcome: FeedOutcome::NotModified,
            canonical_url,
        });
    }

    // 410: the feed is gone for good.
    if status == StatusCode::GONE {
        return Ok(FeedResponse {
            outcome: FeedOutcome::Paused(PauseReason::Gone),
            canonical_url,
        });
    }

    // 429: we are polling too often.
    if status == StatusCode::TOO_MANY_REQUESTS {
        let retry_after = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(parse_retry_after);
        return Ok(FeedResponse {
            outcome: FeedOutcome::Paused(PauseReason::TooManyRequests(retry_after)),
            canonical_url,
        });
    }

    // Any other failing status used to reach feed-rs as a body it cannot parse,
    // surfacing as a misleading parse error. It is now reported for what it is.
    if !status.is_success() {
        return Err(format!("Feed {} answered with HTTP {}", feed.url, status));
    }

    // Read while the response is still whole: `bytes()` consumes it.
    let etag = header_string(&response, reqwest::header::ETAG);
    let last_modified = header_string(&response, reqwest::header::LAST_MODIFIED);

    // The timeout covers the body too: a server too slow to send it is
    // treated like one that never answered
    let body = match response.bytes().await {
        Ok(body) => body,
        Err(e) if is_unreachable(&e) => {
            return Ok(FeedResponse {
                outcome: FeedOutcome::Paused(PauseReason::Unreachable),
                canonical_url: None,
            });
        }
        Err(e) => {
            return Err(format!("Failed to read response body from {}: {}", feed.url, e));
        }
    };

    // Some feeds declare a non-UTF-8 encoding in the XML header (e.g. ISO-8859-15)
    // but the HTTP server serves them as UTF-8, causing mojibake.
    // Detect and fix this by checking the XML declaration and re-encoding if needed.
    let body = fix_feed_encoding(&body);

    let articles = parse_articles(
        &body,
        feed.id,
        &feed.url,
        feed.force_ai_summary,
        start_from,
    )?;

    Ok(FeedResponse {
        outcome: FeedOutcome::Updated {
            articles,
            etag,
            last_modified,
        },
        canonical_url,
    })
}

/// Turns a feed body into the articles worth keeping, skipping entries with no
/// link and anything published before `start_from`.
///
/// Takes the three fields it actually needs rather than the whole `Feed`, so it
/// can also be called right after a feed row is created, when no `Feed` has
/// been read back from the database yet.
fn parse_articles(
    body: &[u8],
    feed_id: i64,
    feed_url: &str,
    force_ai_summary: bool,
    start_from: chrono::DateTime<Utc>,
) -> Result<Vec<Article>, String> {
    let parsed = parser::parse(body)
        .map_err(|e| format!("Failed to parse feed {}: {}", feed_url, e))?;

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

        // Strip HTML tags from the first author name and decode HTML entities.
        // Since feed-rs 3 a person may have no name (e.g. only an email), so the
        // first author that actually has one is taken
        let author = entry
            .authors
            .iter()
            .find_map(|a| a.name.as_deref())
            .map(strip_html_and_decode)
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
            force_ai_summary,
        );

        // Feeds that publish the whole article (RSS content:encoded, Atom
        // content) already hand us what the scraper would otherwise go and
        // fetch from the origin site. Using it saves that request and makes the
        // full text — and therefore the AI summary — available immediately,
        // even on feeds where scraping is disabled.
        let full_text = entry
            .content
            .and_then(|c| c.body)
            .map(|body| strip_html_and_decode(&body))
            .filter(|text| text.len() >= FEED_CONTENT_MIN_LEN && text.len() > summary_text.len())
            .unwrap_or_default();

        articles.push(Article {
            id: Uuid::new_v4().to_string(),
            feed_id,
            url: article_url,
            title,
            author,
            published_at,
            full_text,
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
            pause_reason: None,
            retry_after_seconds: None,
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

    let retention_cutoff = retention_cutoff(db);

    // Feed polling gets its own client so its redirect policy cannot affect
    // the connectivity check above, which would otherwise leave entries for
    // unrelated hosts in the tracker.
    let redirect_tracker: RedirectTracker = Arc::new(Mutex::new(HashMap::new()));
    let feed_client = match build_feed_client(Arc::clone(&redirect_tracker)) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("{}", e);
            return vec![];
        }
    };

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
        // Skip newsletter containers: the "newsletter://" URL is internal and
        // groups articles imported from EML files, there is nothing to fetch
        if feed.url.starts_with("newsletter://") {
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
        let client = feed_client.clone();
        let feed = feed.clone();
        let semaphore = std::sync::Arc::clone(&semaphore);
        let tracker = Arc::clone(&redirect_tracker);
        join_set.spawn(async move {
            // Limit the number of simultaneous HTTP fetches
            let _permit = semaphore.acquire_owned().await;
            let fetch_result = fetch_single_feed(&client, &feed, start_from, &tracker).await;
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

        let italian = crate::is_system_italian();

        // A permanently moved feed is followed silently: the URL is rewritten
        // and the user only learns about it from the terminal. This runs before
        // the validators are saved, because relocating clears them.
        if let Ok(ref fetched) = fetch_result
            && let Some(ref new_url) = fetched.canonical_url
        {
            match db.relocate_feed_url(feed.id, new_url) {
                Ok(_) => {
                    if italian {
                        tracing::info!(
                            "Feed '{}' spostato a questo link: {}",
                            feed.site_name, new_url
                        );
                    } else {
                        tracing::info!(
                            "Moved feed '{}' to this link: {}",
                            feed.site_name, new_url
                        );
                    }
                }
                Err(e) => {
                    // `feeds.url` is UNIQUE: the target is already subscribed as
                    // a separate feed, so the original URL is left untouched.
                    if italian {
                        tracing::warn!(
                            "Impossibile spostare il feed '{}' a {}: {}",
                            feed.site_name, new_url, e
                        );
                    } else {
                        tracing::warn!(
                            "Cannot move feed '{}' to {}: {}",
                            feed.site_name, new_url, e
                        );
                    }
                }
            }
        }

        let result = match fetch_result {
            Ok(FeedResponse {
                outcome: FeedOutcome::Updated { articles, etag, last_modified },
                ..
            }) => {
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

                // Stored after the articles: a validator saved first would mark
                // the feed as up to date even if every insert had failed.
                if let Err(e) = db.set_feed_validators(
                    feed.id,
                    etag.as_deref(),
                    last_modified.as_deref(),
                ) {
                    tracing::warn!("Failed to save validators for feed {}: {}", feed.id, e);
                }

                if italian {
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
                    pause_reason: None,
                    retry_after_seconds: None,
                }
            }
            Ok(FeedResponse { outcome: FeedOutcome::NotModified, .. }) => {
                if italian {
                    tracing::info!("Feed '{}': nessun nuovo articolo nel feed", feed.site_name);
                } else {
                    tracing::info!("Feed '{}': no new articles in the feed", feed.site_name);
                }

                FetchResult {
                    feed_id: feed.id,
                    feed_name: feed.site_name.clone(),
                    articles_found: 0,
                    articles_new: 0,
                    error: None,
                    pause_reason: None,
                    retry_after_seconds: None,
                }
            }
            Ok(FeedResponse { outcome: FeedOutcome::Paused(reason), .. }) => {
                if let Err(e) = db.set_feed_paused(feed.id, true) {
                    tracing::error!("Failed to auto-pause feed {}: {}", feed.id, e);
                }

                let (pause_reason, retry_after_seconds) = match reason {
                    PauseReason::Gone => {
                        if italian {
                            tracing::warn!(
                                "Feed '{}': non esiste più, messo in pausa. Per sbloccarlo, vai nella pagina dei feed.",
                                feed.site_name
                            );
                        } else {
                            tracing::warn!(
                                "Feed '{}': no longer exists, feed is paused. To unlock it, go to the feed page.",
                                feed.site_name
                            );
                        }
                        ("pause_reason_gone", None)
                    }
                    PauseReason::Unreachable => {
                        if italian {
                            tracing::warn!(
                                "Feed '{}': irraggiungibile (timeout o connessione fallita), messo in pausa. Per sbloccarlo, vai nella pagina dei feed.",
                                feed.site_name
                            );
                        } else {
                            tracing::warn!(
                                "Feed '{}': unreachable (timeout or failed connection), feed is paused. To unlock it, go to the feed page.",
                                feed.site_name
                            );
                        }
                        ("pause_reason_timeout", None)
                    }
                    PauseReason::TooManyRequests(retry_after) => {
                        match retry_after.map(|s| format_retry_delay(s, italian)) {
                            Some(wait) if italian => tracing::warn!(
                                "Feed '{}': troppe richieste, messo in pausa. Per sbloccarlo, vai nella pagina dei feed. Prova a farlo tra {}.",
                                feed.site_name, wait
                            ),
                            Some(wait) => tracing::warn!(
                                "Feed '{}': too many requests, feed is paused. To unlock it, go to the feed page. Try to do this after {}.",
                                feed.site_name, wait
                            ),
                            None if italian => tracing::warn!(
                                "Feed '{}': troppe richieste, messo in pausa. Per sbloccarlo, vai nella pagina dei feed.",
                                feed.site_name
                            ),
                            None => tracing::warn!(
                                "Feed '{}': too many requests, feed is paused. To unlock it, go to the feed page.",
                                feed.site_name
                            ),
                        }

                        let key = if retry_after.is_some() {
                            "pause_reason_too_many_retry"
                        } else {
                            "pause_reason_too_many"
                        };
                        (key, retry_after)
                    }
                };

                FetchResult {
                    feed_id: feed.id,
                    feed_name: feed.site_name.clone(),
                    articles_found: 0,
                    articles_new: 0,
                    error: None,
                    pause_reason: Some(pause_reason),
                    retry_after_seconds,
                }
            }
            Err(e) => {
                // Unreachable servers never get here: they come back as
                // FeedOutcome::Paused, so every remaining error leaves the feed active
                tracing::warn!("Failed to fetch feed '{}': {}", feed.url, e);

                FetchResult {
                    feed_id: feed.id,
                    feed_name: feed.site_name.clone(),
                    articles_found: 0,
                    articles_new: 0,
                    error: Some(e),
                    pause_reason: None,
                    retry_after_seconds: None,
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
            pause_reason: wr.auto_paused.then_some("pause_reason_timeout"),
            retry_after_seconds: None,
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
pub async fn test_feed_url(url: &str) -> Result<TestedFeed, String> {
    let client = Client::new();

    let response = client
        .get(url)
        .header("User-Agent", FEED_USER_AGENT)
        .header(
            "Accept",
            "application/xml,application/atom+xml,application/rss+xml,application/rdf+xml,application/feed+json,text/html,*/*;q=0.9",
        )
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    // Read before `bytes()` consumes the response. Keeping these means the
    // first poll of the feed can be conditional instead of a second full
    // download of what we are about to parse here.
    let etag = header_string(&response, reqwest::header::ETAG);
    let last_modified = header_string(&response, reqwest::header::LAST_MODIFIED);

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

    Ok(TestedFeed {
        title,
        site_name,
        body,
        etag,
        last_modified,
    })
}

/// Stores the articles of a feed that was just added, reusing the body already
/// downloaded by `test_feed_url` instead of fetching the same URL again.
///
/// Returns how many articles were actually new. The validators are saved too,
/// so the first scheduled poll is already conditional.
pub fn import_tested_feed(
    db: &Database,
    feed_id: i64,
    feed_url: &str,
    tested: &TestedFeed,
) -> Result<usize, String> {
    // A feed created moments ago cannot have AI summaries forced on yet: that
    // flag has its own endpoint and defaults to off in the schema.
    let articles = parse_articles(
        &tested.body,
        feed_id,
        feed_url,
        false,
        retention_cutoff(db),
    )?;

    let mut new_count = 0;
    for article in &articles {
        match db.insert_article(article) {
            Ok(true) => new_count += 1,
            Ok(false) => {}
            Err(e) => tracing::warn!("Failed to insert article '{}': {}", article.url, e),
        }
    }

    // Saved after the articles, so a failed insert cannot leave the feed
    // marked as up to date with nothing stored.
    if let Err(e) = db.set_feed_validators(
        feed_id,
        tested.etag.as_deref(),
        tested.last_modified.as_deref(),
    ) {
        tracing::warn!("Failed to save validators for feed {}: {}", feed_id, e);
    }

    Ok(new_count)
}