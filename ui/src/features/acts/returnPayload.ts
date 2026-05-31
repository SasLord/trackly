// Phase 3.1 Plan 03: ReturnModal payload builder.
//
// PER-ROW SPLIT INVARIANT (W-4): когда applyToAll=false AND any per-row
// condition/location override differs между rows одного act_item_id, output
// MUST split на N separate ActReturnItemDto, каждый с device_ids=[single_id]
// и собственным override. NEVER collapse с «first row wins».
//
// Когда все checked rows одного act_item_id имеют ОДИНАКОВЫЕ override values —
// coalesce в один ActReturnItemDto с device_ids=[все ids].
//
// Покрыто tests/returnPayload.test.ts (5 cases — composite key splitting +
// coalesce при identical overrides).

import type { ActReturnItemDto } from '../../bindings';
import type { ReturnRowState } from './ReturnItemsTable.svelte';

/**
 * Build ActReturnItemDto[] from per-device-id ReturnRowState[].
 *
 * @example
 * ```
 * // PER-ROW SPLIT: 3 rows одного act_item с разными overrides → 3 separate items.
 * const rows = [
 *   { actItemId: 1, deviceId: 10, conditionOverride: 'A', locationOverrideName: 'X', checked: true, deviceLabel: '' },
 *   { actItemId: 1, deviceId: 11, conditionOverride: 'B', locationOverrideName: 'X', checked: true, deviceLabel: '' },
 *   { actItemId: 1, deviceId: 12, conditionOverride: 'A', locationOverrideName: 'X', checked: true, deviceLabel: '' },
 * ];
 * buildReturnItems(rows, false);
 * // Result:
 * // [
 * //   { act_item_id: 1, device_ids: [10, 12], condition_override: 'A', location_name_override: 'X', ... },
 * //   { act_item_id: 1, device_ids: [11], condition_override: 'B', location_name_override: 'X', ... },
 * // ]
 * // → rows 0+2 (cond='A') coalesce; row 1 (cond='B') gets its own item.
 *
 * // COALESCE: identical overrides → single item с device_ids array.
 * buildReturnItems([
 *   { actItemId: 1, deviceId: 10, conditionOverride: 'A', locationOverrideName: 'X', checked: true, deviceLabel: '' },
 *   { actItemId: 1, deviceId: 11, conditionOverride: 'A', locationOverrideName: 'X', checked: true, deviceLabel: '' },
 * ], false);
 * // Result: [{ act_item_id: 1, device_ids: [10, 11], condition_override: 'A', location_name_override: 'X' }]
 *
 * // APPLY_TO_ALL: groups by act_item_id, overrides=null (backend uses bulk).
 * buildReturnItems([
 *   { actItemId: 1, deviceId: 10, conditionOverride: null, locationOverrideName: '', checked: true, deviceLabel: '' },
 *   { actItemId: 2, deviceId: 20, conditionOverride: null, locationOverrideName: '', checked: true, deviceLabel: '' },
 * ], true);
 * // Result: [
 * //   { act_item_id: 1, device_ids: [10], condition_override: null, location_name_override: null },
 * //   { act_item_id: 2, device_ids: [20], condition_override: null, location_name_override: null },
 * // ]
 * ```
 */
export function buildReturnItems(
  rows: ReturnRowState[],
  applyToAll: boolean,
): ActReturnItemDto[] {
  const checked = rows.filter((r) => r.checked);
  if (checked.length === 0) return [];

  if (applyToAll) {
    // Group by act_item_id; overrides = null (backend uses bulk).
    const byActItem = new Map<number, number[]>();
    for (const r of checked) {
      const arr = byActItem.get(r.actItemId) ?? [];
      arr.push(r.deviceId);
      byActItem.set(r.actItemId, arr);
    }
    return [...byActItem.entries()].map(([act_item_id, device_ids]) => ({
      act_item_id,
      device_id: device_ids[0],
      device_ids,
      quantity: device_ids.length,
      condition_override: null,
      location_id_override: null,
      location_name_override: null,
    }));
  }

  // applyToAll = false: composite-key grouping сохраняет per-row distinctions.
  // PER-ROW SPLIT INVARIANT (W-4): never "first row wins".
  const key = (r: ReturnRowState) =>
    `${r.actItemId}|${r.conditionOverride ?? ''}|${r.locationOverrideName.trim()}`;
  const groups = new Map<
    string,
    { aid: number; cond: string | null; loc: string; ids: number[] }
  >();
  // Order-preserving: первое появление группы определяет порядок в output.
  const groupOrder: string[] = [];
  for (const r of checked) {
    const k = key(r);
    let g = groups.get(k);
    if (!g) {
      g = {
        aid: r.actItemId,
        cond: r.conditionOverride,
        loc: r.locationOverrideName.trim(),
        ids: [],
      };
      groups.set(k, g);
      groupOrder.push(k);
    }
    g.ids.push(r.deviceId);
  }
  return groupOrder.map((k) => {
    const g = groups.get(k)!;
    return {
      act_item_id: g.aid,
      device_id: g.ids[0],
      device_ids: g.ids,
      quantity: g.ids.length,
      condition_override: g.cond,
      location_id_override: null,
      location_name_override: g.loc.length > 0 ? g.loc : null,
    };
  });
}
