// .svelte.ts extension REQUIRED — Svelte 5 runes are only processed in .svelte/.svelte.ts files.

type Resolved = 'light' | 'dark';
type Preference = 'light' | 'dark' | 'system';

export const themeStore = $state({
  preference: 'system' as Preference,
  resolved: 'light' as Resolved,
});

const KEY = 'trackly:theme';
const mql =
  typeof window !== 'undefined' ? window.matchMedia('(prefers-color-scheme: dark)') : null;

export function initTheme(): void {
  const saved = (localStorage.getItem(KEY) ?? 'system') as Preference;
  themeStore.preference = saved;
  applyResolved();
  mql?.addEventListener('change', () => {
    if (themeStore.preference === 'system') applyResolved();
  });
}

export function setTheme(p: Preference): void {
  themeStore.preference = p;
  localStorage.setItem(KEY, p);
  applyResolved();
}

function applyResolved(): void {
  const r: Resolved =
    themeStore.preference === 'system' ? (mql?.matches ? 'dark' : 'light') : themeStore.preference;
  themeStore.resolved = r;
  document.documentElement.classList.add('theme-switching');
  document.documentElement.dataset.theme = r;
  requestAnimationFrame(() => {
    document.documentElement.classList.remove('theme-switching');
  });
}
