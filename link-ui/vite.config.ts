import { defineConfig } from 'vite';
import { svelte, vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import themeInit from '@trackly/design/vite-plugin';

// Trackly Link UI: отдельная сборка рядом с ui/ (D-18). Порт dev-сервера 1421
// совпадает с `build.devUrl` в crates/link-app/tauri.conf.json.
// Пакеты @trackly/* разрешаются через workspace-зависимости (packages/README.md),
// без алиасов.
export default defineConfig({
  plugins: [
    svelte({
      // Токены дизайн-системы подключаются глобально через global.scss в main.ts.
      // scss.prependData сознательно НЕ используется: Svelte скоупит <style>
      // компонента и «захэшировал» бы :root { --var } — переменные не дошли бы до <html>.
      preprocess: vitePreprocess(),
    }),
    themeInit(),
  ],
  clearScreen: false,
  envPrefix: ['VITE_', 'TAURI_'],
  server: {
    port: 1421,
    strictPort: true,
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
});
