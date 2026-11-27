<script lang="ts">
  // UI-SPEC §6.7: блок учётной записи в шапке. Триггер - собственная кнопка (двухстрочная),
  // меню - ActionMenu trigger="none" (общий примитив не менялся).
  import ActionMenu from '@trackly/kit/components/ActionMenu.svelte';
  import { logout, quit } from '../lib/linkApi';
  import type { LoginMethod } from '../lib/linkState';

  type Connection = 'connected' | 'connecting' | 'offline';

  interface Props {
    displayName?: string | null;
    method?: LoginMethod | null;
    server: string;
    connection: Connection;
    /** Linux без трея: добавляет «Закрыть Link» (D-20). */
    showQuit?: boolean;
  }

  const { displayName = null, method = null, server, connection, showQuit = false }: Props =
    $props();

  let menu = $state<{ openAt: (x: number, y: number) => void } | null>(null);
  let btn = $state<HTMLButtonElement | null>(null);
  let expanded = $state(false);

  const STATE_WORD: Record<Connection, string> = {
    connected: 'В сети',
    connecting: 'Подключение…',
    offline: 'Нет связи',
  };

  const stateWord = $derived(STATE_WORD[connection]);
  const methodWord = $derived(
    method === 'sso' ? 'Доменный вход' : method === 'password' ? 'Вход по паролю' : null,
  );
  const line1 = $derived(displayName ?? 'Ожидание сервера');
  const line2 = $derived(methodWord ? `${methodWord} · ${server}` : server);
  const fullTitle = $derived(
    [displayName ?? 'Ожидание сервера', methodWord, server, stateWord].filter(Boolean).join(' · '),
  );
  const ariaLabel = $derived(
    displayName
      ? `Учётная запись: ${displayName}, ${stateWord.toLowerCase()}`
      : `Учётная запись: ожидание сервера, ${stateWord.toLowerCase()}`,
  );

  function openMenu() {
    if (!btn || !menu) return;
    const r = btn.getBoundingClientRect();
    menu.openAt(r.left, r.bottom);
  }

  function doLogout() {
    // D-11: без подтверждения
    logout().catch(() => {});
  }
</script>

<div class="account">
  <button
    bind:this={btn}
    type="button"
    class="trigger"
    aria-haspopup="menu"
    aria-expanded={expanded}
    aria-label={ariaLabel}
    title={fullTitle}
    onclick={openMenu}
  >
    <span class="line1">
      <span
        class="dot"
        class:dot-ok={connection === 'connected'}
        class:dot-wait={connection === 'connecting'}
        class:dot-off={connection === 'offline'}
        aria-hidden="true"
      ></span>
      <span class="name">{line1}</span>
    </span>
    <span class="line2">{line2}</span>
  </button>

  <ActionMenu
    bind:this={menu}
    trigger="none"
    panelMinWidth="240px"
    onOpenChange={(o) => (expanded = o)}
  >
    <div class="menu-account">
      <span class="menu-account-name">{line1}</span>
      <span class="menu-account-meta">{line2} · {stateWord}</span>
    </div>
    <hr />
    <button type="button" role="menuitem" onclick={doLogout}>Выйти из учётной записи</button>
    {#if showQuit}
      <hr />
      <button type="button" role="menuitem" onclick={() => quit().catch(() => {})}
        >Закрыть Link</button
      >
    {/if}
  </ActionMenu>
</div>

<style lang="scss">
  .account {
    flex: 0 1 240px;
    min-width: 160px;
    max-width: 240px;
  }

  .trigger {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    width: 100%;
    min-width: 0;
    padding: var(--tr-space-2xs) var(--tr-space-xs);
    border: 0;
    border-radius: var(--tr-radius-md);
    background: transparent;
    color: var(--tr-text-primary);
    font-family: inherit;
    text-align: left;
    cursor: pointer;

    &:hover {
      background: var(--tr-surface-sunken);
    }
  }

  .line1 {
    display: flex;
    align-items: center;
    gap: var(--tr-space-2xs);
    max-width: 100%;
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-body-strong);
    line-height: 1.5;
  }

  .name,
  .line2 {
    min-width: 0;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .line2 {
    font-size: var(--tr-font-size-caption);
    line-height: 1.35;
    color: var(--tr-text-secondary);
  }

  .dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .dot-ok {
    background: var(--tr-success);
  }

  .dot-wait {
    background: var(--tr-text-tertiary);
  }

  .dot-off {
    background: var(--tr-warning);
  }

  .menu-account {
    display: flex;
    flex-direction: column;
    padding: var(--tr-space-xs) var(--tr-space-sm);
    max-width: 280px;
  }

  .menu-account-name {
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-body-strong);
    color: var(--tr-text-primary);
    overflow-wrap: anywhere;
  }

  .menu-account-meta {
    font-size: var(--tr-font-size-caption);
    color: var(--tr-text-secondary);
    overflow-wrap: anywhere;
  }
</style>
