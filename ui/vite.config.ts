import { defineConfig } from 'vite';
import { svelte, vitePreprocess } from '@sveltejs/vite-plugin-svelte';

// Trackly UI: vanilla Svelte 5 SPA. Served by Tauri webview (desktop) and by
// the axum static-file handler in server mode (Phase 5). Vite dev port 1420
// matches `tauri.conf.json` `build.devUrl`.
export default defineConfig({
  plugins: [
    svelte({
      preprocess: vitePreprocess({
        scss: {
          // Design tokens auto-imported into every <style lang="scss"> block.
          prependData: '@use "src/styles/_tokens.scss" as *;',
        },
      }),
    }),
  ],
  clearScreen: false,
  envPrefix: ['VITE_', 'TAURI_'],
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
});
