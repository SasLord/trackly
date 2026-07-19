// .svelte.ts extension REQUIRED — Svelte 5 runes are only processed in .svelte/.svelte.ts files.
// Mobile drawer state for Layout.svelte / PageHeader.svelte (Phase 26, план 01).
// No localStorage persistence — drawer state resets per session (UI-SPEC §6.3).

export const sidebarNav = $state({ open: false });

export function openNav(): void {
  sidebarNav.open = true;
}

export function closeNav(): void {
  sidebarNav.open = false;
}
