pub mod encryption;
use reqwest::{Client, RequestBuilder};

/// Supported LLM API providers.
/// Each provider has its own endpoint URL and authentication format.
#[derive(Debug, Clone, PartialEq)]
pub enum LlmProvider {
    Mistral500Chars,
    Mistral1000Chars,
    OpenAI,
    Claude,
    Gemini,
}

impl LlmProvider {
    /// Parses a provider string from the database into the enum variant.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "mistral_500_chars" => Some(Self::Mistral500Chars),
            "mistral_1000_chars" => Some(Self::Mistral1000Chars),
            // Legacy values written by versions that named the Mistral providers
            // after the underlying models, kept so existing databases keep working
            "mistral_small" => Some(Self::Mistral500Chars),
            "mistral_large" => Some(Self::Mistral1000Chars),
            "openai" => Some(Self::OpenAI),
            "claude" => Some(Self::Claude),
            "gemini" => Some(Self::Gemini),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mistral500Chars => "mistral_500_chars",
            Self::Mistral1000Chars => "mistral_1000_chars",
            Self::OpenAI => "openai",
            Self::Claude => "claude",
            Self::Gemini => "gemini",
        }
    }

    /// Returns the text generation endpoint URL for the provider.
    /// Every provider takes the model name in the request body, so the
    /// endpoint is a fixed URL and `default_model()` stays the single source
    /// of truth for the model.
    pub fn chat_endpoint(&self) -> &'static str {
        match self {
            Self::Mistral500Chars | Self::Mistral1000Chars => {
                "https://api.mistral.ai/v1/chat/completions"
            }
            // OpenAI Responses API: the current-generation endpoint, successor of
            // Chat Completions, used here for every OpenAI model
            Self::OpenAI => "https://api.openai.com/v1/responses",
            Self::Claude => "https://api.anthropic.com/v1/messages",
            // Gemini Interactions API: GA since June 2026 and recommended for new
            // projects, it supersedes the legacy generateContent endpoint
            Self::Gemini => "https://generativelanguage.googleapis.com/v1beta/interactions",
        }
    }

    /// Returns the default model name for each provider.
    /// Models are chosen for the task at hand (short article summaries):
    /// fast, low-cost tiers are preferred over frontier models.
    pub fn default_model(&self) -> &'static str {
        match self {
            // Ministral 3 8B: same API and key as the Mistral models, but not
            // aliased to the Vibe CLI models, so it is not capped by the Vibe budget
            Self::Mistral500Chars => "ministral-8b-latest",
            // Ministral 3 14B: top tier of the same family, used for longer summaries
            Self::Mistral1000Chars => "ministral-14b-latest",
            // GPT-6 Luna: lowest-cost tier of the GPT-6 family, built for focused
            // high-volume tasks such as short summaries ($0.10/$0.50 per MTok at launch)
            Self::OpenAI => "gpt-6-luna",
            // Claude Haiku 5.5: fastest and lowest-cost Claude tier, ideal for
            // simple summaries ($0.10/$0.50 per MTok for prompts up to 100K tokens)
            Self::Claude => "claude-haiku-5-5",
            // Choose to pass to free Flash Lite version
            Self::Gemini => "gemini-3.5-flash-lite",
        }
    }

    /// Returns the usage/billing endpoint if available for this provider.
    /// Some providers don't expose this via API.
    pub fn usage_endpoint(&self) -> Option<&'static str> {
        match self {
            Self::Mistral500Chars | Self::Mistral1000Chars => None,
            // Now OpenAI's usage endpoint requires an admin key and date parameters,
            // so it always fails with a regular API key; ask the model instead
            Self::OpenAI => None,
            Self::Claude => None,
            Self::Gemini => None,
        }
    }

    /// Whether this provider supports querying plan/usage info.
    /// If false, the "Plan info" button will be hidden in the UI.
    pub fn supports_usage_query(&self) -> bool {
        // Even without a dedicated endpoint, we can ask the model itself.
        // Only return false if neither approach works.
        match self {
            Self::Mistral500Chars | Self::Mistral1000Chars => true,
            Self::OpenAI => true,
            Self::Claude => true,
            Self::Gemini => true,
        }
    }
}

