use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db::{self, ArticleQuery, Database};
use crate::config::{encryption, LlmProvider, WebRenderingProvider};
use crate::feed;
use crate::scraper;
use crate::newsletter;
use crate::llm::LlmClient;
use crate::i18n;
use crate::web_feed;

/// Shared application state passed to all route handlers.
pub struct AppState {
    pub db: Database,
    pub llm: LlmClient,
}

/// Builds the complete API router with all endpoints.
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Article endpoints
        .route("/api/articles", get(get_articles))
        .route("/api/articles/{id}/favorite", put(toggle_favorite))
        .route("/api/articles/{id}/summary", post(generate_summary))
        .route("/api/articles/{id}/scrape", post(scrape_article))
        .route("/api/articles/{id}/read", put(mark_article_read))
        // Feed endpoints
        .route("/api/feeds", get(get_feeds))
        .route("/api/feeds", post(add_feed))
        .route("/api/feeds/{id}", delete(delete_feed))
        .route("/api/feeds/{id}/force-ai", put(set_feed_force_ai))
        .route("/api/feeds/{id}/scraping", put(set_feed_scraping))
        .route("/api/feeds/{id}/pause", put(set_feed_paused))
        .route("/api/feeds/{id}/category", put(change_feed_category))
        .route("/api/feeds/test", post(test_feed))
        .route("/api/feeds/refresh", post(refresh_feeds))
        .route("/api/feeds/export", get(export_feeds))
        .route("/api/feeds/import", post(import_feeds))
        .route("/api/newsletter/import", post(import_newsletter))
        .route("/api/newsletter/preview", post(preview_newsletter))
        .route("/api/newsletter/feeds", get(get_newsletter_feeds))
        .route("/api/newsletter/feeds", post(create_newsletter_feed))
        .route("/api/newsletter/{id}/html", get(get_newsletter_html))
        // Web rendering API config endpoints
        .route("/api/config/web-rendering", get(get_web_rendering_config))
        .route("/api/config/web-rendering", post(save_web_rendering_config))
        .route("/api/config/web-rendering", delete(delete_web_rendering_config))
        // Web feed endpoints
        .route("/api/web-feed/test", post(test_web_feed))
        .route("/api/web-feed", post(add_web_feed))
        // Delete all feeds
        .route("/api/feeds/delete-all", delete(delete_all_feeds))
        // Category endpoints
        .route("/api/categories", get(get_categories))
        .route("/api/categories", post(add_category))
        .route("/api/categories/{id}", put(rename_category))
        .route("/api/categories/{id}", delete(delete_category))
        // API configuration endpoints
        .route("/api/config/api", get(get_api_config))
        .route("/api/config/api", post(save_api_config))
        .route("/api/config/api/test", post(test_api))
        .route("/api/config/api/plan-info", post(get_plan_info))
        .route("/api/config/api", delete(delete_api_config))
        // Settings endpoints
        .route("/api/settings", get(get_settings))
        .route("/api/settings", put(update_settings))
        .route("/api/settings/language", put(update_language))
        // i18n endpoint
        .route("/api/i18n/{lang}", get(get_translations))
        .with_state(state)
}

// ─── Common response types ─────────────────────────────────────────────────

#[derive(Serialize)]
struct ApiResponse<T: Serialize> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    fn ok(data: T) -> Json<Self> {
        Json(Self { success: true, data: Some(data), error: None })
    }

    fn err(msg: impl Into<String>) -> (StatusCode, Json<Self>) {
        (StatusCode::BAD_REQUEST, Json(Self {
            success: false, data: None, error: Some(msg.into())
        }))
    }

    fn internal_err(msg: impl Into<String>) -> (StatusCode, Json<Self>) {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(Self {
            success: false, data: None, error: Some(msg.into())
        }))
    }
}

// ─── Article handlers ──────────────────────────────────────────────────────

/// GET /api/articles - List articles with filters, sorting, pagination
async fn get_articles(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ArticleQuery>,
) -> Result<Json<ApiResponse<db::ArticleListResponse>>, (StatusCode, Json<ApiResponse<db::ArticleListResponse>>)> {
    match state.db.get_articles(&query) {
        Ok(result) => Ok(ApiResponse::ok(result)),
        Err(e) => Err(ApiResponse::internal_err(format!("Database error: {}", e))),
    }
}

/// PUT /api/articles/{id}/favorite - Toggle favorite status
#[derive(Deserialize)]
struct FavoriteRequest {
    is_favorite: bool,
}

