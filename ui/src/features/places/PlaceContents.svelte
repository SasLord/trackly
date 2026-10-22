<script lang="ts">
  // Phase 39 Plan 20 (PLC-06, 39-UI-SPEC.md §9): the right-panel content screen —
  // breadcrumbs + node header (§9.1), Tabs+"Только здесь" control row (§9.2),
  // sticky-header content table (§9.3). Mounted by PlacesPage per selected node
  // (see that file for the {#key place.id:token} remount contract). `onlyHere`
  // is a CONTROLLED prop owned by PlacesPage (UAT gap 4.3 fix) — it
  // deliberately survives this component's own remounts across ordinary place
  // selection changes, and is reset to false only by PlacesPage's
  // `handleShowBlockedContents` (the D-14 "Показать содержимое" same-node edge
  // case).
  //
  // `place`'s own DTO (`places_get`/`places_list_all`) carries no ancestor chain
  // (confirmed against bindings.ts: PlaceDto = {id, parent_id, kind, name, level,
  // is_storage, sort_order, archived_at_utc, notes, full_path, ...} — full_path is
  // a flat "A / B / C" STRING with no per-segment ids). Breadcrumbs therefore walk
  // `parent_id` up via repeated `places_get` calls (cheap: place-tree depth is a
  // handful of levels, not fetched per row) rather than parsing `full_path`, so
  // each segment carries a real place id for `onSelectAncestor`.
  import { push } from 'svelte-spa-router';
  import { apiCall } from '$lib/api/client';
  import Tabs from '$lib/components/Tabs.svelte';
  import Checkbox from '$lib/components/Checkbox.svelte';
  import Table from '$lib/components/Table.svelte';
  import TableRow from '$lib/components/TableRow.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import PlacePicker from '$lib/components/PlacePicker.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { statusTone } from '$lib/stores/deviceStatuses.svelte';
  import {
    notifyPlaceContentChanged,
    placeContentEventsStore,
  } from '$lib/stores/placeContentEvents.svelte';
  import { pluralizeRu } from '$lib/utils/pluralize';
  import PlaceEntityViewModal from './PlaceEntityViewModal.svelte';
  import type { PlaceContentDto, PlaceDto, SubtreeStatsDto } from '../../bindings';

  export type ContentTab = 'all' | 'device' | 'printer' | 'cartridge';

  interface Props {
    place: PlaceDto;
    onSelectAncestor: (_id: number) => void;
    onlyHere: boolean;
    onOnlyHereChange: (_v: boolean) => void;
    // Phase 41.2 (D-22): display mode "show group members as separate rows",
    // a CONTROLLED pair owned by PlacesPage for the same reason as `onlyHere`
    // (this component is remounted on every node selection). There is no local
    // state for it here.
    expandGroups: boolean;
    onExpandGroupsChange: (_v: boolean) => void;
    // GAP-10 (39-UAT.md, Прогон 5): lifted OUT of this component, same reason
    // and same convention as `onlyHere` above (GAP-1/GAP-5) — this component
    // is remounted via PlacesPage's `{#key place.id:token}` on every
    // selection change, so a local `$state` here would reset on every node
    // switch and never survive a route change away from "Места" at all. The
    // caller (PlacesPage) owns persistence (localStorage) and validation
    // (falls back to 'all' for a stored value that no longer matches one of
    // the four tabs); this component just renders whatever it's given.
    activeTab: ContentTab;
    onActiveTabChange: (_v: ContentTab) => void;
  }

  const {
    place,
    onSelectAncestor,
    onlyHere,
    onOnlyHereChange,
    expandGroups,
    onExpandGroupsChange,
    activeTab,
    onActiveTabChange,
  }: Props = $props();

  // §17.1 (mirrors PlaceTreeNode.svelte's identical map — no shared places-utils
  // module exists yet in this codebase; small const, kept local per that file's
  // own precedent).
  const KIND_LABELS: Record<string, string> = {
    territory: 'территория',
    zone: 'зона',
    building: 'здание',
    floor: 'этаж',
    room: 'помещение',
    outdoor: 'уличный объект',
  };

  const CONTENT_KIND_LABELS: Record<string, string> = {
    device: 'Устройство',
    printer: 'Принтер',
    cartridge: 'Картридж',
  };

  // Устройства и принтеры (Phase 41.1, D-09): тон бейджа решает
  // statusTone(row.status_code) из стора статусов устройств — ключ code, а не
  // русское имя, поэтому переименование статуса тон не теряет. Подпись —
  // row.status_name с сервера. У картриджей status_code всегда null.
  //
  // D-07: статусы картриджей вне справочника устройств (свой справочник,
  // cartridge_statuses, V001); для них остаётся отдельная карта «имя -> тон».
  // Гейт check-device-status-labels освобождает её по правилу [B] только при
  // имени с префиксом CARTRIDGE_ и без слов устройственных статусов.
  const CARTRIDGE_STATUS_VARIANT_BY_NAME: Record<
    string,
    'default' | 'accent' | 'warning' | 'destructive'
  > = {
    'На складе': 'default',
    'В работе': 'accent',
    'На заправке': 'warning',
    Списано: 'destructive',
  };

  function cartridgeStatusVariant(
    name: string | null,
  ): 'default' | 'accent' | 'warning' | 'destructive' {
    if (!name) return 'default';
    return CARTRIDGE_STATUS_VARIANT_BY_NAME[name] ?? 'default';
  }

  // GAP-8 (39-UAT.md, Прогон 3): row click used to navigate straight to the
  // entity's OWN section, where it was invisible among many rows and nothing
  // was highlighted (39-20-PLAN's known limitation — no cross-section
  // deep-link/focus infrastructure existed anywhere in this codebase at that
  // point). Now opens a read-only «Просмотр» popup instead; that popup owns
  // the actual "Перейти к…" cross-section navigation (with focus) and the
  // "Редактировать" real edit modal — see PlaceEntityViewModal.svelte.
  let viewRow = $state<PlaceContentDto | null>(null);
  // Bumped by PlaceEntityViewModal's onChanged (after a successful edit-modal
  // save) to force the rows $effect below to re-fetch — the edited entity's
  // name/place/status may have just changed underneath this table.
  let reloadToken = $state(0);

  function openView(row: PlaceContentDto): void {
    viewRow = row;
  }

  // Phase 41.2 (D-19): a group row has no «Просмотр» popup. The only way out of
  // it is a focus-jump into the groups section — via the name button (the sole
  // keyboard path) or a double click on the row. A single click on the row body
  // is deliberately inert (R-03).
  function openGroup(row: PlaceContentDto): void {
    void push(`#/groups?id=${row.id}`);
  }

  // «(6 устройств)» is assembled on the client (UI-SPEC §11.2): the server sends
  // no glued label, only member_count.
  function deviceCountLabel(n: number): string {
    return `(${n} ${pluralizeRu(n, ['устройство', 'устройства', 'устройств'])})`;
  }

  // UAT gap 7: the keyboard-activatable entry point (role="button"/tabindex=0,
  // TableRow's own focus-ring convention targets `> td:first-child`) used to
  // always be the «Тип» cell. Now that «Тип» is conditionally hidden, this
  // handler is shared so whichever cell renders first (Тип when shown,
  // Название otherwise) stays keyboard-reachable.
  function handleRowActivateKeydown(e: KeyboardEvent, row: PlaceContentDto): void {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      openView(row);
    }
  }

  // --- Breadcrumbs (§9.1 row 1) ---
  let breadcrumbs = $state<PlaceDto[]>([]);

  $effect(() => {
    const parentId = place.parent_id;
    let cancelled = false;
    (async () => {
      const chain: PlaceDto[] = [];
      let cur = parentId;
      try {
        while (cur !== null) {
          const p = await apiCall<PlaceDto>('places_get', { id: cur });
          chain.unshift(p);
          cur = p.parent_id;
        }
      } catch {
        // Best-effort — breadcrumbs simply stop at whatever resolved so far.
      }
      if (!cancelled) breadcrumbs = chain;
    })();
    return () => {
      cancelled = true;
    };
  });

  // --- Content (§9.2/§9.3, D-24) ---
  let rows = $state<PlaceContentDto[] | null>(null);
  let loading = $state(false);
  let loadError = $state(false);

  $effect(() => {
    const rootId = place.id;
    const nested = !onlyHere;
    // `expandGroups` (read in the request below) is a reactive dependency of
    // this effect: toggling the checkbox reloads the rows from the server.
    void reloadToken; // GAP-8: forces a re-fetch after an edit-modal save.
    void placeContentEventsStore.reloadSeq; // Фаза 41.7 (D-17): перезагрузка по событию другого клиента
    let cancelled = false;
    loading = true;
    loadError = false;
    apiCall<PlaceContentDto[]>('places_contents', { rootId, nested, expandGroups })
      .then((r) => {
        if (!cancelled) rows = r;
      })
      .catch(() => {
        if (!cancelled) {
          rows = [];
          loadError = true;
        }
      })
      .finally(() => {
        if (!cancelled) loading = false;
      });
    return () => {
      cancelled = true;
    };
  });

  const counts = $derived.by(() => {
    const all = rows ?? [];
    // D-23 / UI-SPEC §11.5: tab counters are PHYSICAL devices in both modes.
    // A collapsed group row is not one unit — it contributes its member counts,
    // so the numbers do not change when the checkbox is toggled (only the
    // number of rows does). Both counts are null on non-group rows only.
    let total = 0;
    let device = 0;
    let printer = 0;
    let cartridge = 0;
    for (const r of all) {
      if (r.kind === 'group') {
        const members = r.member_count ?? 0;
        const printers = r.printer_member_count ?? 0;
        total += members;
        device += members - printers;
        printer += printers;
      } else {
        total += 1;
        if (r.kind === 'device') device += 1;
        else if (r.kind === 'printer') printer += 1;
        else if (r.kind === 'cartridge') cartridge += 1;
      }
    }
    return { all: total, device, printer, cartridge };
  });

  const tabs = $derived([
    { key: 'all', label: 'Все', count: counts.all > 0 ? counts.all : undefined },
    { key: 'device', label: 'Устройства', count: counts.device > 0 ? counts.device : undefined },
    { key: 'printer', label: 'Принтеры', count: counts.printer > 0 ? counts.printer : undefined },
    {
      key: 'cartridge',
      label: 'Картриджи',
      count: counts.cartridge > 0 ? counts.cartridge : undefined,
    },
  ]);

  // D-21 / R-08: a group row stands on «Все» and on «Устройства» (only when it
  // has non-printer members — a printers-only group never had its devices on
  // that tab). No fifth tab; ContentTab is unchanged.
  const filteredRows = $derived(
    (rows ?? []).filter((r) => {
      if (activeTab === 'all') return true;
      if (r.kind === 'group') {
        return activeTab === 'device' && (r.member_count ?? 0) - (r.printer_member_count ?? 0) > 0;
      }
      return r.kind === activeTab;
    }),
  );

  // §14.2's exact copy pair is defined for a genuinely empty NODE, not an empty
  // TAB filter (e.g. "Принтеры" tab with zero rows while "Устройства" has some).
  // Only show the literal spec copy when the whole node (unfiltered) is empty;
  // otherwise a lighter, still-terse fallback per this project's copywriting
  // contract (§14 — no "Упс", say what happened + what to do).
  const isWholeNodeEmpty = $derived(!loading && !loadError && (rows?.length ?? 0) === 0);
  const isTabFilteredEmpty = $derived(
    !loading && !loadError && (rows?.length ?? 0) > 0 && filteredRows.length === 0,
  );

  // R-09: the one place where a physical tab counter and the row count may
  // legitimately disagree — name the checkbox instead of looking like data loss.
  const emptyTabHint = $derived.by(() => {
    if (expandGroups || !isTabFilteredEmpty) return null;
    const groupRows = (rows ?? []).filter((r) => r.kind === 'group');
    if (activeTab === 'printer' && groupRows.some((r) => (r.printer_member_count ?? 0) > 0)) {
      return 'Принтеры этого места входят в группы — включите «Разворачивать группы», чтобы увидеть их.';
    }
    if (activeTab === 'device' && groupRows.length > 0) {
      return 'Устройства этого места входят в группы — включите «Разворачивать группы», чтобы увидеть их.';
    }
    return null;
  });

  // UAT gap 7 (2026-08-25): «Тип» is redundant once the tab already filters to
  // a single kind — only shown on the «Все» tab. Base columns (Название, Инв.
  // № / Серийный №, Статус) = 3, plus «Тип» when activeTab === 'all', plus
  // «Место» when !onlyHere. Must stay correct across all 4 tabs × 2 toggle
  // combinations or the empty-state row's colspan is wrong.
  const showKindColumn = $derived(activeTab === 'all');
  const columnCount = $derived(3 + (showKindColumn ? 1 : 0) + (!onlyHere ? 1 : 0));

  // --- D-28 bulk move ("Перенести всё содержимое в…", Plan 40-19) ---
  // `rows` above is filtered by the `onlyHere` toggle (nested = !onlyHere), but
  // the backend's `move_subtree_contents` always walks the FULL nested subtree
  // (Plan 40-13) regardless of that toggle. The confirm dialog's {N} must match
  // what actually gets moved, so it fetches its own always-nested count on open
  // rather than reusing `counts.all`, which would under-report when "Только
  // здесь" is on.
  let moveModalOpen = $state(false);
  let moveTargetId = $state<number | null>(null);
  let moving = $state(false);
  let moveCount = $state<number | null>(null);
  let moveCountLoading = $state(false);
  // D-23: server-computed numbers read when the modal opens (places_subtree_stats).
  // moveGroupCount = groups that actually move along (empty ones not counted);
  // moveDeviceCount feeds the success toast — places_move_subtree_contents
  // counts ALL content items (cartridges included), so its result would
  // mislabel the toast. null = stats request failed → generic toast.
  let moveGroupCount = $state<number | null>(null);
  let moveDeviceCount = $state<number | null>(null);

  function openMoveModal(): void {
    moveTargetId = null;
    moveCount = null;
    moveGroupCount = null;
    moveDeviceCount = null;
    moveModalOpen = true;
  }

  function closeMoveModal(): void {
    if (moving) return;
    moveModalOpen = false;
  }

  $effect(() => {
    if (!moveModalOpen) return;
    const rootId = place.id;
    let cancelled = false;
    moveCountLoading = true;
    const contents = apiCall<PlaceContentDto[]>('places_contents', {
      rootId,
      nested: true,
      expandGroups: true,
    })
      .then((r) => {
        if (!cancelled) moveCount = r.length;
      })
      .catch(() => {
        if (!cancelled) moveCount = null;
      });
    const stats = apiCall<SubtreeStatsDto>('places_subtree_stats', { rootId })
      .then((st) => {
        if (cancelled) return;
        moveGroupCount = st.moving_group_count;
        moveDeviceCount = st.device_count;
      })
      .catch(() => {
        if (cancelled) return;
        moveGroupCount = null;
        moveDeviceCount = null;
      });
    void Promise.all([contents, stats]).finally(() => {
      if (!cancelled) moveCountLoading = false;
    });
    return () => {
      cancelled = true;
    };
  });

  async function handleMoveConfirm(): Promise<void> {
    if (moveTargetId === null || moving) return;
    moving = true;
    try {
      await apiCall<number>('places_move_subtree_contents', {
        rootId: place.id,
        targetPlaceId: moveTargetId,
        note: null,
      });
      // UAT3-03: invalidate PlaceTree's content counters for source +
      // target — PlaceTree's own ancestorsAndSelf() extends this to their
      // full parent_id ancestor chains.
      notifyPlaceContentChanged([place.id, moveTargetId]);
      if (moveGroupCount !== null && moveGroupCount > 0 && moveDeviceCount !== null) {
        const devices = `${moveDeviceCount} ${pluralizeRu(moveDeviceCount, ['устройство', 'устройства', 'устройств'])}`;
        const groups = `${moveGroupCount} ${pluralizeRu(moveGroupCount, ['группа', 'группы', 'групп'])}`;
        pushToast('success', `Перенесено: ${devices} и ${groups}`);
      } else {
        pushToast('success', 'Содержимое перенесено');
      }
      moveModalOpen = false;
      reloadToken += 1;
    } catch {
      pushToast('error', 'Не удалось перенести содержимое. Попробуйте ещё раз.');
    } finally {
      moving = false;
    }
  }
