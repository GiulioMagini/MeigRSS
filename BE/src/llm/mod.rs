use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::LlmProvider;
use crate::config::encryption;
use crate::db::Database;

/// Builds the summary prompt based on the provider and UI language.
/// The 500-chars provider gets a shorter summary (3-5 sentences, 500 chars),
/// every other provider gets a longer one (3-10 sentences, 1000 chars).
/// The prompt is written entirely in the target language, which anchors
/// the output language better than an English instruction to translate.
/// Any unrecognized language falls back to the English prompt.
fn build_summary_prompt(article_text: &str, provider: &LlmProvider, lang: &str) -> String {
    let is_short = matches!(provider, LlmProvider::Mistral500Chars);

    let (sentences, max_chars) = if is_short {
        ("3-5", "500")
    } else {
        ("3-10", "1000")
    };

    // Match on the two-letter language prefix to select the prompt language
    let code = if lang.starts_with("it") {
        "it"
    } else if lang.starts_with("fr") {
        "fr"
    } else if lang.starts_with("de") {
        "de"
    } else if lang.starts_with("es") {
        "es"
    } else if lang.starts_with("pt") {
        "pt"
    } else {
        "en"
    };

    match code {
        "it" => format!(
            "Dato il seguente articolo:\n\
            {article_text}\n\
            Produci un riassunto in {sentences} frasi (massimo {max_chars} caratteri) seguendo le regole indicate:\n\
            1. Riporta SOLO informazioni presenti nel testo fornito. Non aggiungere contesto, background o dettagli che non compaiono nell'articolo.\n\
            2. Non inventare nomi, cifre, date, citazioni o fonti che non siano esplicitamente nel testo.\n\
            3. Se l'articolo è troppo breve o vago per un riassunto significativo, rispondi: \"Contenuto insufficiente per un riassunto.\"\n\
            4. Non esprimere opinioni, valutazioni o interpretazioni. Riporta solo ciò che l'articolo afferma.\n\
            5. Se l'articolo contiene affermazioni attribuite a qualcuno, mantieni l'attribuzione (es. \"secondo X\", \"X ha dichiarato che\").\n\
            6. Rispondi sempre in italiano, indipendentemente dalla lingua dell'articolo.\n\n\
            Formato di output: Solo il testo del riassunto, senza titoli, etichette o preamboli.
            Evita qualsiasi tipo di formattazione (ad esempio Markdown).",
            article_text = article_text, sentences = sentences, max_chars = max_chars,
        ),
        "fr" => format!(
            "Étant donné l'article suivant :\n\
            {article_text}\n\
            Produis un résumé en {sentences} phrases (maximum {max_chars} caractères) en suivant ces règles :\n\
            1. Rapporte UNIQUEMENT les informations présentes dans le texte fourni. N'ajoute aucun contexte, arrière-plan ou détail absent de l'article.\n\
            2. N'invente aucun nom, chiffre, date, citation ou source qui ne figure pas explicitement dans le texte.\n\
            3. Si l'article est trop court ou vague pour un résumé pertinent, réponds : \"Contenu insuffisant pour un résumé.\"\n\
            4. N'exprime aucune opinion, évaluation ou interprétation. Rapporte uniquement ce que l'article affirme.\n\
            5. Si l'article contient des propos attribués à quelqu'un, conserve l'attribution (par ex. \"selon X\", \"X a déclaré que\").\n\
            6. Réponds toujours en français, quelle que soit la langue de l'article.\n\n\
            Format de sortie : uniquement le texte du résumé, sans titres, étiquettes ni préambules.
            Évitez tout type de mise en forme (par exemple, Markdown).",
            article_text = article_text, sentences = sentences, max_chars = max_chars,
        ),
        "de" => format!(
            "Gegeben sei der folgende Artikel:\n\
            {article_text}\n\
            Erstelle eine Zusammenfassung in {sentences} Sätzen (maximal {max_chars} Zeichen) nach den folgenden Regeln:\n\
            1. Gib AUSSCHLIESSLICH Informationen wieder, die im bereitgestellten Text vorkommen. Füge keinen Kontext, Hintergrund oder Details hinzu, die nicht im Artikel stehen.\n\
            2. Erfinde keine Namen, Zahlen, Daten, Zitate oder Quellen, die nicht ausdrücklich im Text stehen.\n\
            3. Wenn der Artikel zu kurz oder zu vage für eine sinnvolle Zusammenfassung ist, antworte: \"Unzureichender Inhalt für eine Zusammenfassung.\"\n\
            4. Äußere keine Meinungen, Bewertungen oder Interpretationen. Gib nur wieder, was der Artikel aussagt.\n\
            5. Wenn der Artikel jemandem zugeschriebene Aussagen enthält, behalte die Zuordnung bei (z. B. \"laut X\", \"X erklärte, dass\").\n\
            6. Antworte immer auf Deutsch, unabhängig von der Sprache des Artikels.\n\n\
            Ausgabeformat: Nur der Text der Zusammenfassung, ohne Titel, Beschriftungen oder Einleitungen.
            Vermeiden Sie jegliche Art von Formatierung (z. B. Markdown).",
            article_text = article_text, sentences = sentences, max_chars = max_chars,
        ),
        "es" => format!(
            "Dado el siguiente artículo:\n\
            {article_text}\n\
            Produce un resumen en {sentences} frases (máximo {max_chars} caracteres) siguiendo estas reglas:\n\
            1. Recoge SOLO información presente en el texto proporcionado. No añadas contexto, antecedentes o detalles que no aparezcan en el artículo.\n\
            2. No inventes nombres, cifras, fechas, citas o fuentes que no estén explícitamente en el texto.\n\
            3. Si el artículo es demasiado breve o vago para un resumen significativo, responde: \"Contenido insuficiente para un resumen.\"\n\
            4. No expreses opiniones, valoraciones o interpretaciones. Recoge solo lo que el artículo afirma.\n\
            5. Si el artículo contiene afirmaciones atribuidas a alguien, mantén la atribución (p. ej. \"según X\", \"X declaró que\").\n\
            6. Responde siempre en español, independientemente del idioma del artículo.\n\n\
            Formato de salida: Solo el texto del resumen, sin títulos, etiquetas ni preámbulos.
            Evite cualquier tipo de formato (por ejemplo, Markdown).",
            article_text = article_text, sentences = sentences, max_chars = max_chars,
        ),
        "pt" => format!(
            "Dado o seguinte artigo:\n\
            {article_text}\n\
            Produz um resumo em {sentences} frases (máximo {max_chars} caracteres) seguindo estas regras:\n\
            1. Reporta APENAS informações presentes no texto fornecido. Não acrescentes contexto, antecedentes ou detalhes que não constem do artigo.\n\
            2. Não inventes nomes, números, datas, citações ou fontes que não estejam explicitamente no texto.\n\
            3. Se o artigo for demasiado curto ou vago para um resumo significativo, responde: \"Conteúdo insuficiente para um resumo.\"\n\
            4. Não exprimas opiniões, avaliações ou interpretações. Reporta apenas o que o artigo afirma.\n\
            5. Se o artigo contiver afirmações atribuídas a alguém, mantém a atribuição (por ex. \"segundo X\", \"X declarou que\").\n\
            6. Responde sempre em português, independentemente da língua do artigo.\n\n\
            Formato de saída: Apenas o texto do resumo, sem títulos, etiquetas ou preâmbulos.
            Evite qualquer tipo de formatação (por exemplo, Markdown).",
            article_text = article_text, sentences = sentences, max_chars = max_chars,
        ),
        _ => format!(
            "Given the following article:\n\
            {article_text}\n\
            Produce a summary in {sentences} sentences (maximum {max_chars} characters) following these rules:\n\
            1. Report ONLY information present in the provided text. Do not add context, background or details not found in the article.\n\
            2. Do not invent names, figures, dates, quotes or sources that are not explicitly in the text.\n\
            3. If the article is too short or vague for a meaningful summary, reply: \"Insufficient content for a summary.\"\n\
            4. Do not express opinions, evaluations or interpretations. Report only what the article states.\n\
            5. If the article contains statements attributed to someone, keep the attribution (e.g. \"according to X\", \"X stated that\").\n\
            6. Always reply in English, regardless of the article's language.\n\n\
            Output format: Only the summary text, without titles, labels or preambles.
            Avoid any type of formatting (e.g., Markdown).",
            article_text = article_text, sentences = sentences, max_chars = max_chars,
        ),
    }
}

