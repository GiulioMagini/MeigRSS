//! European Portuguese (PT-PT) UI translation dictionary.
//! Mirrors the key set of the Italian and English dictionaries in `mod.rs`.

use std::collections::HashMap;

/// Returns the full European Portuguese translation map.
/// Keys are the translation identifiers used by the frontend.
pub(super) fn translations() -> HashMap<&'static str, &'static str> {
    let mut t = HashMap::new();

    // App title
    t.insert("app_title", "MeigRSS - O teu agregador de notícias");

    // Menu bar
    t.insert("menu_feeds", "Feeds");
    t.insert("menu_categories", "Categorias");
    t.insert("menu_settings", "Definições");
    t.insert("menu_api_config", "Configuração da API");
    t.insert("menu_refresh", "Atualizar feeds");
    t.insert("menu_import", "Importar feeds");
    t.insert("menu_export", "Exportar feeds");
    t.insert("menu_delete_all", "Eliminar todos os feeds");

    // Feed list
    t.insert("summary_from_feed", "Resumo do site");
    t.insert("summary_from_ai", "Resumo gerado por IA");
    t.insert("ai_summary_disclaimer", "O resumo da IA pode conter informações imprecisas. Verifica sempre o artigo original para te certificares do conteúdo.");
    t.insert("no_summary", "Não é possível criar um resumo para este artigo");
    t.insert("no_full_text", "O texto completo do artigo não está disponível.");
    t.insert("scrape_no_content", "Não foi encontrado conteúdo legível na página");
    t.insert("scrape_failed", "Erro ao extrair o texto: ");
    t.insert("btn_ai_summary", "Criar resumo com IA");
    t.insert("btn_ai_summary_done", "Resumo de IA já gerado");
    t.insert("btn_ai_limit_reached", "Limites de utilização excedidos");
    t.insert("btn_api_not_configured", "API mal configurada");
    t.insert("btn_show_full_text", "Mostrar artigo completo");
    t.insert("btn_hide_full_text", "Ocultar artigo completo");
    t.insert("btn_edit_full_text", "Editar o texto completo");
    t.insert("btn_save_full_text", "Guardar o texto completo");
    t.insert("full_text_too_long", "O texto introduzido excede o tamanho máximo permitido.");
    t.insert("link_original", "Ler o artigo original");

    // Favorites
    t.insert("tooltip_add_favorite", "Adicionar aos favoritos");
    t.insert("tooltip_remove_favorite", "Remover dos favoritos");
    t.insert("confirm_remove_expired_favorite",
        "Este artigo ultrapassou o tempo de conservação. Se o removeres dos favoritos, será eliminado definitivamente. Continuar?");

    // Filters and sorting
    t.insert("sort_newest", "Mais recentes");
    t.insert("sort_oldest", "Mais antigos");
    t.insert("sort_feed_newest", "Feed (mais recentes)");
    t.insert("sort_feed_oldest", "Feed (mais antigos)");
    t.insert("filter_all", "Todas as categorias");
    t.insert("filter_favorites_exclude", "Favoritos excluídos");
    t.insert("filter_favorites_all", "Todos os artigos");
    t.insert("filter_favorites_only", "Apenas favoritos");
    t.insert("search_placeholder", "Procurar no título...");
    t.insert("search_no_results", "Nenhum artigo corresponde à pesquisa.");
    t.insert("pagination_per_page", "por página");

    // Categories
    t.insert("cat_uncategorized", "Sem categoria");
    t.insert("cat_add", "Adicionar categoria");
    t.insert("cat_rename", "Mudar o nome");
    t.insert("cat_delete", "Eliminar");
    t.insert("cat_delete_error_has_feeds", "Não é possível eliminar: a categoria ainda tem feeds associados");
    t.insert("cat_delete_error_default", "A categoria predefinida não pode ser eliminada");
    t.insert("cat_name_reserved", "Este nome está reservado para a categoria predefinida");

    // Feed management
    t.insert("feed_add", "Adicionar feed");
    t.insert("feed_test", "Testar feed");
    t.insert("feed_testing", "A testar...");
    t.insert("feed_test_ok", "Feed válido");
    t.insert("feed_test_fail", "Feed inválido");
    t.insert("feed_already_exists", "Este feed já está na lista");
    t.insert("feed_force_ai", "Forçar sempre o resumo de IA");
    t.insert("feed_active", "Ativo");
    t.insert("feed_paused", "Em pausa");
    t.insert("feeds_auto_paused_title", "Feeds bloqueados");
    t.insert("feeds_auto_paused_message", "Os seguintes feeds foram colocados em pausa automaticamente:");
    t.insert("feeds_auto_paused_hint", "Para os desbloquear, vai à página de gestão de feeds.");
    t.insert("pause_reason_timeout", "(tempo esgotado)");
    t.insert("pause_reason_gone", "(feed eliminado)");
    t.insert("pause_reason_too_many", "(demasiados pedidos)");
    t.insert("pause_reason_too_many_retry", "(demasiados pedidos, tenta retirar da pausa dentro de {n})");
    t.insert("feed_enable_scraping", "Ativar a transferência do texto completo");
    t.insert("feed_api_scraping", "Ativar API de scraping");
    t.insert("scraping_disclaimer", "A transferência do texto completo dos artigos fica ao critério do utilizador. Antes de a ativares, certifica-te de que respeitas os termos de utilização do site em questão. O autor do software não é responsável pela utilização indevida desta funcionalidade.");
    t.insert("feed_delete", "Remover feed");
    t.insert("confirm_delete_feed", "Ao eliminar este feed, serão também eliminados todos os artigos associados. Continuar?");
    t.insert("feed_change_category", "Mudar de categoria");
    t.insert("import_select_category", "Categoria para os feeds importados:");

    // API configuration
    t.insert("api_provider", "Fornecedor");
    t.insert("api_key_field", "Chave API");
    t.insert("api_save_test", "Guardar e testar");
    t.insert("api_testing", "A testar...");
    t.insert("api_test_ok", "API a funcionar");
    t.insert("api_test_fail", "Erro durante o teste: ");
    t.insert("api_plan_info", "Informações do plano");
    t.insert("api_plan_loading", "A obter informações...");
    t.insert("api_plan_disclaimer", "Estas informações são geradas pelo modelo de IA e podem não ser precisas. Verifica-as sempre no site oficial do fornecedor de IA que estás a utilizar.");
    t.insert("api_delete", "Eliminar configuração da API");
    t.insert("api_delete_confirm", "Tens a certeza? A chave API será eliminada definitivamente e terás de a introduzir novamente para utilizar os resumos de IA.");

    // Settings
    t.insert("settings_retention", "Tempo de conservação");
    t.insert("settings_retention_hours", "horas");
    t.insert("settings_retention_days", "dias");
    t.insert("settings_and", "e");
    t.insert("settings_retention_hour", "hora");
    t.insert("settings_retention_day", "dia");
    t.insert("settings_auto_update", "Atualização automática");
    t.insert("settings_auto_update_interval", "Intervalo (horas)");
    t.insert("settings_auto_update_disabled", "Desativada");
    t.insert("settings_keywords", "Palavras-chave destacadas");
    t.insert("settings_keywords_desc", "Introduz as palavras ou expressões a destacar, separadas por ponto e vírgula (;)");
    t.insert("settings_keywords_placeholder", "Rust;IA;cibersegurança;...");

    // Confirm dialogs
    t.insert("confirm_delete_all_feeds",
        "Tens a certeza? Esta ação eliminará todos os feeds, incluindo os artigos associados, favoritos ou não, e é irreversível.");
    t.insert("delete_categories_too", "Eliminar também todas as categorias");
    t.insert("confirm_yes", "Confirmar");
    t.insert("confirm_no", "Cancelar");

    // Import/Export
    t.insert("import_success", "Feeds importados com sucesso");
    t.insert("import_count_feeds", "feeds importados");
    t.insert("import_count_favorites", "artigos favoritos importados");
    t.insert("export_success", "Feeds exportados com sucesso");
    t.insert("empty_articles", "Nenhum artigo encontrado. Tenta atualizar os feeds.");
    t.insert("no_internet_title", "Sem ligação");
    t.insert("no_internet_message", "Não é possível atualizar os feeds: verifica a tua ligação à internet e tenta novamente.");
    t.insert("empty_feeds", "Nenhum feed configurado.");
    t.insert("import_description", "Seleciona um ficheiro OPML ou um ficheiro JSON exportado a partir do MeigRSS.");
    t.insert("back_home", "Voltar ao início");
    t.insert("all_feeds_deleted", "Todos os feeds foram eliminados.");
    t.insert("import_confirm", "Foram encontrados {count} feeds no ficheiro. Alguns podem ser duplicados e, por isso, não serão importados. Continuar?");
    t.insert("import_favorites_count", "Incluindo {count} artigos favoritos.");
    t.insert("import_keep_original", "Manter a categoria original");
    t.insert("import_invalid_file", "O ficheiro selecionado é inválido ou não contém feeds.");
    t.insert("export_format_label", "Seleciona o formato de saída");
    t.insert("export_button", "Exportar");

    // Newsletter
    t.insert("menu_import_newsletter", "Importar newsletter");
    t.insert("newsletter_description", "Seleciona um ficheiro EML exportado da tua caixa de correio para importar uma newsletter como artigo.");
    t.insert("newsletter_title", "Título");
    t.insert("newsletter_imported", "Newsletter importada como artigo favorito.");
    t.insert("newsletter_no_html", "O ficheiro não contém conteúdo HTML.");
    t.insert("newsletter_duplicate", "Esta newsletter já foi importada.");
    t.insert("newsletter_invalid_file", "O ficheiro selecionado não é um ficheiro EML válido.");
    t.insert("link_newsletter", "Ver newsletter");
    t.insert("newsletter_choose_feed", "Escolhe um feed de newsletter");
    t.insert("newsletter_new_feed", "Criar novo feed");
    t.insert("newsletter_existing_feed", "Feed existente");
    t.insert("newsletter_feed_name", "Nome do feed");
    t.insert("newsletter_feed_name_placeholder", "ex. Giulio Magini");
    t.insert("newsletter_feed_name_empty", "O nome do feed não pode estar vazio");
    t.insert("newsletter_feed_exists", "Já existe um feed de newsletter com este nome");
    t.insert("newsletter_no_feeds", "Não existe nenhum feed de newsletter. Cria um novo.");
    t.insert("newsletter_mode_single", "Ficheiro único");
    t.insert("newsletter_mode_folder", "Pasta");
    t.insert("newsletter_folder_description", "Seleciona uma pasta: serão importados todos os ficheiros EML que contiver, ignorando qualquer outro tipo de ficheiro.");
    t.insert("newsletter_folder_selected", "{count} ficheiros EML prontos a importar.");
    t.insert("newsletter_folder_no_eml", "A pasta selecionada não contém ficheiros EML.");
    t.insert("newsletter_folder_too_many", "A pasta contém mais de {max} ficheiros EML. Seleciona uma com menos ficheiros.");
    t.insert("newsletter_folder_too_large", "Os ficheiros selecionados excedem o tamanho máximo permitido para um único envio.");
    t.insert("newsletter_bulk_imported", "{count} newsletters importadas como artigos favoritos.");
    t.insert("newsletter_bulk_none", "Nenhuma newsletter importada.");
    t.insert("newsletter_bulk_duplicates", "{count} já presentes no arquivo");
    t.insert("newsletter_bulk_no_html", "{count} sem conteúdo HTML");
    t.insert("newsletter_bulk_invalid", "{count} não reconhecidos como e-mails válidos");

    // Web feed
    t.insert("web_feed_toggle_rss", "Feed RSS");
    t.insert("web_feed_toggle_web", "Análise de página web");
    t.insert("web_feed_test", "Testar análise");
    t.insert("web_feed_testing", "A analisar...");
    t.insert("web_feed_test_ok", "Análise bem-sucedida");
    t.insert("web_feed_no_articles", "Não foi encontrado nenhum artigo na página. Verifica se o URL está correto e se a página contém ligações com formato de data.");
    t.insert("web_feed_articles_found", "artigos encontrados");
    t.insert("web_feed_add", "Adicionar web feed");
    t.insert("web_rendering_test_failed", "Teste da API de renderização falhou: ");
    t.insert("web_rendering_section_title", "API de renderização web");
    t.insert("web_rendering_provider", "Fornecedor");
    t.insert("web_rendering_save_test", "Guardar e testar");
    t.insert("web_rendering_testing", "A testar...");
    t.insert("web_rendering_test_ok", "API a funcionar");
    t.insert("web_rendering_test_fail", "Erro durante o teste: ");
    t.insert("web_rendering_delete", "Eliminar configuração de renderização");
    t.insert("web_rendering_delete_confirm", "Tens a certeza? A chave API será eliminada e os web feeds deixarão de poder ser atualizados.");
    t.insert("tooltip_rss_feed", "Feed RSS");
    t.insert("tooltip_web_feed", "Web feed (análise de página)");
    t.insert("tooltip_newsletter_feed", "Feed de newsletter");

    // General
    t.insert("loading", "A carregar...");
    t.insert("error", "Erro");
    t.insert("save", "Guardar");
    t.insert("cancel", "Cancelar");
    t.insert("close", "Fechar");

    t
}
