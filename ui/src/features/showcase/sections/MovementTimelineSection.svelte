<script lang="ts">
  // Plan 40-15 (HST-02): showcase gallery for the shared MovementTimeline.svelte,
  // mirroring the PlacePickerSection precedent (Phase 39 Plan 13) — invented
  // demo rows only, no real apiCall/DB data (project's hard privacy rule).
  // Exercises every documented behavior case from the plan's <behavior> block:
  // populated (3 rows, given order preserved), empty, error, manual w/o note,
  // manual w/ note, act (clickable act number), and an unrecognized `source`
  // value that must not crash.
  import MovementTimeline from '$lib/components/MovementTimeline.svelte';
  import type { MovementEntryDto } from '../../../bindings';

  const now = Math.floor(Date.now() / 1000);
  const day = 24 * 60 * 60;

  function entry(overrides: Partial<MovementEntryDto> & { id: number }): MovementEntryDto {
    return {
      entity_type: 'device',
      entity_id: 1,
      from_place_id: 5,
      from_place_path: 'Здание А / 2 этаж / 214',
      from_place_path_short: 'Здание А / 2 эт. / 214',
      to_place_id: 6,
      to_place_path: 'Здание А / 2 этаж / Шкаф-склад',
      to_place_path_short: 'Склад',
      actor_display: 'Иванов И.И.',
      source: 'manual',
      note: null,
      act_id: null,
      act_number: null,
      created_at_utc: now,
      ...overrides,
    };
  }

  // "Given an array of 3 entries, renders exactly 3 rows in the given order"
  // — server already returns newest-first, this demo array is pre-sorted the
  // same way (component does not re-sort).
  const populatedEntries: MovementEntryDto[] = [
    entry({
      id: 3,
      source: 'act',
      act_id: 42,
      act_number: '123',
      created_at_utc: now,
    }),
    entry({
      id: 2,
      source: 'manual',
      note: 'уточнение',
      created_at_utc: now - day,
    }),
    entry({
      id: 1,
      source: 'manual',
      note: null,
      actor_display: 'Петров П.П.',
      created_at_utc: now - 2 * day,
    }),
  ];

  // Unrecognized `source` must render a safe fallback, never throw
  // (T-40-30, Pitfall 6/IN-01).
  const garbageSourceEntries: MovementEntryDto[] = [
    entry({ id: 10, source: 'garbage', note: null, actor_display: 'система' }),
  ];

  function handleNavigateToPlace(placeId: number) {
    console.log('[showcase] navigate to place', placeId);
  }

  function handleNavigateToAct(actId: number) {
    console.log('[showcase] navigate to act', actId);
  }
</script>

<section class="movement-timeline-section">
  <h2>MovementTimeline</h2>

  <div class="variant-block">
    <h3 class="variant-label">Заполнено (3 записи — акт / вручную с примечанием / вручную)</h3>
    <div class="demo-anchor">
      <MovementTimeline
        entries={populatedEntries}
        loading={false}
        loadError={false}
        onNavigateToPlace={handleNavigateToPlace}
        onNavigateToAct={handleNavigateToAct}
      />
    </div>
  </div>

  <div class="variant-block">
    <h3 class="variant-label">Пусто</h3>
    <div class="demo-anchor">
      <MovementTimeline entries={[]} loading={false} loadError={false} />
    </div>
  </div>

  <div class="variant-block">
    <h3 class="variant-label">Ошибка загрузки</h3>
    <div class="demo-anchor">
      <MovementTimeline entries={[]} loading={false} loadError={true} />
    </div>
  </div>

  <div class="variant-block">
    <h3 class="variant-label">Неизвестный source — не должен падать (Pitfall 6)</h3>
    <div class="demo-anchor">
      <MovementTimeline entries={garbageSourceEntries} loading={false} loadError={false} />
    </div>
  </div>
</section>

<style lang="scss">
  .movement-timeline-section {
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
    align-items: stretch;
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
    max-width: 560px;
  }
</style>
