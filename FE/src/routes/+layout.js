// Disable SSR: the app runs entirely client-side.
// This is required for static adapter (embedded in Rust binary).
export const ssr = false;
export const prerender = true;