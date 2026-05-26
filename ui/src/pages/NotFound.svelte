<script lang="ts">
  import Button from '$lib/components/Button.svelte';

  // svelte-spa-router passes the current location as a prop
  interface Props {
    location?: { hash: string };
  }

  const { location }: Props = $props();
  const hashPath = $derived(location?.hash ?? '');
</script>

<div class="not-found">
  <h2 class="not-found-heading">Страница не найдена</h2>
  <p class="not-found-body">
    Раздел <code>{hashPath}</code> не существует. Откройте навигацию слева.
  </p>
  <Button variant="secondary" onclick={() => (window.location.hash = '/')}>На главную</Button>
</div>

<style lang="scss">
  .not-found {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--space-2xl);
    text-align: center;
    min-height: 300px;
    gap: var(--space-md);
  }

  .not-found-heading {
    margin: 0;
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  .not-found-body {
    margin: 0;
    font-size: var(--font-size-body);
    color: var(--color-text-secondary);

    code {
      font-family: monospace;
      background: var(--color-surface-sunken);
      padding: 2px 6px;
      border-radius: var(--radius-sm);
    }
  }
</style>