async fn toggle_favorite(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<FavoriteRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.db.set_article_favorite(&id, body.is_favorite) {
        Ok(_) => Ok(ApiResponse::ok(())),
        Err(e) => Err(ApiResponse::internal_err(format!("Database error: {}", e))),
    }
}


#[derive(Deserialize)]
struct SummaryQuery {
    lang: Option<String>,
}

/// POST /api/articles/{id}/summary - Generate AI summary for a specific article.
/// This is the "manual trigger" button — has priority over batch summaries.
async fn generate_summary(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<SummaryQuery>,
) -> Result<Json<ApiResponse<SummaryResponse>>, (StatusCode, Json<ApiResponse<SummaryResponse>>)> {
    // Get the article's full text from DB
    // Get the article's full text from DB
    let article_text = state.db.get_article_full_text(&id);

    let full_text = match article_text {
        Ok(Some(text)) if !text.is_empty() => text,
        Ok(_) => return Err(ApiResponse::err("Article has no full text to summarize")),
        Err(_) => return Err(ApiResponse::err("Article not found")),
    };

    // Get the language setting for the summary prompt
    let lang = query.lang.unwrap_or_else(|| "en".to_string());

    // Generate summary via LLM
    match state.llm.generate_summary(&full_text, &state.db, &lang).await {
        Ok(summary) => {
            // Save the summary to DB
            let _ = state.db.update_article_summary(&id, &summary, "llm");
            Ok(ApiResponse::ok(SummaryResponse { summary, source: "llm".to_string() }))
        }
        Err(e) => Err(ApiResponse::err(format!("Summary generation failed: {}", e))),
    }
}

/// POST /api/articles/{id}/scrape - Re-scrape the full text of a specific article
async fn scrape_article(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    // Get the article URL from DB
    let article_url = state.db.get_article_url(&id);

    let url = match article_url {
        Ok(Some(u)) => u,
        Ok(None) => return Err(ApiResponse::err("Article not found")),
        Err(e) => return Err(ApiResponse::internal_err(format!("Database error: {}", e))),
    };

    // Scrape the full text
    match scraper::extract_article_text(&url).await {
        Ok(text) if !text.is_empty() => {
            let _ = state.db.update_article_full_text(&id, &text);
            Ok(ApiResponse::ok(text))
        }
        Ok(_) => Err(ApiResponse::err("scrape_no_content")),
        Err(e) => Err(ApiResponse::err(format!("scrape_failed:{}", e))),
    }
}

/// PUT /api/articles/{id}/read - Mark an article as read
async fn mark_article_read(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.db.set_article_read(&id) {
        Ok(_) => Ok(ApiResponse::ok(())),
        Err(e) => Err(ApiResponse::internal_err(format!("Database error: {}", e))),
    }
}

#[derive(Serialize)]
struct SummaryResponse {
    summary: String,
    source: String,
}

// ─── Feed handlers ─────────────────────────────────────────────────────────

