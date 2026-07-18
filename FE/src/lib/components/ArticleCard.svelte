<!-- @ts-nocheck -->
<!-- ArticleCard: Single article in the feed list.
     Expandable: click to show summary, full text, and action buttons. -->
<script>
    import { sanitize, stripHtml, apiPost, categoryNameDisplay } from "$lib/api.js";
    import { t } from "$lib/i18n.svelte.js";

    let { article = $bindable(), lastRefreshTimestamp, highlightKeywords, searchQuery, onMarkAsRead, onToggleFavorite, onGenerateSummary } = $props();

    // Article is "new" if it was fetched during or after the last refresh, or if it is already read, so we are returning to the page
    const isNew = $derived(
        !article.is_read && lastRefreshTimestamp && article.fetched_at >= lastRefreshTimestamp
    );

    // Check if the article title matches the search query as a whole word (Unicode-aware)
    const matchesSearch = $derived(() => {
        if (!searchQuery || !searchQuery.trim()) return true;
        const escaped = searchQuery.trim().replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
        try {
            const regex = new RegExp(`(?<![\\p{L}\\p{N}])${escaped}(?![\\p{L}\\p{N}])`, 'iu');
            return regex.test(article.title);
        } catch {
            return true;
        }
    });

    let expanded = $state(false);
    let showFullText = $state(false);
    let summaryLoading = $state(false);
    let summaryError = $state("");
    let scrapeLoading = $state(false);
    let scrapeError = $state("");

    // Summary source label
    const sourceLabel = $derived(
        article.summary_source === "llm" ? t('summary_from_ai')
        : article.summary_source === "feed" ? t('summary_from_feed')
        : ""
    );

    // Disable AI summary button while scraping, if already generated, or if scraping is disabled (if is not newsletter)
    const canGenerateSummary = $derived(
        article.summary_source !== "llm" && !summaryLoading && !scrapeLoading && !!article.full_text && (article.enable_scraping || !!article.newsletter_html)
    );

    // AI summary tooltip
    const summaryTooltip = $derived(
        article.summary_source === "llm" ? t('btn_ai_summary_done')
        : t('btn_ai_summary')
    );

    async function handleGenerateSummary() {
        if (!canGenerateSummary) return;
        summaryLoading = true;
        summaryError = "";
        const result = await onGenerateSummary(article.id);
        if (!result?.success) {
            summaryError = result?.error || t('error');
        }
        summaryLoading = false;
    }

    function handleFavorite(e) {
        e.stopPropagation();
        onToggleFavorite(article.id, article.is_favorite);
    }

    // Scrape full text on demand when the card is expanded
    async function scrapeFullText() {
        if (article.full_text || scrapeLoading || !article.enable_scraping || article.newsletter_html) return;
        scrapeLoading = true;
        scrapeError = "";
        try {
            const res = await apiPost(`/api/articles/${article.id}/scrape`);
            if (res.success) {
                article.full_text = res.data;
                // If force AI summary is enabled, generate it automatically
                await autoGenerateSummaryIfNeeded();
            } else {
                // Translate error key if it matches an i18n key, otherwise show raw message
                const errKey = res.error || "";
                if (errKey.startsWith("scrape_failed:")) {
                    scrapeError = t('scrape_failed') + errKey.split(":").slice(1).join(":");
                } else {
                    scrapeError = t(errKey) || errKey;
                }
            }
        } catch (e) {
            const errMsg = e.message || "";
            if (errMsg.startsWith("scrape_failed:")) {
                scrapeError = t('scrape_failed') + errMsg.split(":").slice(1).join(":");
            } else {
                scrapeError = t(errMsg) || errMsg;
            }
        }
        scrapeLoading = false;
    }

    // Automatically generate AI summary when scraping completes,
    // if the feed has force_ai_summary enabled
    async function autoGenerateSummaryIfNeeded() {
        if (article.force_ai_summary && article.summary_source !== "llm") {
            summaryLoading = true;
            summaryError = "";
            const result = await onGenerateSummary(article.id);
            if (!result?.success) {
                summaryError = result?.error || t('error');
            }
            summaryLoading = false;
        }
    }

    // Format date for display
    function formatDate(dateStr) {
        if (!dateStr) return "";
        try {
            const d = new Date(dateStr);
            return d.toLocaleDateString(undefined, {
                day: '2-digit', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit'
            });
        } catch {
            return dateStr;
        }
    }

    // Parses the keyword string and returns an array of trimmed, non-empty keywords
    function parseKeywords(kwString) {
        if (!kwString) return [];
        return kwString.split(';').map(k => k.trim()).filter(k => k.length > 0);
    }

    // Highlights keywords in text by wrapping matches in <mark> tags.
    // The text is first sanitized in text-only mode (all tags stripped via
    // DOMPurify) so it is safe to render with {@html} afterwards.
    function highlightText(text, kwString) {
        if (!text) return "";
        const clean = sanitize({ input: text, isHTML: true, allowedTags: [], allowedAttrs: [] });
        const keywords = parseKeywords(kwString);
        if (keywords.length === 0) return clean;

        // Escape special regex characters in keywords
        const escaped = keywords.map(k =>
            k.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
        );
        const regex = new RegExp(`(?<![\\p{L}\\p{N}])(${escaped.join('|')})(?![\\p{L}\\p{N}])`, 'giu');
        return clean.replace(regex, '<mark class="kw-highlight">$1</mark>');
    }

