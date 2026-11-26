<script lang="ts">
  import Tabs from '@trackly/kit/components/Tabs.svelte';
  import ThemeSwitcher from '@trackly/kit/components/ThemeSwitcher.svelte';
  import LinkLogo from './components/LinkLogo.svelte';
  import StatusLine from './components/StatusLine.svelte';

  let activeTab = $state('chat');
  const tabs = [{ key: 'chat', label: 'Чат' }];
</script>

<div class="shell">
  <header class="topbar">
    <div class="logo-cell"><LinkLogo /></div>
    <div class="tabs-cell"><Tabs {tabs} bind:active={activeTab} ariaLabel="Разделы Link" /></div>
    <div class="theme-cell"><ThemeSwitcher /></div>
  </header>
  <aside class="sidebar" aria-label="Список бесед">
    <p class="sidebar-stub">Беседы появятся здесь</p>
  </aside>
  <main class="content">
    <div class="stub">
      <p class="stub-text">Здесь скоро будет чат</p>
    </div>
    <footer class="footer"><StatusLine /></footer>
  </main>
</div>

<style lang="scss">
  .shell {
    display: grid;
    grid-template-columns: 240px minmax(0, 1fr);
    grid-template-rows: auto minmax(0, 1fr);
    height: 100vh;
    min-height: 0;
    overflow: hidden;
    background: var(--tr-bg);
    color: var(--tr-text-primary);
  }

  .topbar {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 0 16px;
    height: 48px;
    border-bottom: 1px solid var(--tr-border);
    background: var(--tr-surface);
  }

  .logo-cell {
    width: 96px;
    flex: none;
  }

  .tabs-cell {
    flex: 1;
    min-width: 0;
  }

  .theme-cell {
    flex: none;
    width: 240px;
  }

  .sidebar {
    height: 100%;
    min-height: 0;
    overflow-y: auto;
    padding: 16px;
    border-right: 1px solid var(--tr-border);
    background: var(--tr-surface-sunken);
  }

  .sidebar-stub {
    margin: 0;
    color: var(--tr-text-tertiary);
    font-size: var(--tr-font-size-caption);
  }

  .content {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    overflow: hidden;
  }

  .stub {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .stub-text {
    margin: 0;
    color: var(--tr-text-secondary);
    font-size: var(--tr-font-size-subtitle);
  }

  .footer {
    flex: none;
    padding: 8px 16px;
    border-top: 1px solid var(--tr-border);
  }
</style>
