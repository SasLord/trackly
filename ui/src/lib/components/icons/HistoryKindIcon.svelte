<script lang="ts">
  // Фаза 41.10 (UI-SPEC §7.3): значок вида события слева в строке ленты.
  // Любая нераспознанная строка рисуется нейтральной точкой.
  interface Props {
    kind: string;
  }
  const { kind }: Props = $props();
</script>

<svg
  class="history-kind-icon"
  width="16"
  height="16"
  viewBox="0 0 16 16"
  fill="none"
  stroke="currentColor"
  stroke-width="1.5"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if kind === 'move'}
    <path d="M3 8h10M9 4l4 4-4 4" />
  {:else if kind === 'membership'}
    <rect x="2" y="5" width="7" height="7" rx="1" />
    <path d="M7 5V3.5a1 1 0 0 1 1-1h4.5a1 1 0 0 1 1 1V8a1 1 0 0 1-1 1H9" />
  {:else if kind === 'status'}
    <path d="M2.5 2.5h5l6 6-5 5-6-6z" />
    <circle cx="5.5" cy="5.5" r="1" />
  {:else}
    <circle cx="8" cy="8" r="1.5" fill="currentColor" />
  {/if}
</svg>

<style lang="scss">
  .history-kind-icon {
    flex: none;
    color: var(--tr-text-secondary);
  }
</style>
