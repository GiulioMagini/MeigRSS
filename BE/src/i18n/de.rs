//! German UI translation dictionary.
//! Mirrors the key set of the Italian and English dictionaries in `mod.rs`.

use std::collections::HashMap;

/// Returns the full German translation map.
/// Keys are the translation identifiers used by the frontend.
pub(super) fn translations() -> HashMap<&'static str, &'static str> {
    let mut t = HashMap::new();

    // App title
    t.insert("app_title", "MeigRSS - Dein Nachrichten-Aggregator");

    // Menu bar
    t.insert("menu_feeds", "Feeds");
    t.insert("menu_categories", "Kategorien");
    t.insert("menu_settings", "Einstellungen");
    t.insert("menu_api_config", "API-Konfiguration");
    t.insert("menu_refresh", "Feeds aktualisieren");
    t.insert("menu_import", "Feeds importieren");
    t.insert("menu_export", "Feeds exportieren");
    t.insert("menu_delete_all", "Alle Feeds löschen");

    // Feed list
    t.insert("summary_from_feed", "Zusammenfassung der Website");
    t.insert("summary_from_ai", "Von KI erstellte Zusammenfassung");
    t.insert("ai_summary_disclaimer", "Die KI-Zusammenfassung kann ungenaue Informationen enthalten. Überprüfe immer den Originalartikel, um den Inhalt sicherzustellen.");
    t.insert("no_summary", "Für diesen Artikel kann keine Zusammenfassung erstellt werden");
    t.insert("no_full_text", "Der vollständige Text des Artikels ist nicht verfügbar.");
    t.insert("scrape_no_content", "Kein lesbarer Inhalt auf der Seite gefunden");
    t.insert("scrape_failed", "Fehler beim Extrahieren des Textes: ");
    t.insert("btn_ai_summary", "Zusammenfassung mit KI erstellen");
    t.insert("btn_ai_summary_done", "KI-Zusammenfassung bereits erstellt");
    t.insert("btn_ai_limit_reached", "Nutzungslimit überschritten");
    t.insert("btn_api_not_configured", "API nicht korrekt konfiguriert");
    t.insert("btn_show_full_text", "Vollständigen Artikel anzeigen");
    t.insert("btn_hide_full_text", "Vollständigen Artikel ausblenden");
    t.insert("btn_edit_full_text", "Volltext bearbeiten");
    t.insert("btn_save_full_text", "Volltext speichern");
    t.insert("full_text_too_long", "Der eingegebene Text überschreitet die maximal zulässige Größe.");
    t.insert("link_original", "Originalartikel lesen");

    // Favorites
    t.insert("tooltip_add_favorite", "Zu Favoriten hinzufügen");
    t.insert("tooltip_remove_favorite", "Aus Favoriten entfernen");
    t.insert("confirm_remove_expired_favorite",
        "Dieser Artikel hat die Aufbewahrungsdauer überschritten. Wenn du ihn aus den Favoriten entfernst, wird er endgültig gelöscht. Fortfahren?");

    // Filters and sorting
    t.insert("sort_newest", "Neueste");
    t.insert("sort_oldest", "Älteste");
    t.insert("sort_feed_newest", "Feed (neueste)");
    t.insert("sort_feed_oldest", "Feed (älteste)");
    t.insert("filter_all", "Alle Kategorien");
    t.insert("filter_favorites_exclude", "Favoriten ausgeschlossen");
    t.insert("filter_favorites_all", "Alle Artikel");
    t.insert("filter_favorites_only", "Nur Favoriten");
    t.insert("search_placeholder", "Im Titel suchen...");
    t.insert("search_no_results", "Kein Artikel entspricht der Suche.");
    t.insert("pagination_per_page", "pro Seite");

    // Categories
    t.insert("cat_uncategorized", "Ohne Kategorie");
    t.insert("cat_add", "Kategorie hinzufügen");
    t.insert("cat_rename", "Umbenennen");
    t.insert("cat_delete", "Löschen");
    t.insert("cat_delete_error_has_feeds", "Löschen nicht möglich: Der Kategorie sind noch Feeds zugeordnet");
    t.insert("cat_delete_error_default", "Die Standardkategorie kann nicht gelöscht werden");
    t.insert("cat_name_reserved", "Dieser Name ist für die Standardkategorie reserviert");

    // Feed management
    t.insert("feed_add", "Feed hinzufügen");
    t.insert("feed_test", "Feed testen");
    t.insert("feed_testing", "Test läuft...");
    t.insert("feed_test_ok", "Feed gültig");
    t.insert("feed_test_fail", "Feed ungültig");
    t.insert("feed_already_exists", "Dieser Feed ist bereits in der Liste vorhanden");
    t.insert("feed_force_ai", "KI-Zusammenfassung immer erzwingen");
    t.insert("feed_active", "Aktiv");
    t.insert("feed_paused", "Pausiert");
    t.insert("feeds_auto_paused_title", "Blockierte Feeds");
    t.insert("feeds_auto_paused_message", "Die folgenden Feeds wurden automatisch pausiert:");
    t.insert("feeds_auto_paused_hint", "Um sie zu entsperren, gehe zur Seite der Feed-Verwaltung.");
    t.insert("pause_reason_timeout", "(Zeitüberschreitung)");
    t.insert("pause_reason_gone", "(Feed gelöscht)");
    t.insert("pause_reason_too_many", "(zu viele Anfragen)");
    t.insert("pause_reason_too_many_retry", "(zu viele Anfragen, versuche die Pause in {n} aufzuheben)");
    t.insert("feed_enable_scraping", "Herunterladen des vollständigen Textes aktivieren");
    t.insert("feed_api_scraping", "Scraping-API aktivieren");
    t.insert("scraping_disclaimer", "Das Herunterladen des vollständigen Artikeltextes liegt im Ermessen des Nutzers. Stelle vor der Aktivierung sicher, dass du die Nutzungsbedingungen der betreffenden Website einhältst. Der Autor der Software haftet nicht für den missbräuchlichen Einsatz dieser Funktion.");
    t.insert("feed_delete", "Feed entfernen");
    t.insert("confirm_delete_feed", "Beim Löschen dieses Feeds werden auch alle zugehörigen Artikel gelöscht. Fortfahren?");
    t.insert("feed_change_category", "Kategorie ändern");
    t.insert("import_select_category", "Kategorie für die importierten Feeds:");

    // API configuration
    t.insert("api_provider", "Anbieter");
    t.insert("api_key_field", "API-Schlüssel");
    t.insert("api_save_test", "Speichern und testen");
    t.insert("api_testing", "Test läuft...");
    t.insert("api_test_ok", "API funktioniert");
    t.insert("api_test_fail", "Fehler beim Test: ");
    t.insert("api_plan_info", "Informationen zum Tarif");
    t.insert("api_plan_loading", "Informationen werden abgerufen...");
    t.insert("api_plan_disclaimer", "Diese Informationen werden vom KI-Modell generiert und können ungenau sein. Überprüfe sie immer auf der offiziellen Website des KI-Anbieters, den du verwendest.");
    t.insert("api_delete", "API-Konfiguration löschen");
    t.insert("api_delete_confirm", "Bist du sicher? Der API-Schlüssel wird endgültig gelöscht und du musst ihn erneut eingeben, um die KI-Zusammenfassungen zu nutzen.");

    // Settings
    t.insert("settings_retention", "Aufbewahrungsdauer");
    t.insert("settings_retention_hours", "Stunden");
    t.insert("settings_retention_days", "Tage");
    t.insert("settings_and", "und");
    t.insert("settings_retention_hour", "Stunde");
    t.insert("settings_retention_day", "Tag");
    t.insert("settings_auto_update", "Automatische Aktualisierung");
    t.insert("settings_auto_update_interval", "Intervall (Stunden)");
    t.insert("settings_auto_update_disabled", "Deaktiviert");
    t.insert("settings_keywords", "Hervorgehobene Schlüsselwörter");
    t.insert("settings_keywords_desc", "Gib die hervorzuhebenden Wörter oder Ausdrücke ein, getrennt durch Semikolon (;)");
    t.insert("settings_keywords_placeholder", "Rust;KI;Cybersicherheit;...");

    // Confirm dialogs
    t.insert("confirm_delete_all_feeds",
        "Bist du sicher? Diese Aktion löscht alle Feeds einschließlich der zugehörigen Artikel, ob favorisiert oder nicht, und ist nicht umkehrbar.");
    t.insert("delete_categories_too", "Auch alle Kategorien löschen");
    t.insert("confirm_yes", "Bestätigen");
    t.insert("confirm_no", "Abbrechen");

    // Import/Export
    t.insert("import_success", "Feeds erfolgreich importiert");
    t.insert("import_count_feeds", "importierte Feeds");
    t.insert("import_count_favorites", "importierte Favoriten-Artikel");
    t.insert("export_success", "Feeds erfolgreich exportiert");
    t.insert("empty_articles", "Keine Artikel gefunden. Versuche, die Feeds zu aktualisieren.");
    t.insert("no_internet_title", "Keine Verbindung");
    t.insert("no_internet_message", "Feeds können nicht aktualisiert werden: Überprüfe deine Internetverbindung und versuche es erneut.");
    t.insert("empty_feeds", "Keine Feeds konfiguriert.");
    t.insert("import_description", "Wähle eine OPML-Datei oder eine aus MeigRSS exportierte JSON-Datei aus.");
    t.insert("back_home", "Zurück zur Startseite");
    t.insert("all_feeds_deleted", "Alle Feeds wurden gelöscht.");
    t.insert("import_confirm", "{count} Feeds in der Datei gefunden. Einige davon könnten Duplikate sein und werden daher nicht importiert. Fortfahren?");
    t.insert("import_favorites_count", "Einschließlich {count} Favoriten-Artikel.");
    t.insert("import_keep_original", "Ursprüngliche Kategorie beibehalten");
    t.insert("import_invalid_file", "Die ausgewählte Datei ist ungültig oder enthält keine Feeds.");
    t.insert("export_format_label", "Wähle das Ausgabeformat");
    t.insert("export_button", "Exportieren");

    // Newsletter
    t.insert("menu_import_newsletter", "Newsletter importieren");
    t.insert("newsletter_description", "Wähle eine aus deinem Postfach exportierte EML-Datei aus, um einen Newsletter als Artikel zu importieren.");
    t.insert("newsletter_title", "Titel");
    t.insert("newsletter_imported", "Newsletter als Favoriten-Artikel importiert.");
    t.insert("newsletter_no_html", "Die Datei enthält keinen HTML-Inhalt.");
    t.insert("newsletter_duplicate", "Dieser Newsletter wurde bereits importiert.");
    t.insert("newsletter_invalid_file", "Die ausgewählte Datei ist keine gültige EML-Datei.");
    t.insert("link_newsletter", "Newsletter anzeigen");
    t.insert("newsletter_choose_feed", "Wähle einen Newsletter-Feed");
    t.insert("newsletter_new_feed", "Neuen Feed erstellen");
    t.insert("newsletter_existing_feed", "Vorhandener Feed");
    t.insert("newsletter_feed_name", "Name des Feeds");
    t.insert("newsletter_feed_name_placeholder", "z. B. Giulio Magini");
    t.insert("newsletter_feed_name_empty", "Der Name des Feeds darf nicht leer sein");
    t.insert("newsletter_feed_exists", "Ein Newsletter-Feed mit diesem Namen existiert bereits");
    t.insert("newsletter_no_feeds", "Kein vorhandener Newsletter-Feed. Erstelle einen neuen.");
    t.insert("newsletter_mode_single", "Einzelne Datei");
    t.insert("newsletter_mode_folder", "Ordner");
    t.insert("newsletter_folder_description", "Wähle einen Ordner aus: Alle darin enthaltenen EML-Dateien werden importiert, alle anderen Dateitypen werden ignoriert.");
    t.insert("newsletter_folder_selected", "{count} EML-Dateien bereit zum Import.");
    t.insert("newsletter_folder_no_eml", "Der ausgewählte Ordner enthält keine EML-Dateien.");
    t.insert("newsletter_folder_too_many", "Der Ordner enthält mehr als {max} EML-Dateien. Wähle einen Ordner mit weniger Dateien.");
    t.insert("newsletter_folder_too_large", "Die ausgewählten Dateien überschreiten die maximal zulässige Größe für einen einzelnen Upload.");
    t.insert("newsletter_bulk_imported", "{count} Newsletter als Favoriten-Artikel importiert.");
    t.insert("newsletter_bulk_none", "Kein Newsletter importiert.");
    t.insert("newsletter_bulk_duplicates", "{count} bereits im Archiv vorhanden");
    t.insert("newsletter_bulk_no_html", "{count} ohne HTML-Inhalt");
    t.insert("newsletter_bulk_invalid", "{count} nicht als gültige E-Mails erkannt");

    // Web feed
    t.insert("web_feed_toggle_rss", "RSS-Feed");
    t.insert("web_feed_toggle_web", "Webseiten-Scan");
    t.insert("web_feed_test", "Scan testen");
    t.insert("web_feed_testing", "Scan läuft...");
    t.insert("web_feed_test_ok", "Scan erfolgreich");
    t.insert("web_feed_no_articles", "Keine Artikel auf der Seite gefunden. Überprüfe, ob die URL korrekt ist und die Seite Links im Datumsformat enthält.");
    t.insert("web_feed_articles_found", "Artikel gefunden");
    t.insert("web_feed_add", "Web-Feed hinzufügen");
    t.insert("web_rendering_test_failed", "Test der Rendering-API fehlgeschlagen: ");
    t.insert("web_rendering_section_title", "Web-Rendering-API");
    t.insert("web_rendering_provider", "Anbieter");
    t.insert("web_rendering_save_test", "Speichern und testen");
    t.insert("web_rendering_testing", "Test läuft...");
    t.insert("web_rendering_test_ok", "API funktioniert");
    t.insert("web_rendering_test_fail", "Fehler beim Test: ");
    t.insert("web_rendering_delete", "Rendering-Konfiguration löschen");
    t.insert("web_rendering_delete_confirm", "Bist du sicher? Der API-Schlüssel wird gelöscht und die Web-Feeds können nicht mehr aktualisiert werden.");
    t.insert("tooltip_rss_feed", "RSS-Feed");
    t.insert("tooltip_web_feed", "Web-Feed (Seiten-Scan)");
    t.insert("tooltip_newsletter_feed", "Newsletter-Feed");

    // General
    t.insert("loading", "Wird geladen...");
    t.insert("error", "Fehler");
    t.insert("save", "Speichern");
    t.insert("cancel", "Abbrechen");
    t.insert("close", "Schließen");

    t
}
