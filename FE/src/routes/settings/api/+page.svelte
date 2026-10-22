<!-- API Configuration page: provider, API key, test, plan info -->
<script>
    import { onMount, getContext } from "svelte";
    import { apiGet, apiPost, apiDelete, sanitize } from "#lib/api.js";
    import { t } from "#lib/i18n.svelte.js";

    const setLoading = getContext("setLoading");

    let savedProvider = $state("");
    let selectedProvider = $state("mistral_1000_chars");
    let apiKey = $state("");
    let keyStatus = $state("");
    let supportsUsage = $state(true);

    let testResult = $state(null);   // null | "loading" | "ok" | "error"
    let testError = $state("");
    let planInfo = $state("");
    let planLoading = $state(false);
    let showDeleteConfirm = $state(false);

    // ─── Web Rendering API state ─────────────────────────────────────
    const webRenderingProviders = [
        { value: "scrape_do", label: "Scrape.do" },
        { value: "scraper_api", label: "ScraperAPI" },
        { value: "scrapingbee", label: "ScrapingBee" },
        { value: "scrapfly", label: "Scrapfly" },
        { value: "zenrows", label: "ZenRows" },
    ];

    let wrSavedProvider = $state("");
    let wrSelectedProvider = $state("scrape_do");
    let wrApiKey = $state("");
    let wrKeyStatus = $state("");
    let wrIsConfigured = $state(false);
    let wrTestResult = $state(null); // null | "loading" | "ok" | "error"
    let wrTestError = $state("");
    let wrShowDeleteConfirm = $state(false);

    const wrSavedProviderLabel = $derived(
        webRenderingProviders.find(p => p.value === wrSavedProvider)?.label || wrSavedProvider
    );

    async function loadWebRenderingConfig() {
        try {
            const res = await apiGet("/api/config/web-rendering");
            if (res.success) {
                wrSavedProvider = res.data.provider;
                wrSelectedProvider = res.data.provider;
                wrKeyStatus = res.data.key_status;
                wrIsConfigured = res.data.is_configured;
            }
        } catch (e) {
            console.error("Failed to load web rendering config:", e);
        }
    }

    async function saveAndTestWebRendering() {
        if (!wrApiKey.trim()) return;
        wrTestResult = "loading";
        wrTestError = "";
        try {
            const sanitizedKey = sanitize({ input: wrApiKey.trim() });
            const res = await apiPost("/api/config/web-rendering", {
                provider: wrSelectedProvider,
                api_key: sanitizedKey
            });
            if (res.success) {
                wrTestResult = "ok";
                wrApiKey = "";
                await loadWebRenderingConfig();
            }
        } catch (e) {
            wrTestResult = "error";
            const msg = e.message || "";
            if (msg.startsWith("web_rendering_test_failed:")) {
                wrTestError = t('web_rendering_test_fail') + msg.split(":").slice(1).join(":");
            } else {
                wrTestError = msg;
            }
        }
    }

    async function deleteWebRenderingConfig() {
        setLoading(true);
        wrShowDeleteConfirm = false;
        try {
            await apiDelete("/api/config/web-rendering");
            wrSavedProvider = "";
            wrKeyStatus = "";
            wrSelectedProvider = "scrape_do";
            wrApiKey = "";
            wrTestResult = null;
            wrIsConfigured = false;
        } catch (e) {
            console.error("Failed to delete web rendering config:", e);
        }
        setLoading(false);
    }

    // ─── AI API state ─────────────────────────────────────
    const providers = [
        { value: "mistral_500_chars", label: "Mistral 500 chars" },
        { value: "mistral_1000_chars", label: "Mistral 1000 chars" },
        { value: "openai", label: "OpenAI API" },
        { value: "claude", label: "Claude API" },
        { value: "gemini", label: "Gemini API" },
    ];

    // Label for the currently SAVED provider (shown in the green status box)
    const savedProviderLabel = $derived(
        providers.find(p => p.value === savedProvider)?.label || savedProvider
    );

    async function loadConfig() {
        try {
            const res = await apiGet("/api/config/api");
            if (res.success) {
                savedProvider = res.data.provider;
                selectedProvider = res.data.provider;
                keyStatus = res.data.key_status;
                supportsUsage = res.data.supports_usage_query;
            }
        } catch (e) {
            console.error("Failed to load API config:", e);
        }
    }

    async function saveAndTest() {
        if (!apiKey.trim()) return;
        testResult = "loading";
        testError = "";
        try {
            const sanitizedKey = sanitize({ input: apiKey.trim() });
            const res = await apiPost("/api/config/api", {
                provider: selectedProvider,
                api_key: sanitizedKey
            });
            if (res.success) {
                testResult = "ok";
                keyStatus = "";
                apiKey = "";
                await loadConfig();
            }
        } catch (e) {
            testResult = "error";
            testError = e.message;
        }
    }

    async function fetchPlanInfo() {
        planLoading = true;
        planInfo = "";
        try {
            const res = await apiPost("/api/config/api/plan-info");
            if (res.success) {
                planInfo = res.data;
            }
        } catch (e) {
            planInfo = `Error: ${e.message}`;
        }
        planLoading = false;
    }

    async function deleteApiConfig() {
        setLoading(true);
        showDeleteConfirm = false;
        try {
            await apiDelete("/api/config/api");
            savedProvider = "";
            keyStatus = "";
            selectedProvider = "mistral_1000_chars";
            apiKey = "";
            testResult = null;
            planInfo = "";
            supportsUsage = true;
        } catch (e) {
            console.error("Failed to delete API config:", e);
        }
        setLoading(false);
    }

    onMount(async () => {
        await loadConfig();
        await loadWebRenderingConfig();
    });
