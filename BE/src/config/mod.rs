pub mod encryption;

/// Supported LLM API providers.
/// Each provider has its own endpoint URL and authentication format.
#[derive(Debug, Clone, PartialEq)]
pub enum LlmProvider {
    MistralSmall,
    MistralLarge,
    OpenAI,
    Claude,
    Gemini,
}

impl LlmProvider {
    /// Parses a provider string from the database into the enum variant.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "mistral_small" => Some(Self::MistralSmall),
            "mistral_large" => Some(Self::MistralLarge),
            "openai" => Some(Self::OpenAI),
            "claude" => Some(Self::Claude),
            "gemini" => Some(Self::Gemini),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MistralSmall => "mistral_small",
            Self::MistralLarge => "mistral_large",
            Self::OpenAI => "openai",
            Self::Claude => "claude",
            Self::Gemini => "gemini",
        }
    }

    /// Returns the text generation endpoint URL for the provider.
    pub fn chat_endpoint(&self) -> &'static str {
        match self {
            Self::MistralSmall | Self::MistralLarge => "https://api.mistral.ai/v1/chat/completions",
            // OpenAI Responses API: the current-generation endpoint, successor of
            // Chat Completions (which is now considered a legacy path for GPT-5.x models)
            Self::OpenAI => "https://api.openai.com/v1/responses",
            Self::Claude => "https://api.anthropic.com/v1/messages",
            Self::Gemini => "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.5-flash:generateContent",
        }
    }

    /// Returns the default model name for each provider.
    /// Models are chosen for the task at hand (short article summaries):
    /// fast, low-cost tiers are preferred over frontier models.
    pub fn default_model(&self) -> &'static str {
        match self {
            Self::MistralSmall => "mistral-small-latest",
            Self::MistralLarge => "mistral-large-latest",
            // GPT-5.6 Luna: OpenAI's cost-optimized tier for high-volume workloads,
            // the coherent choice for short summaries ($1/$6 per MTok)
            Self::OpenAI => "gpt-5.6-luna",
            // Fastest and most cost-effective Claude tier, ideal for simple summaries
            Self::Claude => "claude-haiku-4-5",
            Self::Gemini => "gemini-3.5-flash",
        }
    }

    /// Returns the usage/billing endpoint if available for this provider.
    /// Some providers don't expose this via API.
    pub fn usage_endpoint(&self) -> Option<&'static str> {
        match self {
            Self::MistralSmall | Self::MistralLarge => None,
            // OpenAI's usage endpoint requires an admin key and date parameters,
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
            Self::MistralSmall | Self::MistralLarge => true,
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

    /// Builds the full rendering API URL for the given target page URL.
    /// Each provider uses different query parameter names for the API key,
    /// target URL, and JS rendering toggle.
    pub fn render_url(&self, api_key: &str, target_url: &str) -> String {
        let encoded_url = urlencoded(target_url);
        match self {
            Self::ScrapeDo => format!(
                "https://api.scrape.do/?token={}&render=true&url={}",
                urlencoded(api_key), encoded_url
            ),
            Self::ScraperApi => format!(
                "https://api.scraperapi.com?api_key={}&render=true&url={}",
                urlencoded(api_key), encoded_url
            ),
            Self::ScrapingBee => format!(
                "https://app.scrapingbee.com/api/v1/?api_key={}&render_js=true&url={}",
                urlencoded(api_key), encoded_url
            ),
            Self::Scrapfly => format!(
                "https://api.scrapfly.io/scrape?key={}&render_js=true&url={}",
                urlencoded(api_key), encoded_url
            ),
            Self::ZenRows => format!(
                "https://api.zenrows.com/v1/?apikey={}&js_render=true&url={}",
                urlencoded(api_key), encoded_url
            ),
        }
    }

    /// Builds the test URL used to validate the API key.
    /// ScraperAPI has a free account endpoint; others test by scraping example.com.
    pub fn test_url(&self, api_key: &str) -> String {
        match self {
            Self::ScraperApi => format!(
                "https://api.scraperapi.com/account?api_key={}",
                urlencoded(api_key)
            ),
            _ => self.render_url(api_key, "https://example.com"),
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