/// GET /api/feeds - List all configured feeds
async fn get_feeds(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<db::Feed>>>, (StatusCode, Json<ApiResponse<Vec<db::Feed>>>)> {
    match state.db.get_feeds() {
        Ok(feeds) => Ok(ApiResponse::ok(feeds)),
        Err(e) => Err(ApiResponse::internal_err(format!("Database error: {}", e))),
    }
}

#[derive(Deserialize)]
struct AddFeedRequest {
    url: String,
    title: String,
    site_name: String,
    category_id: i64,
}

/// POST /api/feeds - Add a new feed (must have been tested first)
async fn add_feed(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AddFeedRequest>,
) -> Result<Json<ApiResponse<i64>>, (StatusCode, Json<ApiResponse<i64>>)> {
    match state.db.add_feed(&body.url, &body.title, &body.site_name, body.category_id) {
        Ok(id) => Ok(ApiResponse::ok(id)),
        Err(e) => Err(ApiResponse::err(format!("Failed to add feed: {}", e))),
    }
}

/// DELETE /api/feeds/{id} - Remove a feed and its articles
async fn delete_feed(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.db.delete_feed(id) {
        Ok(_) => Ok(ApiResponse::ok(())),
        Err(e) => Err(ApiResponse::internal_err(format!("Database error: {}", e))),
    }
}


#[derive(Deserialize)]
struct DeleteAllQuery {
    delete_categories: Option<bool>,
}

/// DELETE /api/feeds/delete-all - Remove ALL feeds (requires confirmation in UI)
async fn delete_all_feeds(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DeleteAllQuery>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.db.delete_all_feeds() {
        Ok(_) => {
            // Optionally delete all non-default categories
            if query.delete_categories.unwrap_or(false) {
                if let Err(e) = state.db.delete_all_categories() {
                    tracing::error!("Failed to delete categories: {}", e);
                }
            }
            Ok(ApiResponse::ok(()))
        }
        Err(e) => Err(ApiResponse::internal_err(format!("Database error: {}", e))),
    }
}

#[derive(Deserialize)]
struct ForceAiRequest {
    force: bool,
}

/// PUT /api/feeds/{id}/force-ai - Toggle "always use AI summary" for a feed
async fn set_feed_force_ai(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(body): Json<ForceAiRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.db.set_feed_force_ai(id, body.force) {
        Ok(_) => {
            // When disabling force AI, restore feed summaries for articles
            // that still have the feed text saved
            if !body.force {
                let _ = state.db.restore_feed_summaries(id);
            }
            Ok(ApiResponse::ok(()))
        }
        Err(e) => Err(ApiResponse::internal_err(format!("Database error: {}", e))),
    }
}

#[derive(Deserialize)]
struct ScrapingRequest {
    enable: bool,
}

/// PUT /api/feeds/{id}/scraping - Toggle full text scraping for a feed.
/// When disabled, also disables force AI summary.
async fn set_feed_scraping(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(body): Json<ScrapingRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.db.set_feed_scraping(id, body.enable) {
        Ok(_) => Ok(ApiResponse::ok(())),
        Err(e) => Err(ApiResponse::internal_err(format!("Database error: {}", e))),
    }
}

#[derive(Deserialize)]
struct PauseRequest {
    paused: bool,
}

/// PUT /api/feeds/{id}/pause - Pause or unpause a feed.
/// Paused feeds are skipped during updates.
async fn set_feed_paused(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(body): Json<PauseRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.db.set_feed_paused(id, body.paused) {
        Ok(_) => Ok(ApiResponse::ok(())),
        Err(e) => Err(ApiResponse::internal_err(format!("Database error: {}", e))),
    }
}

#[derive(Deserialize)]
struct ChangeCategoryRequest {
    category_id: i64,
}

/// PUT /api/feeds/{id}/category - Change the category of a feed
async fn change_feed_category(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(body): Json<ChangeCategoryRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.db.update_feed_category(id, body.category_id) {
        Ok(_) => Ok(ApiResponse::ok(())),
        Err(e) => Err(ApiResponse::internal_err(format!("Database error: {}", e))),
    }
}

#[derive(Deserialize)]
struct TestFeedRequest {
    url: String,
}

#[derive(Serialize)]
struct TestFeedResponse {
    title: String,
    site_name: String,
}

/// POST /api/feeds/test - Validate a feed URL before adding it
async fn test_feed(
    Json(body): Json<TestFeedRequest>,
) -> Result<Json<ApiResponse<TestFeedResponse>>, (StatusCode, Json<ApiResponse<TestFeedResponse>>)> {
    match feed::test_feed_url(&body.url).await {
        Ok((title, site_name)) => Ok(ApiResponse::ok(TestFeedResponse { title, site_name })),
        Err(e) => Err(ApiResponse::err(e)),
    }
}

/// POST /api/feeds/refresh - Trigger feed update.
/// Waits for all feeds to be fetched before responding,
/// so the frontend knows exactly when to reload the article list.
async fn refresh_feeds(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<feed::FetchResult>>>, (StatusCode, Json<ApiResponse<Vec<feed::FetchResult>>>)> {
    let results = feed::fetch_all_feeds(&state.db).await;
    Ok(ApiResponse::ok(results))
}

/// GET /api/feeds/export - Export all categories and feeds as JSON
async fn export_feeds(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<db::FullExport>>, (StatusCode, Json<ApiResponse<db::FullExport>>)> {
    let categories = state.db.export_categories()
        .map_err(|e| ApiResponse::<db::FullExport>::internal_err(format!("Database error: {}", e)))?;
    let feeds = state.db.export_feeds()
        .map_err(|e| ApiResponse::<db::FullExport>::internal_err(format!("Database error: {}", e)))?;
    let favorites = state.db.export_favorites()
        .map_err(|e| ApiResponse::<db::FullExport>::internal_err(format!("Database error: {}", e)))?;
    let settings = state.db.export_settings();

    Ok(ApiResponse::ok(db::FullExport { categories, feeds, favorites, settings: Some(settings) }))
}

/// POST /api/feeds/import - Import categories and feeds from JSON
async fn import_feeds(
    State(state): State<Arc<AppState>>,
    Json(body): Json<db::FullExport>,
) -> Result<Json<ApiResponse<db::ImportResult>>, (StatusCode, Json<ApiResponse<db::ImportResult>>)> {
    // First import categories
    for cat in &body.categories {
        let _ = state.db.add_category(&cat.name);
    }
    // Then import feeds
    let feeds_imported = state.db.import_feeds(&body.feeds)
        .map_err(|e| ApiResponse::<db::ImportResult>::err(format!("Import failed: {}", e)))?;

    // Finally import favorite articles (must be after feeds, since they reference feed URLs)
    let favorites_imported = state.db.import_favorites(&body.favorites)
        .unwrap_or(0);

    // Import settings if present
    if let Some(settings) = &body.settings {
        let _ = state.db.set_setting("retention_hours", &settings.retention_hours);
        let _ = state.db.set_setting("auto_update_enabled", &settings.auto_update_enabled);
        let _ = state.db.set_setting("auto_update_interval_hours", &settings.auto_update_interval_hours);
        let _ = state.db.set_setting("highlight_keywords", &settings.highlight_keywords);
    }

    Ok(ApiResponse::ok(db::ImportResult { feeds_imported, favorites_imported }))
}

// ─── Category handlers ─────────────────────────────────────────────────────

/// GET /api/categories - List all categories
async fn get_categories(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<db::Category>>>, (StatusCode, Json<ApiResponse<Vec<db::Category>>>)> {
    match state.db.get_categories() {
        Ok(cats) => Ok(ApiResponse::ok(cats)),
        Err(e) => Err(ApiResponse::internal_err(format!("Database error: {}", e))),
    }
}

#[derive(Deserialize)]
struct CategoryRequest {
    name: String,
}

/// POST /api/categories - Create a new category
async fn add_category(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CategoryRequest>,
) -> Result<Json<ApiResponse<i64>>, (StatusCode, Json<ApiResponse<i64>>)> {
    match state.db.add_category(&body.name) {
        Ok(id) => Ok(ApiResponse::ok(id)),
        Err(e) => {
            let msg = format!("{}", e);
            if msg.contains("cat_name_reserved") {
                Err(ApiResponse::err("cat_name_reserved"))
            } else {
                Err(ApiResponse::err(format!("Failed to add category: {}", e)))
            }
        }
    }
}

/// PUT /api/categories/{id} - Rename a category
async fn rename_category(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(body): Json<CategoryRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.db.rename_category(id, &body.name) {
        Ok(0) => Err(ApiResponse::err("Cannot rename the default category")),
        Ok(_) => Ok(ApiResponse::ok(())),
        Err(e) => Err(ApiResponse::err(format!("Failed to rename: {}", e))),
    }
}

/// DELETE /api/categories/{id} - Delete a category (only if empty and not default)
async fn delete_category(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.db.delete_category(id) {
        Ok(0) => Err(ApiResponse::err(
            "Cannot delete: category has feeds or is the default category"
        )),
        Ok(_) => Ok(ApiResponse::ok(())),
        Err(e) => Err(ApiResponse::internal_err(format!("Database error: {}", e))),
    }
}

// ─── API config handlers ───────────────────────────────────────────────────

#[derive(Serialize)]
struct ApiConfigResponse {
    provider: String,
    key_status: String, // "Configurata" / "Non configurata" — never the actual key
    supports_usage_query: bool,
}

/// GET /api/config/api - Get current API config (masked, never exposes key)
async fn get_api_config(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<ApiConfigResponse>> {
    match state.db.get_api_config() {
        Ok(Some((provider, encrypted_key, iv))) => {
            let llm_provider = LlmProvider::from_str(&provider).unwrap_or(LlmProvider::MistralSmall);
            let not_configured_label = if crate::is_system_italian() { "Non configurata" } else { "Not configured" };
            let key_display = if encrypted_key.is_empty() {
                not_configured_label.to_string()
            } else {
                match encryption::decrypt_api_key(&encrypted_key, &iv) {
                    Ok(plain) => encryption::mask_api_key(&plain),
                    Err(_) => if crate::is_system_italian() { "Configurata (errore decrittazione)".to_string() } else { "Configured (decryption error)".to_string() },
                }
            };
            ApiResponse::ok(ApiConfigResponse {
                provider,
                key_status: key_display,
                supports_usage_query: llm_provider.supports_usage_query(),
            })
        }
        Ok(None) => ApiResponse::ok(ApiConfigResponse {
            provider: "mistral_small".to_string(),
            key_status: if crate::is_system_italian() { "Non configurata".to_string() } else { "Not configured".to_string() },
            supports_usage_query: true,
        }),
        Err(e) => {
            tracing::error!("Failed to get API config: {}", e);
            ApiResponse::ok(ApiConfigResponse {
                provider: "mistral_small".to_string(),
                key_status: if crate::is_system_italian() { "Non configurata".to_string() } else { "Not configured".to_string() },
                supports_usage_query: true,
            })
        }
    }
}

#[derive(Deserialize)]
struct SaveApiConfigRequest {
    provider: String,
    api_key: String,
}

/// POST /api/config/api - Save API config, test connection, encrypt and store key.
/// The API key is NEVER returned in any GET response.
async fn save_api_config(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SaveApiConfigRequest>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    let provider = LlmProvider::from_str(&body.provider)
        .ok_or_else(|| ApiResponse::<String>::err("Unknown provider"))?;

    // Test connection before saving
    match state.llm.test_connection(&provider, &body.api_key).await {
        Ok(_) => {
            // Encrypt and store
            let (encrypted, iv) = encryption::encrypt_api_key(&body.api_key)
                .map_err(|e| ApiResponse::<String>::err(format!("Encryption error: {}", e)))?;

            state.db.save_api_config(provider.as_str(), &encrypted, &iv)
                .map_err(|e| ApiResponse::<String>::internal_err(format!("DB error: {}", e)))?;

            Ok(ApiResponse::ok("OK".to_string()))
        }
        Err(e) => Err(ApiResponse::err(format!("{}", e))),
    }
}

/// POST /api/config/api/test - Test existing API config without saving
async fn test_api(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    let (provider_str, encrypted_key, iv) = state.db
        .get_api_config()
        .map_err(|e| ApiResponse::<String>::internal_err(format!("DB error: {}", e)))?
        .ok_or_else(|| ApiResponse::<String>::err("API not configured"))?;

    let api_key = encryption::decrypt_api_key(&encrypted_key, &iv)
        .map_err(|e| ApiResponse::<String>::err(format!("Decryption error: {}", e)))?;

    let provider = LlmProvider::from_str(&provider_str)
        .ok_or_else(|| ApiResponse::<String>::err("Unknown provider"))?;

    match state.llm.test_connection(&provider, &api_key).await {
        Ok(_) => Ok(ApiResponse::ok("OK".to_string())),
        Err(e) => Err(ApiResponse::err(format!("{}", e))),
    }
}

/// POST /api/config/api/plan-info - Query plan/usage info from the provider
async fn get_plan_info(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    let (provider_str, encrypted_key, iv) = state.db
        .get_api_config()
        .map_err(|e| ApiResponse::<String>::internal_err(format!("DB error: {}", e)))?
        .ok_or_else(|| ApiResponse::<String>::err("API not configured"))?;

    let api_key = encryption::decrypt_api_key(&encrypted_key, &iv)
        .map_err(|e| ApiResponse::<String>::err(format!("Decryption error: {}", e)))?;

    let provider = LlmProvider::from_str(&provider_str)
        .ok_or_else(|| ApiResponse::<String>::err("Unknown provider"))?;

    // The plan info is displayed in the UI, so it must follow the UI language
    let lang = state.db
        .get_setting("language")
        .ok()
        .flatten()
        .unwrap_or_else(|| "en".to_string());

    match state.llm.query_plan_info(&provider, &api_key, &lang).await {
        Ok(info) => Ok(ApiResponse::ok(info)),
        Err(e) => Err(ApiResponse::err(e)),
    }
}

/// DELETE /api/config/api - Delete the stored API configuration and encryption key file.
async fn delete_api_config(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    state.db.delete_api_config()
        .map_err(|e| ApiResponse::<()>::internal_err(format!("DB error: {}", e)))?;

    Ok(ApiResponse::ok(()))
}

// ─── Settings handlers ─────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct AppSettings {
    retention_hours: String,
    auto_update_enabled: String,
    auto_update_interval_hours: String,
    #[serde(default)]
    highlight_keywords: String,
    #[serde(default)]
    last_refresh_timestamp: String,
}

/// GET /api/settings - Get all app settings
async fn get_settings(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<AppSettings>> {
    let settings = AppSettings {
        retention_hours: state.db.get_setting("retention_hours").ok().flatten().unwrap_or("24".into()),
        auto_update_enabled: state.db.get_setting("auto_update_enabled").ok().flatten().unwrap_or("false".into()),
        auto_update_interval_hours: state.db.get_setting("auto_update_interval_hours").ok().flatten().unwrap_or("1".into()),
        highlight_keywords: state.db.get_setting("highlight_keywords").ok().flatten().unwrap_or_default(),
        last_refresh_timestamp: state.db.get_setting("last_refresh_timestamp").ok().flatten().unwrap_or_default(),
    };
    ApiResponse::ok(settings)
}

/// PUT /api/settings - Update app settings
async fn update_settings(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AppSettings>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Validate retention_hours: 1-720 (1 hour to 30 days)
    let retention: u32 = body.retention_hours.parse()
        .map_err(|_| ApiResponse::<()>::err("Invalid retention value"))?;
    if retention < 1 || retention > 720 {
        return Err(ApiResponse::err("Retention must be between 1 and 720 hours"));
    }

    // Validate auto_update_interval: 1-72 hours
    let interval: u32 = body.auto_update_interval_hours.parse()
        .map_err(|_| ApiResponse::<()>::err("Invalid interval value"))?;
    if interval < 1 || interval > 72 {
        return Err(ApiResponse::err("Update interval must be between 1 and 72 hours"));
    }

    state.db.set_setting("retention_hours", &body.retention_hours)
        .map_err(|e| ApiResponse::<()>::internal_err(format!("DB error: {}", e)))?;
    state.db.set_setting("auto_update_enabled", &body.auto_update_enabled)
        .map_err(|e| ApiResponse::<()>::internal_err(format!("DB error: {}", e)))?;
    state.db.set_setting("auto_update_interval_hours", &body.auto_update_interval_hours)
        .map_err(|e| ApiResponse::<()>::internal_err(format!("DB error: {}", e)))?;
    state.db.set_setting("highlight_keywords", &body.highlight_keywords)
        .map_err(|e| ApiResponse::<()>::internal_err(format!("DB error: {}", e)))?;

    Ok(ApiResponse::ok(()))
}

#[derive(Deserialize)]
struct LanguageRequest {
    language: String,
}

/// PUT /api/settings/language - Update the UI language setting.
/// Called by the frontend on load and on manual language change.
async fn update_language(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LanguageRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let lang = body.language.trim().to_lowercase();
    // Whitelist of supported ISO 639-1 language codes. A rigid list keeps
    // arbitrary values out of the database (the setting is free-form text).
    if !["it", "en", "fr", "de", "es", "pt"].contains(&lang.as_str()) {
        return Err(ApiResponse::err("Unsupported language"));
    }
    state.db.set_setting("language", &lang)
        .map_err(|e| ApiResponse::<()>::internal_err(format!("DB error: {}", e)))?;
    // Refresh the cached language flag used by backend log messages
    crate::set_language_cache(lang == "it");
    Ok(ApiResponse::ok(()))
}

// ─── Newsletter handlers ──────────────────────────────────────────────

#[derive(Deserialize)]
struct NewsletterImportRequest {
    eml_content: String,
    title: String,
    feed_id: i64,
}

/// POST /api/newsletter/import - Import a newsletter from EML file content
async fn import_newsletter(
    State(state): State<Arc<AppState>>,
    Json(body): Json<NewsletterImportRequest>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    match newsletter::import_eml(&body.eml_content, &body.title, body.feed_id, &state.db) {
        Ok(article_id) => Ok(ApiResponse::ok(article_id)),
        Err(e) => Err(ApiResponse::err(e)),
    }
}

/// GET /api/newsletter/{id}/html - Serve the newsletter HTML for viewing in browser
async fn get_newsletter_html(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<axum::response::Response, (StatusCode, String)> {
    match state.db.get_newsletter_html(&id) {
        Ok(Some(html)) => {
            // Serve with a sandboxed CSP: any script inside the imported
            // newsletter is blocked and the document gets an opaque origin,
            // so it cannot call the application's API endpoints.
            // Styles and images are preserved for correct rendering.
            axum::response::Response::builder()
                .status(StatusCode::OK)
                .header(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")
                .header("Content-Security-Policy", "sandbox")
                .header("X-Content-Type-Options", "nosniff")
                .body(axum::body::Body::from(html))
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Response build error: {}", e)))
        }
        Ok(None) => Err((StatusCode::NOT_FOUND, "Newsletter not found".to_string())),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))),
    }
}

#[derive(Deserialize)]
struct NewsletterPreviewRequest {
    eml_content: String,
}

#[derive(Serialize)]
struct NewsletterPreviewResponse {
    title: String,
    author: String,
    date: String,
}

/// POST /api/newsletter/preview - Parse EML and return metadata for preview
async fn preview_newsletter(
    Json(body): Json<NewsletterPreviewRequest>,
) -> Result<Json<ApiResponse<NewsletterPreviewResponse>>, (StatusCode, Json<ApiResponse<NewsletterPreviewResponse>>)> {
    let message = mail_parser::MessageParser::default()
        .parse(body.eml_content.as_bytes())
        .ok_or_else(|| ApiResponse::<NewsletterPreviewResponse>::err("newsletter_invalid_file"))?;

    let title = message.subject().unwrap_or("").to_string();

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

    let date = message
        .date()
        .map(|d| format!("{:04}-{:02}-{:02} {:02}:{:02}", d.year, d.month, d.day, d.hour, d.minute))
        .unwrap_or_default();

    Ok(ApiResponse::ok(NewsletterPreviewResponse { title, author, date }))
}

/// GET /api/newsletter/feeds - List all newsletter feeds
async fn get_newsletter_feeds(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<Vec<db::Feed>>>, (StatusCode, Json<ApiResponse<Vec<db::Feed>>>)> {
    match state.db.get_newsletter_feeds() {
        Ok(feeds) => Ok(ApiResponse::ok(feeds)),
        Err(e) => Err(ApiResponse::internal_err(format!("Database error: {}", e))),
    }
}

#[derive(Deserialize)]
struct CreateNewsletterFeedRequest {
    name: String,
}

/// POST /api/newsletter/feeds - Create a new newsletter feed
async fn create_newsletter_feed(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateNewsletterFeedRequest>,
) -> Result<Json<ApiResponse<i64>>, (StatusCode, Json<ApiResponse<i64>>)> {
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(ApiResponse::err("newsletter_feed_name_empty"));
    }
    match state.db.create_newsletter_feed(&name) {
        Ok(id) => Ok(ApiResponse::ok(id)),
        Err(e) => {
            let msg = format!("{}", e);
            if msg.contains("UNIQUE") {
                Err(ApiResponse::err("newsletter_feed_exists"))
            } else {
                Err(ApiResponse::err(format!("Failed to create newsletter feed: {}", e)))
            }
        }
    }
}

// ─── Web rendering config handlers ───────────────────────────────────

#[derive(Serialize)]
struct WebRenderingConfigResponse {
    provider: String,
    key_status: String,
    is_configured: bool,
}

/// GET /api/config/web-rendering - Get current web rendering API config (masked)
async fn get_web_rendering_config(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<WebRenderingConfigResponse>> {
    match state.db.get_web_rendering_config() {
        Ok(Some((provider, encrypted_key, iv))) => {
            let not_configured_label = if crate::is_system_italian() { "Non configurata" } else { "Not configured" };
            let key_display = if encrypted_key.is_empty() {
                not_configured_label.to_string()
            } else {
                match encryption::decrypt_api_key(&encrypted_key, &iv) {
                    Ok(plain) => encryption::mask_api_key(&plain),
                    Err(_) => "Error".to_string(),
                }
            };
            let is_configured = !encrypted_key.is_empty();
            ApiResponse::ok(WebRenderingConfigResponse {
                provider,
                key_status: key_display,
                is_configured,
            })
        }
        Ok(None) => ApiResponse::ok(WebRenderingConfigResponse {
            provider: "scrape_do".to_string(),
            key_status: if crate::is_system_italian() { "Non configurata".to_string() } else { "Not configured".to_string() },
            is_configured: false,
        }),
        Err(e) => {
            tracing::error!("Failed to get web rendering config: {}", e);
            ApiResponse::ok(WebRenderingConfigResponse {
                provider: "scrape_do".to_string(),
                key_status: if crate::is_system_italian() { "Non configurata".to_string() } else { "Not configured".to_string() },
                is_configured: false,
            })
        }
    }
}

#[derive(Deserialize)]
struct SaveWebRenderingConfigRequest {
    provider: String,
    api_key: String,
}

/// POST /api/config/web-rendering - Save and test web rendering API config
async fn save_web_rendering_config(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SaveWebRenderingConfigRequest>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    let provider = WebRenderingProvider::from_str(&body.provider)
        .ok_or_else(|| ApiResponse::<String>::err("Unknown web rendering provider"))?;

    // Test the API key before saving
    let client = reqwest::Client::new();
    web_feed::test_rendering_api(&client, &provider, &body.api_key)
        .await
        .map_err(|e| ApiResponse::<String>::err(format!("web_rendering_test_failed:{}", e)))?;

    // Encrypt and store
    let (encrypted, iv) = encryption::encrypt_api_key(&body.api_key)
        .map_err(|e| ApiResponse::<String>::err(format!("Encryption error: {}", e)))?;

    state.db.save_web_rendering_config(provider.as_str(), &encrypted, &iv)
        .map_err(|e| ApiResponse::<String>::internal_err(format!("DB error: {}", e)))?;

    Ok(ApiResponse::ok("OK".to_string()))
}

/// DELETE /api/config/web-rendering - Delete web rendering API config
async fn delete_web_rendering_config(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    state.db.delete_web_rendering_config()
        .map_err(|e| ApiResponse::<()>::internal_err(format!("DB error: {}", e)))?;

    Ok(ApiResponse::ok(()))
}

// ─── Web feed handlers ───────────────────────────────────────────────

#[derive(Deserialize)]
struct TestWebFeedRequest {
    url: String,
}

#[derive(Serialize)]
struct TestWebFeedResponse {
    pattern: String,
    articles_found: usize,
}

/// POST /api/web-feed/test - Test a web page scan before adding as web feed
async fn test_web_feed(
    State(state): State<Arc<AppState>>,
    Json(body): Json<TestWebFeedRequest>,
) -> Result<Json<ApiResponse<TestWebFeedResponse>>, (StatusCode, Json<ApiResponse<TestWebFeedResponse>>)> {
    // Ensure web rendering API is configured
    let (provider_str, encrypted_key, iv) = state.db
        .get_web_rendering_config()
        .map_err(|e| ApiResponse::<TestWebFeedResponse>::internal_err(format!("DB error: {}", e)))?
        .ok_or_else(|| ApiResponse::<TestWebFeedResponse>::err("web_rendering_not_configured"))?;

    if encrypted_key.is_empty() {
        return Err(ApiResponse::err("web_rendering_not_configured"));
    }

    let api_key = encryption::decrypt_api_key(&encrypted_key, &iv)
        .map_err(|e| ApiResponse::<TestWebFeedResponse>::err(format!("Decryption error: {}", e)))?;

    let provider = WebRenderingProvider::from_str(&provider_str)
        .ok_or_else(|| ApiResponse::<TestWebFeedResponse>::err("Unknown web rendering provider"))?;

    let client = reqwest::Client::new();
    let (pattern, count) = web_feed::test_web_page(&client, &provider, &api_key, &body.url)
        .await
        .map_err(|e| ApiResponse::<TestWebFeedResponse>::err(e))?;

    Ok(ApiResponse::ok(TestWebFeedResponse {
        pattern,
        articles_found: count,
    }))
}

#[derive(Deserialize)]
struct AddWebFeedRequest {
    url: String,
    site_name: String,
    category_id: i64,
    pattern: String,
}

/// POST /api/web-feed - Add a new web feed (must have been tested first)
async fn add_web_feed(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AddWebFeedRequest>,
) -> Result<Json<ApiResponse<i64>>, (StatusCode, Json<ApiResponse<i64>>)> {
    match state.db.add_web_feed(&body.url, &body.site_name, body.category_id, &body.pattern) {
        Ok(id) => Ok(ApiResponse::ok(id)),
        Err(e) => {
            let msg = format!("{}", e);
            if msg.contains("UNIQUE") {
                Err(ApiResponse::err("feed_already_exists"))
            } else {
                Err(ApiResponse::err(format!("Failed to add web feed: {}", e)))
            }
        }
    }
}

// ─── i18n handler ──────────────────────────────────────────────────────────

/// GET /api/i18n/{lang} - Get translations for the specified language
async fn get_translations(
    Path(lang): Path<String>,
) -> Json<ApiResponse<std::collections::HashMap<&'static str, &'static str>>> {
    let language = i18n::Language::from_browser_lang(&lang);
    let translations = i18n::get_translations(&language);
    ApiResponse::ok(translations)
}