// ─── Request/Response structures for different providers ───────────────────

/// OpenAI/Mistral compatible chat completion request.
#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

/// OpenAI/Mistral compatible chat completion response.
#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Deserialize)]
struct ChatResponseMessage {
    content: String,
}

/// OpenAI Responses API request format (successor of Chat Completions).
#[derive(Serialize)]
struct OpenAiResponsesRequest {
    model: String,
    input: String,
    max_output_tokens: u32,
    reasoning: OpenAiReasoning,
}

#[derive(Serialize)]
struct OpenAiReasoning {
    effort: String,
}

/// OpenAI Responses API response format.
/// The output array can contain reasoning items before the message item.
#[derive(Deserialize)]
struct OpenAiResponsesResponse {
    output: Vec<OpenAiOutputItem>,
}

#[derive(Deserialize)]
struct OpenAiOutputItem {
    #[serde(rename = "type")]
    item_type: String,
    #[serde(default)]
    content: Vec<OpenAiOutputContent>,
}

#[derive(Deserialize)]
struct OpenAiOutputContent {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: String,
}

/// Claude Messages API request format (differs from OpenAI format).
#[derive(Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ChatMessage>,
    thinking: ClaudeThinking,
    output_config: ClaudeOutputConfig,
}

/// Thinking mode of the request. Claude 5.5 models think adaptively by
/// default, so it has to be disabled explicitly.
#[derive(Serialize)]
struct ClaudeThinking {
    #[serde(rename = "type")]
    thinking_type: String,
}

