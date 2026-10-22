<!-- Settings page: retention period, auto-update interval -->
<script>
    import { onMount, getContext } from "svelte";
    import { apiGet, apiPut } from "#lib/api.js";
    import { t } from "#lib/i18n.svelte.js";

    const setLoading = getContext("setLoading");

    let retention = $state(24);
    let autoUpdateEnabled = $state(false);
    let autoUpdateInterval = $state(1);
    let highlightKeywords = $state("");
    let saved = $state(false);

    async function loadSettings() {
        try {
            const res = await apiGet("/api/settings");
            if (res.success) {
                retention = parseInt(res.data.retention_hours) || 24;
                autoUpdateEnabled = res.data.auto_update_enabled === "true";
                autoUpdateInterval =
                    parseInt(res.data.auto_update_interval_hours) || 1;
                highlightKeywords = res.data.highlight_keywords || "";
            }
        } catch (e) {
            console.error("Failed to load settings:", e);
        }
    }

    async function saveSettings() {
        setLoading(true);
        saved = false;
        try {
            await apiPut("/api/settings", {
                retention_hours: String(retention),
                auto_update_enabled: String(autoUpdateEnabled),
                auto_update_interval_hours: String(autoUpdateInterval),
                highlight_keywords: highlightKeywords,
            });
            saved = true;
            setTimeout(() => (saved = false), 2500);
        } catch (e) {
            console.error("Failed to save settings:", e);
        }
        setLoading(false);
    }

    onMount(loadSettings);
</script>

