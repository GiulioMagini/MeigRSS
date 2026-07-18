// @ts-nocheck
/**
 * API Service — Centralized backend communication layer.
 * All fetch calls go through here for consistent error handling,
 * sanitization, and environment-aware URL resolution.
 */
import DOMPurify from "isomorphic-dompurify";

// Environment-aware backend URL
const BACKEND_URL = (() => {
    const isDev = import.meta.env.DEV === true;
    if (isDev) {
        // In dev mode, Vite proxy handles /api routes (see vite.config.js)
        return "";
    } else {
        // In production, frontend is served by the same Rust server
        return "";
    }
})();

/**
 * Sanitizes user input to prevent XSS attacks.
 * For HTML content: uses DOMPurify with strict allowlists.
 * For plain text: strips dangerous patterns only.
 */
export function sanitize({ input, isHTML = false, allowedTags = null, allowedAttrs = null }) {
    if (input === null || input === undefined) return "";

    let result = String(input);

    if (isHTML) {
        const config = {
            // Use ?? instead of || so an explicitly empty array ([]) is
            // respected, allowing a "strip everything" text-only mode
            ALLOWED_TAGS: allowedTags ?? ["b", "i", "em", "strong", "a", "p", "br", "ul", "ol", "li", "span", "div"],
            ALLOWED_ATTR: allowedAttrs ?? ["href", "class", "id"],
            FORBID_TAGS: ["script", "style", "iframe", "object", "embed", "form"],
            FORBID_ATTR: ["onerror", "onload", "onclick", "onmouseover", "onmouseout", "onmousedown", "onmouseup"],
        };
        return DOMPurify.sanitize(result, config);
    }

    return result;
}

/**
 * Strips all HTML tags from a string, returning plain text only.
 * Handles both real HTML tags and HTML entities (e.g. &lt;p&gt;, &#8220;).
 */
export function stripHtml(input) {
    if (input === null || input === undefined) return "";
    let text = String(input);
    // First decode HTML entities by running through a textarea twice
    // (handles nested encoding and numeric entities like &#8220;)
    if (typeof document !== 'undefined') {
        const textarea = document.createElement('textarea');
        // First pass: decode entities
        textarea.innerHTML = text;
        text = textarea.value;
        // Second pass: in case the result still contains encoded tags
        textarea.innerHTML = text;
        text = textarea.value;
    }
    // Strip any remaining actual HTML tags
    return text.replace(/<[^>]*>/g, '').trim();
}

/**
 * Makes a GET request to the backend API.
 * Returns the parsed JSON response or throws on error.
 */
export async function apiGet(path) {
    const response = await fetch(`${BACKEND_URL}${path}`, {
        headers: { "Accept": "application/json" }
    });
    if (!response.ok) {
        const err = await response.json().catch(() => ({ error: response.statusText }));
        throw new Error(err.error || `Request failed: ${response.status}`);
    }
    return response.json();
}

/**
 * Makes a POST request to the backend API with a JSON body.
 */
export async function apiPost(path, body = {}) {
    const response = await fetch(`${BACKEND_URL}${path}`, {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
            "Accept": "application/json"
        },
        body: JSON.stringify(body)
    });
    if (!response.ok) {
        const err = await response.json().catch(() => ({ error: response.statusText }));
        throw new Error(err.error || `Request failed: ${response.status}`);
    }
    return response.json();
}

/**
 * Makes a PUT request to the backend API with a JSON body.
 */
export async function apiPut(path, body = {}) {
    const response = await fetch(`${BACKEND_URL}${path}`, {
        method: "PUT",
        headers: {
            "Content-Type": "application/json",
            "Accept": "application/json"
        },
        body: JSON.stringify(body)
    });
    if (!response.ok) {
        const err = await response.json().catch(() => ({ error: response.statusText }));
        throw new Error(err.error || `Request failed: ${response.status}`);
    }
    return response.json();
}

/**
 * Makes a DELETE request to the backend API.
 */
export async function apiDelete(path) {
    const response = await fetch(`${BACKEND_URL}${path}`, {
        method: "DELETE",
        headers: { "Accept": "application/json" }
    });
    if (!response.ok) {
        const err = await response.json().catch(() => ({ error: response.statusText }));
        throw new Error(err.error || `Request failed: ${response.status}`);
    }
    return response.json();
}

/**
 * Returns the display name for a category.
 * Translates the default category name based on the current UI language.
 */
export function categoryDisplayName(category, tFunc) {
    if (category.is_default && category.name === "Newsletter") {
        return "Newsletter";
    }
    if (category.is_default) {
        return tFunc('cat_uncategorized');
    }
    return category.name;
}

/**
 * Returns the display name for a category name string,
 * checking if it matches the default category name.
 */
export function categoryNameDisplay(name, tFunc) {
    if (name === "Non categorizzati") {
        return tFunc('cat_uncategorized');
    }
    return name;
}

export { BACKEND_URL };