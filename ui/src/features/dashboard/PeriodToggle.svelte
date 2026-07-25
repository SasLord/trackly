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
    padding: 2px 8px 5px;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    font-family: var(--tr-font-family);
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
    font-weight: 500;
    cursor: pointer;
    white-space: nowrap;
    border-radius: var(--tr-radius-xs) var(--tr-radius-xs) 0 0;
    transition: color 0.1s ease;

    &:hover {
      background: var(--tr-surface-sunken);
      color: var(--tr-text-primary);
    }

    &:focus-visible {
      outline: none;
      box-shadow: inset 0 0 0 2px var(--tr-accent);
    }

    &.active {
      color: var(--tr-accent-text);
      border-bottom-color: var(--tr-accent);
      font-weight: 600;
    }
  }
</style>
