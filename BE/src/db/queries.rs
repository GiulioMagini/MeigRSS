use rusqlite::{params, Connection, OptionalExtension, Result};
use serde::{Deserialize, Serialize};
use super::Database;

// ─── Data models ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub is_default: bool,
}

/// Maximum number of tags a favorite article can have, which is also the
/// maximum number of tags the tag search accepts at once: the two limits are
/// kept equal on purpose.
pub const MAX_TAGS: usize = 3;

/// A favorite tag as listed for management, with the number of articles using
/// it so that deleting a tag in use can ask for confirmation first.
#[derive(Debug, Serialize)]
pub struct FavoriteTag {
    pub id: i64,
    pub name: String,
    pub article_count: i64,
}

/// A tag as attached to an article.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArticleTag {
    pub id: i64,
    pub name: String,
}

/// Result of assigning a tag to an article, one variant per case the API
/// reports differently.
pub enum TagAssignment {
    /// The tag is on the article (just added or already there): its tags.
    Assigned(Vec<ArticleTag>),
    ArticleNotFound,
    /// Tags can only be assigned to favorites.
    NotFavorite,
    TagNotFound,
    /// The article already has `MAX_TAGS` tags.
    LimitReached,
}

/// SQL expression returning the tags of the article aliased `a` as a JSON
/// array of `{id, name}` objects ordered by name (`[]` when it has none).
/// Shared by every query that returns articles together with their tags.
const ARTICLE_TAGS_JSON: &str =
    "(SELECT json_group_array(json_object('id', t.id, 'name', t.name) ORDER BY t.name COLLATE NOCASE, t.name)
      FROM article_tags l
      JOIN favorite_tags t ON t.id = l.tag_id
      WHERE l.article_id = a.id)";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Feed {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub site_name: String,
    pub category_id: i64,
    pub force_ai_summary: bool,
    pub enable_scraping: bool,
    pub is_paused: bool,
    pub is_web_feed: bool,
    pub web_feed_pattern: String,
    pub is_api_scraping: bool,
    // HTTP validators returned by the feed server, stored and sent back
    // verbatim (quotes and the `W/` weak prefix included): they are opaque
    // strings and must never be interpreted. Empty means "no validator yet".
    pub etag: String,
    pub last_modified: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Article {
    pub id: String,
    pub feed_id: i64,
    pub url: String,
    pub title: String,
    pub author: String,
    pub published_at: Option<String>,
    pub full_text: String,
    pub summary: String,
    pub summary_source: String,
    pub is_favorite: bool,
    pub newsletter_html: String,
    pub fetched_at: String,
}

/// Query parameters for listing articles with filters, sorting and pagination.
#[derive(Debug, Deserialize)]
pub struct ArticleQuery {
    pub category_id: Option<i64>,
    pub favorites_filter: Option<String>,  // "all", "only", "exclude"
    pub sort_order: Option<String>,   // "asc" or "desc" (by published_at)
    pub page: Option<u32>,
    pub per_page: Option<u32>,        // 10, 50, or 100
    pub search: Option<String>,       // search in article title
    // Favorite tags to match (any of them), as "3,7,12". Applied only together
    // with favorites_filter = "only"
    #[serde(default, deserialize_with = "deserialize_tag_ids")]
    pub tag_ids: Vec<i64>,
}

/// Parses the comma-separated `tag_ids` query parameter. Duplicates are
/// dropped; a malformed id or more than `MAX_TAGS` distinct ids reject the
/// whole request, since the frontend never produces them.
fn deserialize_tag_ids<'de, D>(deserializer: D) -> std::result::Result<Vec<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    let raw = String::deserialize(deserializer)?;
    let mut ids: Vec<i64> = Vec::new();
    for part in raw.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let id = part
            .parse::<i64>()
            .map_err(|_| D::Error::custom(format!("invalid tag id: {part}")))?;
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    if ids.len() > MAX_TAGS {
        return Err(D::Error::custom(format!("at most {MAX_TAGS} tags can be searched at once")));
    }
    Ok(ids)
}

#[derive(Debug, Serialize)]
pub struct ArticleListResponse {
    pub articles: Vec<ArticleWithFeed>,
    pub total_count: u64,
    pub page: u32,
    pub per_page: u32,
}

/// Article joined with feed info for display purposes.
#[derive(Debug, Serialize, Clone)]
pub struct ArticleWithFeed {
    pub id: String,
    pub feed_id: i64,
    pub url: String,
    pub title: String,
    pub author: String,
    pub published_at: Option<String>,
    pub full_text: String,
    pub summary: String,
    pub summary_source: String,
    pub is_favorite: bool,
    pub fetched_at: String,
    pub site_name: String,
    pub feed_title: String,
    pub category_id: i64,
    pub category_name: String,
    pub enable_scraping: bool,
    pub force_ai_summary: bool,
    pub is_read: bool,
    pub newsletter_html: String,
    pub tags: Vec<ArticleTag>,        // always empty for non-favorites
}

/// Lightweight structure for feed import/export operations.
#[derive(Debug, Serialize, Deserialize)]
pub struct FeedExport {
    pub url: String,
    pub title: String,
    pub site_name: String,
    pub category_name: String,
    pub force_ai_summary: bool,
    pub enable_scraping: bool,
    pub is_paused: bool,
    pub is_web_feed: bool,
    pub web_feed_pattern: String,
    // Defaulted so JSON files exported before this option existed still import
    #[serde(default)]
    pub is_api_scraping: bool,
}

/// Complete export structure including categories and feeds.
#[derive(Debug, Serialize, Deserialize)]
pub struct FullExport {
    pub categories: Vec<CategoryExport>,
    pub feeds: Vec<FeedExport>,
    pub favorites: Vec<FavoriteArticleExport>,
    #[serde(default)]
    pub settings: Option<SettingsExport>,
    // Names of all favorite tags, including unused ones. Defaulted so JSON
    // files exported before favorite tags existed still import
    #[serde(default)]
    pub favorite_tags: Vec<String>,
}

