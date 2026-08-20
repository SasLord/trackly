import js from '@eslint/js';
import ts from '@typescript-eslint/eslint-plugin';
import tsParser from '@typescript-eslint/parser';
import svelte from 'eslint-plugin-svelte';
import svelteParser from 'svelte-eslint-parser';

// Flat config required by ESLint 9.

const browserGlobals = {
  document: 'readonly',
  window: 'readonly',
  console: 'readonly',
  localStorage: 'readonly',
  sessionStorage: 'readonly',
  navigator: 'readonly',
  location: 'readonly',
  history: 'readonly',
  setTimeout: 'readonly',
  clearTimeout: 'readonly',
  setInterval: 'readonly',
  clearInterval: 'readonly',
  requestAnimationFrame: 'readonly',
  cancelAnimationFrame: 'readonly',
  fetch: 'readonly',
  URL: 'readonly',
  URLSearchParams: 'readonly',
  crypto: 'readonly',
  matchMedia: 'readonly',
  Event: 'readonly',
  KeyboardEvent: 'readonly',
  FocusEvent: 'readonly',
  MouseEvent: 'readonly',
  HTMLElement: 'readonly',
  HTMLDivElement: 'readonly',
  HTMLButtonElement: 'readonly',
  HTMLInputElement: 'readonly',
  HTMLTextAreaElement: 'readonly',
  HTMLSelectElement: 'readonly',
  HTMLStyleElement: 'readonly',
  HTMLUListElement: 'readonly',
  DOMParser: 'readonly',
  Node: 'readonly',
  MutationObserver: 'readonly',
  ResizeObserver: 'readonly',
  IntersectionObserver: 'readonly',
  Blob: 'readonly',
  FileReader: 'readonly',
  FormData: 'readonly',
  Notification: 'readonly',
  WebSocket: 'readonly',
  SVGRectElement: 'readonly',
  SVGSVGElement: 'readonly',
  btoa: 'readonly',
  MediaQueryListEvent: 'readonly',
  parent: 'readonly',
  HTMLIFrameElement: 'readonly',
  MessageEvent: 'readonly',
};

// Svelte 5 rune globals (available in .svelte.ts and .svelte files)
const svelteRunes = {
  $state: 'readonly',
  $derived: 'readonly',
  $effect: 'readonly',
  $props: 'readonly',
  $bindable: 'readonly',
  $inspect: 'readonly',
  $host: 'readonly',
};

const nodeGlobals = {
  __dirname: 'readonly',
  __filename: 'readonly',
  process: 'readonly',
  require: 'readonly',
  module: 'readonly',
  exports: 'readonly',
};

export default [
  {
    // public/ holds static assets copied verbatim by Vite (e.g. theme-init.js,
    // an inline theme bootstrap with no module graph) — not lint-able source.
    ignores: ['node_modules/', 'dist/', 'public/', 'src/bindings.ts', 'pnpm-lock.yaml'],
  },
  js.configs.recommended,
  // Phase 33 (D-04/C-02): standalone Paged.js bootstrap script, inlined via
  // Vite's `?raw` import into the print-preview iframe's srcdoc. Plain
  // browser script (no import/export, so sourceType: 'script'), kept under
  // src/ (unlike public/theme-init.js's "no module graph" precedent above)
  // because its raw text is consumed at build time by pagedPreviewBootstrap.ts.
  {
    files: ['src/lib/pdfPreview/bootstrapScript.js'],
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: 'script',
      globals: browserGlobals,
    },
  },
  // Node/config files
  {
    files: ['vite.config.ts', 'svelte.config.js', 'eslint.config.js', 'scripts/**/*.mjs'],
    languageOptions: {
      parser: tsParser,
      ecmaVersion: 2022,
      sourceType: 'module',
      globals: {
        ...nodeGlobals,
        ...browserGlobals,
      },
    },
    plugins: { '@typescript-eslint': ts },
    rules: {
      ...ts.configs.recommended.rules,
    },
  },
  // TypeScript source files (including .svelte.ts rune modules)
  {
    files: ['**/*.ts'],
    languageOptions: {
      parser: tsParser,
      ecmaVersion: 2022,
      sourceType: 'module',
      globals: {
        ...browserGlobals,
        ...svelteRunes,
      },
    },
    plugins: { '@typescript-eslint': ts },
    rules: {
      ...ts.configs.recommended.rules,
      // The base rule misreports parameter names in TS function-type annotations
      // (e.g. `onChange: (months: number) => void`) as unused. Defer entirely to
      // @typescript-eslint/no-unused-vars, which understands type positions.
      'no-unused-vars': 'off',
      '@typescript-eslint/no-unused-vars': [
        'error',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
      ],
    },
  },
  // Svelte component files
  ...svelte.configs['flat/recommended'],
  {
    files: ['**/*.svelte'],
    languageOptions: {
      parser: svelteParser,
      parserOptions: {
        parser: tsParser,
      },
      globals: {
        ...browserGlobals,
        ...svelteRunes,
      },
    },
    plugins: { '@typescript-eslint': ts },
    rules: {
      // See the .ts block: defer to the typescript-eslint variant so TS
      // function-type parameter names are not misreported as unused.
      'no-unused-vars': 'off',
      '@typescript-eslint/no-unused-vars': [
        'error',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
      ],
      // Svelte compile warnings (e.g. state_referenced_locally) are already
      // surfaced by `pnpm svelte-check`; we use them intentionally for
      // initial-prop capture inside {#key} remount blocks. Don't double-fail CI.
      'svelte/valid-compile': ['error', { ignoreWarnings: true }],
    },
  },
];
