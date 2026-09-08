use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::db::{self, ArticleQuery, Database};
use crate::config::{encryption, LlmProvider, WebRenderingProvider};
use crate::feed;
use crate::scraper;
use crate::newsletter;
use crate::llm::LlmClient;
use crate::i18n;
use crate::web_feed;
use crate::opml;

/// How long a tested feed stays available for the add endpoint to reuse.
/// Long enough to cover the user reviewing the title and confirming, short
/// enough that content already gone stale is never imported.
const PENDING_FEED_TTL: Duration = Duration::from_secs(5 * 60);

/// A feed downloaded by the test endpoint and not added yet.
struct PendingFeed {
    stored_at: Instant,
    tested: feed::TestedFeed,
}

/// Shared application state passed to all route handlers.
pub struct AppState {
    pub db: Database,
    pub llm: LlmClient,
    /// Bodies fetched by `/api/feeds/test`, keyed by URL, so that adding the
    /// feed right after does not hit the same server a second time.
    ///
    /// A `std` mutex is correct here: it is only held for the map operation
    /// itself and never across an `await`.
    pending_feeds: Mutex<HashMap<String, PendingFeed>>,
}

impl AppState {
    /// Builds the state with an empty pending-feed cache.
    pub fn new(db: Database, llm: LlmClient) -> Self {
        Self {
            db,
            llm,
            pending_feeds: Mutex::new(HashMap::new()),
        }
    }
}

/// Maximum size of a whole bulk newsletter upload. The batch is buffered in
/// full before it reaches the transaction, so this doubles as the memory
/// ceiling of the endpoint.
const BULK_IMPORT_BODY_LIMIT: usize = 100 * 1024 * 1024;

/// Maximum size of a single EML, whether uploaded on its own or inside a batch.
/// A newsletter carrying inline base64 images stays well below this.
const NEWSLETTER_MAX_FILE_SIZE: usize = 5 * 1024 * 1024;

/// Maximum number of EML files accepted in one batch. Past this point the
/// transaction runs long enough to leave the user without feedback.
const BULK_IMPORT_MAX_FILES: usize = 500;

/// Maximum size of a feed import JSON. The export is cumulative and grows with
/// every archived newsletter, and since the export endpoint is a GET it has no
/// limit of its own: too low a value here produces a backup that can be
/// written but never restored. Deliberately generous for that reason.
const FEEDS_IMPORT_BODY_LIMIT: usize = 256 * 1024 * 1024;

/// Maximum size of a manually edited article full text. A long article stays
/// well under this, and the cap keeps a paste of arbitrary size out of the
/// database. Comfortably inside the default body limit of the route.
const FULL_TEXT_MAX_SIZE: usize = 200 * 1024;

/// Builds the complete API router with all endpoints.
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Article endpoints
        .route("/api/articles", get(get_articles))
        .route("/api/articles/{id}/favorite", put(toggle_favorite))
        .route("/api/articles/{id}/summary", post(generate_summary))
        .route("/api/articles/{id}/scrape", post(scrape_article))
        .route("/api/articles/{id}/read", put(mark_article_read))
        .route("/api/articles/{id}/full-text", put(update_full_text))
        // Feed endpoints
        .route("/api/feeds", get(get_feeds))
        .route("/api/feeds", post(add_feed))
        .route("/api/feeds/{id}", delete(delete_feed))
        .route("/api/feeds/{id}/force-ai", put(set_feed_force_ai))
        .route("/api/feeds/{id}/scraping", put(set_feed_scraping))
        .route("/api/feeds/{id}/pause", put(set_feed_paused))
        .route("/api/feeds/{id}/api-scraping", put(set_feed_api_scraping))
        .route("/api/feeds/{id}/category", put(change_feed_category))
        .route("/api/feeds/test", post(test_feed))
        .route("/api/feeds/refresh", post(refresh_feeds))
        .route("/api/feeds/export", get(export_feeds))
        .route("/api/feeds/export/opml", get(export_feeds_opml))
        .route(
            "/api/feeds/import",
            post(import_feeds).layer(DefaultBodyLimit::max(FEEDS_IMPORT_BODY_LIMIT)),
        )
        .route("/api/feeds/import/opml", post(parse_feeds_opml))
        .route(
            "/api/newsletter/import",
            post(import_newsletter).layer(DefaultBodyLimit::max(NEWSLETTER_MAX_FILE_SIZE)),
        )
        .route(
            "/api/newsletter/import-bulk",
            post(import_newsletter_bulk).layer(DefaultBodyLimit::max(BULK_IMPORT_BODY_LIMIT)),
        )
        .route(
            "/api/newsletter/preview",
            post(preview_newsletter).layer(DefaultBodyLimit::max(NEWSLETTER_MAX_FILE_SIZE)),
        )
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

