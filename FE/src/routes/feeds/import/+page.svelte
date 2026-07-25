<!-- @ts-nocheck -->
<!-- Feed import: upload JSON file to import feeds -->
<script>
    import { onMount, getContext } from "svelte";
    import { apiPost, apiGet, categoryDisplayName } from "$lib/api.js";
    import { t } from "$lib/i18n.svelte.js";

    const setLoading = getContext("setLoading");

    let result = $state(null);
    let parsedFeeds = $state(null);
    let feedCount = $state(0);
    let favCount = $state(0);
    let showConfirm = $state(false);
    let fileError = $state("");
    let categories = $state([]);
    let importCategory = $state("__keep_original__");
    let parsedData = $state(null); // Full export object { categories, feeds }

    async function handleFile(e) {
        const file = e.target.files?.[0];
        if (!file) return;

        fileError = "";
        parsedFeeds = null;
        parsedData = null;
        showConfirm = false;
        result = null;

        try {
            const text = await file.text();

            // OPML is XML, so a leading angle bracket tells the two apart.
            // Parsing is delegated to the backend, which owns the format.
            if (text.trimStart().startsWith("<")) {
                setLoading(true);
                try {
                    const res = await apiPost("/api/feeds/import/opml", { opml: text });
                    if (res.success) {
                        parsedData = res.data;
                        parsedFeeds = res.data.feeds;
                        feedCount = res.data.feeds.length;
                        favCount = 0;
                        showConfirm = true;
                    }
                } catch (err) {
                    fileError = err.message || t("import_invalid_file");
                } finally {
                    setLoading(false);
                }
                return;
            }
            
            const data = JSON.parse(text);

            // Support both new format { categories, feeds } and legacy format (array of feeds)
            if (data.feeds && Array.isArray(data.feeds)) {
                // New format
                if (!data.favorites) data.favorites = [];
                parsedData = data;
                parsedFeeds = data.feeds;
                feedCount = data.feeds.length;
                favCount = data.favorites.length;
            } else if (Array.isArray(data) && data.length > 0) {
                // Legacy format: array of feeds without categories
                parsedData = { categories: [], feeds: data, favorites: [] };
                favCount = 0;
                parsedFeeds = data;
                feedCount = data.length;
            } else {
                fileError = t("import_invalid_file");
                return;
            }

            showConfirm = true;
        } catch (e) {
            fileError = t("import_invalid_file");
        }
    }

    async function confirmImport() {
        if (!parsedData) return;
        setLoading(true);
        showConfirm = false;
        try {
            let dataToImport = { ...parsedData };

            // If user selected a specific category, override all feed categories
            if (importCategory !== "__keep_original__") {
                const selectedCat = categories.find(
                    (c) => c.id === parseInt(importCategory),
                );
                if (selectedCat) {
                    dataToImport.feeds = parsedData.feeds.map((f) => ({
                        ...f,
                        category_name: selectedCat.name,
                    }));
                }
            }

            const res = await apiPost("/api/feeds/import", dataToImport);
            if (res.success) {
                result = { success: true, feedsImported: res.data.feeds_imported, favoritesImported: res.data.favorites_imported };
            }
        } catch (e) {
            result = { success: false, error: e.message };
        }
        parsedFeeds = null;
        parsedData = null;
        setLoading(false);
    }

    function cancelImport() {
        showConfirm = false;
        parsedFeeds = null;
        parsedData = null;
        feedCount = 0;
    }

    async function loadCategories() {
        try {
            const res = await apiGet("/api/categories");
            if (res.success) {
                categories = res.data;
            }
        } catch (e) {
            console.error(e);
        }
    }

    onMount(loadCategories);
</script>

