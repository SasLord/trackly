<script lang="ts">
  // UI-SPEC §6.8: плашка «Нет связи с сервером». Обратного отсчёта нет.
  import Button from '@trackly/kit/components/Button.svelte';
  import { retryNow } from '../lib/linkApi';

  let loading = $state(false);

  async function retry() {
    loading = true;
    try {
      await retryNow();
    } catch {
      // неудача: плашка остаётся, отдельной ошибки нет
    } finally {
      loading = false;
    }
  }
</script>

<div class="banner" role="status" aria-live="polite">
  <svg
    class="icon"
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
    <path d="M4.5 12.5h6.5a3 3 0 0 0 .5-5.96A4 4 0 0 0 4 6.5a3 3 0 0 0 .5 6z" />
    <path d="M2 2l12 12" />
  </svg>
  <span class="text">Нет связи с сервером · переподключение…</span>
  <Button variant="secondary" size="sm" {loading} onclick={retry}>Повторить сейчас</Button>
</div>

<style lang="scss">
  .banner {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: var(--tr-space-sm);
    padding: var(--tr-space-xs) var(--tr-space-md);
    background: var(--tr-warning-soft);
    color: var(--tr-warning-text);
    border-bottom: 1px solid var(--tr-border);
    font-size: var(--tr-font-size-body);
  }

  .icon {
    flex: none;
  }

  .text {
    flex: 1;
    min-width: 0;
  }
</style>
