<script lang="ts">
  // Plan 40-15 (HST-02): shared timeline-row component consumed by THREE
  // parents — PlaceEntityViewModal (device+printer, Plan 40-16),
  // CartridgeDetail (Plan 40-17), PrinterDetail (Plan 40-17, same data as its
  // device). "3 consumers, 1 formula" per RESEARCH.md — this file is the ONLY
  // place the canonical row anatomy (UI-SPEC "Timeline row anatomy") is coded.
  //
  // D-18 / Don't Hand-Roll: this component NEVER re-derives or re-fetches the
  // display-length reduction applied to a place path. `from_place_path_short`/
  // `to_place_path_short` are already server-computed (Plan 40-10, via the
  // single-owner `compute_place_path_short`) — it only displays them, with the
  // FULL `from_place_path`/`to_place_path` snapshot in the native `title=`
  // tooltip (D-17). Introducing a JS mirror here would recreate the exact
  // WR-03/WR-08 divergence class that Phase 39.2 spent itself fixing.
  //
  // Loading state (UI-SPEC States table): this component does NOT render its
  // own spinner — the parent's single fetch already resolves loading/error
  // before mounting rows, so `loading=true` renders nothing here.
  import type { MovementEntryDto } from '../../bindings';

  interface Props {
    entries: MovementEntryDto[];
    loading: boolean;
    loadError: boolean;
    onNavigateToPlace?: (placeId: number) => Promise<void> | void;
    onNavigateToAct?: (actId: number) => Promise<void> | void;
  }

  const { entries, loading, loadError, onNavigateToPlace, onNavigateToAct }: Props = $props();

  // Manual DD.MM.YYYY formatting — same `padStart` approach already used by
  // CartridgeDetail.svelte::formatDate (no `Intl`, per the project's
  // established single-timezone convention). `formatDate` there is a local
  // closure, not a module export, so this is an intentional, tiny, exact
  // duplicate of a formatter (not a re-derivation of business logic like the
  // display-length-reduction formula this component deliberately avoids mirroring).
  function formatDate(utcSeconds: number): string {
    const d = new Date(utcSeconds * 1000);
    const day = String(d.getUTCDate()).padStart(2, '0');
    const month = String(d.getUTCMonth() + 1).padStart(2, '0');
    const year = d.getUTCFullYear();
    return `${day}.${month}.${year}`;
  }

  // Reason composition per UI-SPEC's "Timeline row anatomy" table, exactly.
  // `source == 'act'` renders a clickable act-number segment separately (see
  // markup below) — this helper only returns the LEADING literal text
  // ("актом №" ...) is still assembled inline in the template so the number
  // itself can be a real <button>. For non-act sources this returns the full
  // plain-text reason string.
  function reasonText(entry: MovementEntryDto): string {
    if (entry.source === 'manual') {
      return entry.note && entry.note.trim().length > 0 ? `вручную · ${entry.note}` : 'вручную';
    }
    if (entry.source === 'act') {
      // Rendered as literal + button in the template, this fallback only
      // covers the defensive case where act_number is missing despite
      // source === 'act' (Pitfall 6 — never let an unexpected shape throw).
      return entry.act_number ? `актом №${entry.act_number}` : 'актом';
    }
    // Unrecognized/future source values (`map`, `workstation`, or any other
    // token) — safe fallback label, never a crash (T-40-30, Pitfall 6/IN-01).
    return 'причина не определена';
  }

  function handleNavigateToPlace(placeId: number) {
    void onNavigateToPlace?.(placeId);
  }

  function handleNavigateToAct(actId: number) {
    void onNavigateToAct?.(actId);
  }
</script>

{#if loading}
  <!-- Parent owns the loading spinner (UI-SPEC States table) — nothing to render here. -->
{:else if loadError}
  <p class="timeline-error">
    Не удалось загрузить историю перемещений. Закройте окно и попробуйте ещё раз.
  </p>
{:else if entries.length === 0}
  <div class="timeline-empty">
    <p class="timeline-empty-heading">Перемещений ещё не было</p>
    <p class="timeline-empty-body">
      Место изменится — здесь появится запись: откуда, куда, когда, кем и почему.
    </p>
    <p class="timeline-empty-body">
      Первичное размещение при поступлении в историю не попадает — она начинается с первого
      перемещения между двумя известными местами.
    </p>
  </div>
{:else}
  <ul class="timeline-list">
    {#each entries as entry (entry.id)}
      <li class="timeline-row">
        <span class="timeline-date">{formatDate(entry.created_at_utc)}</span>
        <span class="timeline-dash">—</span>
        <button
          type="button"
          class="timeline-link"
          title={entry.from_place_path}
          onclick={() => handleNavigateToPlace(entry.from_place_id)}
        >
          {entry.from_place_path_short ?? entry.from_place_path}
        </button>
        <span class="timeline-arrow">→</span>
        <button
          type="button"
          class="timeline-link"
          title={entry.to_place_path}
          onclick={() => handleNavigateToPlace(entry.to_place_id)}
        >
          {entry.to_place_path_short ?? entry.to_place_path}
        </button>
        <span class="timeline-sep">·</span>
        <span class="timeline-actor">{entry.actor_display}</span>
        <span class="timeline-sep">·</span>
        {#if entry.source === 'act' && entry.act_id !== null && entry.act_number}
          <span class="timeline-reason">
            актом №<button
              type="button"
              class="timeline-link"
              onclick={() => handleNavigateToAct(entry.act_id as number)}
            >
              {entry.act_number}
            </button>
          </span>
        {:else}
          <span class="timeline-reason">{reasonText(entry)}</span>
        {/if}
      </li>
    {/each}
  </ul>
  <p class="timeline-empty-body">
    Первичное размещение при поступлении в историю не попадает — она начинается с первого
    перемещения между двумя известными местами.
  </p>
{/if}

<style lang="scss">
  .timeline-error {
    margin: 0;
    padding: var(--tr-space-md);
    color: var(--tr-danger-text);
  }

  .timeline-empty {
    padding: var(--tr-space-md) 0;
  }

  .timeline-empty-heading {
    margin: 0 0 var(--tr-space-2xs) 0;
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-tertiary);
    font-style: italic;
  }

  .timeline-empty-body {
    margin: 0;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
    font-style: italic;
  }

  .timeline-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .timeline-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--tr-space-2xs);
    min-height: var(--row-height-dense, 32px);
    padding: var(--tr-space-2xs) 0;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-primary);
    border-bottom: 1px solid var(--tr-border);

    &:last-child {
      border-bottom: none;
    }
  }

  .timeline-dash,
  .timeline-arrow,
  .timeline-sep {
    color: var(--tr-text-secondary);
  }

  .timeline-actor,
  .timeline-reason {
    color: var(--tr-text-primary);
  }

  // Clickable inline-text link contract (UI-SPEC "Clickable-link CSS
  // contract") — `PlaceContents.svelte`'s `.crumb` rule set with
  // `color: inherit` replaced by `var(--tr-accent-text)`, nothing else
  // changed. Reused for both place segments and the act-number segment
  // (exactly the two accent-colored elements per row, per UI-SPEC Color §).
  .timeline-link {
    padding: 0;
    background: none;
    border: none;
    font: inherit;
    color: var(--tr-accent-text);
    cursor: pointer;

    &:hover {
      text-decoration: underline;
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
      border-radius: var(--tr-radius-xs);
    }
  }
</style>
