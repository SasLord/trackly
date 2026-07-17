<script lang="ts">
  import NetworkSettings from '../features/settings/NetworkSettings.svelte';
  import OrgSettings from '../features/settings/OrgSettings.svelte';
  import StorageSettings from '../features/settings/StorageSettings.svelte';
  import BackupSettings from '../features/settings/BackupSettings.svelte';
  import ThresholdSettings from '../features/settings/ThresholdSettings.svelte';
  import TemplateEditor from '../features/settings/TemplateEditor.svelte';
  import ActiveDirectorySettings from '../features/settings/ActiveDirectorySettings.svelte';
  import SettingsSubNav from '../features/settings/SettingsSubNav.svelte';

  // GAP-S2: track active subsection; default to 'network' (first tab)
  let activeSection = $state('network');
</script>

<div class="settings-page">
  <header class="page-header">
    <h1 class="page-title">Настройки</h1>
  </header>
  <div class="settings-content">
    <!-- GAP-S2: sub-section switch-bar -->
    <SettingsSubNav {activeSection} onSectionChange={(s) => (activeSection = s)} />

    <!-- GAP-S2: show only the active subsection -->
    {#if activeSection === 'network'}
      <!-- Серверный режим + Безопасность рабочего стола -->
      <NetworkSettings />
    {:else if activeSection === 'org'}
      <!-- Организация (SET-01, SET-02) -->
      <OrgSettings />
    {:else if activeSection === 'storage'}
      <!-- Хранилище данных (SET-03) -->
      <StorageSettings />
    {:else if activeSection === 'backup'}
      <!-- Бэкапы (SET-05, SET-06, SET-07) -->
      <BackupSettings />
    {:else if activeSection === 'threshold'}
      <!-- Порог низкого остатка (SET-04) -->
      <ThresholdSettings />
    {:else if activeSection === 'templates'}
      <!-- Шаблоны документов (SET-09) — full-width -->
      <TemplateEditor />
    {:else if activeSection === 'ad'}
      <!-- Active Directory (SET-10) -->
      <ActiveDirectorySettings />
    {/if}
  </div>
</div>

<style lang="scss">
  .settings-page {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .page-header {
    padding: var(--tr-space-xl) var(--tr-space-2xl);
    border-bottom: 1px solid var(--tr-border);
    flex-shrink: 0;
  }

  .page-title {
    margin: 0;
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .settings-content {
    flex: 1;
    overflow: auto;
    padding: var(--tr-space-xl) var(--tr-space-2xl);
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xl);
  }
</style>
