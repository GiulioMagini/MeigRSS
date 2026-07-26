<!-- @ts-nocheck -->
<!-- Feed management page: add/remove feeds, test URLs, toggle AI summary, change category -->
<script>
    import { onMount, getContext } from "svelte";
    import {
        apiGet,
        apiPost,
        apiDelete,
        apiPut,
        sanitize,
        categoryDisplayName,
    } from "$lib/api.js";
    import { t } from "$lib/i18n.svelte.js";

    const setLoading = getContext("setLoading");

    let feeds = $state([]);
    let categories = $state([]);
    // Guards the empty state: the message must not appear before the first
    // load has actually resolved
    let feedsLoaded = $state(false);

    // New feed form
    let newUrl = $state("");
    let newCategory = $state(null);
    let testStatus = $state(null); // null | "loading" | "ok" | "error"
    let testedTitle = $state("");
    let testedSiteName = $state("");
    let testError = $state("");

    // Web feed form state
    let feedMode = $state("rss"); // "rss" or "web"
    let webTestStatus = $state(null); // null | "loading" | "ok" | "error"
    let webTestedPattern = $state("");
    let webArticlesFound = $state(0);
    // Mirrors what the successful test used, and stays editable before adding
    let webUseApi = $state(false);
    let webTestError = $state("");

    // Delete confirmation modal
    let showDeleteModal = $state(false);
    let feedToDelete = $state(null);

    async function loadFeeds() {
        try {
            const res = await apiGet("/api/feeds");
            if (res.success) feeds = res.data;
        } catch (e) {
            console.error(e);
        } finally {
            feedsLoaded = true;
        }
    }

    async function loadCategories() {
        try {
            const res = await apiGet("/api/categories");
            if (res.success) {
                categories = res.data;
                if (!newCategory && categories.length > 0) {
                    newCategory = categories[0].id;
                }
            }
        } catch (e) {
            console.error(e);
        }
    }

    async function testWebFeed() {
        if (!newUrl.trim()) return;
        webTestStatus = "loading";
        webTestError = "";
        try {
            const res = await apiPost("/api/web-feed/test", {
                url: newUrl.trim(),
            });
            if (res.success) {
                webTestedPattern = res.data.pattern;
                webArticlesFound = res.data.articles_found;
                webUseApi = res.data.is_api_scraping;
                webTestStatus = "ok";
            }
        } catch (e) {
            webTestStatus = "error";
            const msg = e.message || "";
            if (msg === "web_feed_no_articles") {
                webTestError = t("web_feed_no_articles");
            } else if (msg.startsWith("web_rendering_test_failed:")) {
                webTestError =
                    t("web_rendering_test_fail") +
                    msg.split(":").slice(1).join(":");
            } else {
                webTestError = msg;
            }
        }
    }

    async function addWebFeed() {
        if (webTestStatus !== "ok" || !newCategory) return;
        setLoading(true);
        try {
            // Use domain as site_name
            let siteName = newUrl.trim();
            try {
                siteName = new URL(newUrl.trim()).hostname.replace(/^www\./, '');
            } catch {
                /* keep full URL as fallback */
            }

            await apiPost("/api/web-feed", {
                url: newUrl.trim(),
                site_name: siteName,
                category_id: parseInt(newCategory),
                pattern: webTestedPattern,
                is_api_scraping: webUseApi,
            });
            newUrl = "";
            webTestStatus = null;
            webTestedPattern = "";
            webArticlesFound = 0;
            webUseApi = false;
            await loadFeeds();
        } catch (e) {
            const msg = e.message || "";
            if (msg.includes("UNIQUE") || msg.includes("feed_already_exists")) {
                webTestStatus = "error";
                webTestError = t("feed_already_exists");
            } else {
                webTestStatus = "error";
                webTestError = msg;
            }
        }
        setLoading(false);
    }

    async function testFeed() {
        if (!newUrl.trim()) return;
        testStatus = "loading";
        testError = "";
        try {
            const res = await apiPost("/api/feeds/test", {
                url: newUrl.trim(),
            });
            if (res.success) {
                testedTitle = res.data.title;
                testedSiteName = res.data.site_name;
                testStatus = "ok";
            }
        } catch (e) {
            testStatus = "error";
            testError = e.message;
        }
    }

    async function addFeed() {
        if (testStatus !== "ok" || !newCategory) return;
        setLoading(true);
        try {
            await apiPost("/api/feeds", {
                url: newUrl.trim(),
                title: testedTitle,
                site_name: testedSiteName,
                category_id: parseInt(newCategory),
            });
            newUrl = "";
            feedMode = "rss";
            testStatus = null;
            testedTitle = "";
            testedSiteName = "";
            await loadFeeds();
        } catch (e) {
            const msg = e.message || "";
            if (msg.includes("UNIQUE")) {
                testStatus = "error";
                testError = t("feed_already_exists");
            } else {
                testStatus = "error";
                testError = msg;
            }
        }
        setLoading(false);
    }

    // Opens the delete confirmation modal
    function requestDeleteFeed(feed) {
        feedToDelete = feed;
        showDeleteModal = true;
    }

    // Confirms the deletion
    async function confirmDeleteFeed() {
        if (!feedToDelete) return;
        try {
            await apiDelete(`/api/feeds/${feedToDelete.id}`);
            feeds = feeds.filter((f) => f.id !== feedToDelete.id);
        } catch (e) {
            console.error(e);
        }
        showDeleteModal = false;
        feedToDelete = null;
    }

    // Cancels the deletion
    function cancelDeleteFeed() {
        showDeleteModal = false;
        feedToDelete = null;
    }

    async function toggleForceAi(feed) {
        // Cannot enable force AI if scraping is disabled
        if (!feed.enable_scraping && !feed.force_ai_summary) return;
        try {
            await apiPut(`/api/feeds/${feed.id}/force-ai`, {
                force: !feed.force_ai_summary,
            });
            feeds = feeds.map((f) =>
                f.id === feed.id
                    ? { ...f, force_ai_summary: !f.force_ai_summary }
                    : f,
            );
        } catch (e) {
            console.error(e);
        }
    }

    async function toggleScraping(feed) {
        try {
            const newValue = !feed.enable_scraping;
            await apiPut(`/api/feeds/${feed.id}/scraping`, {
                enable: newValue,
            });
            // If disabling scraping, also disable force AI locally
            if (!newValue) {
                feeds = feeds.map((f) =>
                    f.id === feed.id
                        ? {
                              ...f,
                              enable_scraping: false,
                              force_ai_summary: false,
                          }
                        : f,
                );
            } else {
                feeds = feeds.map((f) =>
                    f.id === feed.id ? { ...f, enable_scraping: true } : f,
                );
            }
        } catch (e) {
            console.error(e);
        }
    }

    async function toggleApiScraping(feed) {
        try {
            const newValue = !feed.is_api_scraping;
            await apiPut(`/api/feeds/${feed.id}/api-scraping`, {
                enable: newValue,
            });
            feeds = feeds.map((f) =>
                f.id === feed.id ? { ...f, is_api_scraping: newValue } : f,
            );
        } catch (e) {
            console.error(e);
        }
    }

    // Change the category of a feed
    async function changeFeedCategory(feed, newCategoryId) {
        try {
            await apiPut(`/api/feeds/${feed.id}/category`, {
                category_id: parseInt(newCategoryId),
            });
            feeds = feeds.map((f) =>
                f.id === feed.id
                    ? { ...f, category_id: parseInt(newCategoryId) }
                    : f,
            );
        } catch (e) {
            console.error(e);
        }
    }

    // Toggles pause on specific feed
    async function togglePause(feed) {
        try {
            await apiPut(`/api/feeds/${feed.id}/pause`, {
                paused: !feed.is_paused,
            });
            feeds = feeds.map((f) =>
                f.id === feed.id ? { ...f, is_paused: !f.is_paused } : f,
            );
        } catch (e) {
            console.error(e);
        }
    }

    // Helper: get category name by id
    function getCategoryName(catId) {
        const cat = categories.find((c) => c.id === catId);
        return cat ? categoryDisplayName(cat, t) : "—";
    }

    onMount(async () => {
        // Covers both calls, so the page never renders its empty state
        // before the feeds have arrived
        setLoading(true);
        await loadCategories();
        await loadFeeds();
        setLoading(false);
    });
