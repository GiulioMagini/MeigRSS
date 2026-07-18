use std::collections::HashMap;
// Per-language translation dictionaries, one module per added language
// In this case the modules are external since they are secondary languages, with the possibile errors written in english
mod fr;
mod de;
mod es;
mod pt;

/// Supported UI languages.
#[derive(Debug, Clone, PartialEq)]
pub enum Language {
    Italian,
    English,
    French,
    German,
    Spanish,
    Portuguese,
}

impl Language {
    /// Parses a browser language string (e.g. "it-IT", "en-US") into a Language.
    pub fn from_browser_lang(lang: &str) -> Self {
        // Prefix-match the browser language tag against supported languages.
        // Any unrecognized language falls back to English.
        if lang.starts_with("it") {
            Self::Italian
        } else if lang.starts_with("fr") {
            Self::French
        } else if lang.starts_with("de") {
            Self::German
        } else if lang.starts_with("es") {
            Self::Spanish
        } else if lang.starts_with("pt") {
            Self::Portuguese
        } else {
            Self::English
        }
    }
}

/// Returns the full translation map for the given language.
/// Keys are translation identifiers used in the frontend.
pub fn get_translations(lang: &Language) -> HashMap<&'static str, &'static str> {
    match lang {
        Language::Italian => italian_translations(),
        Language::English => english_translations(),
        Language::French => fr::translations(),
        Language::German => de::translations(),
        Language::Spanish => es::translations(),
        Language::Portuguese => pt::translations(),
    }
}