/// Result of an import operation with separate counters.
#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub feeds_imported: u32,
    pub favorites_imported: u32,
}

/// Favorite article structure for export/import.
#[derive(Debug, Serialize, Deserialize)]
pub struct FavoriteArticleExport {
    pub url: String,
    pub title: String,
    pub author: String,
    pub published_at: Option<String>,
    pub full_text: String,
    pub summary: String,
    pub summary_source: String,
    pub feed_url: String,
    pub newsletter_html: String,
    // Tag names, re-linked by name on import. Defaulted for older exports
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Lightweight category structure for export (excludes the default category).
#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryExport {
    pub name: String,
}

/// Application settings for export/import.
#[derive(Debug, Serialize, Deserialize)]
pub struct SettingsExport {
    pub retention_hours: String,
    pub auto_update_enabled: String,
    pub auto_update_interval_hours: String,
    pub highlight_keywords: String,
}

// ─── Category operations ───────────────────────────────────────────────────

impl Database {
    /// Returns all categories ordered by name, with default category first.
    pub fn get_categories(&self) -> Result<Vec<Category>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, is_default FROM categories ORDER BY is_default DESC, name ASC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Category {
                id: row.get(0)?,
                name: row.get(1)?,
                is_default: row.get::<_, i64>(2)? == 1,
            })
        })?;
        rows.collect()
    }

    /// Creates a new category. Returns the new category's ID.
    /// Rejects names that conflict with the default category translations.
    pub fn add_category(&self, name: &str) -> Result<i64> {
        let lower = name.trim().to_lowercase();
        let reserved = ["non categorizzati", "uncategorized", "not categorized", "newsletter"];
        if reserved.contains(&lower.as_str()) {
            return Err(rusqlite::Error::InvalidParameterName(
                "cat_name_reserved".to_string()
            ));
        }
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO categories (name, is_default) VALUES (?1, 0)",
            params![name],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Renames an existing category. Fails if the category is the default one
    /// (handled at API layer) or if the new name already exists.
    pub fn rename_category(&self, id: i64, new_name: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE categories SET name = ?1 WHERE id = ?2 AND is_default = 0",
            params![new_name, id],
        )
    }

    /// Deletes a category only if it has no associated feeds and is not the default.
    /// Returns the number of deleted rows (0 if conditions not met).
    pub fn delete_category(&self, id: i64) -> Result<usize> {
        let conn = self.conn.lock().unwrap();

        // Check if any feeds are assigned to this category
        let feed_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM feeds WHERE category_id = ?1",
            params![id],
            |row| row.get(0),
        )?;

        if feed_count > 0 {
            return Ok(0); // Cannot delete: feeds still assigned
        }

        conn.execute(
            "DELETE FROM categories WHERE id = ?1 AND is_default = 0",
            params![id],
        )
    }

    // ─── Favorite tag operations ───────────────────────────────────────────

    /// Returns all favorite tags ordered by name, each with the number of
    /// articles it is assigned to.
    pub fn get_favorite_tags(&self) -> Result<Vec<FavoriteTag>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT t.id, t.name, COUNT(l.article_id)
             FROM favorite_tags t
             LEFT JOIN article_tags l ON l.tag_id = t.id
             GROUP BY t.id
             ORDER BY t.name COLLATE NOCASE, t.name"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(FavoriteTag {
                id: row.get(0)?,
                name: row.get(1)?,
                article_count: row.get(2)?,
            })
        })?;
        rows.collect()
    }

    /// Creates a new favorite tag. Returns the new tag's ID.
    /// Fails with a UNIQUE constraint violation if the name already exists.
    pub fn add_favorite_tag(&self, name: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute("INSERT INTO favorite_tags (name) VALUES (?1)", params![name])?;
        Ok(conn.last_insert_rowid())
    }

    /// Renames a favorite tag. Articles reference tags by ID, so they show the
    /// new name right away. Fails with a UNIQUE constraint violation if the
    /// name already exists; returns 0 if the tag does not exist.
    pub fn rename_favorite_tag(&self, id: i64, new_name: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE favorite_tags SET name = ?1 WHERE id = ?2",
            params![new_name, id],
        )
    }

    /// Deletes a favorite tag, even if in use: the cascade removes it from
    /// every article that has it. Returns 0 if the tag does not exist.
    pub fn delete_favorite_tag(&self, id: i64) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM favorite_tags WHERE id = ?1", params![id])
    }

    /// Assigns a tag to an article, enforcing that the article is a favorite
    /// and has fewer than `MAX_TAGS` tags. Assigning a tag the article already
    /// has succeeds without changes.
    pub fn assign_article_tag(&self, article_id: &str, tag_id: i64) -> Result<TagAssignment> {
        // The connection mutex serializes every database access of the
        // process, so nothing can change between the checks and the insert.
        let conn = self.conn.lock().unwrap();

        let is_favorite: Option<bool> = conn
            .query_row(
                "SELECT is_favorite FROM articles WHERE id = ?1",
                params![article_id],
                |row| Ok(row.get::<_, i64>(0)? == 1),
            )
            .optional()?;
        match is_favorite {
            None => return Ok(TagAssignment::ArticleNotFound),
            Some(false) => return Ok(TagAssignment::NotFavorite),
            Some(true) => {}
        }

        let tag_exists = conn
            .query_row("SELECT 1 FROM favorite_tags WHERE id = ?1", params![tag_id], |_| Ok(()))
            .optional()?
            .is_some();
        if !tag_exists {
            return Ok(TagAssignment::TagNotFound);
        }

        let tags = Self::article_tags(&conn, article_id)?;
        if tags.iter().any(|t| t.id == tag_id) {
            return Ok(TagAssignment::Assigned(tags));
        }
        if tags.len() >= MAX_TAGS {
            return Ok(TagAssignment::LimitReached);
        }

        conn.execute(
            "INSERT INTO article_tags (article_id, tag_id) VALUES (?1, ?2)",
            params![article_id, tag_id],
        )?;
        Ok(TagAssignment::Assigned(Self::article_tags(&conn, article_id)?))
    }

    /// Removes a tag from an article and returns the tags it still has.
    /// Removing a tag the article does not have changes nothing.
    pub fn unassign_article_tag(&self, article_id: &str, tag_id: i64) -> Result<Vec<ArticleTag>> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM article_tags WHERE article_id = ?1 AND tag_id = ?2",
            params![article_id, tag_id],
        )?;
        Self::article_tags(&conn, article_id)
    }

    /// Deletes all favorite tags, and with them every tag assignment.
    pub fn delete_all_favorite_tags(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let result = conn.execute("DELETE FROM favorite_tags", [])?;
        let _ = conn.execute("DELETE FROM sqlite_sequence WHERE name = 'favorite_tags'", []);
        Ok(result)
    }

    /// Exports the names of all favorite tags, including unused ones.
    pub fn export_favorite_tags(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT name FROM favorite_tags ORDER BY name COLLATE NOCASE, name"
        )?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        rows.collect()
    }

    /// Imports favorite tags by name, skipping names that already exist.
    pub fn import_favorite_tags(&self, names: &[String]) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        for name in names.iter().filter(|n| !n.trim().is_empty()) {
            tx.execute("INSERT OR IGNORE INTO favorite_tags (name) VALUES (?1)", params![name])?;
        }
        tx.commit()
    }

    /// Returns the tags of an article ordered by name (empty if the article
    /// does not exist). Takes the connection so callers already holding the
    /// lock can use it.
    fn article_tags(conn: &Connection, article_id: &str) -> Result<Vec<ArticleTag>> {
        let tags = conn
            .query_row(
                &format!("SELECT {ARTICLE_TAGS_JSON} FROM articles a WHERE a.id = ?1"),
                params![article_id],
                |row| json_column(row, 0),
            )
            .optional()?;
        Ok(tags.unwrap_or_default())
    }

    // ─── Feed operations ───────────────────────────────────────────────────

    /// Returns all feeds with their category info.
    pub fn get_feeds(&self) -> Result<Vec<Feed>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, url, title, site_name, category_id, force_ai_summary, enable_scraping, is_paused, 
            is_web_feed, web_feed_pattern, is_api_scraping, etag, last_modified FROM feeds ORDER BY site_name ASC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Feed {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                site_name: row.get(3)?,
                category_id: row.get(4)?,
                force_ai_summary: row.get::<_, i64>(5)? == 1,
                enable_scraping: row.get::<_, i64>(6)? == 1,
                is_paused: row.get::<_, i64>(7)? == 1,
                is_web_feed: row.get::<_, i64>(8)? == 1,
                web_feed_pattern: row.get(9)?,
                is_api_scraping: row.get::<_, i64>(10)? == 1,
                etag: row.get(11)?,
                last_modified: row.get(12)?,
            })
        })?;
        rows.collect()
    }

    /// Adds a new feed. Returns the new feed's ID.
    pub fn add_feed(&self, url: &str, title: &str, site_name: &str, category_id: i64) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO feeds (url, title, site_name, category_id) VALUES (?1, ?2, ?3, ?4)",
            params![url, title, site_name, category_id],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Removes a feed and all its associated articles (CASCADE).
    pub fn delete_feed(&self, id: i64) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM feeds WHERE id = ?1", params![id])
    }

    /// Deletes ALL feeds and their articles. Used for "reset to zero" functionality.
    pub fn delete_all_feeds(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let result = conn.execute("DELETE FROM feeds", [])?;
        let _ = conn.execute("DELETE FROM sqlite_sequence WHERE name = 'feeds'", []);
        Ok(result)
    }

    /// Deletes all categories except the default one.
    pub fn delete_all_categories(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let result = conn.execute("DELETE FROM categories WHERE is_default = 0", [])?;
        // Reset sequence to the highest remaining id (default categories stay)
        let max_id: i64 = conn.query_row(
            "SELECT COALESCE(MAX(id), 0) FROM categories",
            [],
            |row| row.get(0),
        ).unwrap_or(0);
        let _ = conn.execute(
            "UPDATE sqlite_sequence SET seq = ?1 WHERE name = 'categories'",
            params![max_id],
        );
        Ok(result)
    }

    /// Toggles the force_ai_summary flag for a specific feed.
    pub fn set_feed_force_ai(&self, feed_id: i64, force: bool) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE feeds SET force_ai_summary = ?1 WHERE id = ?2",
            params![force as i64, feed_id],
        )
    }

    /// Toggles the enable_scraping flag for a specific feed.
    /// If scraping is disabled, force_ai_summary is also disabled.
    pub fn set_feed_scraping(&self, feed_id: i64, enable: bool) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        if !enable {
            // Disable AI summary too when scraping is turned off
            conn.execute(
                "UPDATE feeds SET enable_scraping = 0, force_ai_summary = 0 WHERE id = ?1",
                params![feed_id],
            )
        } else {
            conn.execute(
                "UPDATE feeds SET enable_scraping = ?1 WHERE id = ?2",
                params![enable as i64, feed_id],
            )
        }
    }

    /// Sets the paused state of a feed.
    /// Paused feeds are skipped during manual and automatic updates.
    pub fn set_feed_paused(&self, feed_id: i64, paused: bool) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE feeds SET is_paused = ?1 WHERE id = ?2",
            params![paused as i64, feed_id],
        )
    }

    /// Enables or disables fetching this web feed through the rendering API.
    /// When disabled the page is fetched with a plain GET, which costs nothing
    /// but cannot execute the page's JavaScript.
    pub fn set_feed_api_scraping(&self, feed_id: i64, enable: bool) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE feeds SET is_api_scraping = ?1 WHERE id = ?2",
            params![enable as i64, feed_id],
        )
    }

    /// Stores the HTTP validators returned by a feed server on a `200`.
    /// Each value is written only when the server actually sent the matching
    /// header: `None` leaves the stored value untouched, so a server that sends
    /// only one of the two never wipes the other.
    pub fn set_feed_validators(
        &self,
        feed_id: i64,
        etag: Option<&str>,
        last_modified: Option<&str>,
    ) -> Result<usize> {
        // Neither header was sent: skip the write entirely
        if etag.is_none() && last_modified.is_none() {
            return Ok(0);
        }
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE feeds
             SET etag = COALESCE(?1, etag),
                 last_modified = COALESCE(?2, last_modified)
             WHERE id = ?3",
            params![etag, last_modified, feed_id],
        )
    }

    /// Replaces the feed URL after an unbroken chain of permanent redirects
    /// (`301` / `308`) and clears both validators, since the stored ones belong
    /// to the previous resource.
    ///
    /// `feeds.url` is `UNIQUE`, so this fails when the redirect target is
    /// already subscribed as a separate feed: the caller must handle the error
    /// and leave the original URL in place.
    pub fn relocate_feed_url(&self, feed_id: i64, new_url: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE feeds SET url = ?1, etag = '', last_modified = '' WHERE id = ?2",
            params![new_url, feed_id],
        )
    }

    /// When force_ai_summary is disabled, restore feed summaries for articles
    /// that have summary text but were marked as "none" (waiting for LLM).
    pub fn restore_feed_summaries(&self, feed_id: i64) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE articles SET summary_source = 'feed'
             WHERE feed_id = ?1 AND summary_source = 'none' AND summary != ''",
            params![feed_id],
        )
    }

    /// Updates the category of a specific feed.
    pub fn update_feed_category(&self, feed_id: i64, category_id: i64) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE feeds SET category_id = ?1 WHERE id = ?2",
            params![category_id, feed_id],
        )
    }

    /// Exports all feeds with their category names for backup/sharing.
    pub fn export_feeds(&self) -> Result<Vec<FeedExport>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT f.url, f.title, f.site_name, c.name, f.force_ai_summary, f.enable_scraping, f.is_paused, f.is_web_feed, f.web_feed_pattern, f.is_api_scraping
             FROM feeds f
             JOIN categories c ON f.category_id = c.id
             ORDER BY c.name, f.site_name"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(FeedExport {
                url: row.get(0)?,
                title: row.get(1)?,
                site_name: row.get(2)?,
                category_name: row.get(3)?,
                force_ai_summary: row.get::<_, i64>(4)? == 1,
                enable_scraping: row.get::<_, i64>(5)? == 1,
                is_paused: row.get::<_, i64>(6)? == 1,
                is_web_feed: row.get::<_, i64>(7)? == 1,
                web_feed_pattern: row.get(8)?,
                is_api_scraping: row.get::<_, i64>(9)? == 1,
            })
        })?;
        rows.collect()
    }

    /// Exports all categories except the default one.
    pub fn export_categories(&self) -> Result<Vec<CategoryExport>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT name FROM categories WHERE is_default = 0 ORDER BY name ASC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(CategoryExport {
                name: row.get(0)?,
            })
        })?;
        rows.collect()
    }

    /// Returns the name of the default, non-deletable category used for feeds
    /// with no explicit category, such as top-level OPML outlines.
    pub fn default_category_name(&self) -> Result<String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT name FROM categories WHERE is_default = 1 AND name != 'Newsletter' LIMIT 1",
            [],
            |row| row.get(0),
        )
    }

    /// Exports the application settings.
    pub fn export_settings(&self) -> SettingsExport {
        SettingsExport {
            retention_hours: self.get_setting("retention_hours").ok().flatten().unwrap_or("24".into()),
            auto_update_enabled: self.get_setting("auto_update_enabled").ok().flatten().unwrap_or("false".into()),
            auto_update_interval_hours: self.get_setting("auto_update_interval_hours").ok().flatten().unwrap_or("1".into()),
            highlight_keywords: self.get_setting("highlight_keywords").ok().flatten().unwrap_or_default(),
        }
    }

    /// Exports all favorite articles with their feed URL for re-association on import.
    pub fn export_favorites(&self) -> Result<Vec<FavoriteArticleExport>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
                &format!(
                "SELECT a.url, a.title, a.author, a.published_at, a.full_text,
                        a.summary, a.summary_source, f.url, a.newsletter_html, {ARTICLE_TAGS_JSON}
                 FROM articles a
                 JOIN feeds f ON a.feed_id = f.id
                 WHERE a.is_favorite = 1
                 ORDER BY a.published_at DESC"
            )
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(FavoriteArticleExport {
                url: row.get(0)?,
                title: row.get(1)?,
                author: row.get(2)?,
                published_at: row.get(3)?,
                full_text: row.get(4)?,
                summary: row.get(5)?,
                summary_source: row.get(6)?,
                feed_url: row.get(7)?,
                newsletter_html: row.get(8)?,
                tags: json_column::<Vec<ArticleTag>>(row, 9)?
                    .into_iter()
                    .map(|t| t.name)
                    .collect(),
            })
        })?;
        rows.collect()
    }

    /// Imports favorite articles, associating them with existing feeds by URL.
    /// Skips articles whose URL already exists in the DB.
    /// Immediately flags them as favorites to prevent deletion by retention purge.
    pub fn import_favorites(&self, favorites: &[FavoriteArticleExport]) -> Result<u32> {
        let conn = self.conn.lock().unwrap();
        let mut imported = 0u32;

        for fav in favorites {
            // Find the feed ID by URL
            let feed_id: Option<i64> = conn.query_row(
                "SELECT id FROM feeds WHERE url = ?1",
                params![fav.feed_url],
                |row| row.get(0),
            ).ok();

            // If the feed doesn't exist, handle special cases or skip
            let feed_id = match feed_id {
                Some(id) => id,
                None => {
                    // Auto-create newsletter feeds during import if needed
                    if fav.feed_url.starts_with("newsletter://") {
                        let newsletter_cat_id: i64 = conn.query_row(
                            "SELECT id FROM categories WHERE name = 'Newsletter' AND is_default = 1",
                            [],
                            |row| row.get(0),
                        ).unwrap_or(1);

                        // Extract feed name from URL (e.g. "newsletter://giulio_magini" -> "giulio_magini")
                        let feed_name = fav.feed_url.trim_start_matches("newsletter://");

                        conn.execute(
                            "INSERT OR IGNORE INTO feeds (url, title, site_name, category_id, enable_scraping, is_paused)
                             VALUES (?1, ?2, ?3, ?4, 1, 1)",
                            params![fav.feed_url, feed_name, feed_name, newsletter_cat_id],
                        )?;

                        conn.query_row(
                            "SELECT id FROM feeds WHERE url = ?1",
                            params![fav.feed_url],
                            |row| row.get(0),
                        ).unwrap_or(0)
                    } else {
                        continue;
                    }
                }
            };

            // Insert the article, skip if URL already exists
            let article_id = uuid::Uuid::new_v4().to_string();
            let result = conn.execute(
                "INSERT OR IGNORE INTO articles
                 (id, feed_id, url, title, author, published_at, full_text, summary, summary_source, is_favorite, fetched_at, newsletter_html)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, datetime('now'), ?10)",
                params![
                    article_id,
                    feed_id,
                    fav.url,
                    fav.title,
                    fav.author,
                    fav.published_at,
                    fav.full_text,
                    fav.summary,
                    fav.summary_source,
                    fav.newsletter_html,
                ],
            )?;

            if result > 0 {
                imported += 1;

                // Re-link the tags by name, creating any the export did not
                // list. Blank names, duplicates and anything past the
                // per-article limit (possible only in hand-edited files) are
                // dropped.
                let mut names: Vec<&str> = Vec::new();
                for name in fav.tags.iter().map(String::as_str) {
                    if !name.trim().is_empty() && !names.contains(&name) {
                        names.push(name);
                    }
                }
                names.truncate(MAX_TAGS);
                for name in names {
                    conn.execute("INSERT OR IGNORE INTO favorite_tags (name) VALUES (?1)", params![name])?;
                    conn.execute(
                        "INSERT OR IGNORE INTO article_tags (article_id, tag_id)
                         SELECT ?1, id FROM favorite_tags WHERE name = ?2",
                        params![article_id, name],
                    )?;
                }
            }
        }

        Ok(imported)
    }

    /// Imports feeds from an export file. Creates categories if they don't exist.
    /// Skips feeds whose URL already exists in the database.
    pub fn import_feeds(&self, feeds: &[FeedExport]) -> Result<u32> {
        let conn = self.conn.lock().unwrap();
        let mut imported = 0u32;

        for feed in feeds {
            // Ensure the category exists, get its ID
            conn.execute(
                "INSERT OR IGNORE INTO categories (name, is_default) VALUES (?1, 0)",
                params![feed.category_name],
            )?;
            let category_id: i64 = conn.query_row(
                "SELECT id FROM categories WHERE name = ?1",
                params![feed.category_name],
                |row| row.get(0),
            )?;

            // Insert feed, skip if URL already exists
            let result = conn.execute(
                "INSERT OR IGNORE INTO feeds (url, title, site_name, category_id, force_ai_summary, enable_scraping, is_paused, is_web_feed, web_feed_pattern, is_api_scraping)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    feed.url,
                    feed.title,
                    feed.site_name,
                    category_id,
                    feed.force_ai_summary as i64,
                    feed.enable_scraping as i64,
                    feed.is_paused as i64,
                    feed.is_web_feed as i64,
                    feed.web_feed_pattern,
                    feed.is_api_scraping as i64
                ],
            )?;
            if result > 0 {
                imported += 1;
            }
        }

        Ok(imported)
    }

    // ─── Article operations ────────────────────────────────────────────────

    /// Inserts a new article if its URL doesn't already exist (deduplication).
    /// Returns true if the article was inserted, false if it was a duplicate.
    pub fn insert_article(&self, article: &Article) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let result = conn.execute(
            "INSERT OR IGNORE INTO articles
             (id, feed_id, url, title, author, published_at, full_text, summary, summary_source, fetched_at, newsletter_html)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                article.id,
                article.feed_id,
                article.url,
                article.title,
                article.author,
                article.published_at,
                article.full_text,
                article.summary,
                article.summary_source,
                article.fetched_at,
                article.newsletter_html,
            ],
        )?;
        Ok(result > 0)
    }

    /// Inserts a batch of newsletter articles in a single transaction and
    /// returns how many rows were actually written.
    ///
    /// `INSERT OR IGNORE` lets the `url` uniqueness constraint do the
    /// deduplication: a message already imported earlier, or repeated twice
    /// inside the same batch, is silently skipped and is simply missing from
    /// the returned count.
    ///
    /// Unlike `insert_article` this writes `is_favorite` inline, since
    /// newsletters are always favorites: it saves one extra statement per row,
    /// which is what makes a folder of several hundred files cheap.
    pub fn insert_newsletter_articles(&self, articles: &[Article]) -> Result<u32> {
        if articles.is_empty() {
            return Ok(0);
        }

        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let mut inserted = 0u32;

        // The statement borrows the transaction, so it has to go out of scope
        // before the commit can consume it.
        {
            let mut stmt = tx.prepare(
                "INSERT OR IGNORE INTO articles
                 (id, feed_id, url, title, author, published_at, full_text, summary, summary_source, is_favorite, fetched_at, newsletter_html)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, ?10, ?11)",
            )?;

            for article in articles {
                inserted += stmt.execute(params![
                    article.id,
                    article.feed_id,
                    article.url,
                    article.title,
                    article.author,
                    article.published_at,
                    article.full_text,
                    article.summary,
                    article.summary_source,
                    article.fetched_at,
                    article.newsletter_html,
                ])? as u32;
            }
        }

        tx.commit()?;
        Ok(inserted)
    }

    /// Updates the summary for an article (after LLM processing or manual trigger).
    pub fn update_article_summary(&self, article_id: &str, summary: &str, source: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE articles SET summary = ?1, summary_source = ?2 WHERE id = ?3",
            params![summary, source, article_id],
        )
    }

    /// Updates the full text of an article (after scraping).
    pub fn update_article_full_text(&self, article_id: &str, full_text: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE articles SET full_text = ?1 WHERE id = ?2",
            params![full_text, article_id],
        )
    }

    /// Replaces the full text of an article with text written by the user.
    ///
    /// If the article has an AI summary and the text actually changes, the
    /// summary is cleared and its source set back to "none", so a new AI
    /// summary can be generated from the updated text. Feed summaries are left
    /// untouched, and so is everything when the text is saved unchanged.
    ///
    /// Returns the resulting `(summary, summary_source)`, or `None` if the
    /// article does not exist.
    pub fn save_manual_full_text(&self, article_id: &str, full_text: &str) -> Result<Option<(String, String)>> {
        let conn = self.conn.lock().unwrap();
        // SET expressions read the pre-update row, so the CASE conditions see
        // the old full_text and summary_source regardless of assignment order.
        let result = conn.query_row(
            "UPDATE articles SET
                 summary = CASE WHEN summary_source = 'llm' AND full_text <> ?1
                                THEN '' ELSE summary END,
                 summary_source = CASE WHEN summary_source = 'llm' AND full_text <> ?1
                                       THEN 'none' ELSE summary_source END,
                 full_text = ?1
             WHERE id = ?2
             RETURNING summary, summary_source",
            params![full_text, article_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        );
        match result {
            Ok(summary) => Ok(Some(summary)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Retrieves the full text of an article by its ID.
    pub fn get_article_full_text(&self, article_id: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT full_text FROM articles WHERE id = ?1",
            params![article_id],
            |row| row.get(0),
        );
        match result {
            Ok(text) => Ok(Some(text)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Retrieves the URL of an article by its ID.
    pub fn get_article_url(&self, article_id: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT url FROM articles WHERE id = ?1",
            params![article_id],
            |row| row.get(0),
        );
        match result {
            Ok(url) => Ok(Some(url)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Returns the most recent published_at date for articles of a given feed.
    /// Used to determine which articles are already in DB during incremental fetch.
    pub fn get_latest_article_date(&self, feed_id: i64) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT published_at FROM articles
             WHERE feed_id = ?1 AND published_at IS NOT NULL
             ORDER BY published_at DESC LIMIT 1",
            params![feed_id],
            |row| row.get(0),
        );
        match result {
            Ok(date) => Ok(Some(date)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Returns the oldest published_at date for articles of a given feed.
    /// Used to detect gaps when retention period has been expanded.
    pub fn get_oldest_article_date(&self, feed_id: i64) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT published_at FROM articles
             WHERE feed_id = ?1 AND published_at IS NOT NULL
             ORDER BY published_at ASC LIMIT 1",
            params![feed_id],
            |row| row.get(0),
        );
        match result {
            Ok(date) => Ok(Some(date)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Toggles the favorite status of an article. Removing it from favorites
    /// also drops its tags, since only favorites can have them.
    pub fn set_article_favorite(&self, article_id: &str, is_favorite: bool) -> Result<usize> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let updated = tx.execute(
            "UPDATE articles SET is_favorite = ?1 WHERE id = ?2",
            params![is_favorite as i64, article_id],
        )?;
        if !is_favorite {
            tx.execute("DELETE FROM article_tags WHERE article_id = ?1", params![article_id])?;
        }
        tx.commit()?;
        Ok(updated)
    }

    /// Marks an article as read.
    pub fn set_article_read(&self, article_id: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE articles SET is_read = 1 WHERE id = ?1",
            params![article_id],
        )
    }

    /// Retrieves the newsletter HTML content for an article.
    pub fn get_newsletter_html(&self, article_id: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT newsletter_html FROM articles WHERE id = ?1",
            params![article_id],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(html) if !html.is_empty() => Ok(Some(html)),
            Ok(_) => Ok(None),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Returns all newsletter feeds (URL starts with "newsletter://").
    /// Used by the import-newsletter page to list existing newsletter feeds.
    pub fn get_newsletter_feeds(&self) -> Result<Vec<Feed>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, url, title, site_name, category_id, force_ai_summary, enable_scraping, is_paused, is_web_feed, web_feed_pattern, is_api_scraping, etag, last_modified
             FROM feeds WHERE url LIKE 'newsletter://%' ORDER BY site_name ASC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Feed {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                site_name: row.get(3)?,
                category_id: row.get(4)?,
                force_ai_summary: row.get::<_, i64>(5)? == 1,
                enable_scraping: row.get::<_, i64>(6)? == 1,
                is_paused: row.get::<_, i64>(7)? == 1,
                is_web_feed: row.get::<_, i64>(8)? == 1,
                web_feed_pattern: row.get(9)?,
                is_api_scraping: row.get::<_, i64>(10)? == 1,
                etag: row.get(11)?,
                last_modified: row.get(12)?,
            })
        })?;
        rows.collect()
    }

    /// Creates a new newsletter feed with the given name.
    /// The URL is generated as "newsletter://<sanitized_name>" and the feed is
    /// always assigned to the "Newsletter" default category, paused, with scraping enabled.
    /// Returns the new feed's ID.
    pub fn create_newsletter_feed(&self, name: &str) -> Result<i64> {
        let conn = self.conn.lock().unwrap();

        // Get the "Newsletter" default category ID
        let newsletter_cat_id: i64 = conn.query_row(
            "SELECT id FROM categories WHERE name = 'Newsletter' AND is_default = 1",
            [],
            |row| row.get(0),
        )?;

        // Generate URL from name: lowercase, spaces to underscores, keep only alphanumeric and underscores
        let sanitized: String = name.trim().to_lowercase()
            .chars()
            .map(|c| if c == ' ' { '_' } else { c })
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect();

        let url = format!("newsletter://{}", sanitized);

        conn.execute(
            "INSERT INTO feeds (url, title, site_name, category_id, enable_scraping, is_paused)
             VALUES (?1, ?2, ?3, ?4, 1, 1)",
            params![url, name.trim(), name.trim(), newsletter_cat_id],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Fetches articles with filters, sorting, and pagination.
    /// Joins with feeds and categories for display.
    pub fn get_articles(&self, query: &ArticleQuery) -> Result<ArticleListResponse> {
        let conn = self.conn.lock().unwrap();

        let (sort_clause, _sort_dir) = match query.sort_order.as_deref() {
            Some("asc") => ("a.published_at ASC", "ASC"),
            Some("feed_desc") => ("f.site_name ASC, a.published_at DESC", "DESC"),
            Some("feed_asc") => ("f.site_name ASC, a.published_at ASC", "ASC"),
            _ => ("a.published_at DESC", "DESC"),
        };

        let per_page = query.per_page.unwrap_or(10).clamp(10, 100);
        let page = query.page.unwrap_or(1).max(1);
        let offset = (page - 1) * per_page;

        // Build WHERE clause dynamically with parameterized queries
        let mut conditions: Vec<String> = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
        let mut param_index = 1;

        if let Some(cat_id) = query.category_id {
            conditions.push(format!("f.category_id = ?{}", param_index));
            param_values.push(Box::new(cat_id));
            param_index += 1;
        }

        match query.favorites_filter.as_deref() {
            Some("only") => {
                conditions.push("a.is_favorite = 1".to_string());

                // Tags exist only on favorites, so the tag filter belongs to
                // this view alone: a selection left over from it never
                // narrows the other views.
                if !query.tag_ids.is_empty() {
                    let mut placeholders = Vec::with_capacity(query.tag_ids.len());
                    for tag_id in &query.tag_ids {
                        placeholders.push(format!("?{}", param_index));
                        param_values.push(Box::new(*tag_id));
                        param_index += 1;
                    }
                    conditions.push(format!(
                        "EXISTS (SELECT 1 FROM article_tags l WHERE l.article_id = a.id AND l.tag_id IN ({}))",
                        placeholders.join(", ")
                    ));
                }
            }
            Some("exclude") => {
                conditions.push("a.is_favorite = 0".to_string());
            }
            _ => {} // "all" or None — no filter
        }

        if let Some(ref search) = query.search {
            let search = search.trim();
            if !search.is_empty() {
                conditions.push(format!("a.title LIKE ?{}", param_index));
                param_values.push(Box::new(format!("%{}%", search)));
                param_index += 1;
            }
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // Get total count for pagination
        let count_sql = format!(
            "SELECT COUNT(*) FROM articles a
             JOIN feeds f ON a.feed_id = f.id
             {}", where_clause
        );
        let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values.iter()
            .map(|p| p.as_ref())
            .collect();
        let total_count: u64 = conn.query_row(
            &count_sql,
            params_refs.as_slice(),
            |row| row.get::<_, i64>(0).map(|v| v as u64),
        )?;

        // Fetch the page of articles
        let select_sql = format!(
            "SELECT a.id, a.feed_id, a.url, a.title, a.author, a.published_at,
                    a.full_text, a.summary, a.summary_source, a.is_favorite, a.fetched_at,
                    f.site_name, f.title, f.category_id, c.name, f.enable_scraping, f.force_ai_summary, a.is_read, a.newsletter_html,
             {}
             FROM articles a
             JOIN feeds f ON a.feed_id = f.id
             JOIN categories c ON f.category_id = c.id
             {}
             ORDER BY {}
             LIMIT ?{} OFFSET ?{}",
             ARTICLE_TAGS_JSON, where_clause, sort_clause, param_index, param_index + 1
        );

        let mut all_params = param_values;
        all_params.push(Box::new(per_page));
        all_params.push(Box::new(offset));

        let params_refs: Vec<&dyn rusqlite::types::ToSql> = all_params.iter()
            .map(|p| p.as_ref())
            .collect();

        let mut stmt = conn.prepare(&select_sql)?;
        let articles = stmt.query_map(params_refs.as_slice(), |row| {
            Ok(ArticleWithFeed {
                id: row.get(0)?,
                feed_id: row.get(1)?,
                url: row.get(2)?,
                title: row.get(3)?,
                author: row.get(4)?,
                published_at: row.get(5)?,
                full_text: row.get(6)?,
                summary: row.get(7)?,
                summary_source: row.get(8)?,
                is_favorite: row.get::<_, i64>(9)? == 1,
                fetched_at: row.get(10)?,
                site_name: row.get(11)?,
                feed_title: row.get(12)?,
                category_id: row.get(13)?,
                category_name: row.get(14)?,
                enable_scraping: row.get::<_, i64>(15)? == 1,
                force_ai_summary: row.get::<_, i64>(16)? == 1,
                is_read: row.get::<_, i64>(17)? == 1,
                newsletter_html: row.get(18)?,
                tags: json_column(row, 19)?,
            })
        })?.collect::<Result<Vec<_>>>()?;

        Ok(ArticleListResponse {
            articles,
            total_count,
            page,
            per_page,
        })
    }

    /// Removes expired articles (beyond retention period) that are NOT favorites.
    /// Uses published_at as the reference date (when the article was actually published).
    /// Falls back to fetched_at for articles without a publication date.
    pub fn purge_expired_articles(&self, retention_hours: i64) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM articles
             WHERE is_favorite = 0
             AND datetime(COALESCE(published_at, fetched_at), '+' || ?1 || ' hours') < datetime('now')",
            params![retention_hours],
        )
    }

    // ─── Settings operations ───────────────────────────────────────────────

    /// Gets a setting value by key.
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        );
        match result {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Sets a setting value (insert or update).
    pub fn set_setting(&self, key: &str, value: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
    }

    // ─── API config operations ─────────────────────────────────────────────

    /// Stores the encrypted API key and provider.
    pub fn save_api_config(&self, provider: &str, api_key_enc: &str, iv: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO api_config (id, provider, api_key_enc, encryption_iv, updated_at)
             VALUES (1, ?1, ?2, ?3, datetime('now'))
             ON CONFLICT(id) DO UPDATE SET
                provider = excluded.provider,
                api_key_enc = excluded.api_key_enc,
                encryption_iv = excluded.encryption_iv,
                updated_at = excluded.updated_at",
            params![provider, api_key_enc, iv],
        )
    }

    /// Retrieves the stored API config (provider, encrypted key, IV).
    /// Returns None if no config has been saved yet.
    pub fn get_api_config(&self) -> Result<Option<(String, String, String)>> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT provider, api_key_enc, encryption_iv FROM api_config WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        );
        match result {
            Ok(config) => Ok(Some(config)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Deletes the stored API configuration (provider and encrypted key).
    pub fn delete_api_config(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM api_config WHERE id = 1", [])
    }

    // ─── Web rendering config operations ──────────────────────────────

    /// Stores the encrypted web rendering API key and provider.
    pub fn save_web_rendering_config(&self, provider: &str, api_key_enc: &str, iv: &str) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO web_rendering_config (id, provider, api_key_enc, encryption_iv, updated_at)
             VALUES (1, ?1, ?2, ?3, datetime('now'))
             ON CONFLICT(id) DO UPDATE SET
                provider = excluded.provider,
                api_key_enc = excluded.api_key_enc,
                encryption_iv = excluded.encryption_iv,
                updated_at = excluded.updated_at",
            params![provider, api_key_enc, iv],
        )
    }

    /// Retrieves the stored web rendering config (provider, encrypted key, IV).
    /// Returns None if no config has been saved yet.
    pub fn get_web_rendering_config(&self) -> Result<Option<(String, String, String)>> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT provider, api_key_enc, encryption_iv FROM web_rendering_config WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        );
        match result {
            Ok(config) => Ok(Some(config)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Deletes the stored web rendering API configuration.
    pub fn delete_web_rendering_config(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM web_rendering_config WHERE id = 1", [])
    }

    // ─── Web feed operations ──────────────────────────────────────────

    /// Adds a new web feed. The feed is always created with enable_scraping=1
    /// and is_web_feed=1. Returns the new feed's ID.
    pub fn add_web_feed(
        &self,
        url: &str,
        site_name: &str,
        category_id: i64,
        web_feed_pattern: &str,
        is_api_scraping: bool,
    ) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO feeds (url, title, site_name, category_id, enable_scraping, is_web_feed, web_feed_pattern, is_api_scraping)
             VALUES (?1, ?2, ?3, ?4, 1, 1, ?5, ?6)",
            params![url, site_name, site_name, category_id, web_feed_pattern, is_api_scraping as i64],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Returns all web feeds (is_web_feed = 1) that are not paused.
    pub fn get_active_web_feeds(&self) -> Result<Vec<Feed>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, url, title, site_name, category_id, force_ai_summary, enable_scraping, is_paused, is_web_feed, web_feed_pattern, is_api_scraping, etag, last_modified
             FROM feeds WHERE is_web_feed = 1 AND is_paused = 0 ORDER BY site_name ASC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Feed {
                id: row.get(0)?,
                url: row.get(1)?,
                title: row.get(2)?,
                site_name: row.get(3)?,
                category_id: row.get(4)?,
                force_ai_summary: row.get::<_, i64>(5)? == 1,
                enable_scraping: row.get::<_, i64>(6)? == 1,
                is_paused: row.get::<_, i64>(7)? == 1,
                is_web_feed: row.get::<_, i64>(8)? == 1,
                web_feed_pattern: row.get(9)?,
                is_api_scraping: row.get::<_, i64>(10)? == 1,
                etag: row.get(11)?,
                last_modified: row.get(12)?,
            })
        })?;
        rows.collect()
    }
}

/// Decodes a result column holding JSON built by SQLite (e.g. with
/// `json_group_array`) into `T`.
fn json_column<T: serde::de::DeserializeOwned>(row: &rusqlite::Row, idx: usize) -> Result<T> {
    let raw: String = row.get(idx)?;
    serde_json::from_str(&raw).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(idx, rusqlite::types::Type::Text, Box::new(e))
    })
}