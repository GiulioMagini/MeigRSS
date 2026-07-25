//! MeigRSS - Your personal news feed aggregator with AI summaries
//! Entry point: initializes DB, starts Axum HTTP server with embedded frontend,
//! and opens the browser automatically.
//!
//! Production: Frontend files are embedded in the binary via include_dir.
//! Debug: Frontend runs separately via `npm run dev` (Svelte dev server on port 5173).

mod api;
mod config;
mod db;
mod feed;
mod i18n;
mod llm;
mod scraper;
mod newsletter;
mod opml;
mod web_feed;

use std::sync::Arc;
use std::process;
use std::sync::atomic::{AtomicU8, Ordering};
use tracing::{error, info, warn};
use tracing_subscriber::{EnvFilter, fmt, prelude::*, registry};

use tower_http::cors::{CorsLayer, AllowOrigin};

#[cfg(not(debug_assertions))]
use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};

use api::AppState;
use db::Database;
use llm::LlmClient;

// ─── Embedded static files (production only) ──────────────────────────────

// Embed the built Svelte frontend at compile time.
// The frontend must be built (npm run build) into the /static folder
// before compiling the Rust binary in release mode.
#[cfg(not(debug_assertions))]
use include_dir::{include_dir, Dir};

#[cfg(not(debug_assertions))]
static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/static");

/// Serves the root path → index.html with security headers.
#[cfg(not(debug_assertions))]
async fn serve_root() -> Response {
    serve_embedded_file("index.html")
}

/// Core file-serving logic: resolves the file from the embedded directory,
/// sets MIME type and security headers (CSP for HTML, standard headers for all).
#[cfg(not(debug_assertions))]
fn serve_embedded_file(file_path: &str) -> Response {
    let clean_path = file_path.strip_prefix('/').unwrap_or(file_path);
    let clean_path = if clean_path.is_empty() { "index.html" } else { clean_path };

    match STATIC_DIR.get_file(clean_path) {
        Some(file) => {
            let mime_type = mime_guess::from_path(clean_path)
                .first_or_octet_stream()
                .to_string();

            let mut builder = Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, &mime_type)
                .header(header::CACHE_CONTROL, "public, max-age=86400")
                .header("X-Content-Type-Options", "nosniff")
                .header("X-Frame-Options", "DENY")
                .header("X-XSS-Protection", "1; mode=block")
                .header("Referrer-Policy", "strict-origin-when-cross-origin");

            // CSP only for HTML files — allows inline styles for Svelte,
            // Google Fonts for Material Icons, and restricts everything else
            if clean_path.ends_with(".html") {
                builder = builder.header(
                    "Content-Security-Policy",
                    "default-src 'self'; \
                     script-src 'self' 'unsafe-inline' 'unsafe-eval'; \
                     style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; \
                     font-src 'self' https://fonts.gstatic.com; \
                     img-src 'self' data:; \
                     connect-src 'self'; \
                     object-src 'none'; \
                     frame-ancestors 'none';"
                );
            }

            builder
                .body(axum::body::Body::from(file.contents().to_vec()))
                .unwrap()
                .into_response()
        }
        None => {
            // SPA fallback: serve index.html for unmatched routes
            // so Svelte's client-side router handles navigation
            if let Some(index) = STATIC_DIR.get_file("index.html") {
                Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "text/html")
                    .header("X-Content-Type-Options", "nosniff")
                    .header("X-Frame-Options", "DENY")
                    .header("X-XSS-Protection", "1; mode=block")
                    .header("Referrer-Policy", "strict-origin-when-cross-origin")
                    .header(
                        "Content-Security-Policy",
                        "default-src 'self'; \
                         script-src 'self' 'unsafe-inline' 'unsafe-eval'; \
                         style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; \
                         font-src 'self' https://fonts.gstatic.com; \
                         img-src 'self' data:; \
                         connect-src 'self'; \
                         object-src 'none'; \
                         frame-ancestors 'none';"
                    )
                    .body(axum::body::Body::from(index.contents().to_vec()))
                    .unwrap()
                    .into_response()
            } else {
                (StatusCode::NOT_FOUND, "File not found").into_response()
            }
        }
    }
}

// ─── Host validation middleware ────────────────────────────────────────────

