<!-- @ts-nocheck -->
<script>
    import { onMount, setContext } from "svelte";
    import { sanitize, apiPut } from "$lib/api.js";
    import {
        t,
        init as initI18n,
        setLanguage,
        getCurrentLang,
        SUPPORTED,
    } from "$lib/i18n.svelte.js";

    let { children } = $props();

    // Loading state shared across all components
    let isLoading = $state(true);

    // Menu state
    let activeMenu = $state(null);

    // Share context with all child components
    setContext("t", t);
    setContext("sanitize", sanitize);
    setContext("isLoading", () => isLoading);
    setContext("setLoading", (v) => {
        isLoading = v;
    });

    // Toggle menu dropdown
    function toggleMenu(menu) {
        activeMenu = activeMenu === menu ? null : menu;
    }

    // Close menu when clicking outside
    function handleClickOutside(e) {
        if (!e.target.closest(".menu-bar")) {
            activeMenu = null;
        }
    }

    // Language change
    async function changeLanguage(e) {
        await setLanguage(e.target.value);
        try {
            await apiPut("/api/settings/language", { language: e.target.value });
        } catch (err) {
            console.error("Failed to save language setting:", err);
        }
    }

    onMount(() => {
        // Registered synchronously so the cleanup can be returned to Svelte
        document.addEventListener("click", handleClickOutside);

        (async () => {
            await initI18n();
            // Save detected language to backend so console logs match
            try {
                await apiPut("/api/settings/language", { language: getCurrentLang() });
            } catch (e) {
                console.error("Failed to save language setting:", e);
            }
            isLoading = false;
        })();

        return () => document.removeEventListener("click", handleClickOutside);
    });
</script>

