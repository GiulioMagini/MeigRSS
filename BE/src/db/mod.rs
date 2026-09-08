use rusqlite::{Connection, Result, params};
use std::sync::Mutex;

pub mod queries;
pub use queries::*;

/// Thread-safe wrapper around SQLite connection.
/// SQLite in WAL mode supports concurrent reads with a single writer,
/// which is sufficient for our single-user local application.
pub struct Database {
    pub conn: Mutex<Connection>,
}

impl Database {
    /// Opens (or creates) the SQLite database at the given path
    /// and initializes the schema if tables don't exist yet.
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // Enable WAL mode for better concurrent read performance
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        // Enable foreign key enforcement
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;

        let db = Database {
            conn: Mutex::new(conn),
        };
        db.initialize_schema()?;
        Ok(db)
    }

    /// Creates all tables if they don't already exist.
    /// This is idempotent and safe to call on every startup.
    fn initialize_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(
            "
            -- Categories for organizing feeds
            CREATE TABLE IF NOT EXISTS categories (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                name        TEXT NOT NULL UNIQUE,
                is_default  INTEGER NOT NULL DEFAULT 0  -- 1 for 'Non categorizzati', cannot be deleted
            );

            -- RSS feed sources
            CREATE TABLE IF NOT EXISTS feeds (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                url             TEXT NOT NULL UNIQUE,
                title           TEXT NOT NULL DEFAULT '',
                site_name       TEXT NOT NULL DEFAULT '',
                category_id     INTEGER NOT NULL,
                force_ai_summary INTEGER NOT NULL DEFAULT 0,  -- 1 = always use LLM for this feed
                enable_scraping  INTEGER NOT NULL DEFAULT 0,  -- 1 = allow full text scraping for this feed
                is_paused        INTEGER NOT NULL DEFAULT 0,  -- 1 = skip this feed during updates
                is_web_feed      INTEGER NOT NULL DEFAULT 0,
                web_feed_pattern TEXT NOT NULL DEFAULT '',
                is_api_scraping  INTEGER NOT NULL DEFAULT 0,  -- 1 = fetch via rendering API, 0 = plain GET
                etag             TEXT NOT NULL DEFAULT '',     -- last ETag returned by the server, sent back verbatim
                last_modified    TEXT NOT NULL DEFAULT '',     -- last Last-Modified returned by the server, sent back verbatim
                created_at       TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (category_id) REFERENCES categories(id)
            );

            -- Individual articles fetched from feeds
            CREATE TABLE IF NOT EXISTS articles (
                id              TEXT PRIMARY KEY,              -- UUID
                feed_id         INTEGER NOT NULL,
                url             TEXT NOT NULL UNIQUE,
                title           TEXT NOT NULL DEFAULT '',
                author          TEXT NOT NULL DEFAULT '',
                published_at    TEXT,                          -- ISO 8601 datetime
                full_text       TEXT NOT NULL DEFAULT '',      -- extracted article body
                summary         TEXT NOT NULL DEFAULT '',
                summary_source  TEXT NOT NULL DEFAULT 'none',  -- 'none', 'feed', 'llm'
                is_favorite     INTEGER NOT NULL DEFAULT 0,
                is_read         INTEGER NOT NULL DEFAULT 0,
                newsletter_html TEXT NOT NULL DEFAULT '',  -- original HTML for imported newsletters
                fetched_at      TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (feed_id) REFERENCES feeds(id) ON DELETE CASCADE
            );

            -- API provider configuration (single row, updated in place)
            CREATE TABLE IF NOT EXISTS api_config (
                id              INTEGER PRIMARY KEY CHECK (id = 1),  -- enforce single row
                provider        TEXT NOT NULL DEFAULT 'mistral_500_chars',
                api_key_enc     TEXT NOT NULL DEFAULT '',            -- encrypted API key
                encryption_iv   TEXT NOT NULL DEFAULT '',            -- initialization vector for decryption
                updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
            );

            -- Application settings (key-value store)
            CREATE TABLE IF NOT EXISTS settings (
                key     TEXT PRIMARY KEY,
                value   TEXT NOT NULL
            );

            -- Create indexes for common query patterns
            CREATE INDEX IF NOT EXISTS idx_articles_feed_id ON articles(feed_id);
            CREATE INDEX IF NOT EXISTS idx_articles_published_at ON articles(published_at);
            CREATE INDEX IF NOT EXISTS idx_articles_is_favorite ON articles(is_favorite);
            CREATE INDEX IF NOT EXISTS idx_articles_fetched_at ON articles(fetched_at);
            CREATE INDEX IF NOT EXISTS idx_feeds_category_id ON feeds(category_id);

            -- Web rendering API configuration (single row, same pattern as api_config)
            CREATE TABLE IF NOT EXISTS web_rendering_config (
                id              INTEGER PRIMARY KEY CHECK (id = 1),
                provider        TEXT NOT NULL DEFAULT 'scrape_do',
                api_key_enc     TEXT NOT NULL DEFAULT '',
                encryption_iv   TEXT NOT NULL DEFAULT '',
                updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
            );
            "
        )?;

        // Migration: databases created before web feeds could work without a
        // rendering API have no is_api_scraping column
        let has_api_scraping = {
            let mut stmt = conn.prepare(
                "SELECT 1 FROM pragma_table_info('feeds') WHERE name = 'is_api_scraping'",
            )?;
            stmt.exists([])?
        };
        if !has_api_scraping {
            conn.execute(
                "ALTER TABLE feeds ADD COLUMN is_api_scraping INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
            // Existing web feeds were created when the rendering API was the
            // only option, so keep them on it to preserve their behaviour
            conn.execute(
                "UPDATE feeds SET is_api_scraping = 1 WHERE is_web_feed = 1",
                [],
            )?;
        }

        // Migration: databases created before conditional GET support have no
        // columns to store the HTTP validators sent back by the feed server.
        // Both columns are always added together, so checking one is enough.
        let has_validators = {
            let mut stmt = conn.prepare(
                "SELECT 1 FROM pragma_table_info('feeds') WHERE name = 'etag'",
            )?;
            stmt.exists([])?
        };
        if !has_validators {
            conn.execute(
                "ALTER TABLE feeds ADD COLUMN etag TEXT NOT NULL DEFAULT ''",
                [],
            )?;
            conn.execute(
                "ALTER TABLE feeds ADD COLUMN last_modified TEXT NOT NULL DEFAULT ''",
                [],
            )?;
        }

        // Ensure the default category exists
        conn.execute(
            "INSERT OR IGNORE INTO categories (name, is_default) VALUES (?1, 1)",
            params!["Non categorizzati"],
        )?;

        // Ensure the default "Newsletter" category exists (non-deletable, like the default category)
        conn.execute(
            "INSERT OR IGNORE INTO categories (name, is_default) VALUES (?1, 1)",
            params!["Newsletter"],
        )?;

        // Ensure default settings exist
        let default_settings = vec![
            ("retention_hours", "24"),           // default 1 day
            ("auto_update_enabled", "false"),    // default no auto update
            ("auto_update_interval_hours", "1"), // default refresh every 1 hour
            ("highlight_keywords", ""),          // keywords to be highlighted
            ("language", "auto"),                // 'auto', 'it', 'en'
        ];
        for (key, value) in default_settings {
            conn.execute(
                "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
                params![key, value],
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_creation() {
        // In-memory DB for testing
        let db = Database::new(":memory:").expect("Failed to create in-memory DB");
        let conn = db.conn.lock().unwrap();

        // Verify default category was created
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM categories WHERE is_default = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}