<!-- @ts-nocheck -->
<!-- Main page: Article feed list with filters, sorting, pagination -->
<script>
    import { onMount, getContext } from "svelte";
    import { apiGet, apiPost, apiPut, categoryDisplayName } from "$lib/api.js";
    import { t, getCurrentLang } from "$lib/i18n.svelte.js";
    import ArticleCard from "$lib/components/ArticleCard.svelte";

    const setLoading = getContext("setLoading");

    // Articles state
    let articles = $state([]);
    let totalCount = $state(0);
    let page = $state(1);
    let perPage = $state(10);
    let sortOrder = $state("feed_desc");
    let categoryFilter = $state(null);
    let favoritesFilter = $state("exclude");
    let searchQuery = $state("");

    let searchTimeout = null;

    function onSearchInput() {
        clearTimeout(searchTimeout);
        searchTimeout = setTimeout(() => {
            page = 1;
            loadArticles();
        }, 400);
    }

    // Categories for filter dropdown
    let categories = $state([]);

    // For understanding new articles and highlight them
    let lastRefreshTimestamp = $state("");

    // For highlighting keywords in the text
    let highlightKeywords = $state("");

    // Poll for auto-update changes every 30 seconds, only if auto update is enabled.
    // If the backend's last_refresh_timestamp changed, reload articles.
    let pollInterval = null;
    let autoUpdateEnabled = $state(false);

    async function pollForUpdates() {
        try {
            const res = await apiGet("/api/settings");
            if (res.success) {
                const newAutoUpdate = res.data.auto_update_enabled === "true";

                // If auto-update was just disabled, stop polling
                if (!newAutoUpdate && autoUpdateEnabled) {
                    autoUpdateEnabled = false;
                    clearInterval(pollInterval);
                    pollInterval = null;
                    return;
                }

                autoUpdateEnabled = newAutoUpdate;

                if (res.data.last_refresh_timestamp) {
                    const serverTs = res.data.last_refresh_timestamp;
                    if (lastRefreshTimestamp && serverTs !== lastRefreshTimestamp) {
                        lastRefreshTimestamp = serverTs;
                        highlightKeywords = res.data.highlight_keywords || "";
                        await loadArticles();
                    }
                }
            }
        } catch (e) {
            // Silently ignore polling errors
        }
    }

    function startPollingIfNeeded() {
        if (autoUpdateEnabled && !pollInterval) {
            pollInterval = setInterval(pollForUpdates, 30000);
        }
    }

    // Load articles from API
    async function loadArticles() {
        setLoading(true);
        try {
            let params = `?page=${page}&per_page=${perPage}&sort_order=${sortOrder}`;
            if (categoryFilter) params += `&category_id=${categoryFilter}`;
            if (favoritesFilter !== "all") params += `&favorites_filter=${favoritesFilter}`;
            if (searchQuery.trim()) params += `&search=${encodeURIComponent(searchQuery.trim())}`;

            const res = await apiGet(`/api/articles${params}`);
            if (res.success) {
                articles = res.data.articles;
                totalCount = res.data.total_count;
            }
        } catch (e) {
            console.error("Failed to load articles:", e);
        }
        setLoading(false);
    }

    // Load categories for the filter
    async function loadCategories() {
        try {
            const res = await apiGet("/api/categories");
            if (res.success) categories = res.data;
        } catch (e) {
            console.error("Failed to load categories:", e);
        }
    }

    // Load the timestamp for new articles
    async function loadRefreshTimestamp() {
        try {
            const res = await apiGet("/api/settings");
            if (res.success) {
                lastRefreshTimestamp = res.data.last_refresh_timestamp || "";
                highlightKeywords = res.data.highlight_keywords || "";
                autoUpdateEnabled = res.data.auto_update_enabled === "true";
                startPollingIfNeeded();
            }
        } catch (e) {
            console.error("Failed to load refresh timestamp:", e);
        }
    }

    // Mark an article as read
    async function markAsRead(articleId) {
        // Update local state immediately
        articles = articles.map(a =>
            a.id === articleId ? { ...a, is_read: true } : a
        );
        // Persist to DB in background (errors are silent)
        try {
            await apiPut(`/api/articles/${articleId}/read`);
        } catch (e) {
            console.error("Failed to mark article as read:", e);
        }
    }

    // Refresh feeds (manual trigger)
    async function refreshFeeds() {
        setLoading(true);
        connectionError = false;
        try {
            const res = await apiPost("/api/feeds/refresh");
            if (res.success && res.data) {
                // Check for internet connectivity error
                const noInternet = res.data.some(r => r.error === "no_internet");
                if (noInternet) {
                    connectionError = true;
                } else {
                    const newlyPaused = res.data.filter(r => r.auto_paused);
                    if (newlyPaused.length > 0) {
                        pausedFeeds = newlyPaused;
                        showPausedPopup = true;
                    }
                }
            }
            await loadArticles();
            await loadRefreshTimestamp();
        } catch (e) {
            console.error("Failed to refresh feeds:", e);
            connectionError = true;
        }
        setLoading(false);
    }

    // Toggle favorite
    async function toggleFavorite(articleId, currentState) {
        try {
            await apiPut(`/api/articles/${articleId}/favorite`, {
                is_favorite: !currentState
            });
            // Update local state
            articles = articles.map(a =>
                a.id === articleId ? { ...a, is_favorite: !currentState } : a
            );
        } catch (e) {
            console.error("Failed to toggle favorite:", e);
        }
    }

    // Generate AI summary for an article
    async function generateSummary(articleId) {
        try {
            const lang = getCurrentLang() || "en";
            const res = await apiPost(`/api/articles/${articleId}/summary?lang=${lang}`);
            if (res.success) {
                articles = articles.map(a =>
                    a.id === articleId
                        ? { ...a, summary: res.data.summary, summary_source: "llm" }
                        : a
                );
            }
            return res;
        } catch (e) {
            console.error("Failed to generate summary:", e);
            return { success: false, error: e.message };
        }
    }

    // Pagination
    const totalPages = $derived(Math.ceil(totalCount / perPage));

    function goToPage(p) {
        page = p;
        loadArticles();
    }

    // React to filter/sort changes
    function applyFilters() {
        page = 1;
        loadArticles();
    }

    // Debounce flag to prevent multiple simultaneous refresh calls
    let isRefreshing = $state(false);

    let pausedFeeds = $state([]);
    let showPausedPopup = $state(false);
    let connectionError = $state(false);

    async function onRefresh() {
        if (isRefreshing) return;
        isRefreshing = true;
        await refreshFeeds();
        isRefreshing = false;
    }

    onMount(async () => {
        await loadRefreshTimestamp();
        await loadCategories();
        await loadArticles();

        // Start polling only if auto-update is enabled
        startPollingIfNeeded();

        return () => {
            if (pollInterval) clearInterval(pollInterval);
        };
    });
