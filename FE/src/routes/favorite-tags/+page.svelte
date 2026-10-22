<!-- Favorite tags management: add, rename, delete -->
<script>
    import { onMount, getContext } from "svelte";
    import { apiGet, apiPost, apiPut, apiDelete } from "#lib/api.js";
    import { t } from "#lib/i18n.svelte.js";

    const setLoading = getContext("setLoading");

    let tags = $state([]);
    let newName = $state("");
    let editingId = $state(null);
    let editingName = $state("");
    let error = $state("");
    // Tag waiting for confirmation before being deleted (only tags in use)
    let tagToDelete = $state(null);

    async function loadTags() {
        try {
            const res = await apiGet("/api/favorite-tags");
            if (res.success) tags = res.data;
        } catch (e) { console.error(e); }
    }

    async function addTag() {
        if (!newName.trim()) return;
        error = "";
        setLoading(true);
        try {
            await apiPost("/api/favorite-tags", { name: newName.trim() });
            newName = "";
            await loadTags();
        } catch (e) {
            const msg = e.message || "";
            error = t(msg) || msg;
        } finally {
            setLoading(false);
        }
    }

    function startEdit(tag) {
        editingId = tag.id;
        editingName = tag.name;
    }

    async function saveEdit() {
        if (!editingName.trim() || !editingId) return;
        error = "";
        setLoading(true);
        try {
            await apiPut(`/api/favorite-tags/${editingId}`, { name: editingName.trim() });
            editingId = null;
            editingName = "";
            await loadTags();
        } catch (e) {
            const msg = e.message || "";
            error = t(msg) || msg;
        } finally {
            setLoading(false);
        }
    }

    function cancelEdit() {
        editingId = null;
        editingName = "";
    }

    /**
     * Deletes a tag right away if no article uses it, otherwise asks for
     * confirmation first, since the tag is removed from all those articles.
     */
    function requestDelete(tag) {
        if (tag.article_count > 0) {
            tagToDelete = tag;
        } else {
            deleteTag(tag.id);
        }
    }

    async function confirmDelete() {
        const id = tagToDelete.id;
        tagToDelete = null;
        await deleteTag(id);
    }

    async function deleteTag(id) {
        error = "";
        setLoading(true);
        try {
            const res = await apiDelete(`/api/favorite-tags/${id}`);
            if (res.success) {
                await loadTags();
            }
        } catch (e) {
            error = e.message;
        } finally {
            setLoading(false);
        }
    }

    onMount(async () => {
        setLoading(true);
        await loadTags();
        setLoading(false);
    });
</script>