<div class="settings-page">
    <h2><span class="material-icons">tune</span> {t("menu_settings")}</h2>

    <div class="settings-card">

        <!-- Retention period -->
        <div class="setting-row">
            <span class="setting-label">
                <span class="material-icons">schedule</span>
                {t("settings_retention")}
            </span>
            <div class="input-group">
                <input
                    type="number"
                    min="1"
                    max="720"
                    bind:value={retention}
                    class="num-input"
                />
                <span class="value">
                    {#if retention < 24}
                        {#if retention == 1}{t("settings_retention_hour")}{:else}{t("settings_retention_hours")}{/if}
                    {:else if retention % 24 === 0}
                        {t("settings_retention_hours")} ({Math.floor(retention / 24)}
                        {#if Math.floor(retention / 24) == 1}{t("settings_retention_day")}{:else}{t("settings_retention_days")}{/if})
                    {:else}
                        {t("settings_retention_hours")} ({Math.floor(retention / 24)}
                        {#if Math.floor(retention / 24) == 1}{t("settings_retention_day")}{:else}{t("settings_retention_days")}{/if}
                        {t("settings_and")} {retention % 24}
                        {#if retention % 24 == 1}{t("settings_retention_hour")}{:else}{t("settings_retention_hours")}{/if})
                    {/if}
                </span>
            </div>
        </div>

        <!-- Highlight keywords -->
        <div class="setting-row">
            <span class="setting-label">
                <span class="material-icons">highlight</span>
                {t('settings_keywords')}
            </span>
            <div class="setting-hint">
                <span class="material-icons">info</span>
                <p>{t('settings_keywords_desc')}</p>
            </div>
            <input
                type="text"
                bind:value={highlightKeywords}
                placeholder={t('settings_keywords_placeholder')}
                class="keywords-input"
            />
        </div>

        <!-- Auto-update toggle -->
        <div class="setting-row">
            <span class="setting-label">
                <span class="material-icons">sync</span>
                {t("settings_auto_update")}
            </span>
            <div class="input-group">
                <label class="toggle">
                    <input type="checkbox" bind:checked={autoUpdateEnabled} />
                    <span class="slider"></span>
                </label>
                <span class="value"
                    >{autoUpdateEnabled
                        ? ""
                        : t("settings_auto_update_disabled")}</span
                >
            </div>
        </div>

        <!-- Auto-update interval (only if enabled) -->
        {#if autoUpdateEnabled}
            <div class="setting-row">
                <span class="setting-label">
                    <span class="material-icons">timer</span>
                    {t("settings_auto_update_interval")}
                </span>
                <div class="input-group">
                    <input
                        type="number"
                        min="1"
                        max="72"
                        bind:value={autoUpdateInterval}
                        class="num-input"
                    />
                    <span class="value">{t("settings_retention_hours")}</span>
                </div>
            </div>
        {/if}

        <button class="save-btn" onclick={saveSettings} disabled={saved}>
            {#if saved}
                👍
            {:else}
                <span class="material-icons">save</span>
                {t("save")}
            {/if}
        </button>
    </div>
</div>

<style>
    .settings-page {
        max-width: 640px;
    }

    h2 {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        font-size: 1.15rem;
        font-weight: 600;
        margin-bottom: 1.25rem;
        color: #1a1a1a;
    }

    h2 .material-icons {
        font-size: 1.3rem;
        color: #ff6b35;
    }

    .settings-card {
        background: #fff;
        border: 1px solid #e0e0e0;
        border-radius: 8px;
        padding: 1.5rem;
        display: flex;
        flex-direction: column;
        gap: 1.25rem;
    }

    .setting-row {
        display: flex;
        flex-direction: column;
        gap: 0.4rem;
    }

    .setting-label {
        display: flex;
        align-items: center;
        gap: 0.4rem;
        font-size: 0.88rem;
        font-weight: 500;
        color: #333;
    }

    .setting-label .material-icons {
        font-size: 1.1rem;
        color: #888;
    }

    .keywords-input {
        width: 100%;
        padding: 0.45rem 0.65rem;
        border: 1px solid #d0d0d0;
        border-radius: 5px;
        font-size: 0.85rem;
        font-family: inherit;
        background: #fafafa;
    }

    .keywords-input:focus {
        border-color: #ff6b35;
        outline: none;
    }

    .input-group {
        display: flex;
        align-items: center;
        gap: 0.75rem;
    }

    .num-input {
        width: 70px;
        padding: 0.35rem 0.5rem;
        border: 1px solid #d0d0d0;
        border-radius: 5px;
        font-size: 0.85rem;
        font-family: "IBM Plex Mono", monospace;
    }

    .value {
        font-size: 0.82rem;
        color: #888;
        font-family: "IBM Plex Mono", monospace;
        min-width: 100px;
    }

    /* Toggle switch */
    .toggle {
        position: relative;
        display: inline-block;
        width: 40px;
        height: 22px;
    }
    .toggle input {
        opacity: 0;
        width: 0;
        height: 0;
    }
    .slider {
        position: absolute;
        cursor: pointer;
        inset: 0;
        background: #ccc;
        border-radius: 22px;
        transition: background 0.2s;
    }
    .slider::before {
        content: "";
        position: absolute;
        height: 16px;
        width: 16px;
        left: 3px;
        bottom: 3px;
        background: white;
        border-radius: 50%;
        transition: transform 0.2s;
    }
    .toggle input:checked + .slider {
        background: #ff6b35;
    }
    .toggle input:checked + .slider::before {
        transform: translateX(18px);
    }

    .save-btn {
        display: flex;
        align-items: center;
        gap: 0.4rem;
        padding: 0.5rem 1.25rem;
        background: #1a1a1a;
        color: #fff;
        border: none;
        border-radius: 6px;
        font-size: 0.85rem;
        font-family: inherit;
        cursor: pointer;
        align-self: flex-start;
        transition: background 0.15s;
    }

    .save-btn:hover {
        background: #333;
    }
    .save-btn .material-icons {
        font-size: 1rem;
    }

    .setting-hint {
        display: flex;
        gap: 0.5rem;
        padding: 0.6rem 0.8rem;
        background: #fdf8e8;
        border: 1px solid #e8ddb5;
        border-radius: 5px;
        align-items: flex-start;
    }

    .setting-hint .material-icons {
        font-size: 1rem;
        color: #b8860b;
        flex-shrink: 0;
        margin-top: 0.1rem;
    }

    .setting-hint p {
        font-size: 0.78rem;
        color: #6b5a1e;
        line-height: 1.5;
        margin: 0;
    }
</style>