/// Middleware that rejects requests whose Host header is not a local address.
/// Protects against DNS rebinding attacks, where a malicious website resolves
/// its own domain to 127.0.0.1 to bypass the browser's same-origin policy.
async fn validate_host(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let host_ok = request
        .headers()
        .get(axum::http::header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(|h| {
            // Strip the port, handling both "host:port" and "[ipv6]:port" forms
            let host = if h.starts_with('[') {
                h.split(']').next().map(|s| format!("{}]", s)).unwrap_or_default()
            } else {
                h.split(':').next().unwrap_or(h).to_string()
            };
            matches!(host.as_str(), "localhost" | "127.0.0.1" | "[::1]")
        })
        .unwrap_or(false);

    if host_ok {
        next.run(request).await
    } else {
        axum::response::Response::builder()
            .status(axum::http::StatusCode::FORBIDDEN)
            .body(axum::body::Body::from("Forbidden: invalid Host header"))
            .unwrap()
    }
}

// ─── Application entry point ───────────────────────────────────────────────

fn main() {
    // Initialize logging
    setup_logging();

    // Display startup banner
    display_banner();

    // Build the tokio async runtime
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads({
            let cpus = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4);
            std::cmp::min(cpus + 1, 16) // Cap at 16 threads
        })
        .thread_stack_size(2 * 1024 * 1024) // 2MB stack per thread
        .thread_name("meigrss-worker")
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    // Run the async application
    match rt.block_on(async_main()) {
        Ok(()) => {
            if is_system_italian() {
                info!("MeigRSS chiuso correttamente.");
            } else {
                info!("MeigRSS shutdown complete.");
            }
        }
        Err(e) => {
            error!("Application error: {}", e);
            process::exit(1);
        }
    }
}

// ─── Async main ────────────────────────────────────────────────────────────

async fn async_main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file if present (useful for development)
    let _ = dotenvy::dotenv();

    // Determine database path: defaults to ./data/meigrss.db relative to the executable
    let db_path = std::env::var("MEIGRSS_DB_PATH")
        .unwrap_or_else(|_| {
            let exe_dir = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                .unwrap_or_else(|| std::path::PathBuf::from("."));
            exe_dir.join("data").join("meigrss.db").to_string_lossy().to_string()
        });

    // Ensure the data directory exists
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db = Database::new(&db_path)?;
    if is_system_italian() {
        info!("Database inizializzato in: {}", db_path);
    } else {
        info!("Database initialized at: {}", db_path);
    }

    // Initialize the LLM client with rate limiting
    let llm = LlmClient::new();

    // Shared application state for all route handlers
    let state = Arc::new(AppState { db, llm });

    // Server configuration
    let port: u16 = std::env::var("MEIGRSS_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let bind_addr = format!("127.0.0.1:{}", port);

    // Frontend URL depends on build mode
    let frontend_link = {
        #[cfg(debug_assertions)]
        { format!("http://localhost:{}", 5173) } // Svelte dev server

        #[cfg(not(debug_assertions))]
        { format!("http://localhost:{}", port) }
    };

    // CORS: only allow requests from the frontend origin
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::exact(frontend_link.parse().unwrap()))
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    // Build the application router: API routes first, then static files
    let mut app = api::create_router(state.clone());

    // In production: serve embedded frontend files
    // In debug: frontend runs on its own dev server, no static serving needed
    #[cfg(not(debug_assertions))]
    {
        app = app
            .route("/", get(serve_root))
            .fallback(|uri: axum::http::Uri| async move {
                let path = uri.path().trim_start_matches('/');
                serve_embedded_file(path)
            });
    }

    // Apply CORS and Host validation (anti DNS-rebinding) to all routes
    let app = app
        .layer(cors)
        .layer(axum::middleware::from_fn(validate_host));

    // Bind and start the HTTP server
    if is_system_italian() {
        info!("MeigRSS avviato su {}", frontend_link);
        info!("Premi Ctrl+C per chiudere il programma");
    } else {
        info!("MeigRSS started on {}", frontend_link);
        info!("Press Ctrl+C to stop the program");
    }

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    // Open the browser automatically after a short delay
    let link_for_browser = frontend_link.clone();
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
        if let Err(e) = open::that(&link_for_browser) {
            if is_system_italian() {
                 warn!(
                "Non è stato possibile aprire il browser automaticamente: {}. Aprire il seguente link {} manualmente.",
                e, link_for_browser
            );
            } else {
                warn!(
                "Could not open browser automatically: {}. Please open {} manually.",
                e, link_for_browser
            );
            }
        }
    });

    // Background task: periodic auto-update and expired article purge.
    // Checks settings every 60 seconds and acts accordingly.
    let bg_state = Arc::clone(&state);
    tokio::spawn(async move {
        background_scheduler(bg_state).await;
    });

    // Graceful shutdown on Ctrl+C
    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to register Ctrl+C handler");
        if is_system_italian() {
            info!("Segnale di chiusura ricevuto, arresto del server...");
        } else {
            info!("Shutdown signal received, stopping server...");
        }
    };

    // Run the server
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    Ok(())
}

// ─── Background scheduler ─────────────────────────────────────────────────