#[derive(Serialize)]
struct SummaryResponse {
    summary: String,
    source: String,
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

#[derive(Deserialize)]
struct FullTextUpdateRequest {
    full_text: String,
}

/// PUT /api/articles/{id}/full-text - Replace the full text of an article.
///
/// Meant for text the user writes or pastes by hand, which is the only way to
/// give a full text to an article the scraper cannot reach.
///
/// An empty text is accepted and clears the field: automatic scraping then
/// takes over again the next time the card is expanded. A non-empty text, on
/// the contrary, permanently stops the automatic scraping for that article, so
/// what the user wrote is never silently overwritten.
async fn update_full_text(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<FullTextUpdateRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    if body.full_text.len() > FULL_TEXT_MAX_SIZE {
        return Err(ApiResponse::err("full_text_too_long"));
    }

    match state.db.update_article_full_text(&id, &body.full_text) {
        Ok(_) => Ok(ApiResponse::ok(())),
        Err(e) => Err(ApiResponse::internal_err(format!("Database error: {}", e))),
    }
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
    let id = match state.db.add_feed(&body.url, &body.title, &body.site_name, body.category_id) {
        Ok(id) => id,
        Err(e) => return Err(ApiResponse::err(format!("Failed to add feed: {}", e))),
    };

    // Take the entry out whether it is fresh or stale: either way it has served
    // its purpose and must not linger.
    let pending = state
        .pending_feeds
        .lock()
        .ok()
        .and_then(|mut map| map.remove(&body.url))
        .filter(|p| p.stored_at.elapsed() < PENDING_FEED_TTL);

    // Nothing cached means the feed was added without a test (bulk import, or
    // the user waited too long): the next scheduled poll fills it in, as before.
    if let Some(pending) = pending {
        match feed::import_tested_feed(&state.db, id, &body.url, &pending.tested) {
            Ok(count) => {
                if crate::is_system_italian() {
                    tracing::info!("Feed '{}': {} articoli importati dal test", body.site_name, count);
                } else {
                    tracing::info!("Feed '{}': {} articles imported from the test", body.site_name, count);
                }
            }
            Err(e) => {
                // The feed row exists and is valid: a failed import only means
                // the articles arrive with the next poll.
                tracing::warn!("Failed to import tested feed '{}': {}", body.url, e);
            }
        }
    }

    Ok(ApiResponse::ok(id))
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
            if query.delete_categories.unwrap_or(false)
                && let Err(e) = state.db.delete_all_categories() {
                tracing::error!("Failed to delete categories: {}", e);
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
struct ApiScrapingRequest {
    enable: bool,
}

/// PUT /api/feeds/{id}/api-scraping - Toggle the rendering API for a web feed.
/// When disabled the page is fetched with a plain GET instead.
async fn set_feed_api_scraping(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(body): Json<ApiScrapingRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    match state.db.set_feed_api_scraping(id, body.enable) {
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
    State(state): State<Arc<AppState>>,
    Json(body): Json<TestFeedRequest>,
) -> Result<Json<ApiResponse<TestFeedResponse>>, (StatusCode, Json<ApiResponse<TestFeedResponse>>)> {
    match feed::test_feed_url(&body.url).await {
        Ok(tested) => {
            let response = TestFeedResponse {
                title: tested.title.clone(),
                site_name: tested.site_name.clone(),
            };

            if let Ok(mut pending) = state.pending_feeds.lock() {
                // Drop whatever expired while nobody was looking, so the map
                // stays bounded by what the user tested in the last few minutes.
                pending.retain(|_, p| p.stored_at.elapsed() < PENDING_FEED_TTL);
                pending.insert(
                    body.url.clone(),
                    PendingFeed { stored_at: Instant::now(), tested },
                );
            }

            Ok(ApiResponse::ok(response))
        }
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

/// GET /api/feeds/export/opml - Export all categories and feeds as OPML 2.0
async fn export_feeds_opml(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    let categories = state.db.export_categories()
        .map_err(|e| ApiResponse::<String>::internal_err(format!("Database error: {}", e)))?;
    let feeds = state.db.export_feeds()
        .map_err(|e| ApiResponse::<String>::internal_err(format!("Database error: {}", e)))?;

    // OPML has no equivalent for favorites and settings, so they stay empty;
    // the JSON export remains the full-fidelity format
    let export = db::FullExport {
        categories,
        feeds,
        favorites: Vec::new(),
        settings: None,
    };

    Ok(ApiResponse::ok(opml::build_opml(&export)))
}

/// Request body for OPML parsing: the raw file content as text.
#[derive(Deserialize)]
struct OpmlImportRequest {
    opml: String,
}

/// POST /api/feeds/import/opml - Convert an uploaded OPML file into the
/// internal import structure. This deliberately does not write to the
/// database: the frontend shows the confirmation and category selector, then
/// posts the result to /api/feeds/import, so OPML and JSON imports share a
/// single write path.
async fn parse_feeds_opml(
    State(state): State<Arc<AppState>>,
    Json(body): Json<OpmlImportRequest>,
) -> Result<Json<ApiResponse<db::FullExport>>, (StatusCode, Json<ApiResponse<db::FullExport>>)> {
    // Outlines outside any folder land in the default category
    let default_category = state.db.default_category_name()
        .map_err(|e| ApiResponse::<db::FullExport>::internal_err(format!("Database error: {}", e)))?;

    match opml::parse_opml(&body.opml, &default_category) {
        Ok(parsed) => Ok(ApiResponse::ok(parsed)),
        Err(e) => Err(ApiResponse::<db::FullExport>::err(e)),
    }
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
            let llm_provider = LlmProvider::from_str(&provider).unwrap_or(LlmProvider::Mistral500Chars);
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
                // Canonical form: legacy values stored in the database are
                // normalized here so the frontend only ever sees current values
                provider: llm_provider.as_str().to_string(),
                key_status: key_display,
                supports_usage_query: llm_provider.supports_usage_query(),
            })
        }
        Ok(None) => ApiResponse::ok(ApiConfigResponse {
            provider: "mistral_500_chars".to_string(),
            key_status: if crate::is_system_italian() { "Non configurata".to_string() } else { "Not configured".to_string() },
            supports_usage_query: true,
        }),
        Err(e) => {
            tracing::error!("Failed to get API config: {}", e);
            ApiResponse::ok(ApiConfigResponse {
                provider: "mistral_500_chars".to_string(),
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
        Err(e) => Err(ApiResponse::err(e.to_string())),
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
        Err(e) => Err(ApiResponse::err(e.to_string())),
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
    if !(1..=720).contains(&retention) {
        return Err(ApiResponse::err("Retention must be between 1 and 720 hours"));
    }

    // Validate auto_update_interval: 1-72 hours
    let interval: u32 = body.auto_update_interval_hours.parse()
        .map_err(|_| ApiResponse::<()>::err("Invalid interval value"))?;
    if !(1..=72).contains(&interval) {
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

/// POST /api/newsletter/import - Import a single newsletter from an EML file.
///
/// The body carries the `file` itself plus a `title` and a `feed_id` text
/// field. The file travels as multipart, not inside JSON, so that the bytes
/// reach the parser exactly as they are on disk.
async fn import_newsletter(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    let mut eml: Option<axum::body::Bytes> = None;
    let mut title = String::new();
    let mut feed_id: Option<i64> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| ApiResponse::<String>::err("newsletter_invalid_file"))?
    {
        // The name has to be read before the field is consumed by its body
        let name = field.name().unwrap_or_default().to_string();

        match name.as_str() {
            "file" => {
                eml = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|_| ApiResponse::<String>::err("newsletter_invalid_file"))?,
                );
            }
            "title" => {
                title = field
                    .text()
                    .await
                    .map_err(|_| ApiResponse::<String>::err("newsletter_invalid_file"))?;
            }
            "feed_id" => {
                let raw = field
                    .text()
                    .await
                    .map_err(|_| ApiResponse::<String>::err("newsletter_invalid_file"))?;
                feed_id = raw.trim().parse::<i64>().ok();
            }
            _ => {
                let _ = field.bytes().await;
            }
        }
    }

    let (Some(eml), Some(feed_id)) = (eml, feed_id) else {
        return Err(ApiResponse::err("newsletter_invalid_file"));
    };

    match newsletter::import_eml(&eml, &title, feed_id, &state.db) {
        Ok(article_id) => Ok(ApiResponse::ok(article_id)),
        Err(e) => Err(ApiResponse::err(e)),
    }
}

/// POST /api/newsletter/import-bulk - Import every EML of a folder at once.
///
/// The body carries a `feed_id` text field followed by one `files` field per
/// EML. Files travel as raw bytes on purpose: decoding them as text in the
/// browser would corrupt every message whose MIME headers declare an encoding
/// other than UTF-8.
async fn import_newsletter_bulk(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result <
    Json<ApiResponse<newsletter::BulkImportOutcome>>,
    (StatusCode, Json<ApiResponse<newsletter::BulkImportOutcome>>),
> {
    let mut feed_id: Option<i64> = None;
    let mut files: Vec<(String, axum::body::Bytes)> = Vec::new();

    while let Some(field) = multipart.next_field().await.map_err(|_| {
        ApiResponse::<newsletter::BulkImportOutcome>::err("newsletter_invalid_file")
    })? {
        // Both names have to be read before the field is consumed by its body
        let name = field.name().unwrap_or_default().to_string();
        let file_name = field.file_name().unwrap_or_default().to_string();

        match name.as_str() {
            "feed_id" => {
                let raw = field.text().await.map_err(|_| {
                    ApiResponse::<newsletter::BulkImportOutcome>::err("newsletter_invalid_file")
                })?;
                feed_id = raw.trim().parse::<i64>().ok();
            }
            "files" => {
                // Checked before reading the body, so an oversized folder is
                // refused without buffering the file that goes over the edge
                if files.len() >= BULK_IMPORT_MAX_FILES {
                    return Err(ApiResponse::err("newsletter_folder_too_many"));
                }

                let bytes = field.bytes().await.map_err(|_| {
                    ApiResponse::<newsletter::BulkImportOutcome>::err("newsletter_invalid_file")
                })?;

                if bytes.len() > NEWSLETTER_MAX_FILE_SIZE {
                    return Err(ApiResponse::err("newsletter_folder_too_large"));
                }

                files.push((file_name, bytes));
            }
            // Drain anything unexpected instead of leaving it in the stream
            _ => {
                let _ = field.bytes().await;
            }
        }
    }

    let Some(feed_id) = feed_id else {
        return Err(ApiResponse::err("newsletter_invalid_file"));
    };

    if files.is_empty() {
        return Err(ApiResponse::err("newsletter_folder_no_eml"));
    }

    // Parsing and Readability are CPU-bound: run on the whole batch inside the
    // async executor they would stall every other request until the folder is
    // done, so the work goes to the blocking pool.
    let outcome = tokio::task::spawn_blocking(move || {
        newsletter::import_eml_batch(&files, feed_id, &state.db)
    })
    .await
    .map_err(|e| ApiResponse::internal_err(format!("Bulk import task failed: {}", e)))?;

    match outcome {
        Ok(outcome) => Ok(ApiResponse::ok(outcome)),
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

#[derive(Serialize)]
struct NewsletterPreviewResponse {
    title: String,
    author: String,
    date: String,
}

/// POST /api/newsletter/preview - Parse an EML and return its metadata.
///
/// Takes the file as multipart for the same reason as the import: the sender
/// name shown in the preview comes from the message headers, and decoding the
/// file as text beforehand would garble it before it is ever displayed.
async fn preview_newsletter(
    mut multipart: Multipart,
) -> Result <
    Json<ApiResponse<NewsletterPreviewResponse>>,
    (StatusCode, Json<ApiResponse<NewsletterPreviewResponse>>),
> {
    let mut eml: Option<axum::body::Bytes> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| ApiResponse::<NewsletterPreviewResponse>::err("newsletter_invalid_file"))?
    {
        let name = field.name().unwrap_or_default().to_string();
        let bytes = field
            .bytes()
            .await
            .map_err(|_| ApiResponse::<NewsletterPreviewResponse>::err("newsletter_invalid_file"))?;

        if name == "file" {
            eml = Some(bytes);
        }
    }

    let Some(eml) = eml else {
        return Err(ApiResponse::err("newsletter_invalid_file"));
    };

    let message = mail_parser::MessageParser::default()
        .parse(eml.as_ref())
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
    // Tells the frontend which mode the successful test used, so the new feed
    // starts out matching the behaviour the user has just seen working
    is_api_scraping: bool,
}

/// POST /api/web-feed/test - Test a web page scan before adding as web feed
async fn test_web_feed(
    State(state): State<Arc<AppState>>,
    Json(body): Json<TestWebFeedRequest>,
) -> Result<Json<ApiResponse<TestWebFeedResponse>>, (StatusCode, Json<ApiResponse<TestWebFeedResponse>>)> {
    // The rendering API is optional: when it is not configured the page is
    // fetched with a plain GET, which works on server-rendered sites
    let config = state.db
        .get_web_rendering_config()
        .map_err(|e| ApiResponse::<TestWebFeedResponse>::internal_err(format!("DB error: {}", e)))?;

    let mut rendering: Option<(WebRenderingProvider, String)> = None;
    if let Some((provider_str, encrypted_key, iv)) = config
        && !encrypted_key.is_empty()
    {
        let api_key = encryption::decrypt_api_key(&encrypted_key, &iv)
            .map_err(|e| ApiResponse::<TestWebFeedResponse>::err(format!("Decryption error: {}", e)))?;
        let provider = WebRenderingProvider::from_str(&provider_str)
            .ok_or_else(|| ApiResponse::<TestWebFeedResponse>::err("Unknown web rendering provider"))?;
        rendering = Some((provider, api_key));
    }

    let used_api = rendering.is_some();
    let rendering_ref = rendering.as_ref().map(|(p, k)| (p, k.as_str()));

    let client = reqwest::Client::new();
    let (pattern, count) = web_feed::test_web_page(&client, rendering_ref, &body.url)
        .await
        .map_err(ApiResponse::<TestWebFeedResponse>::err)?;

    Ok(ApiResponse::ok(TestWebFeedResponse {
        pattern,
        articles_found: count,
        is_api_scraping: used_api,
    }))
}

#[derive(Deserialize)]
struct AddWebFeedRequest {
    url: String,
    site_name: String,
    category_id: i64,
    pattern: String,
    // Defaulted so a request without the field keeps the free fetch mode
    #[serde(default)]
    is_api_scraping: bool,
}

/// POST /api/web-feed - Add a new web feed (must have been tested first)
async fn add_web_feed(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AddWebFeedRequest>,
) -> Result<Json<ApiResponse<i64>>, (StatusCode, Json<ApiResponse<i64>>)> {
    match state.db.add_web_feed(
        &body.url,
        &body.site_name,
        body.category_id,
        &body.pattern,
        body.is_api_scraping,
    ) {
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