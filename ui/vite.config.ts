import { defineConfig } from 'vite';
import { svelte, vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import path from 'path';

// Trackly UI: vanilla Svelte 5 SPA. Served by Tauri webview (desktop) and by
// the axum static-file handler in server mode (Phase 5). Vite dev port 1420
// matches `tauri.conf.json` `build.devUrl`.
export default defineConfig({
  plugins: [
    svelte({
      // Design tokens live globally in `src/styles/_tokens.scss` and are
      // imported once via `global.scss`. We deliberately do NOT use
      // scss.prependData — svelte scopes component <style> blocks, which
      // would hash any `:root { --var: … }` rule injected per component
      // and prevent the CSS variables from applying to <html>.
      preprocess: vitePreprocess(),
    }),
  ],
  resolve: {
    alias: {
      $lib: path.resolve(__dirname, 'src/lib'),
    },
  },
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