<div class="app-shell">
    <!-- Menu bar -->
    <header class="menu-bar">
        <div class="menu-left">
            <a
                href="/"
                class="app-logo"
                onclick={() => {
                    activeMenu = null;
                }}
            >
                <span class="material-icons">rss_feed</span>
                <span class="app-name">MeigRSS</span>
            </a>

            <!-- Feeds menu -->
            <div class="menu-item">
                <button
                    class="menu-trigger"
                    onclick={() => toggleMenu("feeds")}
                >
                    <span class="material-icons">dynamic_feed</span>
                    <span>{t("menu_feeds")}</span>
                </button>
                {#if activeMenu === "feeds"}
                    <div class="menu-dropdown">
                        <a href="/feeds/manage" onclick={() => { activeMenu = null; }}>
                            <span class="material-icons">edit</span>
                            {t("menu_feeds")}
                        </a>
                        <a href="/feeds/import" onclick={() => { activeMenu = null; }}>
                            <span class="material-icons">upload_file</span>
                            {t("menu_import")}
                        </a>
                        <a href="/feeds/import-newsletter" onclick={() => { activeMenu = null; }}>
                            <span class="material-icons">email</span>
                            {t("menu_import_newsletter")}
                        </a>
                        <a href="/feeds/export" onclick={() => { activeMenu = null; }} data-sveltekit-reload>
                            <span class="material-icons">download</span>
                            {t("menu_export")}
                        </a>
                        <hr />
                        <a href="/feeds/delete-all" onclick={() => { activeMenu = null; }} class="danger">
                            <span class="material-icons">delete_forever</span>
                            {t("menu_delete_all")}
                        </a>
                    </div>
                {/if}
            </div>

            <!-- Categories menu -->
            <div class="menu-item">
                <button
                    class="menu-trigger"
                    onclick={() => toggleMenu("categories")}
                >
                    <span class="material-icons">category</span>
                    <span>{t("menu_categories")}</span>
                </button>
                {#if activeMenu === "categories"}
                    <div class="menu-dropdown">
                        <a href="/categories" onclick={() => { activeMenu = null; }}>
                            <span class="material-icons">settings</span>
                            {t("menu_categories")}
                        </a>
                    </div>
                {/if}
            </div>

            <!-- Settings menu -->
            <div class="menu-item">
                <button
                    class="menu-trigger"
                    onclick={() => toggleMenu("settings")}
                >
                    <span class="material-icons">settings</span>
                    <span>{t("menu_settings")}</span>
                </button>
                {#if activeMenu === "settings"}
                    <div class="menu-dropdown">
                        <a href="/settings" onclick={() => { activeMenu = null; }}>
                            <span class="material-icons">tune</span>
                            {t("menu_settings")}
                        </a>
                        <a href="/settings/api" onclick={() => { activeMenu = null; }}>
                            <span class="material-icons">key</span>
                            {t("menu_api_config")}
                        </a>
                    </div>
                {/if}
            </div>
        </div>

        <div class="menu-right">
            <!-- Language selector -->
            <select
                class="lang-select"
                value={getCurrentLang()}
                onchange={changeLanguage}
            >
                {#each SUPPORTED as lang}
                    <option value={lang}>{lang.toUpperCase()}</option>
                {/each}
            </select>
        </div>
    </header>

    <!-- Main content area -->
    <main class="content">
        {#if isLoading}
            <div class="loading-overlay">
                <span class="material-icons spinning">hourglass_empty</span>
                <p>{t("loading")}</p>
            </div>
        {/if}

        {@render children?.()}
    </main>
</div>

<style>
    /* ─── CSS Variables ──────────────────────────────────────────── */
    :global(*) {
        margin: 0;
        padding: 0;
        box-sizing: border-box;
    }

    :global(body) {
        font-family:
            "IBM Plex Sans",
            -apple-system,
            BlinkMacSystemFont,
            sans-serif;
        background: #f5f5f0;
        color: #1a1a1a;
        line-height: 1.5;
    }

    .app-shell {
        min-height: 100vh;
        display: flex;
        flex-direction: column;
    }

    /* ─── Menu Bar ───────────────────────────────────────────────── */
    .menu-bar {
        display: flex;
        align-items: center;
        justify-content: space-between;
        padding: 0 1rem;
        height: 48px;
        background: #1a1a1a;
        color: #e0e0e0;
        font-size: 0.85rem;
        position: sticky;
        top: 0;
        z-index: 100;
        box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
    }

    .menu-left {
        display: flex;
        align-items: center;
        gap: 0;
    }

    .menu-right {
        display: flex;
        align-items: center;
        gap: 0.5rem;
    }

    .app-logo {
        display: flex;
        align-items: center;
        gap: 0.4rem;
        padding: 0 0.75rem 0 0;
        margin-right: 0.5rem;
        border-right: 1px solid #333;
    }

    .app-logo .material-icons {
        color: #ff6b35;
        font-size: 1.3rem;
    }

    a.app-logo {
        text-decoration: none;
        color: inherit;
    }

    .app-name {
        font-weight: 700;
        font-size: 0.95rem;
        letter-spacing: 0.02em;
        color: #fff;
    }

    /* ─── Menu Items ─────────────────────────────────────────────── */
    .menu-item {
        position: relative;
    }

    .menu-trigger {
        display: flex;
        align-items: center;
        gap: 0.3rem;
        padding: 0.4rem 0.65rem;
        background: none;
        border: none;
        color: #c0c0c0;
        cursor: pointer;
        font-size: 0.82rem;
        font-family: inherit;
        border-radius: 4px;
        transition:
            background 0.15s,
            color 0.15s;
    }

    .menu-trigger:hover {
        background: #2a2a2a;
        color: #fff;
    }

    .menu-trigger .material-icons {
        font-size: 1.1rem;
    }

    .menu-dropdown {
        position: absolute;
        top: 100%;
        left: 0;
        min-width: 220px;
        background: #242424;
        border: 1px solid #3a3a3a;
        border-radius: 6px;
        padding: 0.35rem 0;
        box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
        z-index: 200;
    }

    .menu-dropdown a {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.5rem 0.85rem;
        color: #d0d0d0;
        text-decoration: none;
        font-size: 0.82rem;
        transition: background 0.12s;
    }

    .menu-dropdown a:hover {
        background: #333;
        color: #fff;
    }

    .menu-dropdown a.danger {
        color: #ff6b6b;
    }

    .menu-dropdown a.danger:hover {
        background: #3a1a1a;
    }

    .menu-dropdown .material-icons {
        font-size: 1.1rem;
        opacity: 0.7;
    }

    .menu-dropdown hr {
        border: none;
        border-top: 1px solid #3a3a3a;
        margin: 0.25rem 0;
    }

    /* ─── Language Select ────────────────────────────────────────── */
    .lang-select {
        background: #2a2a2a;
        color: #c0c0c0;
        border: 1px solid #3a3a3a;
        border-radius: 4px;
        padding: 0.2rem 0.4rem;
        font-size: 0.78rem;
        font-family: inherit;
        cursor: pointer;
    }

    .lang-select:hover {
        border-color: #555;
    }

    /* ─── Content Area ───────────────────────────────────────────── */
    .content {
        flex: 1;
        padding: 1.5rem 2rem;
        max-width: 1400px;
        width: 100%;
        margin: 0 auto;
        position: relative;
    }

    /* ─── Loading Overlay ────────────────────────────────────────── */
    .loading-overlay {
        position: fixed;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        background: rgba(245, 245, 240, 0.85);
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        z-index: 500;
        gap: 0.75rem;
    }

    .loading-overlay .material-icons {
        font-size: 2rem;
        color: #ff6b35;
    }

    .loading-overlay p {
        font-size: 0.9rem;
        color: #666;
    }

    @keyframes spin {
        from {
            transform: rotate(0deg);
        }
        to {
            transform: rotate(360deg);
        }
    }

    .spinning {
        animation: spin 1.2s linear infinite;
    }
</style>
