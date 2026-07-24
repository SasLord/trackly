<script lang="ts">
  // TableRow — row-state primitrive (normal/hover/selected/indent/last) + group-row
  // mode (chevron/name/toggle). Source of truth for values: TableRows.dc.html (D-09/D-10),
  // see .planning/phases/25-dropdown/25-01-PLAN.md <interfaces>.
  import type { Snippet } from 'svelte';

  interface Props {
    /** Background var(--tr-row-selected) + 3px accent inset box-shadow on the first cell. */
    selected?: boolean;
    /** padding-left: 32px on the row's first <td>. */
    indent?: boolean;
    /** Removes border-bottom from every <td> — literal last row of a table. */
    last?: boolean;
    /** Switches to group-row rendering mode. */
    group?: boolean;
    /** Group-row-mode only: controls chevron rotation. */
    groupExpanded?: boolean;
    /** Group-row-mode only: rendered inside the merged name cell. */
    groupName?: string;
    /** Group-row-mode only: colspan of the merged name cell. */
    groupColspan?: number;
    /** Group-row-mode only: fires on click of the row OR the chevron button. */
    onToggleGroup?: () => void;
    /** Appended to the rendered <tr>'s class list for consumer-supplied local CSS. */
    class?: string;
    /**
     * Normal mode: the row's <td> cells, rendered verbatim.
     * Group mode: the trailing <td> cells AFTER the merged name cell.
     */
    children?: Snippet;
  }

  const {
    selected = false,
    indent = false,
    last = false,
    group = false,
    groupExpanded = false,
    groupName,
    groupColspan = 1,
    onToggleGroup,
    class: className = '',
    children,
  }: Props = $props();

  // stopPropagation so the chevron's own click doesn't ALSO trigger the row's
  // onclick — one toggle per click, not two (see DeviceGroupRow precedent).
  function handleChevronClick(e: MouseEvent) {
    e.stopPropagation();
    onToggleGroup?.();
  }
</script>

{#if group}
  <tr class="tr-row tr-row-group {className}" onclick={onToggleGroup}>
    <td class="tr-row-group-name" colspan={groupColspan}>
      <button
        type="button"
        class="tr-row-chevron"
        class:expanded={groupExpanded}
        aria-label={groupExpanded ? 'Свернуть группу' : 'Развернуть группу'}
        onclick={handleChevronClick}
      >
        <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
          <path
            d="M6 4l4 4-4 4"
            stroke="currentColor"
            stroke-width="1.75"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </button>
      {groupName}
    </td>
    {@render children?.()}
  </tr>
{:else}
  <tr class="tr-row {className}" class:selected class:indent class:last>
    {@render children?.()}
  </tr>
{/if}

<style lang="scss">
  .tr-row {
    &:hover {
      background: var(--tr-row-hover);
    }

    &.selected {
      background: var(--tr-row-selected);
    }
  }

  // Row-wide focus ring: fires from ANY focusable descendant (chevron, single-
  // entry-point cell, kebab button), not just a direct child — replaces 4
  // duplicated cell-level box-shadow rules with one shared primitive rule
  // (Gap 4, 30-VERIFICATION.md; план 30-05). Coexists with .tr-row-chevron's
  // own narrower &:focus-visible ring below (both visible simultaneously).
  .tr-row:has(:focus-visible) {
    box-shadow: inset 0 0 0 2px var(--tr-accent);
  }

  .tr-row-group {
    background: var(--tr-group);
    cursor: pointer;
  }

  // Base <td> metrics — TableRow is the SOLE owner of these values (D-10, TableRows.dc
  // tdBase). Caller-rendered <td> children come from the `children` snippet (a DIFFERENT
  // Svelte scope-hash than this file), so every rule targeting them needs the :global()
  // escape hatch. Selector SHAPE matters: `.tr-row :global(> td)` keeps `.tr-row` in this
  // component's own scope (compiles to `.tr-row.svelte-hash > td`, specificity 0,2,1) and
  // correctly beats a consumer's leftover `.cell` class (0,1,0). The inside-out form
  // `:global(.tr-row > td)` compiles to (0,1,1) and LOSES to `.cell` — do not use it.
  .tr-row :global(> td) {
    height: 40px;
    padding: 0 10px;
    border-bottom: 1px solid var(--tr-border);
    vertical-align: middle;
  }

  .tr-row.indent :global(> td:first-child) {
    padding-left: 32px;
  }

  // Selected-row accent: an inset box-shadow on the first cell, NOT a border —
  // box-shadow is layout-neutral (doesn't consume box width like border-left
  // does), so cell text never shifts when a row becomes selected, in either
  // flat (Acts/Cartridges/Printers) or indent (Devices) rows (UAT gap-fix batch
  // D, FIX D1 — supersedes the padding-compensation approach from batch A/FIX A2,
  // which still let the border shift text by a subpixel).
  .tr-row.selected :global(> td:first-child) {
    box-shadow: inset 3px 0 0 var(--tr-accent);
  }

  .tr-row.last :global(> td) {
    border-bottom: none;
  }

  .tr-row-group-name {
    font-weight: var(--tr-font-weight-semibold);
    white-space: nowrap;
  }

  .tr-row-chevron {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    padding: 0;
    margin-right: var(--tr-space-2xs);
    background: transparent;
    border: none;
    color: var(--tr-text-secondary);
    cursor: pointer;
    transform: none;
    // This ONE transition stays at .15s (TableRows.dc value, verbatim) — not the
    // usual .12s micro-transition used elsewhere in the design system.
    transition: transform 0.15s;

    &.expanded {
      transform: rotate(90deg);
    }

    &:focus-visible {
      outline: none;
      box-shadow: inset 0 0 0 2px var(--tr-accent);
    }
  }
</style>
