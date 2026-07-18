// @ts-nocheck
/**
 * i18n Store — Manages UI translations loaded from the backend.
 * Detects browser language, loads the appropriate translation map,
 * and provides a reactive t() function for all components.
 */
import { apiGet } from "$lib/api.js";

// Supported languages (ISO 639-1). The layout language selector renders
// one option per entry automatically, so adding a code here is enough.
const SUPPORTED = ["it", "en", "fr", "de", "es", "pt"];

// Detect browser language, default to English
function detectLanguage() {
    if (typeof navigator === "undefined") return "en";
    const lang = navigator.language.split("-")[0];
    return SUPPORTED.includes(lang) ? lang : "en";
}

// Reactive state
let currentLang = $state(detectLanguage());
let translations = $state({});
let isLoaded = $state(false);

// Cache to avoid re-fetching
const cache = {};

/**
 * Loads translations for the given language from the backend.
 */
async function loadTranslations(lang) {
    if (cache[lang]) {
        translations = cache[lang];
        isLoaded = true;
        return;
    }
    try {
        const response = await apiGet(`/api/i18n/${lang}`);
        if (response.success && response.data) {
            cache[lang] = response.data;
            translations = response.data;
        }
    } catch (e) {
        console.error(`Failed to load translations for "${lang}":`, e);
        // Fallback to English if not already
        if (lang !== "en") {
            await loadTranslations("en");
            return;
        }
    }
    isLoaded = true;
}

/**
 * Translation function. Returns the translated string for the key,
 * or the key itself if not found.
 */
function t(key) {
    return translations[key] || key;
}

/**
 * Changes the current language and reloads translations.
 */
async function setLanguage(lang) {
    currentLang = lang;
    await loadTranslations(lang);
}

/**
 * Initializes the i18n system. Call once on app mount.
 */
async function init() {
    await loadTranslations(currentLang);
}

function getCurrentLang() { return currentLang; }
function getIsLoaded() { return isLoaded; }

export { t, init, setLanguage, getCurrentLang, getIsLoaded, SUPPORTED };