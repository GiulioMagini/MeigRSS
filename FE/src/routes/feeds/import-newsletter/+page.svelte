<!-- @ts-nocheck -->
<!-- Newsletter import: upload a single EML file or a whole folder, choose or create a newsletter feed, import as articles -->
<script>
    import { onMount, getContext } from "svelte";
    import { apiPost, apiGet, apiPostForm } from "$lib/api.js";
    import { t } from "$lib/i18n.svelte.js";

    const setLoading = getContext("setLoading");

    // Mirrors the limits enforced by the backend. Checking here only spares the
    // user a pointless upload: the backend stays the authority, since nothing
    // reaching it from the browser can be trusted.
    const MAX_FILES = 500;
    const MAX_FILE_SIZE = 5 * 1024 * 1024;
    const MAX_TOTAL_SIZE = 100 * 1024 * 1024;

    let newsletterFeeds = $state([]);
    let selectedFeedId = $state(null);
    let feedMode = $state("existing"); // "existing" or "new"
    let newFeedName = $state("");
    let emlFile = $state(null);
    let parsedTitle = $state("");
    let parsedAuthor = $state("");
    let parsedDate = $state("");
    let showConfirm = $state(false);
    let fileError = $state("");
    let result = $state(null);
    let importMode = $state("single"); // "single" or "folder"
    let folderFiles = $state([]);
    let bulkResult = $state(null);

    async function loadNewsletterFeeds() {
        setLoading(true);
        try {
            const res = await apiGet("/api/newsletter/feeds");
            if (res.success) {
                newsletterFeeds = res.data;
                if (newsletterFeeds.length > 0) {
                    selectedFeedId = newsletterFeeds[0].id;
                } else {
                    // No existing feeds: default to creating a new one
                    feedMode = "new";
                }
            }
        } catch (e) {
            console.error(e);
        } finally {
            setLoading(false);
        }
    }

    async function handleFile(e) {
        const file = e.target.files?.[0];
        if (!file) return;

        fileError = "";
        showConfirm = false;
        result = null;
        bulkResult = null;
        parsedTitle = "";
        parsedAuthor = "";
        parsedDate = "";

        setLoading(true);

        try {
            emlFile = file;

            // The file is uploaded as raw bytes: reading it as text here would
            // decode it as UTF-8 and corrupt any message whose MIME headers
            // declare a different charset
            const form = new FormData();
            form.append("file", file, file.name);

            const res = await apiPostForm("/api/newsletter/preview", form);
            if (res.success) {
                parsedTitle =
                    res.data.title || file.name.replace(/\.eml$/i, "");
                parsedAuthor = res.data.author || "";
                parsedDate = res.data.date || "";
                showConfirm = true;
            }
        } catch (err) {
            fileError = t("newsletter_invalid_file");
        } finally {
            setLoading(false);
        }
    }

    /**
     * Switches between single file and folder mode, clearing everything the
     * previous mode left behind so no stale preview or result survives.
     */
    function switchMode(mode) {
        if (mode === importMode) return;
        importMode = mode;
        fileError = "";
        result = null;
        bulkResult = null;
        showConfirm = false;
        emlFile = null;
        folderFiles = [];
        parsedTitle = "";
        parsedAuthor = "";
        parsedDate = "";
    }

    /**
     * Collects the EML files of the selected folder.
     * The extension filter is a convenience only: whether a file really is an
     * email is decided by the parser on the backend.
     */
    function handleFolder(e) {
        fileError = "";
        showConfirm = false;
        result = null;
        bulkResult = null;
        folderFiles = [];

        const eml = Array.from(e.target.files ?? []).filter((f) =>
            f.name.toLowerCase().endsWith(".eml"),
        );

        if (eml.length === 0) {
            fileError = t("newsletter_folder_no_eml");
            return;
        }
        if (eml.length > MAX_FILES) {
            fileError = t("newsletter_folder_too_many").replace(
                "{max}",
                MAX_FILES,
            );
            return;
        }

        const total = eml.reduce((sum, f) => sum + f.size, 0);
        if (total > MAX_TOTAL_SIZE || eml.some((f) => f.size > MAX_FILE_SIZE)) {
            fileError = t("newsletter_folder_too_large");
            return;
        }

        folderFiles = eml;
        showConfirm = true;
    }

    /**
     * Resolves the feed the articles get attached to: the selected one, or a
     * newly created one. Returns null on failure, having already filled
     * `result` with the reason when there is one worth showing.
     */
    async function resolveFeedId() {
        if (feedMode !== "new") {
            return parseInt(selectedFeedId);
        }
        if (!newFeedName.trim()) {
            result = { success: false, error: t("newsletter_feed_name_empty") };
            return null;
        }

        const feedRes = await apiPost("/api/newsletter/feeds", {
            name: newFeedName.trim(),
        });
        if (!feedRes.success) return null;

        // Refresh the list and select the new feed, so a second import in the
        // same session already finds it available
        const feedId = feedRes.data;
        await loadNewsletterFeeds();
        selectedFeedId = feedId;
        feedMode = "existing";
        newFeedName = "";
        return feedId;
    }

    async function confirmImport() {
        if (!emlFile) return;
        setLoading(true);
        showConfirm = false;

        try {
            const feedId = await resolveFeedId();

            if (!feedId) {
                // A result already set means the helper explained the problem
                // and the user can fix it without reselecting the file
                if (!result) result = { success: false, error: t("error") };
                else showConfirm = true;
                setLoading(false);
                return;
            }

            const form = new FormData();
            form.append("file", emlFile, emlFile.name);
            form.append("title", parsedTitle);
            form.append("feed_id", String(feedId));

            const res = await apiPostForm("/api/newsletter/import", form);

            if (res.success) {
                result = { success: true, id: res.data };
            }
        } catch (e) {
            const msg = e.message || "";

            if (msg.includes("newsletter_no_html")) {
                result = { success: false, error: t("newsletter_no_html") };
            } else if (msg.includes("newsletter_duplicate")) {
                result = { success: false, error: t("newsletter_duplicate") };
            } else if (msg.includes("newsletter_feed_exists")) {
                result = { success: false, error: t("newsletter_feed_exists") };
            } else if (msg.includes("newsletter_feed_name_empty")) {
                result = {
                    success: false,
                    error: t("newsletter_feed_name_empty"),
                };
            } else {
                result = { success: false, error: msg };
            }
        }
        emlFile = null;
        setLoading(false);
    }

    /**
     * Uploads every collected EML in one multipart request and shows the
     * aggregated outcome returned by the backend.
     */
    async function confirmBulkImport() {
        if (folderFiles.length === 0) return;
        setLoading(true);
        showConfirm = false;

        try {
            const feedId = await resolveFeedId();
            if (!feedId) {
                if (!result) result = { success: false, error: t("error") };
                else showConfirm = true;
                setLoading(false);
                return;
            }

            const form = new FormData();
            form.append("feed_id", String(feedId));
            // Send the bare file name: the backend uses it only as a title
            // fallback, never to touch the filesystem
            for (const file of folderFiles) {
                form.append("files", file, file.name);
            }

            const res = await apiPostForm("/api/newsletter/import-bulk", form);
            if (res.success) {
                bulkResult = res.data;
            }
        } catch (e) {
            const msg = e.message || "";
            const known = [
                "newsletter_folder_no_eml",
                "newsletter_folder_too_many",
                "newsletter_folder_too_large",
                "newsletter_invalid_file",
                "newsletter_feed_exists",
                "newsletter_feed_name_empty",
            ].find((key) => msg.includes(key));
            result = {
                success: false,
                error: known ? t(known).replace("{max}", MAX_FILES) : msg,
            };
        }
        folderFiles = [];
        setLoading(false);
    }

    function cancelImport() {
        showConfirm = false;
        emlFile = null;
        folderFiles = [];
        parsedTitle = "";
        parsedAuthor = "";
        parsedDate = "";
    }

    onMount(loadNewsletterFeeds);
