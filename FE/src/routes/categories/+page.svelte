<!-- @ts-nocheck -->
<!-- Categories management: add, rename, delete -->
<script>
    import { onMount, getContext } from "svelte";
    import { apiGet, apiPost, apiPut, apiDelete, sanitize, categoryDisplayName } from "#lib/api.js";
    import { t } from "#lib/i18n.svelte.js";

    const setLoading = getContext("setLoading");

    let categories = $state([]);
    let newName = $state("");
    let editingId = $state(null);
    let editingName = $state("");
    let error = $state("");

    async function loadCategories() {
        try {
            const res = await apiGet("/api/categories");
            if (res.success) categories = res.data;
        } catch (e) { console.error(e); }
    }

    async function addCategory() {
        if (!newName.trim()) return;
        error = "";
        setLoading(true);
        try {
            const cleaned = sanitize({ input: newName.trim() });
            await apiPost("/api/categories", { name: cleaned });
            newName = "";
            await loadCategories();
        } catch (e) {
            const msg = e.message || "";
            error = t(msg) || msg;
        } finally {
            setLoading(false);
        }
    }

    function startEdit(cat) {
        editingId = cat.id;
        editingName = cat.name;
    }

    async function saveEdit() {
        if (!editingName.trim() || !editingId) return;
        error = "";
        setLoading(true);
        try {
            const cleaned = sanitize({ input: editingName.trim() });
            await apiPut(`/api/categories/${editingId}`, { name: cleaned });
            editingId = null;
            editingName = "";
            await loadCategories();
        } catch (e) { error = e.message; } 
        finally {
            setLoading(false);
        }
    }

    function cancelEdit() {
        editingId = null;
        editingName = "";
    }

    async function deleteCategory(id) {
        error = "";
        setLoading(true);
        try {
            const res = await apiDelete(`/api/categories/${id}`);
            if (res.success) {
                await loadCategories();
            }
        } catch (e) {
            error = e.message;
        } finally {
            setLoading(false);
        }
    }

    onMount(async () => {
        setLoading(true);
        await loadCategories();
        setLoading(false);
    });
</script>

<div class="categories-page">
    <h2><span class="material-icons">category</span> {t('menu_categories')}</h2>

    <!-- Add category -->
    <div class="add-row">
        <input type="text" bind:value={newName} placeholder={t('cat_add')} class="input flex-1"
            onkeydown={(e) => { if (e.key === 'Enter') addCategory(); }} />
        <button class="primary-btn" onclick={addCategory} disabled={!newName.trim()}>
            <span class="material-icons">add</span>
            {t('cat_add')}
        </button>
    </div>

    {#if error}
        <p class="error-msg">{error}</p>
    {/if}

    <!-- Category list -->
    <div class="cat-list">
        {#each categories as cat (cat.id)}
            <div class="cat-row">
                {#if editingId === cat.id}
                    <input type="text" bind:value={editingName} class="input flex-1"
                        onkeydown={(e) => { if (e.key === 'Enter') saveEdit(); if (e.key === 'Escape') cancelEdit(); }} />
                    <button class="icon-btn ok" onclick={saveEdit}><span class="material-icons">check</span></button>
                    <button class="icon-btn" onclick={cancelEdit}><span class="material-icons">close</span></button>
                {:else}
                    <span class="cat-name">
                        {sanitize({ input: categoryDisplayName(cat, t) })}
                        {#if cat.is_default}
                            <span class="default-tag">default</span>
                        {/if}
                    </span>
                    <div class="cat-actions">
                        {#if !cat.is_default}
                            <button class="icon-btn" title={t('cat_rename')} onclick={() => startEdit(cat)}>
                                <span class="material-icons">edit</span>
                            </button>
                            <button class="icon-btn danger" title={t('cat_delete')} onclick={() => deleteCategory(cat.id)}>
                                <span class="material-icons">delete</span>
                            </button>
                        {/if}
                    </div>
                {/if}
            </div>
        {/each}
    </div>
</div>

<style>
    .categories-page { max-width: 560px; }

    h2 { display: flex; align-items: center; gap: 0.5rem; font-size: 1.15rem; font-weight: 600; margin-bottom: 1.25rem; }
    h2 .material-icons { font-size: 1.3rem; color: #ff6b35; }

    .add-row { display: flex; gap: 0.5rem; margin-bottom: 1rem; }

    .input {
        padding: 0.4rem 0.6rem; border: 1px solid #d0d0d0; border-radius: 5px;
        font-size: 0.85rem; font-family: inherit; background: #fafafa;
    }
    .input:focus { border-color: #ff6b35; outline: none; }
    .flex-1 { flex: 1; }

    .primary-btn {
        display: flex; align-items: center; gap: 0.35rem;
        padding: 0.4rem 0.9rem; background: #1a1a1a; color: #fff;
        border: none; border-radius: 5px; font-size: 0.82rem; font-family: inherit; cursor: pointer;
    }
    .primary-btn:hover { background: #333; }
    .primary-btn:disabled { opacity: 0.5; cursor: not-allowed; }
    .primary-btn .material-icons { font-size: 1rem; }

    .error-msg {
        font-size: 0.8rem; color: #dc3545; padding: 0.4rem 0.6rem;
        background: #fef0f0; border-radius: 4px; margin-bottom: 0.75rem;
    }

    .cat-list { display: flex; flex-direction: column; }

    .cat-row {
        display: flex; justify-content: space-between; align-items: center;
        padding: 0.6rem 1rem; background: #fff; border: 1px solid #e8e8e4; gap: 0.5rem;
    }
    .cat-row + .cat-row { border-top: none; }
    .cat-row:first-child { border-radius: 8px 8px 0 0; }
    .cat-row:last-child { border-radius: 0 0 8px 8px; }
    .cat-row:only-child { border-radius: 8px; }

    .cat-name { font-size: 0.88rem; font-weight: 500; color: #1a1a1a; display: flex; align-items: center; gap: 0.5rem; }
    .default-tag {
        font-size: 0.65rem; padding: 0.1rem 0.35rem; background: #e8e8e4;
        color: #888; border-radius: 3px; text-transform: uppercase; letter-spacing: 0.04em;
    }

    .cat-actions { display: flex; gap: 0.25rem; }

    .icon-btn {
        background: none; border: none; cursor: pointer; padding: 0.25rem;
        color: #bbb; transition: color 0.15s; display: flex;
    }
    .icon-btn:hover { color: #555; }
    .icon-btn.ok:hover { color: #2d7a2d; }
    .icon-btn.danger:hover { color: #dc3545; }
    .icon-btn .material-icons { font-size: 1.15rem; }
</style>
