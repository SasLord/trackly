<script lang="ts">
  // Phase 39 Plan 13 (D-17/CMP-analog of the 25-06 Dropdown showcase precedent):
  // showcase gallery for PlacePicker.svelte — invented demo tree only ("Здание А
  // / 2 этаж / 214" per UI-SPEC §6.4), served from local component state via the
  // component's fetchChildren/fetchSearchResults/fetchOne/createPlace injection
  // props (see PlacePicker.svelte's header comment) — no real apiCall/DB rows,
  // per the project's hard privacy rule (no seeded data for a public repo demo).
  import PlacePicker from '$lib/components/PlacePicker.svelte';
  import type { PlaceDto, PlaceNewDto, PlacePathDto } from '../../../bindings';

  const now = Date.now();
  function place(
    id: number,
    parentId: number | null,
    kind: string,
    name: string,
    fullPath: string,
    opts: {
      isStorage?: boolean;
      archived?: boolean;
      level?: number | null;
      pathVariantOverride?: string | null;
    } = {},
  ): PlaceDto {
    return {
      id,
      parent_id: parentId,
      kind,
      name,
      level: opts.level ?? null,
      is_storage: opts.isStorage ?? false,
      sort_order: null,
      archived_at_utc: opts.archived ? now : null,
      notes: null,
      full_path: fullPath,
      // Phase 39.1 Plan 07 added path_variant_override to PlaceDto. The showcase
      // tree inherits the organization default everywhere (null), which is the
      // shape PlacePicker actually sees for most places.
      path_variant_override: opts.pathVariantOverride ?? null,
      created_at_utc: now,
      updated_at_utc: now,
      version: 1,
    };
  }

  let demoPlaces = $state<PlaceDto[]>([
    place(1, null, 'building', 'Здание А', 'Здание А'),
    place(2, 1, 'floor', '1 этаж', 'Здание А / 1 этаж', { level: 1 }),
    place(3, 2, 'room', '101', 'Здание А / 1 этаж / 101'),
    place(4, 1, 'floor', '2 этаж', 'Здание А / 2 этаж', { level: 2 }),
    place(5, 4, 'room', '214', 'Здание А / 2 этаж / 214'),
    place(6, 4, 'room', 'Шкаф-склад', 'Здание А / 2 этаж / Шкаф-склад', { isStorage: true }),
    place(7, 4, 'room', '216', 'Здание А / 2 этаж / 216', { archived: true }),
    place(8, null, 'territory', 'Территория Северная', 'Территория Северная'),
    place(9, 8, 'outdoor', 'КПП-1', 'Территория Северная / КПП-1'),
  ]);
  let nextDemoId = 100;

  /** Small artificial delay so the "Загрузка…" branch is visibly reachable, same
   *  intent as DropdownSection's dedicated loading demo block. */
  function delay(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  async function demoFetchChildren(parentId: number | null): Promise<PlaceDto[]> {
    await delay(120);
    // D-15: архивные узлы не возвращаются в обычном списке детей — витрина
    // намеренно повторяет серверное поведение, чтобы демонстрировать §10.2's
    // exception (архивный узел показывается только когда он — текущее value).
    return demoPlaces.filter((p) => p.parent_id === parentId && p.archived_at_utc === null);
  }

  async function demoFetchOne(id: number): Promise<PlaceDto> {
    await delay(60);
    const found = demoPlaces.find((p) => p.id === id);
    if (!found) throw new Error('demo place not found');
    return found;
  }

  async function demoFetchSearchResults(query: string): Promise<PlacePathDto[]> {
    await delay(120);
    const q = query.trim().toLowerCase();
    if (!q) return [];
    return demoPlaces
      .filter((p) => p.archived_at_utc === null && (p.full_path ?? '').toLowerCase().includes(q))
      .map((p) => ({ place_id: p.id, full_path: p.full_path ?? '', kind: p.kind }));
  }

  async function demoCreatePlace(newPlace: PlaceNewDto): Promise<PlaceDto> {
    await delay(120);
    const parent =
      newPlace.parent_id !== null ? demoPlaces.find((p) => p.id === newPlace.parent_id) : null;
    const fullPath = parent ? `${parent.full_path} / ${newPlace.name}` : newPlace.name;
    const created = place(
      nextDemoId++,
      newPlace.parent_id,
      newPlace.kind,
      newPlace.name,
      fullPath,
      {
        isStorage: newPlace.is_storage,
        level: newPlace.level,
      },
    );
    demoPlaces = [...demoPlaces, created];
    return created;
  }

  let valueTree = $state<number | null>(null);
  let valueArchived = $state<number | null>(7);
</script>

<section class="place-picker-section">
  <h2>PlacePicker</h2>

  <div class="variant-block">
    <h3 class="variant-label">Дерево и поиск (демо-данные: «Здание А / 2 этаж / 214»)</h3>
    <div class="demo-anchor">
      <PlacePicker
        value={valueTree}
        onChange={(v) => (valueTree = v)}
        fetchChildren={demoFetchChildren}
        fetchSearchResults={demoFetchSearchResults}
        fetchOne={demoFetchOne}
        createPlace={demoCreatePlace}
      />
    </div>
  </div>

  <div class="variant-block">
    <h3 class="variant-label">Значение — архивный узел (D-15, исключение)</h3>
    <div class="demo-anchor">
      <PlacePicker
        value={valueArchived}
        onChange={(v) => (valueArchived = v)}
        fetchChildren={demoFetchChildren}
        fetchSearchResults={demoFetchSearchResults}
        fetchOne={demoFetchOne}
        createPlace={demoCreatePlace}
      />
    </div>
  </div>

  <div class="variant-block">
    <h3 class="variant-label">Заблокировано / некорректно</h3>
    <div class="demo-anchor">
      <PlacePicker value={null} onChange={() => {}} disabled />
    </div>
    <div class="demo-anchor">
      <PlacePicker value={null} onChange={() => {}} invalid />
    </div>
  </div>
</section>

<style lang="scss">
  .place-picker-section {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-lg);
  }

  h2 {
    margin: 0;
    font-size: var(--tr-font-size-h2);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .variant-block {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--tr-space-sm);
  }

  .variant-label {
    margin: 0;
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-secondary);
    text-transform: uppercase;
  }

  .demo-anchor {
    width: 320px;
    max-width: 100%;
  }
</style>
