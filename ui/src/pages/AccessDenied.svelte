<script lang="ts">
  // Plan 10-04 (D-DENY-01): экран показывается, когда Сотрудник переходит по прямой
  // ссылке/хэшу на раздел, к которому у его роли нет доступа. Структура — точная копия
  // NotFound.svelte с изменённым текстом и целевым CTA.
  import Button from '$lib/components/Button.svelte';

  // svelte-spa-router passes the current location as a prop (unused here — the
  // copy is the same regardless of which forbidden route was hit).
  interface Props {
    location?: { hash: string };
  }

  // Props are accepted (router passes `location`) but intentionally unused — the
  // copy is route-independent. `void` keeps both eslint (no-empty-pattern) and
  // svelte-check (noUnusedLocals) satisfied without reintroducing a dev warning.
  const props: Props = $props();
  void props;
</script>

<div class="access-denied">
  <h2 class="access-denied-heading">Нет доступа</h2>
  <p class="access-denied-body">
    У вашей роли («Сотрудник») нет доступа к этому разделу. Доступны только заявки.
  </p>
  <Button variant="secondary" onclick={() => (window.location.hash = '/requests')}>К заявкам</Button
  >
</div>

<style lang="scss">
  .access-denied {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--tr-space-4xl);
    text-align: center;
    min-height: 300px;
    gap: var(--tr-space-md);
  }

  .access-denied-heading {
    margin: 0;
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .access-denied-body {
    margin: 0;
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-secondary);
  }
</style>
