<script lang="ts">
  import NetworkSettings from '../features/settings/NetworkSettings.svelte';
  import OrgSettings from '../features/settings/OrgSettings.svelte';
  import StorageSettings from '../features/settings/StorageSettings.svelte';
  import BackupSettings from '../features/settings/BackupSettings.svelte';
  import ThresholdSettings from '../features/settings/ThresholdSettings.svelte';
  import TemplateEditor from '../features/settings/TemplateEditor.svelte';
</script>

<div class="settings-page">
  <header class="page-header">
    <h1 class="page-title">Настройки</h1>
  </header>
  <div class="settings-content">
    <!-- Section order per UI-SPEC §Settings Layout -->
    <!-- 1. Серверный режим + Безопасность рабочего стола -->
    <NetworkSettings />
    <!-- 2. Организация (SET-01, SET-02) -->
    <OrgSettings />
    <!-- 3. Хранилище данных (SET-03) -->
    <StorageSettings />
    <!-- 4. Бэкапы (SET-05, SET-06, SET-07) -->
    <BackupSettings />
    <!-- 5. Порог низкого остатка (SET-04) -->
    <ThresholdSettings />
    <!-- 6. Шаблоны документов (SET-09) — full-width -->
    <TemplateEditor />
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
