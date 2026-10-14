import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
    plugins: [
        sveltekit({
            // Force runes mode on project components only, leaving dependencies untouched
            dynamicCompileOptions: ({ filename }) =>
                filename.includes('node_modules') ? undefined : { runes: true },

            adapter: adapter({
                pages: 'build',
                assets: 'build',
                fallback: 'index.html',
                precompress: false
            })
        })
    ],
    server: {
        port: 5173,
        proxy: {
            '/api': { target: 'http://127.0.0.1:3000', changeOrigin: true }
        }
    }
});