/// Supported web rendering API providers for scraping JS-rendered pages.
/// Each provider offers a proxy endpoint that renders JavaScript before returning HTML.
#[derive(Debug, Clone, PartialEq)]
pub enum WebRenderingProvider {
    ScrapeDo,
    ScraperApi,
    ScrapingBee,
    Scrapfly,
    ZenRows,
}

impl WebRenderingProvider {
    /// Parses a provider string from the database into the enum variant.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "scrape_do" => Some(Self::ScrapeDo),
            "scraper_api" => Some(Self::ScraperApi),
            "scrapingbee" => Some(Self::ScrapingBee),
            "scrapfly" => Some(Self::Scrapfly),
            "zenrows" => Some(Self::ZenRows),
            _ => None,
        }
    }

    /// Returns the database-stored string identifier for this provider.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ScrapeDo => "scrape_do",
            Self::ScraperApi => "scraper_api",
            Self::ScrapingBee => "scrapingbee",
            Self::Scrapfly => "scrapfly",
            Self::ZenRows => "zenrows",
        }
    }

    /// Builds the request that fetches the target page through the rendering API.
    /// Each provider uses different parameter names for the API key, target URL
    /// and JS rendering toggle. The key travels as a query parameter, except for
    /// ScrapingBee, which has deprecated that form in favor of a Bearer token.
    pub fn render_request(&self, client: &Client, api_key: &str, target_url: &str) -> RequestBuilder {
        let encoded_url = urlencoded(target_url);
        match self {
            Self::ScrapeDo => client.get(format!(
                "https://api.scrape.do/?token={}&render=true&url={}",
                urlencoded(api_key), encoded_url
            )),
            Self::ScraperApi => client.get(format!(
                "https://api.scraperapi.com?api_key={}&render=true&url={}",
                urlencoded(api_key), encoded_url
            )),
            Self::ScrapingBee => client
                .get(format!(
                    "https://app.scrapingbee.com/api/v1?render_js=true&url={}",
                    encoded_url
                ))
                .bearer_auth(api_key),
            // proxified_response returns the page itself: without it Scrapfly
            // wraps the HTML in a JSON envelope under result.content
            Self::Scrapfly => client.get(format!(
                "https://api.scrapfly.io/scrape?key={}&render_js=true&proxified_response=true&url={}",
                urlencoded(api_key), encoded_url
            )),
            Self::ZenRows => client.get(format!(
                "https://api.zenrows.com/v1/?apikey={}&js_render=true&url={}",
                urlencoded(api_key), encoded_url
            )),
        }
    }

    /// Builds the request used to validate the API key.
    /// Every provider exposes a free account/usage endpoint, so the check
    /// costs no credits, unlike a rendered test scrape.
    pub fn test_request(&self, client: &Client, api_key: &str) -> RequestBuilder {
        match self {
            // Capped at 10 calls per minute, far above what a key test needs
            Self::ScrapeDo => client.get(format!(
                "https://api.scrape.do/info?token={}",
                urlencoded(api_key)
            )),
            Self::ScraperApi => client.get(format!(
                "https://api.scraperapi.com/account?api_key={}",
                urlencoded(api_key)
            )),
            Self::ScrapingBee => client
                .get("https://app.scrapingbee.com/api/v1/usage")
                .bearer_auth(api_key),
            Self::Scrapfly => client.get(format!(
                "https://api.scrapfly.io/account?key={}",
                urlencoded(api_key)
            )),
            // This endpoint accepts the key only as a header, not as a query parameter
            Self::ZenRows => client
                .get("https://api.zenrows.com/v1/subscriptions/self/details")
                .header("X-API-Key", api_key),
        }
    }
}

/// Minimal percent-encoding for URL query parameter values.
/// Encodes characters that are unsafe in query strings.
fn urlencoded(input: &str) -> String {
    let mut result = String::with_capacity(input.len() * 2);
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
            | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push('%');
                result.push_str(&format!("{:02X}", byte));
            }
        }
    }
    result
}