//! Newsletter import module.
//! Parses EML files and creates articles from newsletter emails, one file at a
//! time or as a whole folder in a single batch.

use mail_parser::{Message, MessageParser};
use serde::Serialize;
use uuid::Uuid;
use chrono::Utc;

use crate::db::{Database, Article};
use scraper::Html;

/// Parses a single EML file and imports it as an article.
/// The title is decided by the caller, which reads it from the preview endpoint.
///
/// The bytes must be the untouched file content, for the same reason as the
/// batch path: decoding them as text beforehand corrupts every message whose
/// MIME headers declare an encoding other than UTF-8.
///
/// Returns the article ID on success.
pub fn import_eml(
    eml_bytes: &[u8],
    title: &str,
    feed_id: i64,
    db: &Database,
) -> Result<String, String> {
    let message = MessageParser::default()
        .parse(eml_bytes)
        .ok_or("Failed to parse EML file. The file may be corrupted or not a valid email.")?;

    let article = build_article(&message, title, feed_id)?;
    let article_id = article.id.clone();

    match db.insert_article(&article) {
        Ok(true) => {
            // Ensure it's marked as favorite (insert_article doesn't set is_favorite)
            let _ = db.set_article_favorite(&article_id, true);
            Ok(article_id)
        }
        Ok(false) => Err("newsletter_duplicate".to_string()),
        Err(e) => Err(format!("Failed to save newsletter: {}", e)),
    }
}

/// Outcome of a bulk import, one counter per possible fate of a file.
/// The four counters always add up to the number of files received, so the
/// caller can report the whole batch without recounting anything.
#[derive(Debug, Serialize)]
pub struct BulkImportOutcome {
    pub imported: u32,
    pub duplicates: u32,
    pub no_html: u32,
    pub invalid: u32,
}

/// Imports every EML of a batch under a single newsletter feed.
///
/// Files are parsed and classified first, then the surviving articles are
/// written in one transaction: a folder of several hundred newsletters costs
/// one commit instead of one per file.
///
/// A file that cannot be parsed, or that carries no HTML part, is counted and
/// skipped: a single bad file never aborts the rest of the folder. The only
/// error returned is a database failure, which invalidates the whole batch.
///
/// `files` pairs each file name with its raw bytes. The bytes must be the
/// untouched file content: decoding them as text beforehand corrupts every
/// message whose MIME headers declare something other than UTF-8.
///
/// This function is CPU-bound (it runs Readability on every file) and blocks:
/// the caller must run it outside the async executor.
pub fn import_eml_batch<B: AsRef<[u8]>>(
    files: &[(String, B)],
    feed_id: i64,
    db: &Database,
) -> Result<BulkImportOutcome, String> {
    let mut articles = Vec::with_capacity(files.len());
    let mut no_html = 0u32;
    let mut invalid = 0u32;

    for (file_name, bytes) in files {
        let Some(message) = MessageParser::default().parse(bytes.as_ref()) else {
            invalid += 1;
            continue;
        };

        // The subject is the natural title; a message without a usable one
        // falls back to the file name, the same rule the single-file page
        // already applies in the browser.
        let title = match message.subject() {
            Some(subject) if !subject.trim().is_empty() => subject.trim().to_string(),
            _ => title_from_file_name(file_name),
        };

        match build_article(&message, &title, feed_id) {
            Ok(article) => articles.push(article),
            Err(_) => no_html += 1,
        }
    }

    let inserted = db
        .insert_newsletter_articles(&articles)
        .map_err(|e| format!("Failed to save newsletters: {}", e))?;

    Ok(BulkImportOutcome {
        imported: inserted,
        // Whatever reached the insert and did not land was rejected by the URL
        // uniqueness constraint, i.e. it had already been imported before, or
        // appears twice inside this same folder.
        duplicates: articles.len() as u32 - inserted,
        no_html,
        invalid,
    })
}

