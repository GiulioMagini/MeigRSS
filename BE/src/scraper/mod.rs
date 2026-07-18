use reqwest::Client;
use scraper::{Html, Selector};
use readabilityrs::Readability;

/// Shared HTTP client reused across all scraping calls.
/// Reusing the client enables connection pooling and avoids
/// rebuilding TLS state on every article fetch.
static HTTP_CLIENT: std::sync::OnceLock<Client> = std::sync::OnceLock::new();

/// Returns the process-wide shared HTTP client.
fn http_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(Client::new)
}

/// Fetches a web page and extracts the main readable content.
/// Uses Mozilla's Readability algorithm (via readabilityrs) as the primary strategy,
/// falling back to a manual CSS-selector-based approach if Readability fails.
pub async fn extract_article_text(url: &str) -> Result<String, String> {
    let client = http_client();

    let response = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
        .header("Accept-Language", "en-US,en;q=0.9,it;q=0.8")
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch article {}: {}", url, e))?;

    // Read the response as bytes with a hard size cap, then try UTF-8 decoding.
    // Some sites declare a wrong charset in headers, so we attempt
    // UTF-8 first (most common), then fall back to Latin-1 re-encoding.
    // The cap prevents arbitrarily large pages from exhausting memory.
    const MAX_BODY_BYTES: usize = 10 * 1024 * 1024; // 10 MB

    if let Some(len) = response.content_length() {
        if len as usize > MAX_BODY_BYTES {
            return Err(format!("Article body too large ({} bytes) from {}", len, url));
        }
    }

    let mut response = response;
    let mut bytes: Vec<u8> = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| format!("Failed to read article body from {}: {}", url, e))?
    {
        if bytes.len() + chunk.len() > MAX_BODY_BYTES {
            return Err(format!("Article body too large (over {} bytes) from {}", MAX_BODY_BYTES, url));
        }
        bytes.extend_from_slice(&chunk);
    }

    let html = match String::from_utf8(bytes.to_vec()) {
        Ok(text) => text,
        Err(_) => {
            // Fallback: treat as Latin-1 and convert to UTF-8
            bytes.iter().map(|&b| b as char).collect::<String>()
        }
    };

    // Primary strategy: Mozilla Readability algorithm
    if let Some(text) = extract_with_readability(&html, url) {
        if text.len() > 50 {
            return Ok(clean_extracted_text(&text));
        }
    }

    // Fallback: manual CSS selector-based extraction
    let fallback = extract_readable_content(&html);
    if !fallback.is_empty() {
        return Ok(fallback);
    }

    Ok(String::new())
}

/// Extracts article text using Mozilla's Readability algorithm.
/// Returns None if the algorithm cannot find meaningful content.
fn extract_with_readability(html: &str, url: &str) -> Option<String> {
    let readability = Readability::new(html, Some(url), None).ok()?;
    let article = readability.parse()?;

    // Get the plain text content from the extracted HTML
    let content_html = article.content?;
    if content_html.is_empty() {
        return None;
    }

    // Convert the cleaned HTML to plain text by stripping remaining tags
    let document = Html::parse_document(&content_html);
    let text: String = document.root_element().text().collect::<Vec<_>>().join(" ");
    let text = text.trim().to_string();

    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// Parses raw HTML and extracts the main content text.
/// Uses a simple approach: find the best content container,
/// then collect all text nodes from it.
fn extract_readable_content(html: &str) -> String {
    let document = Html::parse_document(html);

    // Priority-ordered list of selectors for the main content area.
    let content_selectors = [
        "article",
        "main",
        "[role='main']",
        ".post-content",
        ".article-content",
        ".article-body",
        ".entry-content",
        ".story-body",
        "#article-body",
        ".body-text",
        ".paywall_body",
        ".content",
    ];

    // Try each selector in priority order
    for selector_str in &content_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            if let Some(element) = document.select(&selector).next() {
                let text = collect_text_simple(&element);
                if text.len() > 50 {
                    return text;
                }
            }
        }
    }

    // Fallback: extract from body
    if let Ok(body_selector) = Selector::parse("body") {
        if let Some(body) = document.select(&body_selector).next() {
            return collect_text_simple(&body);
        }
    }

    String::new()
}

