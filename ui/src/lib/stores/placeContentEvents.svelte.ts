// .svelte.ts extension REQUIRED — Svelte 5 runes.
//
// Phase 40 Plan 32 (UAT3-03, 40-HUMAN-UAT.md): a general-purpose "content of
// these places changed" signal, module-level $state store, mirroring the
// toast.svelte.ts pattern (plain importable functions, no context/provider
// boilerplate).
//
// Two decisions fixed here on purpose:
//
// (a) This is NOT a WebSocket. Invalidation solves a same-tab/same-session
//     staleness problem for a cache the CURRENT client already loaded — it
//     is not cross-client sync. Same reasoning as D-29 (40-CONTEXT.md,
//     timeline "live update"): the event is rare and there is no
//     meaningful cross-user race at single-LAN-org scale, so a WebSocket
//     round-trip would be pure overhead for zero benefit.
//
// (b) The mechanism itself is GENERAL — any future producer may import
//     `notifyPlaceContentChanged` to invalidate PlaceTree's per-node
//     content counters. This plan (40-32) wires up exactly ONE producer,
//     the one confirmed broken in live UAT: PlaceContents.svelte's bulk
//     "Перенести всё содержимое в…" move. Other potential staleness
//     sources for the same statsCache (single device/cartridge
//     create/delete/move via PlaceEntityViewModal or device/cartridge
//     lists, cartridge install into a printer, printer-place cascade) are
//     deliberately NOT wired in this plan — this is a conscious scope
//     boundary of this specific gap-closure (see 40-HUMAN-UAT.md UAT3-03),
//     not a rejected/deferred idea per 40-CONTEXT.md's "deferred idea"
//     convention.

export const placeContentEventsStore = $state<{ seq: number; placeIds: number[] }>({
  seq: 0,
  placeIds: [],
});

export function notifyPlaceContentChanged(placeIds: number[]): void {
  if (placeIds.length === 0) return;
  placeContentEventsStore.seq += 1;
  placeContentEventsStore.placeIds = placeIds;
}