/// Builds the article row from a parsed email message.
/// Shared by the single-file and the bulk paths, so both store exactly the same
/// fields no matter how the EML reached the program.
///
/// Fails with `newsletter_no_html` when the message carries no usable HTML
/// part: the HTML is what the reader displays, so a message without it would
/// be stored as an empty article.
fn build_article(
    message: &Message<'_>,
    title: &str,
    feed_id: i64,
) -> Result<Article, String> {
    let html_body = message
        .body_html(0)
        .map(|s| s.to_string())
        .ok_or("newsletter_no_html")?;

    if html_body.trim().is_empty() {
        return Err("newsletter_no_html".to_string());
    }

    // Prefer the display name of the sender, fall back to the bare address
    let author = message
        .from()
        .and_then(|addrs| addrs.first())
        .map(|addr| {
            addr.name()
                .map(|n| n.to_string())
                .unwrap_or_else(|| {
                    addr.address()
                        .map(|a| a.to_string())
                        .unwrap_or_default()
                })
        })
        .unwrap_or_default();

    // Convert the mail-parser DateTime to RFC 3339
    let published_at = message
        .date()
        .map(|d| {
            format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}+00:00",
                d.year, d.month, d.day, d.hour, d.minute, d.second
            )
        })
        .unwrap_or_else(|| Utc::now().to_rfc3339());

    // The Message-ID is what makes the article unique: a message without one
    // gets a random URL and can therefore be imported twice.
    let message_id = message
        .message_id()
        .map(|id| format!("newsletter://{}", id))
        .unwrap_or_else(|| format!("newsletter://{}", Uuid::new_v4()));

    Ok(Article {
        id: Uuid::new_v4().to_string(),
        feed_id,
        url: message_id,
        title: title.to_string(),
        author,
        published_at: Some(published_at),
        full_text: extract_text_from_html(&html_body),
        summary: String::new(),
        summary_source: "none".to_string(),
        is_favorite: true,
        fetched_at: Utc::now().to_rfc3339(),
        newsletter_html: html_body,
    })
}

/// Derives a title from an uploaded file name: keeps the last path segment and
/// drops a trailing `.eml`, case-insensitively.
///
/// The name comes from the client and is deliberately used as text only, never
/// to touch the filesystem, so a name carrying directory separators is harmless
/// and simply gets trimmed down to its final segment.
fn title_from_file_name(file_name: &str) -> String {
    let name = file_name.rsplit(['/', '\\']).next().unwrap_or(file_name);

    let Some(cut) = name.len().checked_sub(4) else {
        return name.to_string();
    };

    match name.get(cut..) {
        Some(ext) if ext.eq_ignore_ascii_case(".eml") => name[..cut].to_string(),
        _ => name.to_string(),
    }
}


/// Extracts readable text from HTML content using Mozilla's Readability algorithm.
/// Falls back to simple text extraction if Readability fails.
fn extract_text_from_html(html: &str) -> String {
    // Try Readability first for clean content extraction
    if let Ok(readability) = readabilityrs::Readability::new(html, None, None)
        && let Some(article) = readability.parse()
        && let Some(content_html) = article.content {
        let document = Html::parse_document(&content_html);
        let text: String = document.root_element().text().collect::<Vec<_>>().join(" ");
        let cleaned = clean_whitespace(&text);
        if cleaned.len() > 50 {
            return cleaned;
        }
    }

    // Fallback: simple text extraction
    let document = Html::parse_document(html);
    let text: String = document.root_element().text().collect::<Vec<_>>().join(" ");
    clean_whitespace(&text)
}

/// Cleans up whitespace in extracted text.
fn clean_whitespace(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut prev_was_space = false;
    for ch in text.chars() {
        match ch {
            ' ' | '\t' | '\r' => {
                if !prev_was_space {
                    result.push(' ');
                    prev_was_space = true;
                }
            }
            '\n' => {
                if !prev_was_space {
                    result.push('\n');
                    prev_was_space = true;
                }
            }
            _ => {
                result.push(ch);
                prev_was_space = false;
            }
        }
    }
    result.trim().to_string()
}