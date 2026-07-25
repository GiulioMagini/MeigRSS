//! French UI translation dictionary.
//! Mirrors the key set of the Italian and English dictionaries in `mod.rs`.

use std::collections::HashMap;

/// Returns the full French translation map.
/// Keys are the translation identifiers used by the frontend.
pub(super) fn translations() -> HashMap<&'static str, &'static str> {
    let mut t = HashMap::new();

    // App title
    t.insert("app_title", "MeigRSS - Votre agrégateur d'actualités");

    // Menu bar
    t.insert("menu_feeds", "Flux");
    t.insert("menu_categories", "Catégories");
    t.insert("menu_settings", "Paramètres");
    t.insert("menu_api_config", "Configuration API");
    t.insert("menu_refresh", "Actualiser les flux");
    t.insert("menu_import", "Importer des flux");
    t.insert("menu_export", "Exporter des flux");
    t.insert("menu_delete_all", "Supprimer tous les flux");

    // Feed list
    t.insert("summary_from_feed", "Résumé du site");
    t.insert("summary_from_ai", "Résumé généré par l'IA");
    t.insert("ai_summary_disclaimer", "Le résumé de l'IA peut contenir des informations inexactes. Vérifie toujours l'article original pour t'assurer du contenu.");
    t.insert("no_summary", "Impossible de créer un résumé pour cet article");
    t.insert("no_full_text", "Le texte complet de l'article n'est pas disponible.");
    t.insert("scrape_no_content", "Aucun contenu lisible trouvé dans la page");
    t.insert("scrape_failed", "Erreur lors de l'extraction du texte : ");
    t.insert("btn_ai_summary", "Créer un résumé avec l'IA");
    t.insert("btn_ai_summary_done", "Résumé IA déjà généré");
    t.insert("btn_ai_limit_reached", "Limites d'utilisation dépassées");
    t.insert("btn_api_not_configured", "API mal configurée");
    t.insert("btn_show_full_text", "Afficher l'article complet");
    t.insert("btn_hide_full_text", "Masquer l'article complet");
    t.insert("link_original", "Lire l'article original");

    // Favorites
    t.insert("tooltip_add_favorite", "Ajouter aux favoris");
    t.insert("tooltip_remove_favorite", "Retirer des favoris");
    t.insert("confirm_remove_expired_favorite",
        "Cet article a dépassé la durée de conservation. Le retirer des favoris le supprimera définitivement. Continuer ?");

    // Filters and sorting
    t.insert("sort_newest", "Plus récents");
    t.insert("sort_oldest", "Plus anciens");
    t.insert("sort_feed_newest", "Flux (plus récents)");
    t.insert("sort_feed_oldest", "Flux (plus anciens)");
    t.insert("filter_all", "Toutes les catégories");
    t.insert("filter_favorites_exclude", "Favoris exclus");
    t.insert("filter_favorites_all", "Tous les articles");
    t.insert("filter_favorites_only", "Favoris uniquement");
    t.insert("search_placeholder", "Rechercher dans le titre...");
    t.insert("search_no_results", "Aucun article ne correspond à la recherche.");
    t.insert("pagination_per_page", "par page");

    // Categories
    t.insert("cat_uncategorized", "Sans catégorie");
    t.insert("cat_add", "Ajouter une catégorie");
    t.insert("cat_rename", "Renommer");
    t.insert("cat_delete", "Supprimer");
    t.insert("cat_delete_error_has_feeds", "Suppression impossible : la catégorie contient encore des flux associés");
    t.insert("cat_delete_error_default", "La catégorie par défaut ne peut pas être supprimée");
    t.insert("cat_name_reserved", "Ce nom est réservé à la catégorie par défaut");

    // Feed management
    t.insert("feed_add", "Ajouter un flux");
    t.insert("feed_test", "Tester le flux");
    t.insert("feed_testing", "Test en cours...");
    t.insert("feed_test_ok", "Flux valide");
    t.insert("feed_test_fail", "Flux non valide");
    t.insert("feed_already_exists", "Ce flux est déjà présent dans la liste");
    t.insert("feed_force_ai", "Toujours forcer le résumé IA");
    t.insert("feed_active", "Actif");
    t.insert("feed_paused", "En pause");
    t.insert("feeds_auto_paused_title", "Flux bloqués");
    t.insert("feeds_auto_paused_message", "Les flux suivants n'ont pas répondu et ont été mis en pause automatiquement :");
    t.insert("feeds_auto_paused_hint", "Pour les débloquer, va dans la page de gestion des flux.");
    t.insert("feed_enable_scraping", "Activer le téléchargement du texte complet");
    t.insert("feed_api_scraping", "Activer l'API de scraping");
    t.insert("scraping_disabled", "Téléchargement du texte complet désactivé pour ce flux");
    t.insert("scraping_disclaimer", "Le téléchargement du texte complet des articles est à la discrétion de l'utilisateur. Avant de l'activer, assure-toi de respecter les conditions d'utilisation du site concerné. L'auteur du logiciel n'est pas responsable de l'usage abusif de cette fonctionnalité.");
    t.insert("feed_delete", "Supprimer le flux");
    t.insert("confirm_delete_feed", "La suppression de ce flux entraînera aussi la suppression de tous les articles associés. Continuer ?");
    t.insert("feed_change_category", "Changer de catégorie");
    t.insert("import_select_category", "Catégorie pour les flux importés :");

    // API configuration
    t.insert("api_provider", "Fournisseur");
    t.insert("api_key_field", "Clé API");
    t.insert("api_save_test", "Enregistrer et tester");
    t.insert("api_testing", "Test en cours...");
    t.insert("api_test_ok", "API fonctionnelle");
    t.insert("api_test_fail", "Erreur lors du test : ");
    t.insert("api_plan_info", "Informations sur le forfait");
    t.insert("api_plan_loading", "Récupération des informations...");
    t.insert("api_plan_disclaimer", "Ces informations sont générées par le modèle d'IA et peuvent être inexactes. Vérifie toujours sur le site officiel du fournisseur d'IA que tu utilises.");
    t.insert("api_delete", "Supprimer la configuration API");
    t.insert("api_delete_confirm", "Es-tu sûr ? La clé API sera supprimée définitivement et tu devras la ressaisir pour utiliser les résumés IA.");

    // Settings
    t.insert("settings_retention", "Durée de conservation");
    t.insert("settings_retention_hours", "heures");
    t.insert("settings_retention_days", "jours");
    t.insert("settings_and", "et");
    t.insert("settings_retention_hour", "heure");
    t.insert("settings_retention_day", "jour");
    t.insert("settings_auto_update", "Mise à jour automatique");
    t.insert("settings_auto_update_interval", "Intervalle (heures)");
    t.insert("settings_auto_update_disabled", "Désactivée");
    t.insert("settings_keywords", "Mots-clés surlignés");
    t.insert("settings_keywords_desc", "Saisis les mots ou expressions à surligner, séparés par un point-virgule (;)");
    t.insert("settings_keywords_placeholder", "Rust;IA;cybersécurité;...");

    // Confirm dialogs
    t.insert("confirm_delete_all_feeds",
        "Es-tu sûr ? Cette action supprimera tous les flux, y compris les articles associés, favoris ou non, et elle est irréversible.");
    t.insert("delete_categories_too", "Supprimer aussi toutes les catégories");
    t.insert("confirm_yes", "Confirmer");
    t.insert("confirm_no", "Annuler");

    // Import/Export
    t.insert("import_success", "Flux importés avec succès");
    t.insert("import_count_feeds", "flux importés");
    t.insert("import_count_favorites", "articles favoris importés");
    t.insert("export_success", "Flux exportés avec succès");
    t.insert("empty_articles", "Aucun article trouvé. Essaie d'actualiser les flux.");
    t.insert("no_internet_title", "Connexion absente");
    t.insert("no_internet_message", "Impossible d'actualiser les flux : vérifie ta connexion internet et réessaie.");
    t.insert("empty_feeds", "Aucun flux configuré.");
    t.insert("import_description", "Sélectionne un fichier OPML ou un fichier JSON exporté depuis MeigRSS.");
    t.insert("back_home", "Retour à l'accueil");
    t.insert("all_feeds_deleted", "Tous les flux ont été supprimés.");
    t.insert("import_confirm", "{count} flux trouvés dans le fichier. Certains peuvent être des doublons et ne seront donc pas importés. Continuer ?");
    t.insert("import_favorites_count", "Y compris {count} articles favoris.");
    t.insert("import_keep_original", "Conserver la catégorie d'origine");
    t.insert("import_invalid_file", "Le fichier sélectionné n'est pas valide ou ne contient pas de flux.");
    t.insert("export_format_label", "Sélectionne le format de sortie");
    t.insert("export_button", "Exporter");

    // Newsletter
    t.insert("menu_import_newsletter", "Importer une newsletter");
    t.insert("newsletter_description", "Sélectionne un fichier EML exporté depuis ta boîte mail pour importer une newsletter en tant qu'article.");
    t.insert("newsletter_title", "Titre");
    t.insert("newsletter_imported", "Newsletter importée comme article favori.");
    t.insert("newsletter_no_html", "Le fichier ne contient pas de contenu HTML.");
    t.insert("newsletter_duplicate", "Cette newsletter a déjà été importée.");
    t.insert("newsletter_invalid_file", "Le fichier sélectionné n'est pas un fichier EML valide.");
    t.insert("link_newsletter", "Afficher la newsletter");
    t.insert("newsletter_choose_feed", "Choisis un flux newsletter");
    t.insert("newsletter_new_feed", "Créer un nouveau flux");
    t.insert("newsletter_existing_feed", "Flux existant");
    t.insert("newsletter_feed_name", "Nom du flux");
    t.insert("newsletter_feed_name_placeholder", "ex. Giulio Magini");
    t.insert("newsletter_feed_name_empty", "Le nom du flux ne peut pas être vide");
    t.insert("newsletter_feed_exists", "Un flux newsletter portant ce nom existe déjà");
    t.insert("newsletter_no_feeds", "Aucun flux newsletter existant. Crées-en un nouveau.");

    // Web feed
    t.insert("web_feed_toggle_rss", "Flux RSS");
    t.insert("web_feed_toggle_web", "Analyse de page web");
    t.insert("web_feed_test", "Tester l'analyse");
    t.insert("web_feed_testing", "Analyse en cours...");
    t.insert("web_feed_test_ok", "Analyse réussie");
    t.insert("web_feed_no_articles", "Aucun article trouvé dans la page. Vérifie que l'URL est correcte et que la page contient des liens au format date.");
    t.insert("web_feed_articles_found", "articles trouvés");
    t.insert("web_feed_add", "Ajouter un web feed");
    t.insert("web_rendering_test_failed", "Échec du test de l'API de rendu : ");
    t.insert("web_rendering_section_title", "API de rendu web");
    t.insert("web_rendering_provider", "Fournisseur");
    t.insert("web_rendering_save_test", "Enregistrer et tester");
    t.insert("web_rendering_testing", "Test en cours...");
    t.insert("web_rendering_test_ok", "API fonctionnelle");
    t.insert("web_rendering_test_fail", "Erreur lors du test : ");
    t.insert("web_rendering_delete", "Supprimer la configuration de rendu");
    t.insert("web_rendering_delete_confirm", "Es-tu sûr ? La clé API sera supprimée et les web feeds ne pourront plus être actualisés.");
    t.insert("tooltip_rss_feed", "Flux RSS");
    t.insert("tooltip_web_feed", "Web feed (analyse de page)");
    t.insert("tooltip_newsletter_feed", "Flux newsletter");

    // General
    t.insert("loading", "Chargement...");
    t.insert("error", "Erreur");
    t.insert("save", "Enregistrer");
    t.insert("cancel", "Annuler");
    t.insert("close", "Fermer");

    t
}
