<!-- @ts-nocheck -->
<!-- Feed export: download all feeds as OPML or JSON -->
<script>
    import { getContext } from "svelte";
    import { apiGet } from "#lib/api.js";
    import { t } from "#lib/i18n.svelte.js";

    const setLoading = getContext("setLoading");

    let exported = $state(false);
    let error = $state("");
    let format = $state("opml");

    async function exportFeeds() {
        exported = false;
        error = "";
        setLoading(true);
        try {
            // Both formats are produced by the backend: OPML carries categories
            // and feeds, JSON also favorites and settings
            const isOpml = format === "opml";
            const res = await apiGet(
                isOpml ? "/api/feeds/export/opml" : "/api/feeds/export",
            );
            if (res.success) {
                const content = isOpml
                    ? res.data
                    : JSON.stringify(res.data, null, 2);
                const blob = new Blob([content], {
                    type: isOpml ? "text/x-opml" : "application/json",
                });
                const url = URL.createObjectURL(blob);
                const a = document.createElement("a");
                a.href = url;
                a.download = isOpml
                    ? "meigrss-feeds.opml"
                    : "meigrss-feeds.json";
                a.click();
                URL.revokeObjectURL(url);
                exported = true;
            }
        } catch (e) {
            error = e.message;
        } finally {
            // Clear the overlay on every exit path, including blob/download failures
            setLoading(false);
        }
    }
</script>

<div class="export-page">
    <h2><span class="material-icons">download</span> {t('menu_export')}</h2>

    <div class="card">
        <div class="format-row">
            <label class="format-label" for="export-format-select">{t('export_format_label')}</label>
            <select id="export-format-select" bind:value={format} class="format-input">
                <option value="opml">OPML</option>
                <option value="json">JSON</option>
            </select>
        </div>

        <button class="primary-btn" onclick={exportFeeds}>
            <span class="material-icons">download</span>
            {t('export_button')}
        </button>

        {#if exported}
            <div class="result-row">
                <span class="material-icons success-icon">check_circle</span>
                <span>{t('export_success')}</span>
            </div>
        {:else if error}
            <p class="error-msg"><span class="material-icons">error</span> {error}</p>
        {/if}

        <a href="/" class="back-link">
            <span class="material-icons">arrow_back</span>
            {t('back_home')}
        </a>
    </div>
</div>

<style>
    .export-page { max-width: 480px; }
    h2 { display: flex; align-items: center; gap: 0.5rem; font-size: 1.15rem; font-weight: 600; margin-bottom: 1.25rem; }
    h2 .material-icons { font-size: 1.3rem; color: #ff6b35; }
    .card { background: #fff; border: 1px solid #e0e0e0; border-radius: 8px; padding: 1.5rem; display: flex; flex-direction: column; gap: 1rem; }
    .error-msg { color: #dc3545; font-size: 0.82rem; }
    .format-row { display: flex; flex-direction: column; gap: 0.35rem; }
    .format-label { font-size: 0.82rem; color: #555; font-weight: 500; }
    .format-input {
        padding: 0.4rem 0.6rem; border: 1px solid #d0d0d0; border-radius: 5px;
        font-size: 0.85rem; font-family: inherit; background: #fafafa;
    }
    .format-input:focus { border-color: #ff6b35; outline: none; }
    .primary-btn {
        display: flex; align-items: center; gap: 0.35rem; align-self: flex-start;
        padding: 0.4rem 1rem; background: #1a1a1a; color: #fff; border: none;
        border-radius: 5px; font-size: 0.82rem; font-family: inherit; cursor: pointer;
    }
    .primary-btn:hover { background: #333; }
    .primary-btn .material-icons { font-size: 1rem; }
    a { font-size: 0.85rem; color: #ff6b35; }
    .result-row { display: flex; align-items: center; gap: 0.5rem; font-size: 0.9rem; color: #333; }
    .success-icon { color: #2d7a2d; font-size: 1.2rem; }
    .back-link {
        display: inline-flex; align-items: center; gap: 0.3rem;
        font-size: 0.85rem; color: #555; text-decoration: none;
        padding: 0.4rem 0.8rem; border: 1px solid #d0d0d0; border-radius: 5px;
        transition: all 0.12s; align-self: flex-start;
    }
    .back-link:hover { border-color: #ff6b35; color: #ff6b35; }
    .back-link .material-icons { font-size: 1rem; }
</style>