</script>

<div class="api-page">
    <h2><span class="material-icons">key</span> {t('menu_api_config')}</h2>

    <div class="api-card">
        <!-- Provider selection -->
        <div class="field">
            <label for="api-provider-select">{t('api_provider')}</label>
            <select id="api-provider-select" bind:value={selectedProvider} class="input">
                {#each providers as p}
                    <option value={p.value}>{p.label}</option>
                {/each}
            </select>
        </div>

        <!-- Current key status -->
        {#if keyStatus}
            <div class="key-status">
                <span class="material-icons">vpn_key</span>
                <code>{keyStatus.includes("...") ? `${savedProviderLabel} — ${keyStatus}` : keyStatus}</code>
            </div>
        {/if}

        <!-- API Key input -->
        <div class="field">
            <label for="api-key-input">{t('api_key_field')}</label>
            <input id="api-key-input" type="password" bind:value={apiKey} placeholder="mgrss..." class="input" />
        </div>

        <!-- Save & Test button -->
        <button class="primary-btn" onclick={saveAndTest} disabled={!apiKey.trim() || testResult === 'loading'}>
            <span class="material-icons">
                {#if testResult === 'loading'}hourglass_empty
                {:else if testResult === 'ok'}check_circle
                {:else if testResult === 'error'}error
                {:else}save{/if}
            </span>
            {#if testResult === 'loading'}{t('api_testing')}
            {:else if testResult === 'ok'}{t('api_test_ok')}
            {:else if testResult === 'error'}{t('api_test_fail')}
            {:else}{t('api_save_test')}{/if}
        </button>

        {#if testResult === 'error' && testError}
            <p class="error-msg">{testError}</p>
        {/if}

        <!-- Plan info section -->
        {#if supportsUsage}
            <hr />
            <button class="secondary-btn" onclick={fetchPlanInfo} disabled={planLoading}>
                <span class="material-icons">{planLoading ? 'hourglass_empty' : 'info'}</span>
                {planLoading ? t('api_plan_loading') : t('api_plan_info')}
            </button>

            {#if planInfo}
                <textarea class="plan-output" readonly>{planInfo}</textarea>
                <p class="plan-disclaimer">
                    <span class="material-icons">info</span>
                    {t('api_plan_disclaimer')}
                </p>
            {/if}
        {/if}

        <!-- Delete API config -->
        {#if keyStatus && keyStatus !== "Non configurata" && keyStatus !== "Not configured"}
            <hr />
            {#if showDeleteConfirm}
                <div class="delete-confirm">
                    <p class="delete-warning">{t('api_delete_confirm')}</p>
                    <div class="btn-row">
                        <button class="danger-btn" onclick={deleteApiConfig}>
                            <span class="material-icons">delete_forever</span>
                            {t('confirm_yes')}
                        </button>
                        <button class="cancel-btn" onclick={() => showDeleteConfirm = false}>
                            {t('confirm_no')}
                        </button>
                    </div>
                </div>
            {:else}
                <button class="delete-btn" onclick={() => showDeleteConfirm = true}>
                    <span class="material-icons">delete</span>
                    {t('api_delete')}
                </button>
            {/if}
        {/if}
    </div>
    <!-- Web Rendering API section -->
    <h2 style="margin-top: 2rem;"><span class="material-icons">public</span> {t('web_rendering_section_title')}</h2>

    <div class="api-card">
        <!-- Provider selection -->
        <div class="field">
            <label for="wr-provider-select">{t('web_rendering_provider')}</label>
            <select id="wr-provider-select" bind:value={wrSelectedProvider} class="input">
                {#each webRenderingProviders as p}
                    <option value={p.value}>{p.label}</option>
                {/each}
            </select>
        </div>

        <!-- Current key status -->
        {#if wrKeyStatus}
            <div class="key-status">
                <span class="material-icons">vpn_key</span>
                <code>{wrKeyStatus.includes("...") ? `${wrSavedProviderLabel} — ${wrKeyStatus}` : wrKeyStatus}</code>
            </div>
        {/if}

        <!-- API Key input -->
        <div class="field">
            <label for="wr-key-input">{t('api_key_field')}</label>
            <input id="wr-key-input" type="password" bind:value={wrApiKey} placeholder="key..." class="input" />
        </div>

        <!-- Save & Test button -->
        <button class="primary-btn" onclick={saveAndTestWebRendering} disabled={!wrApiKey.trim() || wrTestResult === 'loading'}>
            <span class="material-icons">
                {#if wrTestResult === 'loading'}hourglass_empty
                {:else if wrTestResult === 'ok'}check_circle
                {:else if wrTestResult === 'error'}error
                {:else}save{/if}
            </span>
            {#if wrTestResult === 'loading'}{t('web_rendering_testing')}
            {:else if wrTestResult === 'ok'}{t('web_rendering_test_ok')}
            {:else if wrTestResult === 'error'}{t('web_rendering_test_fail')}
            {:else}{t('web_rendering_save_test')}{/if}
        </button>

        {#if wrTestResult === 'error' && wrTestError}
            <p class="error-msg">{wrTestError}</p>
        {/if}

        <!-- Delete web rendering config -->
        {#if wrIsConfigured}
            <hr />
            {#if wrShowDeleteConfirm}
                <div class="delete-confirm" id="wr-delete-confirm">
                    <p class="delete-warning">{t('web_rendering_delete_confirm')}</p>
                    <div class="btn-row">
                        <button class="danger-btn" onclick={deleteWebRenderingConfig}>
                            <span class="material-icons">delete_forever</span>
                            {t('confirm_yes')}
                        </button>
                        <button class="cancel-btn" onclick={() => wrShowDeleteConfirm = false}>
                            {t('confirm_no')}
                        </button>
                    </div>
                </div>
            {:else}
                <button class="delete-btn" onclick={() => { wrShowDeleteConfirm = true; setTimeout(() => { const el = document.getElementById('wr-delete-confirm'); if (el) el.scrollIntoView({ behavior: 'smooth', block: 'nearest' }); }, 50); }}>
                    <span class="material-icons">delete</span>
                    {t('web_rendering_delete')}
                </button>
            {/if}
        {/if}
    </div>
</div>

<style>
    .api-page { max-width: 560px; }

    h2 {
        display: flex; align-items: center; gap: 0.5rem;
        font-size: 1.15rem; font-weight: 600; margin-bottom: 1.25rem; color: #1a1a1a;
    }
    h2 .material-icons { font-size: 1.3rem; color: #ff6b35; }

    .api-card {
        background: #fff; border: 1px solid #e0e0e0; border-radius: 8px;
        padding: 1.5rem; display: flex; flex-direction: column; gap: 1rem;
    }

    .field { display: flex; flex-direction: column; gap: 0.3rem; }
    .field label { font-size: 0.85rem; font-weight: 500; color: #555; }

    .input {
        padding: 0.45rem 0.65rem; border: 1px solid #d0d0d0; border-radius: 5px;
        font-size: 0.88rem; font-family: inherit; background: #fafafa;
    }
    .input:focus { border-color: #ff6b35; outline: none; box-shadow: 0 0 0 2px rgba(255, 107, 53, 0.12); }

    .key-status {
        display: flex; align-items: center; gap: 0.5rem;
        padding: 0.5rem 0.75rem; background: #f0f7f0; border-radius: 5px;
        font-size: 0.82rem; color: #2d7a2d;
    }
    .key-status .material-icons { font-size: 1rem; }
    .key-status code { font-family: 'IBM Plex Mono', monospace; font-size: 0.82rem; }

    .primary-btn {
        display: flex; align-items: center; gap: 0.4rem;
        padding: 0.5rem 1.25rem; background: #1a1a1a; color: #fff;
        border: none; border-radius: 6px; font-size: 0.85rem;
        font-family: inherit; cursor: pointer; align-self: flex-start;
        transition: background 0.15s;
    }
    .primary-btn:hover:not(:disabled) { background: #333; }
    .primary-btn:disabled { opacity: 0.5; cursor: not-allowed; }
    .primary-btn .material-icons { font-size: 1rem; }

    .secondary-btn {
        display: flex; align-items: center; gap: 0.4rem;
        padding: 0.4rem 1rem; background: #fff; color: #555;
        border: 1px solid #d0d0d0; border-radius: 6px; font-size: 0.82rem;
        font-family: inherit; cursor: pointer; align-self: flex-start;
        transition: all 0.15s;
    }
    .secondary-btn:hover:not(:disabled) { border-color: #ff6b35; color: #ff6b35; }
    .secondary-btn:disabled { opacity: 0.5; cursor: not-allowed; }
    .secondary-btn .material-icons { font-size: 1rem; }

    .error-msg {
        font-size: 0.8rem; color: #dc3545;
        padding: 0.4rem 0.6rem; background: #fef0f0; border-radius: 4px;
    }

    hr { border: none; border-top: 1px solid #eee; margin: 0.25rem 0; }

    .plan-output {
        width: 100%; min-height: 140px; padding: 0.75rem;
        font-family: 'IBM Plex Mono', monospace; font-size: 0.78rem;
        border: 1px solid #e0e0e0; border-radius: 5px; background: #fafaf6;
        resize: vertical; color: #333;
    }

    .plan-disclaimer {
        display: flex;
        align-items: flex-start;
        gap: 0.4rem;
        font-size: 0.75rem;
        color: #888;
        font-style: italic;
        margin: 0;
    }
    .plan-disclaimer .material-icons {
        font-size: 0.9rem;
        flex-shrink: 0;
        margin-top: 0.05rem;
    }

    .delete-btn {
        display: flex;
        align-items: center;
        gap: 0.4rem;
        padding: 0.4rem 1rem;
        background: #fff;
        color: #dc3545;
        border: 1px solid #dc3545;
        border-radius: 6px;
        font-size: 0.82rem;
        font-family: inherit;
        cursor: pointer;
        align-self: flex-start;
        transition: all 0.15s;
    }
    .delete-btn:hover { background: #dc3545; color: #fff; }
    .delete-btn .material-icons { font-size: 1rem; }

    .delete-confirm {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
        padding: 1rem;
        background: #fef0f0;
        border: 1px solid #f0c0c0;
        border-radius: 6px;
    }
    .delete-warning { font-size: 0.88rem; color: #333; margin: 0; }
    .btn-row { display: flex; gap: 0.5rem; }
    .danger-btn {
        display: flex; align-items: center; gap: 0.3rem;
        padding: 0.4rem 1rem; background: #dc3545; color: #fff;
        border: none; border-radius: 5px; font-size: 0.82rem;
        font-family: inherit; cursor: pointer;
    }
    .danger-btn:hover { background: #c82333; }
    .danger-btn .material-icons { font-size: 1rem; }
    .cancel-btn {
        padding: 0.4rem 1rem; background: #fff; color: #555;
        border: 1px solid #d0d0d0; border-radius: 5px;
        font-size: 0.82rem; font-family: inherit; cursor: pointer;
    }
    .cancel-btn:hover { border-color: #999; }
</style>