</script>

<div class="feed-page">
    <!-- Toolbar: filters, sorting, pagination -->
    <div class="toolbar">
        <div class="toolbar-left">

            <!-- Sort order -->
            <select class="control" value={sortOrder} onchange={(e) => { sortOrder = e.target.value; applyFilters(); }}>
                <option value="desc">{t('sort_newest')}</option>
                <option value="asc">{t('sort_oldest')}</option>
                <option value="feed_desc">{t('sort_feed_newest')}</option>
                <option value="feed_asc">{t('sort_feed_oldest')}</option>
            </select>

            <!-- Category filter -->
            <select class="control" value={categoryFilter ?? ''} onchange={(e) => { categoryFilter = e.target.value ? parseInt(e.target.value) : null; applyFilters(); }}>
                <option value="">{t('filter_all')}</option>
                {#each categories as cat}
                    <option value={cat.id}>{categoryDisplayName(cat, t)}</option>
                {/each}
            </select>

            <!-- Favorites selection -->
            <select class="control" value={favoritesFilter} onchange={(e) => { favoritesFilter = e.target.value; applyFilters(); }}>
                <option value="exclude">🚫 {t('filter_favorites_exclude')}</option>
                <option value="all">☆ {t('filter_favorites_all')}</option>
                <option value="only">★ {t('filter_favorites_only')}</option>
            </select>

            <!-- Search box for keywords -->
            <div class="search-box">
                <span class="material-icons search-icon">search</span>
                <input
                    type="text"
                    bind:value={searchQuery}
                    oninput={onSearchInput}
                    placeholder={t('search_placeholder')}
                    class="search-input"
                />
                {#if searchQuery}
                    <button class="search-clear" onclick={() => { searchQuery = ""; page = 1; loadArticles(); }}>
                        <span class="material-icons">close</span>
                    </button>
                {/if}
            </div>
        </div>

        <div class="toolbar-right">

            <!-- Refresh feeds button -->
            <button class="refresh-btn" onclick={onRefresh} disabled={isRefreshing}>
                <span class="material-icons" class:spinning={isRefreshing}>refresh</span>
                {t('menu_refresh')}
            </button>

            <!-- Per page -->
            <select class="control" value={perPage} onchange={(e) => { perPage = parseInt(e.target.value); applyFilters(); }}>
                <option value={10}>10 {t('pagination_per_page')}</option>
                <option value={50}>50 {t('pagination_per_page')}</option>
                <option value={100}>100 {t('pagination_per_page')}</option>
            </select>

            <span class="count">{totalCount} {t('menu_feeds').toLowerCase()}</span>
        </div>
    </div>

    {#if connectionError}
        <div class="connection-error">
            <span class="material-icons">wifi_off</span>
            <div>
                <p class="error-title">{t('no_internet_title')}</p>
                <p class="error-desc">{t('no_internet_message')}</p>
            </div>
        </div>
    {/if}

    <!-- Article list -->
    {#if articles.length === 0}
        <div class="empty-state">
            <span class="material-icons">{searchQuery ? 'search_off' : 'inbox'}</span>
            <p>{searchQuery ? t('search_no_results') : t('empty_articles')}</p>
        </div>
    {:else}
        <div class="article-list">
            {#each articles as _article, i (_article.id)}
                {#if (sortOrder === 'feed_desc' || sortOrder === 'feed_asc') && i > 0 && articles[i - 1].feed_id !== articles[i].feed_id}
                    <div class="feed-separator">
                        <span class="separator-line"></span>
                    </div>
                {/if}
                <ArticleCard
                    bind:article={articles[i]}
                    {lastRefreshTimestamp}
                    {highlightKeywords}
                    {searchQuery}
                    onMarkAsRead={markAsRead}
                    onToggleFavorite={toggleFavorite}
                    onGenerateSummary={generateSummary}
                />
            {/each}
        </div>
    {/if}

    <!-- Pagination -->
    {#if totalPages > 1}
        <nav class="pagination">
            <button class="page-btn" disabled={page <= 1} onclick={() => goToPage(1)}>
                <span class="material-icons">first_page</span>
            </button>

            <button class="page-btn" disabled={page <= 1} onclick={() => goToPage(page - 1)}>
                <span class="material-icons">chevron_left</span>
            </button>

            {#each Array.from({length: Math.min(totalPages, 15)}, (_, i) => {
                if (totalPages <= 15) return i + 1;
                if (page <= 8) return i + 1;
                if (page >= totalPages - 7) return totalPages - 14 + i;
                return page - 7 + i;
            }) as p}
                <button class="page-btn" class:active={p === page} onclick={() => goToPage(p)}>{p}</button>
            {/each}

            <button class="page-btn" disabled={page >= totalPages} onclick={() => goToPage(page + 1)}>
                <span class="material-icons">chevron_right</span>
            </button>

            <button class="page-btn" disabled={page >= totalPages} onclick={() => goToPage(totalPages)}>
                <span class="material-icons">last_page</span>
            </button>
        </nav>
    {/if}

    <!-- Paused feeds popup -->
    {#if showPausedPopup}
        <div class="modal-backdrop" onclick={() => showPausedPopup = false} role="presentation">
            <div class="modal-dialog" onclick={(e) => e.stopPropagation()} onkeydown={(e) => { if (e.key === 'Escape') showPausedPopup = false; }} role="dialog" aria-modal="true" tabindex="-1">
                <div class="modal-header">
                    <span class="material-icons modal-icon">warning</span>
                    <h3>{t('feeds_auto_paused_title')}</h3>
                </div>
                <p class="modal-text">{t('feeds_auto_paused_message')}</p>
                <ul class="paused-list">
                    {#each pausedFeeds as feed}
                        <li>{feed.feed_name}</li>
                    {/each}
                </ul>
                <p class="modal-hint">{t('feeds_auto_paused_hint')}</p>
                <div class="modal-actions">
                    <button class="modal-ok-btn" onclick={() => showPausedPopup = false}>
                        {t('close')}
                    </button>
                </div>
            </div>
        </div>
    {/if}
</div>

<style>
    .feed-page {
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }

    /* ─── Toolbar ────────────────────────────────────────────────── */
    .toolbar {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 0.75rem 1rem;
        background: #fff;
        border: 1px solid #e0e0e0;
        border-radius: 8px;
        flex-wrap: wrap;
        gap: 0.5rem;
    }

    .toolbar-left, .toolbar-right {
        display: flex;
        align-items: center;
        gap: 0.65rem;
    }

    .control {
        padding: 0.35rem 0.6rem;
        border: 1px solid #d0d0d0;
        border-radius: 5px;
        font-size: 0.82rem;
        font-family: inherit;
        background: #fafafa;
        color: #333;
        cursor: pointer;
    }

    .control:hover {
        border-color: #999;
    }

    .search-box {
        display: flex;
        align-items: center;
        position: relative;
    }

    .search-icon {
        position: absolute;
        left: 0.5rem;
        font-size: 1rem;
        color: #999;
        pointer-events: none;
    }

    .search-input {
        padding: 0.35rem 0.6rem 0.35rem 2rem;
        border: 1px solid #d0d0d0;
        border-radius: 5px;
        font-size: 0.82rem;
        font-family: inherit;
        background: #fafafa;
        color: #333;
        width: 200px;
        transition: border-color 0.15s, width 0.2s;
    }

    .search-input:focus {
        border-color: #ff6b35;
        outline: none;
        width: 280px;
    }

    .search-input::placeholder {
        color: #bbb;
    }

    .search-clear {
        position: absolute;
        right: 0.3rem;
        background: none;
        border: none;
        cursor: pointer;
        color: #bbb;
        display: flex;
        padding: 0.15rem;
    }

    .search-clear:hover {
        color: #666;
    }

    .search-clear .material-icons {
        font-size: 0.95rem;
    }

    .count {
        font-size: 0.78rem;
        color: #888;
        font-family: 'IBM Plex Mono', monospace;
    }

    .refresh-btn {
        display: flex;
        align-items: center;
        gap: 0.35rem;
        padding: 0.35rem 0.75rem;
        background: #1a1a1a;
        color: #fff;
        border: none;
        border-radius: 5px;
        font-size: 0.82rem;
        font-family: inherit;
        cursor: pointer;
        transition: background 0.15s;
        white-space: nowrap;
    }

    .refresh-btn:hover:not(:disabled) { background: #333; }
    .refresh-btn:disabled { opacity: 0.55; cursor: not-allowed; }
    .refresh-btn .material-icons { font-size: 1.1rem; }

    @keyframes spin {
        from { transform: rotate(0deg); }
        to { transform: rotate(360deg); }
    }

    .spinning {
        animation: spin 1s linear infinite;
    }

    /* ─── Article List ───────────────────────────────────────────── */
    .article-list {
        display: flex;
        flex-direction: column;
        gap: 0;
    }

    .feed-separator {
        display: flex;
        align-items: center;
        padding: 0.6rem 0;
    }

    .separator-line {
        flex: 1;
        height: 2px;
        background: #ff6b35;
        border-radius: 1px;
        opacity: 0.4;
    }

    /* ─── Empty State ────────────────────────────────────────────── */
    .empty-state {
        text-align: center;
        padding: 4rem 2rem;
        color: #999;
    }

    .empty-state .material-icons {
        font-size: 3rem;
        margin-bottom: 0.75rem;
        opacity: 0.4;
    }

    .empty-state p {
        font-size: 0.9rem;
    }

    /* ─── Pagination ─────────────────────────────────────────────── */
    .pagination {
        display: flex;
        justify-content: center;
        gap: 0.25rem;
        padding: 1rem 0;
    }

    .page-btn {
        display: flex;
        align-items: center;
        justify-content: center;
        min-width: 34px;
        height: 34px;
        border: 1px solid #d0d0d0;
        border-radius: 5px;
        background: #fff;
        color: #555;
        font-size: 0.82rem;
        font-family: inherit;
        cursor: pointer;
        transition: all 0.12s;
    }

    .page-btn:hover:not(:disabled) {
        border-color: #ff6b35;
        color: #ff6b35;
    }

    .page-btn.active {
        background: #ff6b35;
        border-color: #ff6b35;
        color: #fff;
    }

    .page-btn:disabled {
        opacity: 0.35;
        cursor: not-allowed;
    }

    .page-btn .material-icons {
        font-size: 1.1rem;
    }

    .modal-backdrop {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.4);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 1000;
    }

    .modal-dialog {
        background: #fff;
        border-radius: 10px;
        padding: 1.5rem;
        max-width: 460px;
        width: 90%;
        box-shadow: 0 8px 32px rgba(0, 0, 0, 0.18);
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
    }

    .modal-header {
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }

    .modal-header h3 {
        font-size: 1rem;
        font-weight: 600;
        color: #1a1a1a;
        margin: 0;
    }

    .modal-icon {
        font-size: 1.3rem;
        color: #e67e22;
    }

    .modal-text {
        font-size: 0.88rem;
        color: #555;
        line-height: 1.5;
        margin: 0;
    }

    .paused-list {
        margin: 0;
        padding-left: 1.5rem;
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
    }

    .paused-list li {
        font-size: 0.85rem;
        font-weight: 500;
        color: #1a1a1a;
    }

    .modal-hint {
        font-size: 0.78rem;
        color: #888;
        font-style: italic;
        margin: 0;
    }

    .modal-actions {
        display: flex;
        justify-content: flex-end;
    }

    .modal-ok-btn {
        padding: 0.4rem 1.25rem;
        background: #1a1a1a;
        color: #fff;
        border: none;
        border-radius: 5px;
        font-size: 0.82rem;
        font-family: inherit;
        cursor: pointer;
    }

    .modal-ok-btn:hover {
        background: #333;
    }

    .connection-error {
        display: flex;
        align-items: flex-start;
        gap: 0.75rem;
        padding: 1rem 1.25rem;
        background: #fef0f0;
        border: 1px solid #f0c0c0;
        border-radius: 8px;
    }

    .connection-error .material-icons {
        font-size: 1.5rem;
        color: #dc3545;
        flex-shrink: 0;
        margin-top: 0.1rem;
    }

    .error-title {
        font-size: 0.9rem;
        font-weight: 600;
        color: #333;
        margin: 0 0 0.25rem 0;
    }

    .error-desc {
        font-size: 0.82rem;
        color: #666;
        margin: 0;
        line-height: 1.5;
    }
</style>
