<script lang="ts">
  import { onMount } from 'svelte';
  import Tabs from '@trackly/kit/components/Tabs.svelte';
  import ThemeSwitcher from '@trackly/kit/components/ThemeSwitcher.svelte';
  import LinkLogo from './components/LinkLogo.svelte';
  import StatusLine from './components/StatusLine.svelte';
  import AccountMenu from './components/AccountMenu.svelte';
  import OfflineBanner from './components/OfflineBanner.svelte';
  import ConnectingScreen from './components/ConnectingScreen.svelte';
  import SetupScreen from './components/SetupScreen.svelte';
  import PinMismatchScreen from './components/PinMismatchScreen.svelte';
  import LoginScreen from './components/LoginScreen.svelte';
  import { linkStore, screenFor } from './lib/linkState';

  let activeTab = $state('chat');
  const tabs = [{ key: 'chat', label: 'Чат' }];

  onMount(() => {
    void linkStore.init();
  });

  const st = $derived($linkStore.state);
  const screen = $derived(screenFor(st));
  // Признак «трея нет» приходит из Rust, webview его не угадывает (D-20).
  const showQuit = $derived(!$linkStore.tray_available);
</script>

{#if screen === 'shell' && (st.kind === 'connected' || st.kind === 'offline')}
  <div class="shell" class:has-banner={st.kind === 'offline'}>
    <header class="topbar">
      <div class="logo-cell"><LinkLogo /></div>
      <div class="tabs-cell"><Tabs {tabs} bind:active={activeTab} ariaLabel="Разделы Link" /></div>
      <AccountMenu
        displayName={st.kind === 'connected' ? st.user.display_name : null}
        method={st.kind === 'connected' ? st.method : null}
        server={st.server}
        connection={st.kind === 'connected' ? 'connected' : 'offline'}
        {showQuit}
      />
      <div class="theme-cell"><ThemeSwitcher /></div>
    </header>
    {#if st.kind === 'offline'}
      <OfflineBanner />
    {/if}
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
{:else if st.kind === 'needs_setup'}
  <div class="screen"><SetupScreen {showQuit} /></div>
{:else if st.kind === 'pin_mismatch'}
  <div class="screen">
    <PinMismatchScreen server={st.server} expected={st.expected} got={st.got} {showQuit} />
  </div>
{:else if st.kind === 'needs_login' || st.kind === 'revoked'}
  <div class="screen">
    <LoginScreen
      linkState={st}
      secretsPersistent={$linkStore.secrets_persistent}
      platform={$linkStore.platform}
      {showQuit}
    />
  </div>
{:else}
  <div class="screen"><ConnectingScreen {showQuit} /></div>
{/if}

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

  .shell.has-banner {
    grid-template-rows: auto auto minmax(0, 1fr);
  }

  .screen {
    height: 100vh;
    overflow-y: auto;
    background: var(--tr-bg);
  }

  .topbar {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    gap: var(--tr-space-md);
    padding: 0 var(--tr-space-md);
    height: 56px;
    border-bottom: 1px solid var(--tr-border);
    background: var(--tr-surface);
  }

  .logo-cell {
    width: 96px;
    flex: none;
  }

  .tabs-cell {
    flex: 1 1 auto;
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
    padding: var(--tr-space-md);
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
    padding: var(--tr-space-xs) var(--tr-space-md);
    border-top: 1px solid var(--tr-border);
  }
</style>
