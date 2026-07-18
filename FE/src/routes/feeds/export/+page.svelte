<!-- @ts-nocheck -->
<!-- Feed export: download all feeds as JSON -->
<script>
    import { onMount } from "svelte";
    import { apiGet } from "$lib/api.js";
    import { t } from "$lib/i18n.svelte.js";

    let exported = $state(false);
    let error = $state("");

    async function exportFeeds() {
        try {
            const res = await apiGet("/api/feeds/export");
            if (res.success) {
                const blob = new Blob([JSON.stringify(res.data, null, 2)], { type: "application/json" });
                const url = URL.createObjectURL(blob);
                const a = document.createElement("a");
                a.href = url;
                a.download = "meigrss-feeds.json";
                a.click();
                URL.revokeObjectURL(url);
                exported = true;
            }
        } catch (e) {
            error = e.message;
        }
    }

    onMount(exportFeeds);
</script>

<div class="export-page">
    <h2><span class="material-icons">download</span> {t('menu_export')}</h2>

    <div class="card">
        {#if exported}
            <div class="result-row">
                <span class="material-icons success-icon">check_circle</span>
                <span>{t('export_success')}</span>
            </div>
        {:else if error}
            <p class="error-msg"><span class="material-icons">error</span> {error}</p>
        {:else}
            <p>{t('loading')}</p>
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