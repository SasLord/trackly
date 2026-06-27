<script lang="ts">
  // Plan 07-05: PeriodToggle — переключатель периода 3/6/12 месяцев для ChartWidget.
  // Паттерн: status-bar tabs из CartridgeFilters.svelte.

  interface Props {
    windowMonths: 3 | 6 | 12;
    onWindowChange: (months: 3 | 6 | 12) => void;
  }

  const { windowMonths, onWindowChange }: Props = $props();
</script>

<div class="period-toggle" role="group" aria-label="Период графика">
  {#each [3, 6, 12] as const as m}
    <button
      class="toggle-btn"
      class:active={windowMonths === m}
      onclick={() => onWindowChange(m)}
      type="button"
    >
      {m} мес.
    </button>
  {/each}
</div>

<style lang="scss">
  .period-toggle {
    display: flex;
    gap: 2px;
    overflow-x: auto;
  }

  .toggle-btn {
    display: inline-flex;
    align-items: center;
    padding: var(--space-xs) var(--space-sm);
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    font-family: var(--font-family-base);
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
    cursor: pointer;
    white-space: nowrap;
    border-radius: var(--radius-sm) var(--radius-sm) 0 0;
    transition: color 0.1s ease;

    &:hover {
      background: var(--color-surface-sunken);
      color: var(--color-text-primary);
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--color-accent-focus);
    }

    &.active {
      color: var(--color-accent);
      border-bottom-color: var(--color-accent);
      font-weight: var(--font-weight-medium);
    }
  }
</style>
