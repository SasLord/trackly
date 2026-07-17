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
    padding: var(--tr-space-4xl);
    text-align: center;
    min-height: 300px;
    gap: var(--tr-space-md);
  }

  .not-found-heading {
    margin: 0;
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .not-found-body {
    margin: 0;
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-secondary);

    code {
      font-family: monospace;
      background: var(--tr-surface-sunken);
      padding: 2px 6px;
      border-radius: var(--tr-radius-xs);
    }
  }
</style>