/// Output settings of the request: effort governs every output token,
/// with or without thinking.
#[derive(Serialize)]
struct ClaudeOutputConfig {
    effort: String,
}

/// Claude Messages API response format.
/// Content blocks are selected by type, since a response is not guaranteed
/// to start with a text block.
#[derive(Deserialize)]
struct ClaudeResponse {
    #[serde(default)]
    content: Vec<ClaudeContent>,
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
struct ClaudeContent {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: String,
}

/// Gemini Interactions API request format.
#[derive(Serialize)]
struct GeminiInteractionRequest {
    model: String,
    input: String,
    store: bool,
    generation_config: GeminiGenerationConfig,
}

#[derive(Serialize)]
struct GeminiGenerationConfig {
    max_output_tokens: u32,
    thinking_level: String,
}

/// Gemini Interactions API response format.
/// The steps array can contain thought steps before the model output step.
#[derive(Deserialize)]
struct GeminiInteractionResponse {
    #[serde(default)]
    steps: Vec<GeminiStep>,
}

#[derive(Deserialize)]
struct GeminiStep {
    #[serde(rename = "type")]
    step_type: String,
    #[serde(default)]
    content: Vec<GeminiStepContent>,
}

#[derive(Deserialize)]
struct GeminiStepContent {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: String,
}

// ─── LLM client ────────────────────────────────────────────────────────────

/// Main LLM client that handles summary generation across different providers.
pub struct LlmClient {
    http_client: Client,
}

impl LlmClient {
    pub fn new() -> Self {
        Self {
            http_client: Client::new(),
        }
    }

    /// Generates a summary for the given article text using the configured LLM provider.
    /// Returns the summary text or an error message.
    pub async fn generate_summary(
        &self,
        article_text: &str,
        db: &Database,
        lang: &str,
    ) -> Result<String, String> {
        // Retrieve and decrypt API configuration
        let (provider_str, encrypted_key, iv) = db
            .get_api_config()
            .map_err(|e| format!("DB error: {}", e))?
            .ok_or("API not configured")?;

        if encrypted_key.is_empty() {
            return Err("API key not configured".to_string());
        }

        let api_key = encryption::decrypt_api_key(&encrypted_key, &iv)?;
        let provider = LlmProvider::from_str(&provider_str)
            .ok_or(format!("Unknown provider: {}", provider_str))?;

        // Build the prompt based on provider and language
        let prompt = build_summary_prompt(article_text, &provider, lang);

        // Route to the appropriate provider implementation
        match provider {
            LlmProvider::Mistral500Chars | LlmProvider::Mistral1000Chars => {
                self.call_mistral(&api_key, &provider, &prompt).await
            }
            LlmProvider::OpenAI => {
                self.call_openai(&api_key, &prompt).await
            }
            LlmProvider::Claude => {
                self.call_claude(&api_key, &prompt).await
            }
            LlmProvider::Gemini => {
                self.call_gemini(&api_key, &prompt).await
            }
        }
    }