</script>

<div class="feeds-page">
    <h2><span class="material-icons">dynamic_feed</span> {t("menu_feeds")}</h2>

    <!-- Add feed form -->
    <div class="add-card">
        <h3>{t("feed_add")}</h3>

        <!-- Feed type toggle -->
        <div class="feed-mode-toggle">
            <button
                class="mode-btn"
                class:active={feedMode === "rss"}
                onclick={() => {
                    feedMode = "rss";
                    testStatus = null;
                    webTestStatus = null;
                    newUrl = "";
                }}
            >
                <span class="material-icons">rss_feed</span>
                {t("web_feed_toggle_rss")}
            </button>
            <button
                class="mode-btn"
                class:active={feedMode === "web"}
                onclick={() => {
                    feedMode = "web";
                    testStatus = null;
                    webTestStatus = null;
                    newUrl = "";
                }}
            >
                <span class="material-icons">language</span>
                {t("web_feed_toggle_web")}
            </button>
        </div>

        {#if feedMode === "rss"}
            <div class="add-row">
                <input
                    type="url"
                    bind:value={newUrl}
                    placeholder="https://example.com/feed.xml"
                    class="input flex-1"
                />
                <button
                    class="secondary-btn"
                    onclick={testFeed}
                    disabled={!newUrl.trim() || testStatus === "loading"}
                >
                    <span class="material-icons"
                        >{testStatus === "loading"
                            ? "hourglass_empty"
                            : "wifi_tethering"}</span
                    >
                    {t("feed_test")}
                </button>
            </div>

            {#if testStatus === "ok"}
                <div class="test-ok">
                    <span class="material-icons">check_circle</span>
                    <span
                        ><strong>{testedTitle || testedSiteName}</strong> — {t(
                            "feed_test_ok",
                        )}</span
                    >
                </div>

                <div class="add-row">
                    <select bind:value={newCategory} class="input">
                        {#each categories as cat}
                            <option value={cat.id}
                                >{categoryDisplayName(cat, t)}</option
                            >
                        {/each}
                    </select>
                    <button class="primary-btn" onclick={addFeed}>
                        <span class="material-icons">add</span>
                        {t("feed_add")}
                    </button>
                </div>
            {:else if testStatus === "error"}
                <p class="error-msg">
                    <span class="material-icons">error</span>
                    {testError}
                </p>
            {/if}
        {/if}

        {#if feedMode === "web"}
            <div class="add-row">
                <input
                    type="url"
                    bind:value={newUrl}
                    placeholder="https://www.example.com/category/"
                    class="input flex-1"
                />
                <button
                    class="secondary-btn"
                    onclick={testWebFeed}
                    disabled={!newUrl.trim() || webTestStatus === "loading"}
                >
                    <span class="material-icons"
                        >{webTestStatus === "loading"
                            ? "hourglass_empty"
                            : "travel_explore"}</span
                    >
                    {webTestStatus === "loading"
                        ? t("web_feed_testing")
                        : t("web_feed_test")}
                </button>
            </div>

            {#if webTestStatus === "ok"}
                <div class="test-ok">
                    <span class="material-icons">check_circle</span>
                    <span
                        >{webArticlesFound}
                        {t("web_feed_articles_found")} — {t(
                            "web_feed_test_ok",
                        )}</span
                    >
                </div>

                <label class="api-scraping-row">
                    <input type="checkbox" bind:checked={webUseApi} />
                    {t("feed_api_scraping")}
                </label>

                <div class="add-row">
                    <select bind:value={newCategory} class="input">
                        {#each categories as cat}
                            <option value={cat.id}
                                >{categoryDisplayName(cat, t)}</option
                            >
                        {/each}
                    </select>
                    <button class="primary-btn" onclick={addWebFeed}>
                        <span class="material-icons">add</span>
                        {t("web_feed_add")}
                    </button>
                </div>
            {:else if webTestStatus === "error"}
                <p class="error-msg">
                    <span class="material-icons">error</span>
                    {webTestError}
                </p>
            {/if}
        {/if}
    </div>

    <div class="disclaimer">
        <span class="material-icons">info</span>
        <p>{t("scraping_disclaimer")}</p>
    </div>

    <!-- Feed list -->
    <div class="feed-list">
        {#each feeds as feed (feed.id)}
            <div class="feed-row">
                <div class="feed-info">
                    <div class="feed-name-row">
                        <span
                            class="feed-type-icon"
                            title={feed.is_web_feed
                                ? t("tooltip_web_feed")
                                : feed.url.startsWith("newsletter://")
                                  ? t("tooltip_newsletter_feed")
                                  : t("tooltip_rss_feed")}
                        >
                            <span class="material-icons"
                                >{feed.is_web_feed
                                    ? "language"
                                    : feed.url.startsWith("newsletter://")
                                      ? "email"
                                      : "rss_feed"}</span
                            >
                        </span>
                        <span
                            class="feed-name"
                            class:paused-name={feed.is_paused}
                            >{sanitize({
                                input: feed.site_name || feed.title,
                            })}</span
                        >
                        <span class="feed-category-badge"
                            >{getCategoryName(feed.category_id)}</span
                        >
                        {#if feed.is_paused && !feed.url.startsWith("newsletter://")}
                            <span class="paused-badge">{t("feed_paused")}</span>
                        {/if}
                    </div>
                    <span class="feed-url">{sanitize({ input: feed.url })}</span
                    >
                </div>

                <div class="feed-actions">
                    {#if !feed.url.startsWith("newsletter://") && !feed.is_web_feed}
                        <select
                            class="category-select"
                            value={feed.category_id}
                            title={t("feed_change_category")}
                            onchange={(e) =>
                                changeFeedCategory(feed, e.target.value)}
                        >
                            {#each categories as cat}
                                <option value={cat.id}
                                    >{categoryDisplayName(cat, t)}</option
                                >
                            {/each}
                        </select>

                        <button
                            class="feed-action-btn"
                            class:active={!feed.is_paused}
                            onclick={() => togglePause(feed)}
                        >
                            <span class="material-icons"
                                >{feed.is_paused
                                    ? "pause_circle"
                                    : "play_circle"}</span
                            >
                            {feed.is_paused
                                ? t("feed_paused")
                                : t("feed_active")}
                        </button>

                        <button
                            class="feed-action-btn"
                            class:active={feed.enable_scraping}
                            onclick={() => toggleScraping(feed)}
                        >
                            <span class="material-icons"
                                >{feed.enable_scraping
                                    ? "download_done"
                                    : "download"}</span
                            >
                            {t("feed_enable_scraping")}
                        </button>
                    {/if}

                    {#if feed.is_web_feed}
                        <select
                            class="category-select"
                            value={feed.category_id}
                            title={t('feed_change_category')}
                            onchange={(e) => changeFeedCategory(feed, e.target.value)}
                        >
                            {#each categories as cat}
                                <option value={cat.id}>{categoryDisplayName(cat, t)}</option>
                            {/each}
                        </select>

                        <button class="feed-action-btn" class:active={!feed.is_paused} onclick={() => togglePause(feed)}>
                            <span class="material-icons">{feed.is_paused ? 'pause_circle' : 'play_circle'}</span>
                            {feed.is_paused ? t('feed_paused') : t('feed_active')}
                        </button>

                        <button
                            class="feed-action-btn"
                            class:active={feed.is_api_scraping}
                            onclick={() => toggleApiScraping(feed)}
                        >
                            <span class="material-icons">{feed.is_api_scraping ? 'cloud_done' : 'cloud_off'}</span>
                            {t('feed_api_scraping')}
                        </button>
                    {/if}

                    <button
                        class="feed-action-btn"
                        class:active={feed.force_ai_summary}
                        class:disabled-btn={!feed.enable_scraping}
                        disabled={!feed.enable_scraping}
                        onclick={() => toggleForceAi(feed)}
                    >
                        <span class="material-icons"
                            >{feed.force_ai_summary
                                ? "auto_awesome"
                                : "auto_awesome_mosaic"}</span
                        >
                        {t("feed_force_ai")}
                    </button>

                    <button
                        class="feed-action-btn danger-btn"
                        onclick={() => requestDeleteFeed(feed)}
                    >
                        <span class="material-icons">delete</span>
                        {t("feed_delete")}
                    </button>
                </div>
            </div>
        {:else}
            {#if feedsLoaded}
                <p class="empty">{t("empty_feeds")}</p>
            {/if}
        {/each}
    </div>
</div>

<!-- Delete confirmation modal -->
{#if showDeleteModal}
    <div
        class="modal-backdrop"
        onclick={cancelDeleteFeed}
        onkeydown={(e) => {
            if (e.key === "Escape") cancelDeleteFeed();
        }}
        role="presentation"
    >
        <div
            class="modal-dialog"
            onclick={(e) => e.stopPropagation()}
            onkeydown={(e) => {
                if (e.key === "Escape") cancelDeleteFeed();
            }}
            role="dialog"
            aria-modal="true"
            tabindex="-1"
        >
            <div class="modal-header">
                <span class="material-icons modal-icon">warning</span>
                <h3>{t("feed_delete")}</h3>
            </div>
            <p class="modal-text">{t("confirm_delete_feed")}</p>
            {#if feedToDelete}
                <p class="modal-feed-name">
                    {sanitize({
                        input: feedToDelete.site_name || feedToDelete.title,
                    })}
                </p>
            {/if}
            <div class="modal-actions">
                <button class="modal-cancel-btn" onclick={cancelDeleteFeed}>
                    {t("confirm_no")}
                </button>
                <button class="modal-confirm-btn" onclick={confirmDeleteFeed}>
                    <span class="material-icons">delete</span>
                    {t("confirm_yes")}
                </button>
            </div>
        </div>
    </div>
{/if}

<style>
    .feeds-page {
        max-width: 1400px;
    }

    h2 {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        font-size: 1.15rem;
        font-weight: 600;
        margin-bottom: 1.25rem;
    }
    h2 .material-icons {
        font-size: 1.3rem;
        color: #ff6b35;
    }

    h3 {
        font-size: 0.92rem;
        font-weight: 600;
        margin-bottom: 0.75rem;
        color: #333;
    }

    .add-card {
        background: #fff;
        border: 1px solid #e0e0e0;
        border-radius: 8px;
        padding: 1.25rem;
        margin-bottom: 1.25rem;
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
    }

    .add-row {
        display: flex;
        gap: 0.5rem;
        align-items: center;
    }

    .input {
        padding: 0.4rem 0.6rem;
        border: 1px solid #d0d0d0;
        border-radius: 5px;
        font-size: 0.85rem;
        font-family: inherit;
        background: #fafafa;
    }
    .input:focus {
        border-color: #ff6b35;
        outline: none;
    }
    .flex-1 {
        flex: 1;
    }

    .test-ok {
        display: flex;
        align-items: center;
        gap: 0.4rem;
        padding: 0.5rem 0.75rem;
        background: #f0f7f0;
        border-radius: 5px;
        font-size: 0.82rem;
        color: #2d7a2d;
    }
    .test-ok .material-icons {
        font-size: 1rem;
    }

    .error-msg {
        display: flex;
        align-items: center;
        gap: 0.3rem;
        font-size: 0.8rem;
        color: #dc3545;
        padding: 0.4rem 0.6rem;
        background: #fef0f0;
        border-radius: 4px;
    }
    .error-msg .material-icons {
        font-size: 0.95rem;
    }

    .primary-btn,
    .secondary-btn {
        display: flex;
        align-items: center;
        gap: 0.35rem;
        padding: 0.4rem 0.9rem;
        border-radius: 5px;
        font-size: 0.82rem;
        font-family: inherit;
        cursor: pointer;
        white-space: nowrap;
        border: none;
    }
    .primary-btn {
        background: #1a1a1a;
        color: #fff;
    }
    .primary-btn:hover {
        background: #333;
    }
    .secondary-btn {
        background: #fff;
        color: #555;
        border: 1px solid #d0d0d0;
    }
    .secondary-btn:hover:not(:disabled) {
        border-color: #ff6b35;
        color: #ff6b35;
    }
    .secondary-btn:disabled,
    .primary-btn:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }
    .primary-btn .material-icons,
    .secondary-btn .material-icons {
        font-size: 1rem;
    }

    /* Feed list */
    .feed-list {
        display: flex;
        flex-direction: column;
    }

    .feed-row {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 0.75rem 1rem;
        background: #fff;
        border: 1px solid #e8e8e4;
        gap: 0.75rem;
        flex-wrap: wrap;
    }
    .feed-row + .feed-row {
        border-top: none;
    }
    .feed-row:first-child {
        border-radius: 8px 8px 0 0;
    }
    .feed-row:last-child {
        border-radius: 0 0 8px 8px;
    }
    .feed-row:only-child {
        border-radius: 8px;
    }

    .feed-info {
        display: flex;
        flex-direction: column;
        gap: 0.15rem;
        overflow: hidden;
        flex: 1;
        min-width: 0;
    }

    .feed-name-row {
        display: flex;
        align-items: center;
        gap: 0.45rem;
    }
    .feed-name {
        font-size: 0.88rem;
        font-weight: 500;
        color: #1a1a1a;
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
    }
    .feed-category-badge {
        font-size: 0.68rem;
        padding: 0.1rem 0.4rem;
        background: #e8f4e8;
        color: #2d7a2d;
        border-radius: 3px;
        font-weight: 500;
        white-space: nowrap;
        flex-shrink: 0;
    }
    .feed-url {
        font-size: 0.72rem;
        color: #999;
        font-family: "IBM Plex Mono", monospace;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        max-width: 450px;
    }

    .feed-actions {
        display: flex;
        align-items: center;
        gap: 0.4rem;
        flex-shrink: 0;
        flex-wrap: wrap;
    }

    .category-select {
        font-size: 0.75rem;
        padding: 0.2rem 0.35rem;
        border: 1px solid #d8d8d4;
        border-radius: 4px;
        background: #fafafa;
        font-family: inherit;
        color: #555;
        cursor: pointer;
        max-width: 130px;
    }
    .category-select:focus {
        border-color: #ff6b35;
        outline: none;
    }

    .feed-action-btn {
        display: flex;
        align-items: center;
        gap: 0.25rem;
        padding: 0.25rem 0.55rem;
        font-size: 0.72rem;
        font-family: inherit;
        border: 1px solid #d8d8d4;
        border-radius: 4px;
        background: #fafafa;
        color: #888;
        cursor: pointer;
        transition: all 0.12s;
        white-space: nowrap;
    }

    .feed-action-btn:hover:not(:disabled) {
        border-color: #999;
        color: #555;
    }

    .feed-action-btn.active {
        border-color: #ff6b35;
        color: #ff6b35;
        background: #fff5f0;
    }

    .feed-action-btn.disabled-btn {
        opacity: 0.3;
        cursor: not-allowed;
    }

    .feed-action-btn.danger-btn {
        color: #cc4444;
        border-color: #e0c0c0;
    }

    .feed-action-btn.danger-btn:hover {
        background: #fef0f0;
        border-color: #dc3545;
        color: #dc3545;
    }

    .feed-action-btn .material-icons {
        font-size: 0.95rem;
    }

    .paused-name {
        opacity: 0.5;
    }

    .paused-badge {
        font-size: 0.65rem;
        padding: 0.1rem 0.35rem;
        background: #ffeeba;
        color: #856404;
        border-radius: 3px;
        font-weight: 500;
        white-space: nowrap;
    }

    .disclaimer {
        display: flex;
        gap: 0.6rem;
        padding: 0.85rem 1rem;
        background: #fdf8e8;
        border: 1px solid #e8ddb5;
        border-radius: 6px;
        margin-bottom: 1.25rem;
        align-items: flex-start;
    }

    .disclaimer .material-icons {
        font-size: 1.1rem;
        color: #b8860b;
        flex-shrink: 0;
        margin-top: 0.1rem;
    }

    .disclaimer p {
        font-size: 0.78rem;
        color: #6b5a1e;
        line-height: 1.5;
        margin: 0;
    }

    .empty {
        text-align: center;
        padding: 2rem;
        color: #999;
        font-size: 0.88rem;
    }

    /* ─── Delete confirmation modal ──────────────────────────────── */
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
        max-width: 420px;
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
    .modal-feed-name {
        font-size: 0.82rem;
        color: #1a1a1a;
        font-weight: 500;
        padding: 0.4rem 0.7rem;
        background: #f5f5f0;
        border-radius: 4px;
        margin: 0;
    }
    .modal-actions {
        display: flex;
        justify-content: flex-end;
        gap: 0.5rem;
        margin-top: 0.25rem;
    }
    .modal-cancel-btn {
        padding: 0.4rem 1rem;
        background: #fff;
        color: #555;
        border: 1px solid #d0d0d0;
        border-radius: 5px;
        font-size: 0.82rem;
        font-family: inherit;
        cursor: pointer;
    }
    .modal-cancel-btn:hover {
        border-color: #999;
    }
    .modal-confirm-btn {
        display: flex;
        align-items: center;
        gap: 0.3rem;
        padding: 0.4rem 1rem;
        background: #dc3545;
        color: #fff;
        border: none;
        border-radius: 5px;
        font-size: 0.82rem;
        font-family: inherit;
        cursor: pointer;
    }
    .modal-confirm-btn:hover {
        background: #c82333;
    }
    .modal-confirm-btn .material-icons {
        font-size: 1rem;
    }
    .feed-mode-toggle {
        display: flex;
        gap: 0.5rem;
    }

    .mode-btn {
        display: flex;
        align-items: center;
        gap: 0.3rem;
        padding: 0.4rem 0.85rem;
        font-size: 0.82rem;
        font-family: inherit;
        border: 1px solid #d0d0d0;
        border-radius: 5px;
        background: #fafafa;
        color: #888;
        cursor: pointer;
        transition: all 0.15s;
    }

    .mode-btn:hover:not(:disabled) {
        border-color: #999;
        color: #555;
    }

    .mode-btn.active {
        border-color: #ff6b35;
        color: #ff6b35;
        background: #fff5f0;
    }

    .mode-btn:disabled {
        opacity: 0.35;
        cursor: not-allowed;
    }

    .mode-btn .material-icons {
        font-size: 1rem;
    }

    .feed-type-icon {
        display: flex;
        align-items: center;
        flex-shrink: 0;
    }

    .feed-type-icon .material-icons {
        font-size: 1rem;
        color: #999;
    }

    .api-scraping-row {
        display: flex;
        align-items: center;
        gap: 0.4rem;
        font-size: 0.82rem;
        color: #555;
        cursor: pointer;
        margin: 0.5rem 0;
    }
    .api-scraping-row input {
        cursor: pointer;
    }
</style>