<div class="tags-page">
    <h2><span class="material-icons">sell</span> {t('menu_favorite_tags')}</h2>
    <!-- Explains that tags belong to favorite articles, to tell them apart from feed categories -->
    <p class="desc">{t('tag_description')}</p>

    <!-- Add tag -->
    <div class="add-row">
        <input type="text" bind:value={newName} placeholder={t('tag_add')} class="input flex-1"
            onkeydown={(e) => { if (e.key === 'Enter') addTag(); }} />
        <button class="primary-btn" onclick={addTag} disabled={!newName.trim()}>
            <span class="material-icons">add</span>
            {t('tag_add')}
        </button>
    </div>

    {#if error}
        <p class="error-msg">{error}</p>
    {/if}

    <!-- Tag list -->
    <div class="tag-list">
        {#each tags as tag (tag.id)}
            <div class="tag-row">
                {#if editingId === tag.id}
                    <input type="text" bind:value={editingName} class="input flex-1"
                        onkeydown={(e) => { if (e.key === 'Enter') saveEdit(); if (e.key === 'Escape') cancelEdit(); }} />
                    <button class="icon-btn ok" onclick={saveEdit}><span class="material-icons">check</span></button>
                    <button class="icon-btn" onclick={cancelEdit}><span class="material-icons">close</span></button>
                {:else}
                    <span class="tag-name">{tag.name}</span>
                    <div class="tag-actions">
                        <button class="icon-btn" title={t('tag_rename')} onclick={() => startEdit(tag)}>
                            <span class="material-icons">edit</span>
                        </button>
                        <button class="icon-btn danger" title={t('tag_delete')} onclick={() => requestDelete(tag)}>
                            <span class="material-icons">delete</span>
                        </button>
                    </div>
                {/if}
            </div>
        {/each}
    </div>
</div>

<!-- Delete confirmation, shown only for tags in use -->
{#if tagToDelete}
    <div class="modal-backdrop" onclick={() => tagToDelete = null} role="presentation">
        <div class="modal-dialog" onclick={(e) => e.stopPropagation()}
            onkeydown={(e) => { if (e.key === 'Escape') tagToDelete = null; }}
            role="dialog" aria-modal="true" tabindex="-1">
            <div class="modal-header">
                <span class="material-icons modal-icon">warning</span>
                <h3>{t('tag_delete')}</h3>
            </div>
            <p class="modal-text">{t('tag_delete_confirm').replace('{n}', tagToDelete.article_count)}</p>
            <p class="modal-tag-name">{tagToDelete.name}</p>
            <div class="modal-actions">
                <button class="modal-cancel-btn" onclick={() => tagToDelete = null}>
                    {t('confirm_no')}
                </button>
                <button class="modal-confirm-btn" onclick={confirmDelete}>
                    <span class="material-icons">delete</span>
                    {t('confirm_yes')}
                </button>
            </div>
        </div>
    </div>
{/if}

<style>
    .tags-page { max-width: 560px; }

    h2 { display: flex; align-items: center; gap: 0.5rem; font-size: 1.15rem; font-weight: 600; margin-bottom: 1.25rem; }
    h2 .material-icons { font-size: 1.3rem; color: #ff6b35; }

    .desc { font-size: 0.88rem; color: #666; line-height: 1.5; margin-bottom: 1.25rem; }

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

    .tag-list { display: flex; flex-direction: column; }

    .tag-row {
        display: flex; justify-content: space-between; align-items: center;
        padding: 0.6rem 1rem; background: #fff; border: 1px solid #e8e8e4; gap: 0.5rem;
    }
    .tag-row + .tag-row { border-top: none; }
    .tag-row:first-child { border-radius: 8px 8px 0 0; }
    .tag-row:last-child { border-radius: 0 0 8px 8px; }
    .tag-row:only-child { border-radius: 8px; }

    .tag-name { font-size: 0.88rem; font-weight: 500; color: #1a1a1a; }

    .tag-actions { display: flex; gap: 0.25rem; }

    .icon-btn {
        background: none; border: none; cursor: pointer; padding: 0.25rem;
        color: #bbb; transition: color 0.15s; display: flex;
    }
    .icon-btn:hover { color: #555; }
    .icon-btn.ok:hover { color: #2d7a2d; }
    .icon-btn.danger:hover { color: #dc3545; }
    .icon-btn .material-icons { font-size: 1.15rem; }

    /* ─── Delete confirmation ────────────────────────────────────── */
    .modal-backdrop {
        position: fixed; inset: 0; background: rgba(0, 0, 0, 0.4);
        display: flex; align-items: center; justify-content: center; z-index: 1000;
    }
    .modal-dialog {
        background: #fff; border-radius: 10px; padding: 1.5rem; max-width: 420px; width: 90%;
        box-shadow: 0 8px 32px rgba(0, 0, 0, 0.18);
        display: flex; flex-direction: column; gap: 0.75rem;
    }
    .modal-header { display: flex; align-items: center; gap: 0.5rem; }
    .modal-header h3 { font-size: 1rem; font-weight: 600; color: #1a1a1a; margin: 0; }
    .modal-icon { font-size: 1.3rem; color: #e67e22; }
    .modal-text { font-size: 0.88rem; color: #555; line-height: 1.5; margin: 0; }
    .modal-tag-name {
        font-size: 0.82rem; color: #1a1a1a; font-weight: 500;
        padding: 0.4rem 0.7rem; background: #f5f5f0; border-radius: 4px; margin: 0;
    }
    .modal-actions { display: flex; justify-content: flex-end; gap: 0.5rem; margin-top: 0.25rem; }
    .modal-cancel-btn {
        padding: 0.4rem 1rem; background: #fff; color: #555;
        border: 1px solid #d0d0d0; border-radius: 5px; font-size: 0.82rem; font-family: inherit; cursor: pointer;
    }
    .modal-cancel-btn:hover { border-color: #999; }
    .modal-confirm-btn {
        display: flex; align-items: center; gap: 0.3rem;
        padding: 0.4rem 1rem; background: #dc3545; color: #fff;
        border: none; border-radius: 5px; font-size: 0.82rem; font-family: inherit; cursor: pointer;
    }
    .modal-confirm-btn:hover { background: #c82333; }
    .modal-confirm-btn .material-icons { font-size: 1rem; }
</style>