</script>

<article class="card" class:expanded class:is-new={isNew} class:search-dimmed={!matchesSearch()}>
    <!-- Header row: always visible -->
    <div class="card-header" role="button" tabindex="0" onclick={() => { expanded = !expanded; if (expanded) { onMarkAsRead(article.id); scrapeFullText(); } }} onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); expanded = !expanded; if (expanded) { onMarkAsRead(article.id); scrapeFullText(); } } }}>
        <div class="header-left">
            <span class="site-badge">{sanitize({ input: article.site_name })}</span>
            <span class="category-badge">{sanitize({ input: categoryNameDisplay(article.category_name, t) })}</span>
        </div>

        <h3 class="title">{@html highlightText(article.title, highlightKeywords)}</h3>

        <div class="header-right">
            <span class="date">{formatDate(article.published_at)}</span>

            <button class="fav-btn" title={article.is_favorite ? t('tooltip_remove_favorite') : t('tooltip_add_favorite')}
                onclick={handleFavorite}>
                <span class="material-icons">{article.is_favorite ? 'star' : 'star_border'}</span>
            </button>

            <span class="material-icons expand-icon">{expanded ? 'expand_less' : 'expand_more'}</span>
        </div>
    </div>

    <!-- Expanded content -->
    {#if expanded}
        <div class="card-body">
            <!-- Summary -->
            {#if article.summary}
                <div class="summary-section">
                    {#if sourceLabel}
                        <span class="source-tag">{sourceLabel}</span>
                    {/if}
                    <p class="summary-text">{@html highlightText(stripHtml(article.summary), highlightKeywords)}</p>
                </div>
                {#if article.summary_source === "llm"}
                    <p class="ai-disclaimer">
                        <span class="material-icons">info</span>
                        {t('ai_summary_disclaimer')}
                    </p>
                {/if}
            {:else}
                <p class="no-content">{t('no_summary')}</p>
            {/if}

            <!-- Error message -->
            {#if summaryError}
                <p class="error-msg">{summaryError}</p>
            {/if}

            {#if scrapeError}
                <p class="error-msg">{scrapeError}</p>
            {/if}

            <!-- Action buttons -->
            <div class="actions">
                <!-- AI Summary button -->
                <button class="action-btn" class:disabled={!canGenerateSummary}
                    title={!article.enable_scraping ? t('scraping_disabled') : summaryTooltip}
                    disabled={!canGenerateSummary}
                    onclick={handleGenerateSummary}>
                    <span class="material-icons">{summaryLoading || scrapeLoading ? 'hourglass_empty' : 'auto_awesome'}</span>
                    {#if scrapeLoading}
                        {t('loading')}
                    {:else if summaryLoading}
                        {t('loading')}
                    {:else}
                        {t('btn_ai_summary')}
                    {/if}
                </button>

                <!-- Show full text -->
                <button class="action-btn" class:disabled={!article.enable_scraping && !article.newsletter_html || (scrapeLoading && !article.full_text)}
                    disabled={(!article.enable_scraping && !article.newsletter_html) || (scrapeLoading && !article.full_text)}
                    title={!article.enable_scraping && !article.newsletter_html ? t('scraping_disabled') : ''}
                    onclick={() => showFullText = !showFullText}>
                    <span class="material-icons">{scrapeLoading && !article.full_text ? 'hourglass_empty' : (showFullText ? 'visibility_off' : 'visibility')}</span>
                    {#if scrapeLoading && !article.full_text}
                        {t('loading')}
                    {:else if showFullText}
                        {t('btn_hide_full_text')}
                    {:else}
                        {t('btn_show_full_text')}
                    {/if}
                </button>

                <!-- Link to original article or to the email html -->
                {#if article.newsletter_html}
                    <a class="action-btn" href={`/api/newsletter/${article.id}/html`} target="_blank" rel="noopener noreferrer">
                        <span class="material-icons">email</span>
                        {t('link_newsletter')}
                    </a>
                {:else}
                    <a class="action-btn" href={article.url} target="_blank" rel="noopener noreferrer">
                        <span class="material-icons">open_in_new</span>
                        {t('link_original')}
                    </a>
                {/if}
            </div>

            <!-- Full text (expandable) -->
            {#if showFullText}
                <div class="full-text">
                    {#if article.full_text}
                        <pre class="text-content">{@html highlightText(article.full_text, highlightKeywords)}</pre>
                    {:else}
                        <p class="no-content">{t('no_full_text')}</p>
                    {/if}
                </div>
            {/if}

            <!-- Author -->
            {#if article.author}
                <p class="author">— {sanitize({ input: article.author })}</p>
            {/if}
        </div>
    {/if}
</article>

<style>
    .card {
        background: #fff;
        border: 1px solid #e8e8e4;
        border-radius: 0;
        transition: border-color 0.15s;
    }

    .card:first-child {
        border-radius: 8px 8px 0 0;
    }

    .card:last-child {
        border-radius: 0 0 8px 8px;
    }

    .card:only-child {
        border-radius: 8px;
    }

    .card:hover {
        border-color: #ccc;
    }

    .card.is-new {
        border-left: 3px solid #ff6b35;
        background: #fff8f5;
    }

    .card.is-new .title {
        font-weight: 600;
    }

    .card.search-dimmed {
        opacity: 0.35;
    }

    .card.search-dimmed:hover {
        opacity: 0.6;
    }

    .card.expanded {
        border-color: #ff6b35;
        z-index: 1;
        position: relative;
    }

    /* ─── Header ─────────────────────────────────────────────────── */
    .card-header {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        padding: 0.7rem 1rem;
        cursor: pointer;
        user-select: none;
    }

    .header-left {
        display: flex;
        gap: 0.35rem;
        flex-shrink: 0;
    }

    .site-badge {
        font-size: 0.72rem;
        padding: 0.15rem 0.45rem;
        background: #1a1a1a;
        color: #e0e0e0;
        border-radius: 3px;
        font-weight: 500;
        white-space: nowrap;
        max-width: 140px;
        overflow: hidden;
        text-overflow: ellipsis;
    }

    .category-badge {
        font-size: 0.72rem;
        padding: 0.15rem 0.45rem;
        background: #e8f4e8;
        color: #2d7a2d;
        border-radius: 3px;
        font-weight: 500;
        white-space: nowrap;
    }

    .title {
        flex: 1;
        font-size: 0.88rem;
        font-weight: 500;
        color: #1a1a1a;
        line-height: 1.35;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .header-right {
        display: flex;
        align-items: center;
        gap: 0.4rem;
        flex-shrink: 0;
    }

    .date {
        font-size: 0.72rem;
        color: #999;
        font-family: 'IBM Plex Mono', monospace;
        white-space: nowrap;
    }

    .fav-btn {
        background: none;
        border: none;
        cursor: pointer;
        padding: 0.15rem;
        color: #ccc;
        transition: color 0.15s;
        display: flex;
    }

    .fav-btn:hover,
    .fav-btn .material-icons:not(:empty) {
        color: #f5a623;
    }

    .expand-icon {
        font-size: 1.2rem;
        color: #bbb;
        transition: color 0.15s;
    }

    /* ─── Body ───────────────────────────────────────────────────── */
    .card-body {
        padding: 0 1rem 1rem;
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
    }

    .summary-section {
        padding: 0.75rem;
        background: #fafaf6;
        border-left: 3px solid #ff6b35;
        border-radius: 0 4px 4px 0;
    }

    .source-tag {
        font-size: 0.68rem;
        font-weight: 600;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        color: #ff6b35;
        display: block;
        margin-bottom: 0.3rem;
    }

    .summary-text {
        font-size: 0.88rem;
        line-height: 1.55;
        color: #333;
    }

    .ai-disclaimer {
        display: flex;
        align-items: flex-start;
        gap: 0.35rem;
        font-size: 0.72rem;
        color: #999;
        font-style: italic;
        margin: 0;
    }

    .ai-disclaimer .material-icons {
        font-size: 0.85rem;
        flex-shrink: 0;
        margin-top: 0.05rem;
    }

    .error-msg {
        font-size: 0.8rem;
        color: #dc3545;
        padding: 0.4rem 0.6rem;
        background: #fef0f0;
        border-radius: 4px;
    }

    /* ─── Actions ────────────────────────────────────────────────── */
    .actions {
        display: flex;
        gap: 0.5rem;
        flex-wrap: wrap;
    }

    .action-btn {
        display: flex;
        align-items: center;
        gap: 0.3rem;
        padding: 0.35rem 0.7rem;
        font-size: 0.78rem;
        font-family: inherit;
        border: 1px solid #d8d8d4;
        border-radius: 5px;
        background: #fff;
        color: #555;
        cursor: pointer;
        text-decoration: none;
        transition: all 0.12s;
    }

    .action-btn:hover:not(.disabled) {
        border-color: #ff6b35;
        color: #ff6b35;
    }

    .action-btn.disabled {
        opacity: 0.45;
        cursor: not-allowed;
    }

    .action-btn .material-icons {
        font-size: 1rem;
    }

    @keyframes spin {
        from { transform: rotate(0deg); }
        to { transform: rotate(360deg); }
    }

    .action-btn .material-icons:global(.hourglass_empty) {
        animation: spin 1.2s linear infinite;
    }

    /* ─── Full Text ──────────────────────────────────────────────── */
    .full-text {
        border: 1px solid #e8e8e4;
        border-radius: 6px;
        padding: 1rem;
        background: #fcfcfa;
        max-height: 500px;
        overflow-y: auto;
    }

    .text-content {
        font-size: 0.85rem;
        line-height: 1.65;
        color: #333;
        white-space: pre-wrap;
        word-wrap: break-word;
        font-family: 'IBM Plex Sans', sans-serif;
    }

    .no-content {
        font-size: 0.85rem;
        color: #999;
        font-style: italic;
    }

    .author {
        font-size: 0.78rem;
        color: #888;
        font-style: italic;
    }

    :global(.kw-highlight) {
        background: #fff3cd;
        color: #664d03;
        padding: 0.05rem 0.15rem;
        border-radius: 2px;
        font-weight: inherit;
    }
</style>