/// Collects all visible text from an element, skipping script/style/nav/footer tags.
/// Uses scraper's built-in text iterator for reliability.
fn collect_text_simple(element: &scraper::ElementRef) -> String {
    // First, try to get text only from <p>, <h1>-<h6>, <li> tags inside the element
    // This avoids picking up menu items, buttons, etc.
    let content_tags = ["p", "h1", "h2", "h3", "h4", "h5", "h6", "li", "blockquote", "figcaption"];
    let mut paragraphs: Vec<String> = Vec::new();

    for tag in &content_tags {
        if let Ok(sel) = Selector::parse(tag) {
            for el in element.select(&sel) {
                // Skip elements inside nav, footer, aside, header
                let mut skip = false;
                let mut parent = el.parent();
                while let Some(p) = parent {
                    if let Some(p_el) = scraper::ElementRef::wrap(p) {
                        let tag_name = p_el.value().name();
                        if ["nav", "footer", "aside", "header", "script", "style", "noscript"].contains(&tag_name) {
                            skip = true;
                            break;
                        }
                        let classes = p_el.value().attr("class").unwrap_or("");
                        let id = p_el.value().attr("id").unwrap_or("");
                        let combined = format!("{} {}", classes, id).to_lowercase();
                        if ["sidebar", "nav", "menu", "footer", "header", "comment", "social", "share", "related", "advertisement", "cookie", "popup", "modal", "newsletter"]
                            .iter().any(|p| combined.contains(p)) {
                            skip = true;
                            break;
                        }
                    }
                    parent = p.parent();
                }

                if !skip {
                    let text: String = el.text().collect::<Vec<_>>().join(" ");
                    let text = text.trim().to_string();
                    if !text.is_empty() && text.len() > 3 {
                        paragraphs.push(text);
                    }
                }
            }
        }
    }

    let result = paragraphs.join("\n\n");
    clean_extracted_text(&result)
}

/// Cleans up extracted text: normalizes whitespace, removes
/// excessive newlines, and trims the result.
fn clean_extracted_text(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut prev_was_newline = false;
    let mut prev_was_space = false;

    for ch in text.chars() {
        match ch {
            '\n' | '\r' => {
                if !prev_was_newline {
                    result.push('\n');
                    prev_was_newline = true;
                    prev_was_space = false;
                }
            }
            ' ' | '\t' => {
                if !prev_was_space && !prev_was_newline {
                    result.push(' ');
                    prev_was_space = true;
                }
            }
            _ => {
                result.push(ch);
                prev_was_newline = false;
                prev_was_space = false;
            }
        }
    }

    // Sanitize any remaining HTML entities or tags
    let sanitized = ammonia::clean(&result);
    sanitized.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_from_article_tag() {
        let html = r#"
        <html>
        <body>
            <nav>Menu items here</nav>
            <article>
                <h1>Test Article Title</h1>
                <p>This is the first paragraph of the article with enough content to pass the length check. 
                It contains multiple sentences to simulate a real news article body text that would be 
                extracted by the scraper for summarization purposes.</p>
                <p>Second paragraph continues with more meaningful content about the topic at hand.</p>
            </article>
            <footer>Copyright 2024</footer>
        </body>
        </html>"#;

        let result = extract_readable_content(html);
        assert!(result.contains("first paragraph"));
        assert!(result.contains("Second paragraph"));
        assert!(!result.contains("Menu items"));
        assert!(!result.contains("Copyright"));
    }

    #[test]
    fn test_clean_whitespace() {
        let input = "Hello   world\n\n\n\nSecond   line";
        let result = clean_extracted_text(input);
        assert_eq!(result, "Hello world\nSecond line");
    }

    #[test]
    fn test_readability_extraction() {
        let html = r#"
        <html>
        <head><title>Test Article</title></head>
        <body>
            <nav><ul><li>Home</li><li>About</li></ul></nav>
            <div class="content">
                <h1>Test Article Title</h1>
                <p>This is a long paragraph that contains enough meaningful content
                to be detected by the Readability algorithm as the main article body.
                It discusses important topics and provides detailed information that
                a reader would find valuable and worth reading in its entirety.</p>
                <p>A second paragraph adds more depth to the article, covering related
                aspects of the topic and providing additional context that helps the
                reader understand the full picture of what is being discussed here.</p>
                <p>The third paragraph concludes with final thoughts and a summary
                of the key points that were made throughout this test article content.</p>
            </div>
            <footer>Copyright 2024 - All rights reserved</footer>
        </body>
        </html>"#;

        let result = extract_with_readability(html, "https://example.com/article");
        assert!(result.is_some(), "Readability should extract content");
        let text = result.unwrap();
        assert!(text.contains("important topics"), "Should contain article text");
        assert!(!text.contains("Copyright"), "Should not contain footer");
    }
    
}