fn italian_translations() -> HashMap<&'static str, &'static str> {
    let mut t = HashMap::new();

    // App title
    t.insert("app_title", "MeigRSS - Il tuo aggregatore di notizie");

    // Menu bar
    t.insert("menu_feeds", "Feed");
    t.insert("menu_categories", "Categorie");
    t.insert("menu_settings", "Impostazioni");
    t.insert("menu_api_config", "Configurazione API");
    t.insert("menu_refresh", "Aggiorna feed");
    t.insert("menu_import", "Importa feed");
    t.insert("menu_export", "Esporta feed");
    t.insert("menu_delete_all", "Elimina tutti i feed");

    // Feed list
    t.insert("summary_from_feed", "Riassunto dal sito");
    t.insert("summary_from_ai", "Riassunto generato da AI");
    t.insert("ai_summary_disclaimer", "Il riassunto AI può contenere informazioni imprecise. Verifica sempre l'articolo originale per essere sicuro del contenuto.");
    t.insert("no_summary", "Impossibile creare un riassunto per questo articolo");
    t.insert("no_full_text", "Il testo completo dell'articolo non è disponibile.");
    t.insert("scrape_no_content", "Nessun contenuto leggibile trovato nella pagina");
    t.insert("scrape_failed", "Errore durante l'estrazione del testo: ");
    t.insert("btn_ai_summary", "Crea riassunto con AI");
    t.insert("btn_ai_summary_done", "Riassunto AI già generato");
    t.insert("btn_ai_limit_reached", "Limiti di utilizzo superati");
    t.insert("btn_api_not_configured", "API non configurata correttamente");
    t.insert("btn_show_full_text", "Mostra articolo completo");
    t.insert("btn_hide_full_text", "Nascondi articolo completo");
    t.insert("link_original", "Leggi l'articolo originale");

    // Favorites
    t.insert("tooltip_add_favorite", "Aggiungi ai preferiti");
    t.insert("tooltip_remove_favorite", "Rimuovi dai preferiti");
    t.insert("confirm_remove_expired_favorite",
        "Questo articolo è oltre il tempo di mantenimento. Rimuovendolo dai preferiti verrà eliminato definitivamente. Continuare?");

    // Filters and sorting
    t.insert("sort_newest", "Più recenti");
    t.insert("sort_oldest", "Meno recenti");
    t.insert("sort_feed_newest", "Feed (più recenti)");
    t.insert("sort_feed_oldest", "Feed (meno recenti)");
    t.insert("filter_all", "Tutte le categorie");
    t.insert("filter_favorites_exclude", "Preferiti esclusi");
    t.insert("filter_favorites_all", "Tutti gli articoli");
    t.insert("filter_favorites_only", "Solo preferiti");
    t.insert("search_placeholder", "Cerca nel titolo...");
    t.insert("search_no_results", "Nessun articolo corrisponde alla ricerca.");
    t.insert("pagination_per_page", "per pagina");

    // Categories
    t.insert("cat_uncategorized", "Non categorizzati");
    t.insert("cat_add", "Aggiungi categoria");
    t.insert("cat_rename", "Rinomina");
    t.insert("cat_delete", "Elimina");
    t.insert("cat_delete_error_has_feeds", "Impossibile eliminare: la categoria ha ancora dei feed associati");
    t.insert("cat_delete_error_default", "La categoria predefinita non può essere eliminata");
    t.insert("cat_name_reserved", "Questo nome è riservato alla categoria predefinita");

    // Feed management
    t.insert("feed_add", "Aggiungi feed");
    t.insert("feed_test", "Testa feed");
    t.insert("feed_testing", "Test in corso...");
    t.insert("feed_test_ok", "Feed valido");
    t.insert("feed_test_fail", "Feed non valido");
    t.insert("feed_already_exists", "Questo feed è già presente nella lista");
    t.insert("feed_force_ai", "Forza sempre riassunto AI");
    t.insert("feed_active", "Attivo");
    t.insert("feed_paused", "In pausa");
    t.insert("feeds_auto_paused_title", "Feed bloccati");
    t.insert("feeds_auto_paused_message", "I seguenti feed non hanno risposto e sono stati messi in pausa automaticamente:");
    t.insert("feeds_auto_paused_hint", "Per sbloccarli, vai nella pagina di gestione dei feed.");
    t.insert("feed_enable_scraping", "Abilita scaricamento testo completo");
    t.insert("scraping_disabled", "Scaricamento testo completo disabilitato per questo feed");
    t.insert("scraping_disclaimer", "Lo scaricamento del testo completo degli articoli è a discrezione dell'utente. Prima di attivarlo, assicurati di rispettare i termini di utilizzo del sito in questione. L'autore del software non è responsabile dell'uso improprio di questa funzionalità.");
    t.insert("feed_delete", "Rimuovi feed");
    t.insert("confirm_delete_feed", "Eliminando questo feed verranno eliminati anche tutti gli articoli associati. Continuare?");
    t.insert("feed_change_category", "Cambia categoria");
    t.insert("import_select_category", "Categoria per i feed importati:");

    // API configuration
    t.insert("api_provider", "Provider");
    t.insert("api_key_field", "API Key");
    t.insert("api_save_test", "Salva e testa");
    t.insert("api_testing", "Test in corso...");
    t.insert("api_test_ok", "API funzionante");
    t.insert("api_test_fail", "Errore in fase di test: ");
    t.insert("api_plan_info", "Informazioni piano tariffario");
    t.insert("api_plan_loading", "Recupero informazioni...");
    t.insert("api_plan_disclaimer", "Queste informazioni sono generate dal modello AI e potrebbero non essere accurate. Verifica sempre sul sito ufficiale del provider AI che stai utilizzando.");
    t.insert("api_delete", "Elimina configurazione API");
    t.insert("api_delete_confirm", "Sei sicuro? La chiave API verrà eliminata definitivamente e dovrai reinserirla per utilizzare i riassunti AI.");

    // Settings
    t.insert("settings_retention", "Tempo di mantenimento");
    t.insert("settings_retention_hours", "ore");
    t.insert("settings_retention_days", "giorni");
    t.insert("settings_and", "e");
    t.insert("settings_retention_hour", "ora");
    t.insert("settings_retention_day", "giorno");
    t.insert("settings_auto_update", "Aggiornamento automatico");
    t.insert("settings_auto_update_interval", "Intervallo (ore)");
    t.insert("settings_auto_update_disabled", "Disattivato");
    t.insert("settings_keywords", "Parole chiave evidenziate");
    t.insert("settings_keywords_desc", "Inserisci le parole o frasi da evidenziare, separate da punto e virgola (;)");
    t.insert("settings_keywords_placeholder", "Rust;AI;cybersecurity;...");

    // Confirm dialogs
    t.insert("confirm_delete_all_feeds",
        "Sei sicuro? Questa azione eliminerà tutti i feed, compresi gli articoli a loro associati, sia preferiti che non, e non è reversibile.");
        t.insert("delete_categories_too", "Elimina anche tutte le categorie");
    t.insert("confirm_yes", "Conferma");
    t.insert("confirm_no", "Annulla");

    // Import/Export
    t.insert("import_success", "Feed importati con successo");
    t.insert("import_count_feeds", "feed importati");
    t.insert("import_count_favorites", "articoli preferiti importati");
    t.insert("export_success", "Feed esportati con successo");
    t.insert("empty_articles", "Nessun articolo trovato. Prova ad aggiornare i feed.");
    t.insert("no_internet_title", "Connessione assente");
    t.insert("no_internet_message", "Impossibile aggiornare i feed: verifica la tua connessione a internet e riprova.");
    t.insert("empty_feeds", "Nessun feed configurato.");
    t.insert("import_description", "Seleziona un file JSON esportato da MeigRSS.");
    t.insert("back_home", "Torna alla home");
    t.insert("all_feeds_deleted", "Tutti i feed sono stati eliminati.");
    t.insert("import_confirm", "Trovati {count} feed nel file. Alcuni di questi potrebbero essere duplicati, quindi non verranno importati. Procedere?");
    t.insert("import_favorites_count", "Inclusi {count} articoli preferiti.");
    t.insert("import_keep_original", "Mantieni la categoria originale");
    t.insert("import_invalid_file", "Il file selezionato non è valido o non contiene feed.");

    // Newsletter
    t.insert("menu_import_newsletter", "Importa newsletter");
    t.insert("newsletter_description", "Seleziona un file EML esportato dalla tua casella di posta per importare una newsletter come articolo.");
    t.insert("newsletter_title", "Titolo");
    t.insert("newsletter_imported", "Newsletter importata come articolo preferito.");
    t.insert("newsletter_no_html", "Il file non contiene contenuto HTML.");
    t.insert("newsletter_duplicate", "Questa newsletter è già stata importata.");
    t.insert("newsletter_invalid_file", "Il file selezionato non è un file EML valido.");
    t.insert("link_newsletter", "Visualizza newsletter");
    t.insert("newsletter_choose_feed", "Scegli un feed newsletter");
    t.insert("newsletter_new_feed", "Crea nuovo feed");
    t.insert("newsletter_existing_feed", "Feed esistente");
    t.insert("newsletter_feed_name", "Nome del feed");
    t.insert("newsletter_feed_name_placeholder", "es. Giulio Magini");
    t.insert("newsletter_feed_name_empty", "Il nome del feed non può essere vuoto");
    t.insert("newsletter_feed_exists", "Esiste già un feed newsletter con questo nome");
    t.insert("newsletter_no_feeds", "Nessun feed newsletter esistente. Creane uno nuovo.");

    // Web feed
    t.insert("web_feed_toggle_rss", "Feed RSS");
    t.insert("web_feed_toggle_web", "Scansione pagina web");
    t.insert("web_feed_test", "Testa scansione");
    t.insert("web_feed_testing", "Scansione in corso...");
    t.insert("web_feed_test_ok", "Scansione riuscita");
    t.insert("web_feed_no_articles", "Nessun articolo trovato nella pagina. Verifica che l'URL sia corretto e che la pagina contenga link con formato data (dominio/aaaa/mm/gg/...).");
    t.insert("web_feed_articles_found", "articoli trovati");
    t.insert("web_feed_add", "Aggiungi web feed");
    t.insert("web_rendering_not_configured", "API di rendering web non configurata. Configurala nella pagina Configurazione API.");
    t.insert("web_rendering_test_failed", "Test API di rendering fallito: ");
    t.insert("web_rendering_section_title", "API Rendering Web");
    t.insert("web_rendering_provider", "Provider");
    t.insert("web_rendering_save_test", "Salva e testa");
    t.insert("web_rendering_testing", "Test in corso...");
    t.insert("web_rendering_test_ok", "API funzionante");
    t.insert("web_rendering_test_fail", "Errore in fase di test: ");
    t.insert("web_rendering_delete", "Elimina configurazione rendering");
    t.insert("web_rendering_delete_confirm", "Sei sicuro? La chiave API verrà eliminata e i web feed non potranno più essere aggiornati.");
    t.insert("tooltip_rss_feed", "Feed RSS");
    t.insert("tooltip_web_feed", "Web feed (scansione pagina)");
    t.insert("tooltip_newsletter_feed", "Feed newsletter");

    // General
    t.insert("loading", "Caricamento...");
    t.insert("error", "Errore");
    t.insert("save", "Salva");
    t.insert("cancel", "Annulla");
    t.insert("close", "Chiudi");

    t
}

