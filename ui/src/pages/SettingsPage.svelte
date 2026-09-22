<script lang="ts">
  import NetworkSettings from '../features/settings/NetworkSettings.svelte';
  import OrgSettings from '../features/settings/OrgSettings.svelte';
  import StorageSettings from '../features/settings/StorageSettings.svelte';
  import BackupSettings from '../features/settings/BackupSettings.svelte';
  import ThresholdSettings from '../features/settings/ThresholdSettings.svelte';
  import TemplateEditor from '../features/settings/TemplateEditor.svelte';
  import ActiveDirectorySettings from '../features/settings/ActiveDirectorySettings.svelte';
  import SettingsSubNav from '../features/settings/SettingsSubNav.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import { untrack } from 'svelte';
  import { router } from 'svelte-spa-router';

  // FE-WR-13: `#/settings?section=org` (ссылка из меню «Вставка» поля номера)
  // открывает нужный раздел. Неизвестное/отсутствующее значение — прежнее
  // поведение: первая вкладка.
  const SECTION_KEYS = ['network', 'org', 'storage', 'threshold', 'templates', 'ad'];

  function sectionFromQuery(qs: string | undefined): string | null {
    const s = new URLSearchParams(qs ?? '').get('section');
    return s !== null && SECTION_KEYS.includes(s) ? s : null;
  }

  // GAP-S2: track active subsection; default to 'network' (first tab)
  let activeSection = $state(sectionFromQuery(router.querystring) ?? 'network');

  // Переход на `?section=…`, когда страница уже открыта (hash меняется без
  // размонтирования). Эффект читает только querystring и пишет activeSection.
  $effect(() => {
    const s = sectionFromQuery(router.querystring);
    if (s !== null) {
      untrack(() => {
        activeSection = s;
      });
    }
  });
</script>

<div class="settings-page">
  <PageHeader title="Настройки" />
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
      <!-- Хранилище данных (SET-03) + Бэкапы (SET-05, SET-06, SET-07) — объединены в один раздел «Хранилище» -->
      <StorageSettings />
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

  .settings-content {
    flex: 1;
    overflow: auto;
    padding: var(--tr-space-xl) var(--tr-space-2xl);
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xl);
  }
</style>
