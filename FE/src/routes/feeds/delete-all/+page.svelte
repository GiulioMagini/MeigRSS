<!-- @ts-nocheck -->
<!-- Delete all feeds: confirmation page -->
<script>
    import { getContext } from "svelte";
    import { apiDelete } from "#lib/api.js";
    import { t } from "#lib/i18n.svelte.js";
    import { goto } from "$app/navigation";

    const setLoading = getContext("setLoading");

    let done = $state(false);
    let deleteCategories = $state(false);

    async function deleteAll() {
        setLoading(true);
        try {
            await apiDelete(`/api/feeds/delete-all?delete_categories=${deleteCategories}`);
            done = true;
        } catch (e) {
            console.error(e);
        }
        setLoading(false);
    }
</script>

<div class="delete-page">
    <h2><span class="material-icons">delete_forever</span> {t('menu_delete_all')}</h2>

    <div class="card">
        {#if done}
            <p class="success"><span class="material-icons">check_circle</span> {t('all_feeds_deleted')}</p>
            <a href="/" class="back-link">
                <span class="material-icons">arrow_back</span>
                {t('back_home')}
            </a>
        {:else}
            <p class="warning-text">{t('confirm_delete_all_feeds')}</p>
            <label class="checkbox-row">
                <input type="checkbox" bind:checked={deleteCategories} />
                <span>{t('delete_categories_too')}</span>
            </label>
            <div class="btn-row">
                <button class="danger-btn" onclick={deleteAll}>
                    <span class="material-icons">delete_forever</span>
                    {t('confirm_yes')}
                </button>
                <button class="cancel-btn" onclick={() => goto('/')}>
                    {t('confirm_no')}
                </button>
            </div>
        {/if}
    </div>
</div>

<style>
    .delete-page { max-width: 480px; }
    h2 { display: flex; align-items: center; gap: 0.5rem; font-size: 1.15rem; font-weight: 600; margin-bottom: 1.25rem; color: #dc3545; }
    h2 .material-icons { font-size: 1.3rem; }
    .card { background: #fff; border: 1px solid #e0e0e0; border-radius: 8px; padding: 1.5rem; display: flex; flex-direction: column; gap: 1rem; }
    .warning-text { font-size: 0.92rem; color: #333; line-height: 1.5; }
    .checkbox-row {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        font-size: 0.88rem;
        color: #555;
        cursor: pointer;
    }
    .checkbox-row input { cursor: pointer; }
    .btn-row { display: flex; gap: 0.75rem; }
    .danger-btn {
        display: flex; align-items: center; gap: 0.35rem;
        padding: 0.5rem 1.25rem; background: #dc3545; color: #fff;
        border: none; border-radius: 6px; font-size: 0.85rem; font-family: inherit; cursor: pointer;
    }
    .danger-btn:hover { background: #c82333; }
    .danger-btn .material-icons { font-size: 1rem; }
    .cancel-btn {
        padding: 0.5rem 1.25rem; background: #fff; color: #555;
        border: 1px solid #d0d0d0; border-radius: 6px; font-size: 0.85rem; font-family: inherit; cursor: pointer;
    }
    .cancel-btn:hover { border-color: #999; }
    .success { display: flex; align-items: center; gap: 0.4rem; color: #2d7a2d; font-size: 0.88rem; }
    .success .material-icons { font-size: 1.1rem; }
    a { font-size: 0.85rem; color: #ff6b35; }
    .back-link {
        display: inline-flex; align-items: center; gap: 0.3rem;
        font-size: 0.85rem; color: #555; text-decoration: none;
        padding: 0.4rem 0.8rem; border: 1px solid #d0d0d0; border-radius: 5px;
        transition: all 0.12s; align-self: flex-start;
    }
    .back-link:hover { border-color: #ff6b35; color: #ff6b35; }
    .back-link .material-icons { font-size: 1rem; }
</style>