</script>

<div class="newsletter-page">
    <h2>
        <span class="material-icons">email</span>
        {t("menu_import_newsletter")}
    </h2>

    <div class="card">
        <div class="mode-toggle">
            <button
                class="mode-btn"
                class:active={importMode === "single"}
                onclick={() => switchMode("single")}
            >
                <span class="material-icons">description</span>
                {t("newsletter_mode_single")}
            </button>
            <button
                class="mode-btn"
                class:active={importMode === "folder"}
                onclick={() => switchMode("folder")}
            >
                <span class="material-icons">folder</span>
                {t("newsletter_mode_folder")}
            </button>
        </div>

        {#if importMode === "single"}
            <p class="desc">{t("newsletter_description")}</p>
            <input type="file" accept=".eml" onchange={handleFile} />
        {:else}
            <p class="desc">{t("newsletter_folder_description")}</p>
            <!-- webkitdirectory is outside the W3C spec but implemented by every
                 current desktop browser: there is no standard way to pick a
                 whole folder from a file input -->
            <input
                type="file"
                webkitdirectory
                multiple
                onchange={handleFolder}
            />
        {/if}

        {#if fileError}
            <p class="error-msg">
                <span class="material-icons">error</span>
                {fileError}
            </p>
        {/if}

        {#if showConfirm}
            <div class="confirm-box">
                {#if importMode === "single"}
                    <div class="field">
                        <label for="nl-title">{t("newsletter_title")}</label>
                        <input
                            id="nl-title"
                            type="text"
                            bind:value={parsedTitle}
                            class="text-input"
                        />
                    </div>

                    {#if parsedAuthor}
                        <p class="preview-info">
                            <span class="material-icons">person</span>
                            {parsedAuthor}
                        </p>
                    {/if}

                    {#if parsedDate}
                        <p class="preview-info">
                            <span class="material-icons">schedule</span>
                            {parsedDate}
                        </p>
                    {/if}
                {:else}
                    <p class="preview-info">
                        <span class="material-icons">folder</span>
                        {t("newsletter_folder_selected").replace(
                            "{count}",
                            folderFiles.length,
                        )}
                    </p>
                {/if}

                <!-- Feed selection -->
                <div class="feed-section">
                    <span class="section-label"
                        >{t("newsletter_choose_feed")}</span
                    >

                    <div class="mode-toggle">
                        {#if newsletterFeeds.length > 0}
                            <button
                                class="mode-btn"
                                class:active={feedMode === "existing"}
                                onclick={() => (feedMode = "existing")}
                            >
                                <span class="material-icons">list</span>
                                {t("newsletter_existing_feed")}
                            </button>
                        {/if}
                        <button
                            class="mode-btn"
                            class:active={feedMode === "new"}
                            onclick={() => (feedMode = "new")}
                        >
                            <span class="material-icons"
                                >add_circle_outline</span
                            >
                            {t("newsletter_new_feed")}
                        </button>
                    </div>

                    {#if feedMode === "existing" && newsletterFeeds.length > 0}
                        <select bind:value={selectedFeedId} class="text-input">
                            {#each newsletterFeeds as feed}
                                <option value={feed.id}>{feed.site_name}</option
                                >
                            {/each}
                        </select>
                    {:else if feedMode === "existing" && newsletterFeeds.length === 0}
                        <p class="hint">{t("newsletter_no_feeds")}</p>
                    {:else}
                        <div class="field">
                            <label for="nl-feed-name"
                                >{t("newsletter_feed_name")}</label
                            >
                            <input
                                id="nl-feed-name"
                                type="text"
                                bind:value={newFeedName}
                                placeholder={t(
                                    "newsletter_feed_name_placeholder",
                                )}
                                class="text-input"
                            />
                        </div>
                    {/if}
                </div>

                <div class="btn-row">
                    <button
                        class="primary-btn"
                        onclick={importMode === "single"
                            ? confirmImport
                            : confirmBulkImport}
                        disabled={(importMode === "single" &&
                            !parsedTitle.trim()) ||
                            (feedMode === "new" && !newFeedName.trim()) ||
                            (feedMode === "existing" && !selectedFeedId)}
                    >
                        <span class="material-icons">check</span>
                        {t("confirm_yes")}
                    </button>
                    <button class="cancel-btn" onclick={cancelImport}>
                        {t("confirm_no")}
                    </button>
                </div>
            </div>
        {/if}

        {#if bulkResult}
            <div class="result-row">
                <span class="material-icons success-icon">check_circle</span>
                <span>
                    {bulkResult.imported > 0
                        ? t("newsletter_bulk_imported").replace(
                              "{count}",
                              bulkResult.imported,
                          )
                        : t("newsletter_bulk_none")}
                </span>
            </div>

            {#if bulkResult.duplicates > 0 || bulkResult.no_html > 0 || bulkResult.invalid > 0}
                <ul class="skipped-list">
                    {#if bulkResult.duplicates > 0}
                        <li>
                            {t("newsletter_bulk_duplicates").replace(
                                "{count}",
                                bulkResult.duplicates,
                            )}
                        </li>
                    {/if}
                    {#if bulkResult.no_html > 0}
                        <li>
                            {t("newsletter_bulk_no_html").replace(
                                "{count}",
                                bulkResult.no_html,
                            )}
                        </li>
                    {/if}
                    {#if bulkResult.invalid > 0}
                        <li>
                            {t("newsletter_bulk_invalid").replace(
                                "{count}",
                                bulkResult.invalid,
                            )}
                        </li>
                    {/if}
                </ul>
            {/if}
        {/if}

        {#if result?.success}
            <div class="result-row">
                <span class="material-icons success-icon">check_circle</span>
                <span>{t("newsletter_imported")}</span>
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
    .newsletter-page {
        max-width: 520px;
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

    .field {
        display: flex;
        flex-direction: column;
        gap: 0.3rem;
    }
    .field label {
        font-size: 0.82rem;
        font-weight: 500;
        color: #555;
    }

    .text-input {
        padding: 0.4rem 0.6rem;
        border: 1px solid #d0d0d0;
        border-radius: 5px;
        font-size: 0.85rem;
        font-family: inherit;
        background: #fff;
    }
    .text-input:focus {
        border-color: #ff6b35;
        outline: none;
    }

    .preview-info {
        display: flex;
        align-items: center;
        gap: 0.4rem;
        font-size: 0.82rem;
        color: #666;
        margin: 0;
    }
    .preview-info .material-icons {
        font-size: 1rem;
        color: #999;
    }

    /* Feed selection section */
    .feed-section {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        padding-top: 0.5rem;
        border-top: 1px solid #e8e4d8;
    }

    .section-label {
        font-size: 0.82rem;
        font-weight: 600;
        color: #444;
    }

    .mode-toggle {
        display: flex;
        gap: 0.4rem;
    }

    .mode-btn {
        display: flex;
        align-items: center;
        gap: 0.25rem;
        padding: 0.3rem 0.7rem;
        font-size: 0.78rem;
        font-family: inherit;
        border: 1px solid #d0d0d0;
        border-radius: 5px;
        background: #fff;
        color: #777;
        cursor: pointer;
        transition: all 0.12s;
    }
    .mode-btn:hover {
        border-color: #999;
        color: #555;
    }
    .mode-btn.active {
        border-color: #ff6b35;
        color: #ff6b35;
        background: #fff5f0;
    }
    .mode-btn .material-icons {
        font-size: 0.95rem;
    }

    .hint {
        font-size: 0.8rem;
        color: #999;
        font-style: italic;
        margin: 0;
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
    .primary-btn:disabled {
        opacity: 0.5;
        cursor: not-allowed;
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

    .skipped-list {
        margin: 0;
        padding-left: 1.6rem;
        font-size: 0.82rem;
        color: #666;
    }
    .skipped-list li {
        margin-top: 0.15rem;
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
</style>
