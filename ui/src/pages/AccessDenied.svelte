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

  const {}: Props = $props();
</script>

<div class="access-denied">
  <h2 class="access-denied-heading">Нет доступа</h2>
  <p class="access-denied-body">
    У вашей роли («Сотрудник») нет доступа к этому разделу. Доступны только заявки.
  </p>
  <Button variant="secondary" onclick={() => (window.location.hash = '/requests')}
    >К заявкам</Button
  >
</div>

<style lang="scss">
  .access-denied {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--space-2xl);
    text-align: center;
    min-height: 300px;
    gap: var(--space-md);
  }

  .access-denied-heading {
    margin: 0;
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  .access-denied-body {
    margin: 0;
    font-size: var(--font-size-body);
    color: var(--color-text-secondary);
  }
</style>
