//! Newsletter import module.
//! Parses EML files and creates articles from newsletter emails.

use mail_parser::MessageParser;
use uuid::Uuid;
use chrono::Utc;

use crate::db::{Database, Article};
use scraper::Html;

/// Parses an EML file content and imports it as an article.
/// Returns the article ID on success.
pub fn import_eml(
    eml_content: &str,
    title: &str,
    feed_id: i64,
    db: &Database,
) -> Result<String, String> {
    // Parse the EML file
    let message = MessageParser::default()
        .parse(eml_content.as_bytes())
        .ok_or("Failed to parse EML file. The file may be corrupted or not a valid email.")?;

    // Extract HTML content
    let html_body = message
        .body_html(0)
        .map(|s| s.to_string())
        .ok_or("newsletter_no_html")?;

    if html_body.trim().is_empty() {
        return Err("newsletter_no_html".to_string());
    }

    // Extract author from the From field
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

    // Extract date from the email
    let published_at = message
        .date()
        .map(|d| {
            // Convert mail-parser DateTime to RFC 3339
            format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}+00:00",
                d.year, d.month, d.day, d.hour, d.minute, d.second
            )
        })
        .unwrap_or_else(|| Utc::now().to_rfc3339());

    // Extract Message-ID for deduplication
    let message_id = message
        .message_id()
        .map(|id| format!("newsletter://{}", id))
        .unwrap_or_else(|| format!("newsletter://{}", Uuid::new_v4()));

    // Extract plain text from the HTML for the full_text field
    let full_text = extract_text_from_html(&html_body);

    let article_id = Uuid::new_v4().to_string();

    let article = Article {
        id: article_id.clone(),
        feed_id,
        url: message_id,
        title: title.to_string(),
        author,
        published_at: Some(published_at),
        full_text,
        summary: String::new(),
        summary_source: "none".to_string(),
        is_favorite: true,
        fetched_at: Utc::now().to_rfc3339(),
        newsletter_html: html_body,
    };

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

/// Extracts readable text from HTML content using Mozilla's Readability algorithm.
/// Falls back to simple text extraction if Readability fails.
fn extract_text_from_html(html: &str) -> String {
    // Try Readability first for clean content extraction
    if let Ok(readability) = readabilityrs::Readability::new(html, None, None) {
        if let Some(article) = readability.parse() {
            if let Some(content_html) = article.content {
                let document = Html::parse_document(&content_html);
                let text: String = document.root_element().text().collect::<Vec<_>>().join(" ");
                let cleaned = clean_whitespace(&text);
                if cleaned.len() > 50 {
                    return cleaned;
                }
            }
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