// .svelte.ts extension REQUIRED — Svelte 5 runes.
//
// Phase 40 Plan 32 (UAT3-03, 40-HUMAN-UAT.md): a general-purpose "content of
// these places changed" signal, module-level $state store, mirroring the
// toast.svelte.ts pattern (plain importable functions, no context/provider
// boilerplate).
//
// Two decisions fixed here on purpose:
//
// (a) [ОТМЕНЕНО Фазой 41.7 (D-05/D-17) в части межклиентской синхронизации.]
//     Раньше здесь стояло «это НЕ WebSocket»: инвалидация решала только
//     проблему устаревания кэша ТЕКУЩЕГО клиента. Теперь поверх неё лежит
//     мост: WS-событие `entities_changed` (его открывает Layout.svelte)
//     вызывает `notifyEntitiesChanged`. Локальные вызовы
//     `notifyPlaceContentChanged` НЕ удалены: это быстрый путь автора и
//     единственный путь при разрыве сокета (broadcast без replay).
//     У автора возможен двойной refetch — принято (D-05).
//
//     Две функции разделены намеренно:
//       - `notifyPlaceContentChanged(placeIds)` — локальная; пустой список —
//         нет-оп (часть ~20 вызовов не защищена проверкой length > 0);
//       - `notifyEntitiesChanged(placeIds)` — только для WS-моста; ВСЕГДА
//         поднимает `reloadSeq` (перезагрузить дерево мест, состав места,
//         дерево групп и панели групп), а при непустом списке ещё и
//         вытесняет счётчики затронутых мест. Пустой `placeIds` из WS =
//         «перезагрузить всё», а не «ничего не делать».
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

export const placeContentEventsStore = $state<{
  seq: number;
  placeIds: number[];
  reloadSeq: number;
}>({
  seq: 0,
  placeIds: [],
  reloadSeq: 0,
});

export function notifyPlaceContentChanged(placeIds: number[]): void {
  if (placeIds.length === 0) return;
  placeContentEventsStore.seq += 1;
  placeContentEventsStore.placeIds = placeIds;
}

// Фаза 41.7 (D-17): реактивный доступ к reloadSeq функцией. Нужен PlaceTree:
// гейт INV-1 (check-place-tree-invalidation.mjs) допускает там РОВНО ОДИН
// $effect, упоминающий placeContentEventsStore (эффект вытеснения statsCache),
// поэтому эффект загрузки дерева читает счётчик через эту функцию.
export function reloadSeqNow(): number {
  return placeContentEventsStore.reloadSeq;
}

// Фаза 41.7 (D-17): вызывается только WS-мостом (Layout.svelte).
export function notifyEntitiesChanged(placeIds: number[]): void {
  placeContentEventsStore.reloadSeq += 1;
  if (placeIds.length > 0) notifyPlaceContentChanged(placeIds);
}