    /// Calls the Mistral API (OpenAI-compatible Chat Completions format).
    async fn call_mistral(
        &self,
        api_key: &str,
        provider: &LlmProvider,
        prompt: &str,
    ) -> Result<String, String> {
        let request = ChatCompletionRequest {
            model: provider.default_model().to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            max_tokens: 600, // Enough for up to 1000 chars in longer summaries
        };

        let response = self.http_client
            .post(provider.chat_endpoint())
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("API request failed: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(format!("API returned {}: {}", status, error_body));
        }

        let body: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse API response: {}", e))?;

        body.choices
            .first()
            .map(|c| c.message.content.trim().to_string())
            .ok_or("API returned empty response".to_string())
    }

    /// Calls the OpenAI Responses API (the current OpenAI generation endpoint).
    /// GPT-6 models are reasoning models, but reasoning is disabled here: a short
    /// factual summary gains nothing from it, and without reasoning tokens the
    /// output budget only has to cover the summary itself.
    async fn call_openai(
        &self,
        api_key: &str,
        prompt: &str,
    ) -> Result<String, String> {
        let request = OpenAiResponsesRequest {
            model: LlmProvider::OpenAI.default_model().to_string(),
            input: prompt.to_string(),
            max_output_tokens: 600, // Enough for up to 1000 chars in longer summaries
            reasoning: OpenAiReasoning {
                // Set explicitly, since the model default is "medium"
                effort: "none".to_string(),
            },
        };

        let response = self.http_client
            .post(LlmProvider::OpenAI.chat_endpoint())
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("OpenAI API request failed: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(format!("OpenAI API returned {}: {}", status, error_body));
        }

        let body: OpenAiResponsesResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse OpenAI response: {}", e))?;

        // Skip reasoning items and extract the text from the message item
        body.output
            .iter()
            .find(|item| item.item_type == "message")
            .and_then(|item| item.content.iter().find(|c| c.content_type == "output_text"))
            .map(|c| c.text.trim().to_string())
            .filter(|t| !t.is_empty())
            .ok_or("OpenAI returned empty response".to_string())
    }

    /// Calls the Anthropic Claude Messages API.
    /// Thinking is disabled and effort set to low: a short factual summary gains
    /// nothing from reasoning, and thinking tokens would count against `max_tokens`.
    async fn call_claude(
        &self,
        api_key: &str,
        prompt: &str,
    ) -> Result<String, String> {
        let request = ClaudeRequest {
            model: LlmProvider::Claude.default_model().to_string(),
            max_tokens: 600, // Enough for up to 1000 chars in longer summaries
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            thinking: ClaudeThinking {
                thinking_type: "disabled".to_string(),
            },
            output_config: ClaudeOutputConfig {
                // Haiku 5.5 accepts disabled thinking only at effort "high" or below
                effort: "low".to_string(),
            },
        };

        let response = self.http_client
            .post(LlmProvider::Claude.chat_endpoint())
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Claude API request failed: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(format!("Claude API returned {}: {}", status, error_body));
        }

        let body: ClaudeResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse Claude response: {}", e))?;

        // Haiku 5.5 has no server-side fallback model when its classifiers decline
        if body.stop_reason.as_deref() == Some("refusal") {
            return Err("Claude declined to process the request".to_string());
        }

        // Concatenate the text blocks, skipping any other block type
        let text: String = body.content
            .iter()
            .filter(|c| c.content_type == "text")
            .map(|c| c.text.as_str())
            .collect();

        let text = text.trim();
        if text.is_empty() {
            return Err("Claude returned empty response".to_string());
        }
        Ok(text.to_string())
    }

    /// Calls the Google Gemini Interactions API.
    /// Interactions are not stored server-side (`store: false`): MeigRSS sends
    /// stateless one-shot requests, so there is no reason to keep article text
    /// on Google's side (55 days on paid tier, 1 day on free tier by default).
    async fn call_gemini(
        &self,
        api_key: &str,
        prompt: &str,
    ) -> Result<String, String> {
        let request = GeminiInteractionRequest {
            model: LlmProvider::Gemini.default_model().to_string(),
            input: prompt.to_string(),
            store: false,
            generation_config: GeminiGenerationConfig {
                max_output_tokens: 600, // Enough for up to 1000 chars in longer summaries
                // Set explicitly to the lowest level, which keeps thinking
                // (billed as output tokens) to little or nothing
                thinking_level: "minimal".to_string(),
            },
        };

        // Send the API key via header instead of URL query parameter,
        // so it never appears in URLs (which can leak into logs or error messages)
        let response = self.http_client
            .post(LlmProvider::Gemini.chat_endpoint())
            .header("x-goog-api-key", api_key)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Gemini API request failed: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(format!("Gemini API returned {}: {}", status, error_body));
        }

