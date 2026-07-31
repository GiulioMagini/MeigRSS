//! Spanish UI translation dictionary.
//! Mirrors the key set of the Italian and English dictionaries in `mod.rs`.

use std::collections::HashMap;

/// Returns the full Spanish translation map.
/// Keys are the translation identifiers used by the frontend.
pub(super) fn translations() -> HashMap<&'static str, &'static str> {
    let mut t = HashMap::new();

    // App title
    t.insert("app_title", "MeigRSS - Tu agregador de noticias");

    // Menu bar
    t.insert("menu_feeds", "Feeds");
    t.insert("menu_categories", "Categorías");
    t.insert("menu_settings", "Ajustes");
    t.insert("menu_api_config", "Configuración de API");
    t.insert("menu_refresh", "Actualizar feeds");
    t.insert("menu_import", "Importar feeds");
    t.insert("menu_export", "Exportar feeds");
    t.insert("menu_delete_all", "Eliminar todos los feeds");

    // Feed list
    t.insert("summary_from_feed", "Resumen del sitio");
    t.insert("summary_from_ai", "Resumen generado por IA");
    t.insert("ai_summary_disclaimer", "El resumen de la IA puede contener información imprecisa. Verifica siempre el artículo original para asegurarte del contenido.");
    t.insert("no_summary", "No se puede crear un resumen para este artículo");
    t.insert("no_full_text", "El texto completo del artículo no está disponible.");
    t.insert("scrape_no_content", "No se ha encontrado contenido legible en la página");
    t.insert("scrape_failed", "Error al extraer el texto: ");
    t.insert("btn_ai_summary", "Crear resumen con IA");
    t.insert("btn_ai_summary_done", "Resumen de IA ya generado");
    t.insert("btn_ai_limit_reached", "Límites de uso superados");
    t.insert("btn_api_not_configured", "API mal configurada");
    t.insert("btn_show_full_text", "Mostrar artículo completo");
    t.insert("btn_hide_full_text", "Ocultar artículo completo");
    t.insert("btn_edit_full_text", "Editar el texto completo");
    t.insert("btn_save_full_text", "Guardar el texto completo");
    t.insert("full_text_too_long", "El texto introducido supera el tamaño máximo permitido.");
    t.insert("link_original", "Leer el artículo original");

    // Favorites
    t.insert("tooltip_add_favorite", "Añadir a favoritos");
    t.insert("tooltip_remove_favorite", "Quitar de favoritos");
    t.insert("confirm_remove_expired_favorite",
        "Este artículo ha superado el tiempo de conservación. Si lo quitas de favoritos se eliminará definitivamente. ¿Continuar?");

    // Filters and sorting
    t.insert("sort_newest", "Más recientes");
    t.insert("sort_oldest", "Más antiguos");
    t.insert("sort_feed_newest", "Feed (más recientes)");
    t.insert("sort_feed_oldest", "Feed (más antiguos)");
    t.insert("filter_all", "Todas las categorías");
    t.insert("filter_favorites_exclude", "Favoritos excluidos");
    t.insert("filter_favorites_all", "Todos los artículos");
    t.insert("filter_favorites_only", "Solo favoritos");
    t.insert("search_placeholder", "Buscar en el título...");
    t.insert("search_no_results", "Ningún artículo coincide con la búsqueda.");
    t.insert("pagination_per_page", "por página");

    // Categories
    t.insert("cat_uncategorized", "Sin categoría");
    t.insert("cat_add", "Añadir categoría");
    t.insert("cat_rename", "Renombrar");
    t.insert("cat_delete", "Eliminar");
    t.insert("cat_delete_error_has_feeds", "No se puede eliminar: la categoría todavía tiene feeds asociados");
    t.insert("cat_delete_error_default", "La categoría predeterminada no se puede eliminar");
    t.insert("cat_name_reserved", "Este nombre está reservado para la categoría predeterminada");

    // Feed management
    t.insert("feed_add", "Añadir feed");
    t.insert("feed_test", "Probar feed");
    t.insert("feed_testing", "Probando...");
    t.insert("feed_test_ok", "Feed válido");
    t.insert("feed_test_fail", "Feed no válido");
    t.insert("feed_already_exists", "Este feed ya está en la lista");
    t.insert("feed_force_ai", "Forzar siempre el resumen de IA");
    t.insert("feed_active", "Activo");
    t.insert("feed_paused", "En pausa");
    t.insert("feeds_auto_paused_title", "Feeds bloqueados");
    t.insert("feeds_auto_paused_message", "Los siguientes feeds se han pausado automáticamente:");
    t.insert("feeds_auto_paused_hint", "Para desbloquearlos, ve a la página de gestión de feeds.");
    t.insert("pause_reason_timeout", "(tiempo de espera agotado)");
    t.insert("pause_reason_gone", "(feed eliminado)");
    t.insert("pause_reason_too_many", "(demasiadas solicitudes)");
    t.insert("pause_reason_too_many_retry", "(demasiadas solicitudes, intenta reactivarlo en {n})");
    t.insert("feed_enable_scraping", "Habilitar la descarga del texto completo");
    t.insert("feed_api_scraping", "Habilitar API de scraping");
    t.insert("scraping_disclaimer", "La descarga del texto completo de los artículos queda a discreción del usuario. Antes de activarla, asegúrate de respetar las condiciones de uso del sitio en cuestión. El autor del software no es responsable del uso indebido de esta función.");
    t.insert("feed_delete", "Eliminar feed");
    t.insert("confirm_delete_feed", "Al eliminar este feed también se eliminarán todos los artículos asociados. ¿Continuar?");
    t.insert("feed_change_category", "Cambiar de categoría");
    t.insert("import_select_category", "Categoría para los feeds importados:");

    // API configuration
    t.insert("api_provider", "Proveedor");
    t.insert("api_key_field", "Clave API");
    t.insert("api_save_test", "Guardar y probar");
    t.insert("api_testing", "Probando...");
    t.insert("api_test_ok", "API funcionando");
    t.insert("api_test_fail", "Error durante la prueba: ");
    t.insert("api_plan_info", "Información del plan");
    t.insert("api_plan_loading", "Obteniendo información...");
    t.insert("api_plan_disclaimer", "Esta información la genera el modelo de IA y podría no ser precisa. Verifícala siempre en el sitio oficial del proveedor de IA que estás utilizando.");
    t.insert("api_delete", "Eliminar configuración de API");
    t.insert("api_delete_confirm", "¿Estás seguro? La clave API se eliminará definitivamente y tendrás que volver a introducirla para usar los resúmenes de IA.");

    // Settings
    t.insert("settings_retention", "Tiempo de conservación");
    t.insert("settings_retention_hours", "horas");
    t.insert("settings_retention_days", "días");
    t.insert("settings_and", "y");
    t.insert("settings_retention_hour", "hora");
    t.insert("settings_retention_day", "día");
    t.insert("settings_auto_update", "Actualización automática");
    t.insert("settings_auto_update_interval", "Intervalo (horas)");
    t.insert("settings_auto_update_disabled", "Desactivada");
    t.insert("settings_keywords", "Palabras clave resaltadas");
    t.insert("settings_keywords_desc", "Introduce las palabras o frases que resaltar, separadas por punto y coma (;)");
    t.insert("settings_keywords_placeholder", "Rust;IA;ciberseguridad;...");

    // Confirm dialogs
    t.insert("confirm_delete_all_feeds",
        "¿Estás seguro? Esta acción eliminará todos los feeds, incluidos los artículos asociados, favoritos o no, y no se puede deshacer.");
    t.insert("delete_categories_too", "Eliminar también todas las categorías");
    t.insert("confirm_yes", "Confirmar");
    t.insert("confirm_no", "Cancelar");

    // Import/Export
    t.insert("import_success", "Feeds importados correctamente");
    t.insert("import_count_feeds", "feeds importados");
    t.insert("import_count_favorites", "artículos favoritos importados");
    t.insert("export_success", "Feeds exportados correctamente");
    t.insert("empty_articles", "No se ha encontrado ningún artículo. Prueba a actualizar los feeds.");
    t.insert("no_internet_title", "Sin conexión");
    t.insert("no_internet_message", "No se pueden actualizar los feeds: comprueba tu conexión a internet e inténtalo de nuevo.");
    t.insert("empty_feeds", "Ningún feed configurado.");
    t.insert("import_description", "Selecciona un archivo OPML o un archivo JSON exportado desde MeigRSS.");
    t.insert("back_home", "Volver al inicio");
    t.insert("all_feeds_deleted", "Se han eliminado todos los feeds.");
    t.insert("import_confirm", "Se han encontrado {count} feeds en el archivo. Algunos podrían estar duplicados, por lo que no se importarán. ¿Continuar?");
    t.insert("import_favorites_count", "Incluidos {count} artículos favoritos.");
    t.insert("import_keep_original", "Mantener la categoría original");
    t.insert("import_invalid_file", "El archivo seleccionado no es válido o no contiene feeds.");
    t.insert("export_format_label", "Selecciona el formato de salida");
    t.insert("export_button", "Exportar");

    // Newsletter
    t.insert("menu_import_newsletter", "Importar newsletter");
    t.insert("newsletter_description", "Selecciona un archivo EML exportado desde tu bandeja de correo para importar una newsletter como artículo.");
    t.insert("newsletter_title", "Título");
    t.insert("newsletter_imported", "Newsletter importada como artículo favorito.");
    t.insert("newsletter_no_html", "El archivo no contiene contenido HTML.");
    t.insert("newsletter_duplicate", "Esta newsletter ya se ha importado.");
    t.insert("newsletter_invalid_file", "El archivo seleccionado no es un archivo EML válido.");
    t.insert("link_newsletter", "Ver newsletter");
    t.insert("newsletter_choose_feed", "Elige un feed de newsletter");
    t.insert("newsletter_new_feed", "Crear nuevo feed");
    t.insert("newsletter_existing_feed", "Feed existente");
    t.insert("newsletter_feed_name", "Nombre del feed");
    t.insert("newsletter_feed_name_placeholder", "p. ej. Giulio Magini");
    t.insert("newsletter_feed_name_empty", "El nombre del feed no puede estar vacío");
    t.insert("newsletter_feed_exists", "Ya existe un feed de newsletter con este nombre");
    t.insert("newsletter_no_feeds", "No hay ningún feed de newsletter existente. Crea uno nuevo.");
    t.insert("newsletter_mode_single", "Archivo único");
    t.insert("newsletter_mode_folder", "Carpeta");
    t.insert("newsletter_folder_description", "Selecciona una carpeta: se importarán todos los archivos EML que contenga, ignorando cualquier otro tipo de archivo.");
    t.insert("newsletter_folder_selected", "{count} archivos EML listos para importar.");
    t.insert("newsletter_folder_no_eml", "La carpeta seleccionada no contiene archivos EML.");
    t.insert("newsletter_folder_too_many", "La carpeta contiene más de {max} archivos EML. Selecciona una con menos archivos.");
    t.insert("newsletter_folder_too_large", "Los archivos seleccionados superan el tamaño máximo permitido para un solo envío.");
    t.insert("newsletter_bulk_imported", "{count} newsletters importadas como artículos favoritos.");
    t.insert("newsletter_bulk_none", "No se ha importado ninguna newsletter.");
    t.insert("newsletter_bulk_duplicates", "{count} ya presentes en el archivo");
    t.insert("newsletter_bulk_no_html", "{count} sin contenido HTML");
    t.insert("newsletter_bulk_invalid", "{count} no reconocidos como correos válidos");

    // Web feed
    t.insert("web_feed_toggle_rss", "Feed RSS");
    t.insert("web_feed_toggle_web", "Escaneo de página web");
    t.insert("web_feed_test", "Probar escaneo");
    t.insert("web_feed_testing", "Escaneando...");
    t.insert("web_feed_test_ok", "Escaneo correcto");
    t.insert("web_feed_no_articles", "No se ha encontrado ningún artículo en la página. Comprueba que la URL sea correcta y que la página contenga enlaces con formato de fecha.");
    t.insert("web_feed_articles_found", "artículos encontrados");
    t.insert("web_feed_add", "Añadir web feed");
    t.insert("web_rendering_test_failed", "Prueba de la API de renderizado fallida: ");
    t.insert("web_rendering_section_title", "API de renderizado web");
    t.insert("web_rendering_provider", "Proveedor");
    t.insert("web_rendering_save_test", "Guardar y probar");
    t.insert("web_rendering_testing", "Probando...");
    t.insert("web_rendering_test_ok", "API funcionando");
    t.insert("web_rendering_test_fail", "Error durante la prueba: ");
    t.insert("web_rendering_delete", "Eliminar configuración de renderizado");
    t.insert("web_rendering_delete_confirm", "¿Estás seguro? La clave API se eliminará y los web feeds ya no podrán actualizarse.");
    t.insert("tooltip_rss_feed", "Feed RSS");
    t.insert("tooltip_web_feed", "Web feed (escaneo de página)");
    t.insert("tooltip_newsletter_feed", "Feed de newsletter");

    // General
    t.insert("loading", "Cargando...");
    t.insert("error", "Error");
    t.insert("save", "Guardar");
    t.insert("cancel", "Cancelar");
    t.insert("close", "Cerrar");

    t
}
