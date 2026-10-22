<!-- TagPicker: dropdown to pick favorite tags, plus a removable chip for each
     tag already picked. Shared by the tag search and the article tag editor:
     what picking or removing a tag means is decided by the parent. -->
<script module>
    // Max tags per favorite and per search: must match MAX_TAGS in the backend
    export const MAX_TAGS = 3;
</script>

<script>
    import { t } from "#lib/i18n.svelte.js";

    let { allTags, selected, disabled = false, onAdd, onRemove } = $props();

    // Tags that can still be picked
    const available = $derived(allTags.filter(tag => !selected.some(s => s.id === tag.id)));
    const limitReached = $derived(selected.length >= MAX_TAGS);

    function handleSelect(e) {
        const id = parseInt(e.target.value);
        // Back to the placeholder, so the dropdown never shows a picked tag
        e.target.value = "";
        const tag = available.find(tag => tag.id === id);
        if (tag) onAdd(tag);
    }
</script>

<div class="tag-picker">
    {#if allTags.length === 0}
        <span class="hint">{t('tag_none_defined')}</span>
    {:else}
        <select class="control" disabled={disabled || limitReached || available.length === 0} onchange={handleSelect}>
            <option value="">
                {limitReached ? t('tag_limit_hint').replace('{n}', MAX_TAGS) : t('tag_select_placeholder')}
            </option>
            {#each available as tag (tag.id)}
                <option value={tag.id}>{tag.name}</option>
            {/each}
        </select>
    {/if}

    {#each selected as tag (tag.id)}
        <span class="chip">
            {tag.name}
            <button class="chip-remove" title={t('tag_remove')} aria-label={t('tag_remove')}
                {disabled} onclick={() => onRemove(tag)}>
                <span class="material-icons">close</span>
            </button>
        </span>
    {/each}
</div>

<style>
    .tag-picker {
        display: flex;
        align-items: center;
        flex-wrap: wrap;
        gap: 0.5rem;
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
    .control:disabled { opacity: 0.6; cursor: not-allowed; }

    .hint {
        font-size: 0.82rem;
        color: #999;
        font-style: italic;
    }

    .chip {
        display: inline-flex;
        align-items: center;
        gap: 0.2rem;
        padding: 0.2rem 0.3rem 0.2rem 0.65rem;
        background: #fff3ec;
        border: 1px solid #ffd2bd;
        border-radius: 999px;
        font-size: 0.8rem;
        color: #b4471c;
    }

    .chip-remove {
        display: flex;
        align-items: center;
        justify-content: center;
        width: 1.2rem;
        height: 1.2rem;
        padding: 0;
        border: none;
        border-radius: 50%;
        background: none;
        color: #b4471c;
        cursor: pointer;
    }
    .chip-remove:hover:not(:disabled) { background: #ffd2bd; }
    .chip-remove:disabled { opacity: 0.5; cursor: not-allowed; }
    .chip-remove .material-icons { font-size: 0.9rem; }
</style>
