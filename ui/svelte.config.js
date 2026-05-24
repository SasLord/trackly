import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

// Re-export the same preprocess config used in vite.config.ts so svelte-check
// (which does NOT go through Vite) picks up the SCSS preprocessor + design-token
// auto-import. Without this, svelte-check fails on <style lang="scss"> blocks.
export default {
  preprocess: vitePreprocess({
    scss: {
      prependData: '@use "src/styles/_tokens.scss" as *;',
    },
  }),
};
