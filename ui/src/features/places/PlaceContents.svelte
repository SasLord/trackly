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
  import { apiCall } from '$lib/api/client';
  import Tabs from '$lib/components/Tabs.svelte';
  import Checkbox from '$lib/components/Checkbox.svelte';
  import Table from '$lib/components/Table.svelte';
  import TableRow from '$lib/components/TableRow.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import PlaceEntityViewModal from './PlaceEntityViewModal.svelte';
  import type { PlaceContentDto, PlaceDto } from '../../bindings';
  import { shortenPlacePath } from '$lib/utils/placePath';
  import { authStore } from '$lib/stores/auth.svelte';

  export type ContentTab = 'all' | 'device' | 'printer' | 'cartridge';

  interface Props {
    place: PlaceDto;
    onSelectAncestor: (_id: number) => void;
    onlyHere: boolean;
    onOnlyHereChange: (_v: boolean) => void;
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

  // status_name is a free-text string (device_statuses.name / cartridge_statuses.name,
  // Plan 08/12) — no status_id crosses the wire on PlaceContentDto, so variant is
  // matched by the seeded name text itself (V001__init_pragmas_and_lookups.sql),
  // not by id (unlike DeviceListRow/CartridgeListRow, which have the id).
  const STATUS_VARIANT_BY_NAME: Record<string, 'default' | 'accent' | 'warning' | 'destructive'> = {
    'На складе': 'default',
    'В работе': 'accent',
    'На ремонте': 'warning',
    'На заправке': 'warning',
    Списано: 'destructive',
  };

  function statusVariant(name: string | null): 'default' | 'accent' | 'warning' | 'destructive' {
    if (!name) return 'default';
    return STATUS_VARIANT_BY_NAME[name] ?? 'default';
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
    void reloadToken; // GAP-8: forces a re-fetch after an edit-modal save.
    let cancelled = false;
    loading = true;
    loadError = false;
    apiCall<PlaceContentDto[]>('places_contents', { rootId, nested })
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
    return {
      all: all.length,
      device: all.filter((r) => r.kind === 'device').length,
      printer: all.filter((r) => r.kind === 'printer').length,
      cartridge: all.filter((r) => r.kind === 'cartridge').length,
    };
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

  const filteredRows = $derived(
    (rows ?? []).filter((r) => activeTab === 'all' || r.kind === activeTab),
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

  // UAT gap 7 (2026-08-25): «Тип» is redundant once the tab already filters to
  // a single kind — only shown on the «Все» tab. Base columns (Название, Инв.
  // № / Серийный №, Статус) = 3, plus «Тип» when activeTab === 'all', plus
  // «Место» when !onlyHere. Must stay correct across all 4 tabs × 2 toggle
  // combinations or the empty-state row's colspan is wrong.
  const showKindColumn = $derived(activeTab === 'all');
  const columnCount = $derived(3 + (showKindColumn ? 1 : 0) + (!onlyHere ? 1 : 0));
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
    <Checkbox checked={onlyHere} onchange={onOnlyHereChange} id="place-contents-only-here">
      <span title="Не показывать содержимое вложенных мест">Только здесь</span>
    </Checkbox>
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
        : 'Переключите вкладку, чтобы увидеть остальное содержимое.'}
    >
      {#snippet head()}
        {#if showKindColumn}<th>Тип</th>{/if}
        <th>Название</th>
        <th>Инв. № / Серийный №</th>
        {#if !onlyHere}<th>Место</th>{/if}
        <th>Статус</th>
      {/snippet}
      {#each filteredRows as row (`${row.kind}-${row.id}`)}
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
              {shortenPlacePath(row.full_path, authStore.placePathDisplay)}
            </td>
          {/if}
          <td class="cell" onclick={() => openView(row)}>
            {#if row.status_name}
              <Badge variant={statusVariant(row.status_name)}>{row.status_name}</Badge>
            {:else}
              —
            {/if}
          </td>
        </TableRow>
      {/each}
    </Table>
  </div>
</div>

{#if viewRow}
  <PlaceEntityViewModal
    row={viewRow}
    onClose={() => (viewRow = null)}
    onChanged={() => (reloadToken += 1)}
  />
{/if}

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

  .cell-kind {
    color: var(--tr-text-secondary);
    font-size: var(--tr-font-size-caption);
  }
</style>