<div class="import-page">
    <h2><span class="material-icons">upload_file</span> {t("menu_import")}</h2>

    <div class="card">
        <p class="desc">{t("import_description")}</p>
        <input type="file" accept=".json,.opml,.xml" onchange={handleFile} />

        {#if fileError}
            <p class="error-msg">
                <span class="material-icons">error</span>
                {fileError}
            </p>
        {/if}

        {#if showConfirm}
            <div class="confirm-box">
                <p class="confirm-text">
                    {t("import_confirm").replace("{count}", feedCount)}
                    {#if favCount > 0}
                        <br/>{t("import_favorites_count").replace("{count}", favCount)}
                    {/if}
                </p>
                <div class="category-row">
                    <label class="category-label" for="import-category-select">{t('import_select_category')}</label>
                    <select id="import-category-select" bind:value={importCategory} class="category-input">
                        <option value="__keep_original__">{t('import_keep_original')}</option>
                        {#each categories as cat}
                            <option value={cat.id}>{categoryDisplayName(cat, t)}</option>
                        {/each}
                    </select>
                </div>
                <div class="btn-row">
                    <button class="primary-btn" onclick={confirmImport}>
                        <span class="material-icons">check</span>
                        {t("confirm_yes")}
                    </button>
                    <button class="cancel-btn" onclick={cancelImport}>
                        {t("confirm_no")}
                    </button>
                </div>
            </div>
        {/if}

        {#if result?.success}
            <div class="result-row">
                <span class="material-icons success-icon">check_circle</span>
                <span>{result.feedsImported} {t("import_count_feeds")}{#if result.favoritesImported > 0}, {result.favoritesImported} {t("import_count_favorites")}{/if}</span>
            </div>
        {:else if result?.error}
            <p class="error-msg">
                <span class="material-icons">error</span>
                {result.error}
            </p>
        {/if}

        <a href="/" class="back-link">
            <span class="material-icons">arrow_back</span>
            {t("back_home")}
        </a>
    </div>
</div>

<style>
    .import-page {
        max-width: 480px;
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
    .card {
        background: #fff;
        border: 1px solid #e0e0e0;
        border-radius: 8px;
        padding: 1.5rem;
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }
    .desc {
        font-size: 0.88rem;
        color: #666;
    }

    .confirm-box {
        padding: 1rem;
        background: #fafaf6;
        border: 1px solid #e8e4d8;
        border-radius: 6px;
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
    }
    .confirm-text {
        font-size: 0.9rem;
        color: #333;
    }
    .btn-row {
        display: flex;
        gap: 0.5rem;
    }

    .primary-btn {
        display: flex;
        align-items: center;
        gap: 0.35rem;
        padding: 0.4rem 1rem;
        background: #1a1a1a;
        color: #fff;
        border: none;
        border-radius: 5px;
        font-size: 0.82rem;
        font-family: inherit;
        cursor: pointer;
    }
    .primary-btn:hover {
        background: #333;
    }
    .primary-btn .material-icons {
        font-size: 1rem;
    }

    .cancel-btn {
        padding: 0.4rem 1rem;
        background: #fff;
        color: #555;
        border: 1px solid #d0d0d0;
        border-radius: 5px;
        font-size: 0.82rem;
        font-family: inherit;
        cursor: pointer;
    }
    .cancel-btn:hover {
        border-color: #999;
    }

    .result-row {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        font-size: 0.9rem;
        color: #333;
    }
    .success-icon {
        color: #2d7a2d;
        font-size: 1.2rem;
    }
    .error-msg {
        display: flex;
        align-items: center;
        gap: 0.3rem;
        color: #dc3545;
        font-size: 0.82rem;
    }
    .error-msg .material-icons {
        font-size: 0.95rem;
    }

    .back-link {
        display: inline-flex;
        align-items: center;
        gap: 0.3rem;
        font-size: 0.85rem;
        color: #555;
        text-decoration: none;
        padding: 0.4rem 0.8rem;
        border: 1px solid #d0d0d0;
        border-radius: 5px;
        transition: all 0.12s;
        align-self: flex-start;
    }
    .back-link:hover {
        border-color: #ff6b35;
        color: #ff6b35;
    }
    .back-link .material-icons {
        font-size: 1rem;
    }
    .category-row { display: flex; flex-direction: column; gap: 0.35rem; }
    .category-label { font-size: 0.82rem; color: #555; font-weight: 500; }
    .category-input {
        padding: 0.4rem 0.6rem; border: 1px solid #d0d0d0; border-radius: 5px;
        font-size: 0.85rem; font-family: inherit; background: #fafafa;
    }
    .category-input:focus { border-color: #ff6b35; outline: none; }
</style>