        let body: GeminiInteractionResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse Gemini response: {}", e))?;

        // Skip thought steps and concatenate the text of the model output steps
        let text: String = body.steps
            .iter()
            .filter(|step| step.step_type == "model_output")
            .flat_map(|step| step.content.iter())
            .filter(|c| c.content_type == "text")
            .map(|c| c.text.as_str())
            .collect();

        let text = text.trim();
        if text.is_empty() {
            return Err("Gemini returned empty response".to_string());
        }
        Ok(text.to_string())
    }

    /// Tests the API connection by sending a simple request.
    /// Returns Ok(()) if the API responds successfully.
    pub async fn test_connection(
        &self,
        provider: &LlmProvider,
        api_key: &str,
    ) -> Result<String, String> {
        let test_prompt = "Reply only with: OK";

        match provider {
            LlmProvider::Mistral500Chars | LlmProvider::Mistral1000Chars => {
                self.call_mistral(api_key, provider, test_prompt).await
            }
            LlmProvider::OpenAI => {
                self.call_openai(api_key, test_prompt).await
            }
            LlmProvider::Claude => {
                self.call_claude(api_key, test_prompt).await
            }
            LlmProvider::Gemini => {
                self.call_gemini(api_key, test_prompt).await
            }
        }
    }

    /// Queries the LLM for information about the current plan/usage.
    /// Falls back to asking the model itself if no dedicated endpoint exists.
    /// The response is shown in the UI, so it follows the UI language;
    /// Italian/English stay primary only for terminal logs and handled errors.
    pub async fn query_plan_info(
        &self,
        provider: &LlmProvider,
        api_key: &str,
        lang: &str,
    ) -> Result<String, String> {
        // First try the dedicated usage endpoint if available
        if let Some(usage_url) = provider.usage_endpoint() {
            let response = self.http_client
                .get(usage_url)
                .header("Authorization", format!("Bearer {}", api_key))
                .send()
                .await;

            if let Ok(resp) = response
                && resp.status().is_success()
                && let Ok(text) = resp.text().await {
                return Ok(text);
            }
            // If dedicated endpoint fails, fall through to asking the model
        }

        // Build a provider-specific prompt for plan information
        let provider_name = match provider {
            LlmProvider::Mistral500Chars => "Mistral AI (Ministral 3 8B model)",
            LlmProvider::Mistral1000Chars => "Mistral AI (Ministral 3 14B model)",
            LlmProvider::OpenAI => "OpenAI",
            LlmProvider::Claude => "Anthropic Claude",
            LlmProvider::Gemini => "Google Gemini",
        };

        // The final instruction is written in the target language itself,
        // which anchors the output language of the model
        let lang_instruction = if lang.starts_with("it") {
            "Rispondi in italiano."
        } else if lang.starts_with("fr") {
            "Réponds en français."
        } else if lang.starts_with("de") {
            "Antworte auf Deutsch."
        } else if lang.starts_with("es") {
            "Responde en español."
        } else if lang.starts_with("pt") {
            "Responde em português."
        } else {
            "Reply in English."
        };

        // Ask the AI about plan information
        let info_prompt = format!(
            "What are the current pricing plan limits for the {} API? \
            Include information about: requests per minute, tokens per minute, tokens per month, \
            cost per million tokens (input and output), and any other relevant plan information. \
            Distinguish between the free plan (Experiment/Free) and paid plans if applicable. \
            If you do not have access to this information, say so clearly. {}",
            provider_name, lang_instruction
        );

        match provider {
            LlmProvider::Mistral500Chars | LlmProvider::Mistral1000Chars => {
                self.call_mistral(api_key, provider, &info_prompt).await
            }
            LlmProvider::OpenAI => {
                self.call_openai(api_key, &info_prompt).await
            }
            LlmProvider::Claude => {
                self.call_claude(api_key, &info_prompt).await
            }
            LlmProvider::Gemini => {
                self.call_gemini(api_key, &info_prompt).await
            }
        }
    }
}