<script lang="ts">
  import { isPrinterType } from '$lib/deviceTypeIds';

  // Маркер принтера перед именем строки (фаза 41.9, D-20, UI-SPEC §7.5).
  // Для других типов ничего не рисует; обработчиков событий нет.
  interface Props {
    typeId?: number;
    /** Серверный признак принтера (41.6, D-05); если передан, приоритетнее typeId. */
    isPrinter?: boolean;
  }

  const { typeId, isPrinter }: Props = $props();
</script>

{#if isPrinter !== undefined ? isPrinter : isPrinterType(typeId)}
  <span class="printer-mark" role="img" aria-label="Принтер" title="Принтер">
    <svg
      width="14"
      height="14"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
    >
      <path d="M6 9V3h12v6" />
      <path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2" />
      <rect x="6" y="14" width="12" height="7" />
    </svg>
  </span>
{/if}

<style lang="scss">
  .printer-mark {
    display: inline-block;
    flex-shrink: 0;
    vertical-align: middle;
    line-height: 0;
    margin-right: var(--tr-space-xs);
    color: var(--tr-text-secondary);
  }
</style>