fn english_translations() -> HashMap<&'static str, &'static str> {
    let mut t = HashMap::new();

    // App title
    t.insert("app_title", "MeigRSS - Your news feed");

    // Menu bar
    t.insert("menu_feeds", "Feeds");
    t.insert("menu_categories", "Categories");
    t.insert("menu_settings", "Settings");
    t.insert("menu_api_config", "API Configuration");
    t.insert("menu_refresh", "Refresh feeds");
    t.insert("menu_import", "Import feeds");
    t.insert("menu_export", "Export feeds");
    t.insert("menu_delete_all", "Delete all feeds");
    t.insert("confirm_delete_feed", "Deleting this feed will also remove all its associated articles. Continue?");
    t.insert("feed_change_category", "Change category");
    t.insert("import_select_category", "Category for imported feeds:");

    // Feed list
    t.insert("summary_from_feed", "Summary from website");
    t.insert("summary_from_ai", "AI-generated summary");
    t.insert("ai_summary_disclaimer", "The AI summary may contain inaccurate information. Always verify the original article to be sure of the content.");
    t.insert("no_summary", "Unable to create a summary for this article");
    t.insert("no_full_text", "The full article text is not available.");
    t.insert("scrape_no_content", "No readable content found on the page");
    t.insert("scrape_failed", "Failed to extract text: ");
    t.insert("btn_ai_summary", "Generate AI summary");
    t.insert("btn_ai_summary_done", "AI summary already generated");
    t.insert("btn_ai_limit_reached", "Usage limits exceeded");
    t.insert("btn_api_not_configured", "API not configured correctly");
    t.insert("btn_show_full_text", "Show full article");
    t.insert("btn_hide_full_text", "Hide full article");
    t.insert("link_original", "Read original article");

    // Favorites
    t.insert("tooltip_add_favorite", "Add to favorites");
    t.insert("tooltip_remove_favorite", "Remove from favorites");
    t.insert("confirm_remove_expired_favorite",
        "This article is past its retention period. Removing it from favorites will permanently delete it. Continue?");

    // Filters and sorting
    t.insert("sort_newest", "Newest first");
    t.insert("sort_oldest", "Oldest first");
    t.insert("sort_feed_newest", "Feed (newest first)");
    t.insert("sort_feed_oldest", "Feed (oldest first)");
    t.insert("filter_all", "All categories");
    t.insert("filter_favorites_exclude", "Favorites excluded");
    t.insert("filter_favorites_all", "All articles");
    t.insert("filter_favorites_only", "Favorites only");
    t.insert("search_placeholder", "Search in title...");
    t.insert("search_no_results", "No articles match your search.");
    t.insert("pagination_per_page", "per page");

    // Categories
    t.insert("cat_uncategorized", "Uncategorized");
    t.insert("cat_add", "Add category");
    t.insert("cat_rename", "Rename");
    t.insert("cat_delete", "Delete");
    t.insert("cat_delete_error_has_feeds", "Cannot delete: category still has feeds assigned");
    t.insert("cat_delete_error_default", "The default category cannot be deleted");
    t.insert("cat_name_reserved", "This name is reserved for the default category");

    // Feed management
    t.insert("feed_add", "Add feed");
    t.insert("feed_test", "Test feed");
    t.insert("feed_testing", "Testing...");
    t.insert("feed_test_ok", "Valid feed");
    t.insert("feed_test_fail", "Invalid feed");
    t.insert("feed_already_exists", "This feed already exists in the list");
    t.insert("feed_force_ai", "Always force AI summary");
    t.insert("feed_active", "Active");
    t.insert("feed_paused", "Paused");
    t.insert("feeds_auto_paused_title", "Feeds paused");
    t.insert("feeds_auto_paused_message", "The following feeds did not respond and have been automatically paused:");
    t.insert("feeds_auto_paused_hint", "To resume them, go to the feed management page.");
    t.insert("feed_enable_scraping", "Enable full text download");
    t.insert("scraping_disabled", "Full text download is disabled for this feed");
    t.insert("scraping_disclaimer", "Downloading the full text of articles is at the user's discretion. Before enabling it, make sure you comply with the terms of use of the website in question. The software author is not responsible for any misuse of this feature.");
    t.insert("feed_delete", "Remove feed");

    // API configuration
    t.insert("api_provider", "Provider");
    t.insert("api_key_field", "API Key");
    t.insert("api_save_test", "Save and test");
    t.insert("api_testing", "Testing...");
    t.insert("api_test_ok", "API working");
    t.insert("api_test_fail", "Test error: ");
    t.insert("api_plan_info", "Plan information");
    t.insert("api_plan_loading", "Fetching information...");
    t.insert("api_plan_disclaimer", "This information is generated by the AI model and may not be accurate. Always verify on the official website of the AI provider you are using.");
    t.insert("api_delete", "Delete API configuration");
    t.insert("api_delete_confirm", "Are you sure? The API key will be permanently deleted and you will need to re-enter it to use AI summaries.");

    // Settings
    t.insert("settings_retention", "Retention period");
    t.insert("settings_retention_hours", "hours");
    t.insert("settings_retention_days", "days");
    t.insert("settings_and", "and");
    t.insert("settings_retention_hour", "hour");
    t.insert("settings_retention_day", "day");
    t.insert("settings_auto_update", "Auto-update");
    t.insert("settings_auto_update_interval", "Interval (hours)");
    t.insert("settings_auto_update_disabled", "Disabled");
    t.insert("settings_keywords", "Highlighted keywords");
    t.insert("settings_keywords_desc", "Enter words or phrases to highlight, separated by semicolons (;)");
    t.insert("settings_keywords_placeholder", "Rust;AI;cybersecurity;...");

    // Confirm dialogs
    t.insert("confirm_delete_all_feeds",
        "Are you sure? This will delete all feeds, including their articles (both favorites and not favorites), and cannot be undone.");
        t.insert("delete_categories_too", "Also delete all categories");
    t.insert("confirm_yes", "Confirm");
    t.insert("confirm_no", "Cancel");

    // Import/Export
    t.insert("import_success", "Feeds imported successfully");
    t.insert("import_count_feeds", "feeds imported");
    t.insert("import_count_favorites", "favorite articles imported");
    t.insert("export_success", "Feeds exported successfully");
    t.insert("empty_articles", "No articles found. Try refreshing your feeds.");
    t.insert("no_internet_title", "No internet connection");
    t.insert("no_internet_message", "Unable to update feeds: check your internet connection and try again.");
    t.insert("empty_feeds", "No feeds configured.");
    t.insert("import_description", "Select a JSON file exported from MeigRSS.");
    t.insert("back_home", "Back to home");
    t.insert("all_feeds_deleted", "All feeds have been deleted.");
    t.insert("import_confirm", "Found {count} feeds in the file. Some of this feed could be duplicates, in that case they will not be inserted again. Proceed?");
    t.insert("import_favorites_count", "Including {count} favorite articles.");
    t.insert("import_keep_original", "Keep original category");
    t.insert("import_invalid_file", "The selected file is not valid or contains no feeds.");

    // Newsletter
    t.insert("menu_import_newsletter", "Import newsletter");
    t.insert("newsletter_description", "Select an EML file exported from your mailbox to import a newsletter as an article.");
    t.insert("newsletter_title", "Title");
    t.insert("newsletter_imported", "Newsletter imported as a favorite article.");
    t.insert("newsletter_no_html", "The file does not contain HTML content.");
    t.insert("newsletter_duplicate", "This newsletter has already been imported.");
    t.insert("newsletter_invalid_file", "The selected file is not a valid EML file.");
    t.insert("link_newsletter", "View newsletter");
    t.insert("newsletter_choose_feed", "Choose a newsletter feed");
    t.insert("newsletter_new_feed", "Create new feed");
    t.insert("newsletter_existing_feed", "Existing feed");
    t.insert("newsletter_feed_name", "Feed name");
    t.insert("newsletter_feed_name_placeholder", "e.g. Giulio Magini");
    t.insert("newsletter_feed_name_empty", "Feed name cannot be empty");
    t.insert("newsletter_feed_exists", "A newsletter feed with this name already exists");
    t.insert("newsletter_no_feeds", "No newsletter feeds exist yet. Create a new one.");

    // Web feed
    t.insert("web_feed_toggle_rss", "RSS Feed");
    t.insert("web_feed_toggle_web", "Web page scan");
    t.insert("web_feed_test", "Test scan");
    t.insert("web_feed_testing", "Scanning...");
    t.insert("web_feed_test_ok", "Scan successful");
    t.insert("web_feed_no_articles", "No articles found on the page. Verify the URL is correct and the page contains date-patterned links (domain/yyyy/mm/dd/...).");
    t.insert("web_feed_articles_found", "articles found");
    t.insert("web_feed_add", "Add web feed");
    t.insert("web_rendering_not_configured", "Web rendering API not configured. Set it up in the API Configuration page.");
    t.insert("web_rendering_test_failed", "Rendering API test failed: ");
    t.insert("web_rendering_section_title", "Web Rendering API");
    t.insert("web_rendering_provider", "Provider");
    t.insert("web_rendering_save_test", "Save and test");
    t.insert("web_rendering_testing", "Testing...");
    t.insert("web_rendering_test_ok", "API working");
    t.insert("web_rendering_test_fail", "Test error: ");
    t.insert("web_rendering_delete", "Delete rendering configuration");
    t.insert("web_rendering_delete_confirm", "Are you sure? The API key will be deleted and web feeds will no longer be updated.");
    t.insert("tooltip_rss_feed", "RSS feed");
    t.insert("tooltip_web_feed", "Web feed (page scan)");
    t.insert("tooltip_newsletter_feed", "Newsletter feed");

    // General
    t.insert("loading", "Loading...");
    t.insert("error", "Error");
    t.insert("save", "Save");
    t.insert("cancel", "Cancel");
    t.insert("close", "Close");

    t
}