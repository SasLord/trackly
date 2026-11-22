<script lang="ts">
  // Фаза 41.10 (D-21, UI-SPEC §6.2): ряд чипов-тумблеров по видам событий.
  // Порядок фиксирован: Перемещения · Состав · Статус. Состояние живёт у
  // родителя (`selected`: false = выключен, всё прочее = включён) и нигде не хранится.
  const LABELS: Record<string, string> = {
    move: 'Перемещения',
    membership: 'Состав',
    status: 'Статус',
  };
  const ORDER = ['move', 'membership', 'status'];

  interface Props {
    /** Какие чипы показать (порядок задаёт ORDER, не этот массив). */
    kinds: string[];
    selected: Record<string, boolean>;
  }

  let { kinds, selected = $bindable() }: Props = $props();

  const shown = $derived(ORDER.filter((k) => kinds.includes(k)));

  function isOn(kind: string): boolean {
    return selected[kind] !== false;
  }

  function toggle(kind: string) {
    selected = { ...selected, [kind]: !isOn(kind) };
  }
</script>

<div class="chips" role="group" aria-label="Виды событий">
  {#each shown as kind (kind)}
    <button
      type="button"
      class="chip"
      class:on={isOn(kind)}
      aria-pressed={isOn(kind)}
      onclick={() => toggle(kind)}
    >
      {#if isOn(kind)}
        <svg
          width="12"
          height="12"
          viewBox="0 0 12 12"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <path d="M2.5 6.5l2.5 2.5 4.5-5.5" />
        </svg>
      {/if}
      {LABELS[kind]}
    </button>
  {/each}
</div>

<style lang="scss">
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--tr-space-2xs);
    margin-bottom: var(--tr-space-sm);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: var(--tr-space-2xs);
    height: 28px;
    padding: 0 var(--tr-space-sm);
    border-radius: var(--tr-radius-full);
    border: 1px solid var(--tr-border-strong);
    background: transparent;
    color: var(--tr-text-secondary);
    font: inherit;
    font-size: var(--tr-font-size-label);
    white-space: nowrap;
    cursor: pointer;

    &:hover {
      background: var(--tr-surface-sunken);
      color: var(--tr-text-primary);
    }

    &.on {
      background: var(--tr-accent-soft);
      border-color: var(--tr-accent);
      color: var(--tr-accent-text);

      &:hover {
        background: var(--tr-accent-soft);
        color: var(--tr-accent-text);
      }
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
  }
</style>