/// Periodic background task that handles:
/// 1. Auto-update: fetches all feeds at the configured interval
/// 2. Purge: removes expired non-favorite articles based on retention setting
///
/// Runs in a loop, checking settings every 60 seconds.
/// The actual operations only fire when enough time has passed since the last run.
async fn background_scheduler(state: Arc<AppState>) {
    use tokio::time::{sleep, Duration, Instant};

    // Track when we last ran the auto-update
    let mut last_auto_update: Option<Instant> = None;

    loop {
        // Wait 60 seconds between checks to avoid busy-looping
        sleep(Duration::from_secs(60)).await;

        // ── Auto-update ────────────────────────────────────────────
        let auto_enabled = state.db
            .get_setting("auto_update_enabled")
            .ok()
            .flatten()
            .map(|v| v == "true")
            .unwrap_or(false);

        if auto_enabled {
            let interval_hours: u64 = state.db
                .get_setting("auto_update_interval_hours")
                .ok()
                .flatten()
                .and_then(|v| v.parse().ok())
                .unwrap_or(6);

            let update_interval = Duration::from_secs(interval_hours * 3600);

            let should_update = match last_auto_update {
                None => true,
                Some(last) => last.elapsed() >= update_interval,
            };

            if should_update {
                if is_system_italian() {
                    info!("Aggiornamento automatico dei feed in corso...");
                } else {
                    info!("Auto-updating feeds...");
                }

                feed::fetch_all_feeds(&state.db).await;
                last_auto_update = Some(Instant::now());

                if is_system_italian() {
                    info!("Aggiornamento automatico completato.");
                } else {
                    info!("Auto-update complete.");
                }
            }
        }
    }
}

// ─── Logging ───────────────────────────────────────────────────────────────

fn setup_logging() {
    // Debug: verbose with file/line info. Release: minimal with timestamps.
    #[cfg(debug_assertions)]
    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true);

    #[cfg(not(debug_assertions))]
    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false);

    #[cfg(debug_assertions)]
    let default_filter = "warn,meigrss=debug";

    #[cfg(not(debug_assertions))]
    let default_filter = "warn,meigrss=info";

    let filter_layer = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default_filter));

    registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();
}

// ─── Banner ────────────────────────────────────────────────────────────────

fn display_banner() {
    let version = env!("CARGO_PKG_VERSION");
    let is_italian = is_system_italian();

    println!("==================================================");
    println!("  MeigRSS v{}", version);
    if is_italian {
        println!("  Il tuo aggregatore di notizie personale");
        println!("  con riassunti potenziati dall'AI");
    } else {
        println!("  Your personal news feed aggregator");
        println!("  with AI-powered summaries");
    }
    println!("==================================================");
}

/// Cached language flag: 0 = English, 1 = Italian, 2 = not yet resolved.
/// Avoids opening a new database connection on every log call.
static LANG_IS_ITALIAN: AtomicU8 = AtomicU8::new(2);

/// Updates the cached language flag. Called lazily on first resolution
/// and by the API handler when the user changes the UI language.
pub fn set_language_cache(is_italian: bool) {
    LANG_IS_ITALIAN.store(if is_italian { 1 } else { 0 }, Ordering::Relaxed);
}

/// Detects if the application language is Italian, using a process-wide
/// cache to avoid re-reading the database on every call.
/// Priority: 1) cached value, 2) DB setting, 3) system environment.
pub fn is_system_italian() -> bool {
    // Fast path: return the cached value if already resolved
    match LANG_IS_ITALIAN.load(Ordering::Relaxed) {
        0 => return false,
        1 => return true,
        _ => {}
    }
    let result = resolve_system_italian();
    set_language_cache(result);
    result
}

/// Performs the actual language resolution (DB setting, then environment).
/// Called only once per process, or again after a language change.
fn resolve_system_italian() -> bool {
    // Priority 1: check DB setting saved by the frontend
    let db_path = std::env::var("MEIGRSS_DB_PATH").unwrap_or_else(|_| {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        exe_dir.join("data").join("meigrss.db").to_string_lossy().to_string()
    });

    if std::path::Path::new(&db_path).exists()
        && let Ok(conn) = rusqlite::Connection::open(&db_path)
        && let Ok(lang) = conn.query_row(
            "SELECT value FROM settings WHERE key = 'language'",
            [],
            |row| row.get::<_, String>(0),
        ) {
        if lang == "it" {
            return true;
        }
        if lang == "en" {
            return false;
        }
        // "auto" or unknown → fall through to system detection
    }

    // Priority 2: check system environment
    for var in &["LANG", "LC_ALL", "LC_MESSAGES"] {
        if let Ok(val) = std::env::var(var)
            && val.to_lowercase().starts_with("it") {
            return true;
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(val) = std::env::var("USERLANG")
            && val.to_lowercase().starts_with("it") {
            return true;
        }
    }

    false
}