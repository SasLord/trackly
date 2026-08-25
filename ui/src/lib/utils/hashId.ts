// GAP-8 (39-UAT.md, Прогон 3): cross-section "focus a specific record" deep
// link — `#/devices|printers|cartridges?id=…`. The ONLY precedent for this
// shape in the codebase before this fix was PlacesPage.svelte's own
// `parseIdFromHash` (Plan 14), which also handles that page's localStorage
// persistence — a concern this shared helper deliberately does NOT own, so
// it lives here instead of being imported from PlacesPage.
export function parseIdFromHash(): number | null {
  if (typeof window === 'undefined') return null;
  const hash = window.location.hash;
  const qIdx = hash.indexOf('?');
  if (qIdx === -1) return null;
  const qs = new URLSearchParams(hash.slice(qIdx + 1));
  const raw = qs.get('id');
  if (!raw) return null;
  const n = Number(raw);
  return Number.isInteger(n) ? n : null;
}