</script>

<div class="place-contents">
  <header class="contents-header">
    {#if breadcrumbs.length > 0}
      <nav class="breadcrumbs" aria-label="Путь">
        {#each breadcrumbs as ancestor, i (ancestor.id)}
          {#if i > 0}<span class="crumb-sep">/</span>{/if}
          <button type="button" class="crumb" onclick={() => onSelectAncestor(ancestor.id)}>
            {ancestor.name}
          </button>
        {/each}
      </nav>
    {/if}
    <div class="node-row">
      <h2 class="node-name">{place.name}</h2>
      {#if place.is_storage}<Badge variant="default">Склад</Badge>{/if}
      {#if place.archived_at_utc !== null}<Badge variant="default">Архив</Badge>{/if}
      <span class="node-kind">{KIND_LABELS[place.kind] ?? place.kind}</span>
    </div>
  </header>

  <div class="controls-row">
    <Tabs
      variant="underline"
      {tabs}
      active={activeTab}
      onchange={(k) => onActiveTabChange(k as ContentTab)}
      ariaLabel="Фильтр содержимого"
    />
    <div class="controls-right">
      <div class="display-modes">
        <Checkbox checked={onlyHere} onchange={onOnlyHereChange} id="place-contents-only-here">
          <span title="Не показывать содержимое вложенных мест">Только здесь</span>
        </Checkbox>
        <Checkbox
          checked={expandGroups}
          onchange={onExpandGroupsChange}
          id="place-contents-expand-groups"
        >
          <span title="Показывать устройства из групп отдельными строками вместо строки группы"
            >Разворачивать группы</span
          >
        </Checkbox>
      </div>
      <Button variant="secondary" onclick={openMoveModal}>Перенести всё содержимое в…</Button>
    </div>
  </div>

  <div class="table-region">
    <Table
      columns={columnCount}
      fillHeight
      framed={false}
      loading={loading && rows === null}
      empty={isWholeNodeEmpty || isTabFilteredEmpty}
      emptyTitle={isWholeNodeEmpty
        ? onlyHere
          ? 'Здесь ничего нет'
          : 'В этом месте пусто'
        : 'Здесь нет предметов этого типа'}
      emptyBody={isWholeNodeEmpty
        ? onlyHere
          ? 'Содержимое вложенных мест скрыто — выключите «Только здесь», чтобы увидеть его.'
          : 'Здесь и во вложенных местах ничего не размещено.'
        : (emptyTabHint ?? 'Переключите вкладку, чтобы увидеть остальное содержимое.')}
    >
      {#snippet head()}
        {#if showKindColumn}<th>Тип</th>{/if}
        <th>Название</th>
        <th>Инв. № / Серийный №</th>
        {#if !onlyHere}<th>Место</th>{/if}
        <th>Статус</th>
      {/snippet}
      {#each filteredRows as row (`${row.kind}-${row.id}`)}
        {#if row.kind === 'group'}
          <TableRow
            class="place-content-row place-content-group-row"
            ondblclick={() => openGroup(row)}
          >
            {#if showKindColumn}
              <td class="cell cell-kind cell-static">{row.group_type_name ?? ''}</td>
            {/if}
            <td class="cell cell-static" title={row.name}>
              <button type="button" class="group-name-link" onclick={() => openGroup(row)}
                >{row.name}</button
              >
              <span class="group-count">{deviceCountLabel(row.member_count ?? 0)}</span>
            </td>
            <td class="cell cell-static"><span class="tr-mono">—</span></td>
            {#if !onlyHere}
              <td class="cell cell-static" title={row.full_path}>{row.place_path_short}</td>
            {/if}
            <td class="cell cell-static">—</td>
          </TableRow>
        {:else}
          <TableRow class="place-content-row">
            {#if showKindColumn}
              <td
                class="cell cell-kind"
                role="button"
                tabindex="0"
                onclick={() => openView(row)}
                onkeydown={(e) => handleRowActivateKeydown(e, row)}
              >
                {CONTENT_KIND_LABELS[row.kind] ?? row.kind}
              </td>
              <td class="cell" title={row.name} onclick={() => openView(row)}>{row.name}</td>
            {:else}
              <td
                class="cell"
                title={row.name}
                role="button"
                tabindex="0"
                onclick={() => openView(row)}
                onkeydown={(e) => handleRowActivateKeydown(e, row)}
              >
                {row.name}
              </td>
            {/if}
            <td class="cell" onclick={() => openView(row)}>
              <span class="tr-mono">{row.inventory_or_code ?? '—'}</span>
            </td>
            {#if !onlyHere}
              <td class="cell" title={row.full_path} onclick={() => openView(row)}>
                {row.place_path_short}
              </td>
            {/if}
            <td class="cell" onclick={() => openView(row)}>
              {#if row.status_name}
                {#if row.kind === 'cartridge'}
                  <Badge variant={cartridgeStatusVariant(row.status_name)}>{row.status_name}</Badge>
                {:else}
                  <Badge
                    variant={statusTone(row.status_code).variant}
                    appearance={statusTone(row.status_code).appearance}
                    muted={statusTone(row.status_code).muted}>{row.status_name}</Badge
                  >
                {/if}
              {:else}
                —
              {/if}
            </td>
          </TableRow>
        {/if}
      {/each}
    </Table>
  </div>
</div>

{#if viewRow}
  <PlaceEntityViewModal
    row={viewRow}
    onClose={() => (viewRow = null)}
    onChanged={(changedPlaceIds) => {
      // WARNING-1 gap-closure round 2 (live UAT, 2026-09-18): the first fix
      // only invalidated the currently VIEWED root (`place.id`) — over-
      // invalidating the root is safe for THAT node (PlaceTree's own
      // ancestorsAndSelf() walk never produces a wrong/stale counter there),
      // but it does NOT reach the row's actual old/new place when that place
      // is a different node (a sibling branch, or a nested sub-place under
      // this root whose OWN counter also needs evicting) — exactly the "only
      // the currently open section updates" symptom UAT reported.
      // `PlaceEntityViewModal` now forwards the row's exact old+new place ids
      // (already known to `DeviceFormBody`/the saved `CartridgeDto`, no extra
      // request) — invalidate the viewed root AND those, deduplicated.
      notifyPlaceContentChanged(Array.from(new Set([place.id, ...changedPlaceIds])));
      reloadToken += 1;
    }}
  />
{/if}

<Modal open={moveModalOpen} title="Перенести содержимое в другое место?" onClose={closeMoveModal}>
  <div class="move-form">
    <div class="form-field">
      <label class="form-label" for="place-contents-move-target">Новое место</label>
      <PlacePicker
        id="place-contents-move-target"
        value={moveTargetId}
        disabled={moving}
        onChange={(id) => (moveTargetId = id)}
      />
    </div>

    {#if moveCountLoading}
      <div class="stats-loading">
        <Spinner size="sm" />
        <span>Загрузка…</span>
      </div>
    {:else}
      <p class="confirm-body">
        Место изменится у {moveCount ?? 0} предметов в этом разделе и всех вложенных местах. Для каждого
        появится запись в истории перемещений с причиной «вручную».
      </p>
      {#if moveGroupCount !== null && moveGroupCount > 0}
        <p class="confirm-body">
          Вместе с содержимым переедут {moveGroupCount}
          {pluralizeRu(moveGroupCount, ['группа', 'группы', 'групп'])} и весь их состав.
        </p>
      {/if}
    {/if}
  </div>

  {#snippet footer()}
    <Button variant="secondary" onclick={closeMoveModal} disabled={moving}>Отмена</Button>
    <Button
      variant="primary"
      loading={moving}
      disabled={moveTargetId === null}
      onclick={handleMoveConfirm}
    >
      Перенести
    </Button>
  {/snippet}
</Modal>

<style lang="scss">
  .place-contents {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .contents-header {
    flex: none;
    padding: var(--tr-space-sm) var(--tr-space-md);
    border-bottom: 1px solid var(--tr-border);
  }

  .breadcrumbs {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--tr-space-3xs, 2px);
    font-size: var(--tr-font-size-caption);
    color: var(--tr-text-secondary);
    margin-bottom: var(--tr-space-2xs);
  }

  .crumb {
    padding: 0;
    background: none;
    border: none;
    font: inherit;
    color: inherit;
    cursor: pointer;

    &:hover {
      color: var(--tr-text-primary);
      text-decoration: underline;
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
      border-radius: var(--tr-radius-xs);
    }
  }

  .crumb-sep {
    color: var(--tr-text-tertiary);
  }

  .node-row {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
    flex-wrap: wrap;
  }

  .node-name {
    margin: 0;
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .node-kind {
    font-size: var(--tr-font-size-caption);
    color: var(--tr-text-secondary);
  }

  .controls-row {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--tr-space-md);
    padding: var(--tr-space-sm) var(--tr-space-md);
    border-bottom: 1px solid var(--tr-border);
  }

  .table-region {
    flex: 1 1 auto;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 0 var(--tr-space-md) var(--tr-space-md);
  }

  .cell {
    cursor: pointer;
  }

  // Group-row cells are inert on click (R-03), so they do not advertise it.
  .cell-static {
    cursor: default;
  }

  .group-name-link {
    padding: 0;
    background: none;
    border: none;
    font: var(--tr-text-body-strong);
    color: var(--tr-accent-text);
    cursor: pointer;
    text-align: left;

    &:hover {
      text-decoration: underline;
    }
  }

  .group-count {
    margin-left: var(--tr-space-xs);
    color: var(--tr-text-secondary);
  }

  .cell-kind {
    color: var(--tr-text-secondary);
    font-size: var(--tr-font-size-caption);
  }

  .controls-right {
    display: flex;
    align-items: center;
    gap: var(--tr-space-md);
  }

  // The two display-mode checkboxes read as neighbours; the gap to the
  // «Перенести…» button stays the wider .controls-right gap.
  .display-modes {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
  }

  // Bulk-move confirm dialog (D-28, Plan 40-19) — mirrors PlaceMoveModal.svelte's
  // form-field/stats-loading pattern for visual consistency across the two
  // "move something" dialogs in this feature.
  .move-form {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
    padding: var(--tr-space-md) 0;
  }

  .form-field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  .form-label {
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-secondary);
  }

  .stats-loading {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
    color: var(--tr-text-secondary);
    font-size: var(--tr-font-size-body);
  }

  .confirm-body {
    margin: 0;
    color: var(--tr-text-secondary);
    line-height: var(--tr-line-height-body);
  }
</style>
