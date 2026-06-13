<script lang="ts">
  import NetworkSettings from '../features/settings/NetworkSettings.svelte';
</script>

<div class="settings-page">
  <header class="page-header">
    <h1 class="page-title">Настройки</h1>
  </header>
  <div class="settings-content">
    <NetworkSettings />
  </div>
</div>

<style lang="scss">
  .settings-page {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .page-header {
    padding: var(--space-lg) var(--space-xl);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .page-title {
    margin: 0;
    font-size: var(--font-size-page-title, var(--font-size-heading));
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  .settings-content {
    flex: 1;
    overflow: auto;
    padding: var(--space-lg) var(--space-xl);
  }
</style>
