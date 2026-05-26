import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

// Mirrors vite.config.ts so svelte-check (which does NOT go through Vite)
// processes <style lang="scss"> blocks the same way. Design tokens are
// loaded globally via global.scss — see vite.config.ts for the rationale
// against scss.prependData.
export default {
  preprocess: vitePreprocess(),
};
