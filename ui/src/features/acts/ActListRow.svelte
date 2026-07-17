<script lang="ts">
  // Plan 03-02: two-line list row for ActsList master panel.
  import Badge from '$lib/components/Badge.svelte';
  import type { ActDto } from '../../bindings';

  interface Props {
    act: ActDto;
    selected: boolean;
    showArchivedBadge?: boolean;
    onSelect: (_id: number) => void;
  }

  const { act, selected, showArchivedBadge = false, onSelect }: Props = $props();

  // Format unix seconds → «28 мая 2026»
  const MONTHS_RU = [
    'января',
    'февраля',
    'марта',
    'апреля',
    'мая',
    'июня',
    'июля',
    'августа',
    'сентября',
    'октября',
    'ноября',
    'декабря',
  ];

  function formatDate(utcSeconds: number): string {
    const d = new Date(utcSeconds * 1000);
    return `${d.getUTCDate()} ${MONTHS_RU[d.getUTCMonth()]} ${d.getUTCFullYear()}`;
  }

  const dateLabel = $derived(formatDate(act.handover_date_utc));
  const itemsCount = $derived(act.items.length);
  // act.number is already formatted (D-Numbering-01) — e.g. "42" / "42в1".

  function handleClick() {
    onSelect(act.id);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onSelect(act.id);
    }
  }
</script>

<div
  class="row"
  class:selected
  role="button"
  tabindex="0"
  aria-pressed={selected}
  onclick={handleClick}
  onkeydown={handleKeydown}
>
  <div class="top">
    <span class="number"><span class="tr-mono">№{act.number}</span></span>
    <span class="separator">·</span>
    <span class="date">{dateLabel}</span>
    {#if showArchivedBadge}
      <span class="badge-wrap">
        <Badge variant="default">В архиве</Badge>
      </span>
    {/if}
  </div>
  <div class="bottom">
    <span class="receiver">{act.receiver_name}</span>
    <span class="separator">·</span>
    <span class="count">{itemsCount} устр.</span>
  </div>
</div>

<style lang="scss">
  .row {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: var(--tr-space-2xs);
    min-height: 64px;
    padding: var(--tr-space-md);
    border-bottom: 1px solid var(--tr-border);
    cursor: pointer;
    border-left: 3px solid transparent;

    &:hover {
      background: var(--tr-surface-sunken);
    }
    &:focus-visible {
      outline: none;
      box-shadow: inset 0 0 0 2px var(--tr-accent);
    }

    &.selected {
      border-left-color: var(--tr-accent);
      background: color-mix(in srgb, var(--tr-accent) 8%, transparent);
    }
  }

  .top {
    display: flex;
    align-items: center;
    gap: var(--tr-space-2xs);
    font-size: var(--tr-font-size-body);
    line-height: 1.2;
  }
  .number {
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    color: var(--tr-text-primary);
  }
  .date {
    color: var(--tr-text-secondary);
  }
  .separator {
    color: var(--tr-text-tertiary);
  }
  .badge-wrap {
    margin-left: auto;
  }

  .bottom {
    display: flex;
    align-items: center;
    gap: var(--tr-space-2xs);
    font-size: var(--tr-font-size-label);
    font-weight: 500;
  }
  .receiver {
    color: var(--tr-text-primary);
  }
  .count {
    color: var(--tr-text-secondary);
  }
</style>
