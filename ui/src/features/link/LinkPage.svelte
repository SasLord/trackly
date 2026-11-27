<script lang="ts">
  // Раздел «Link» (фаза 44, план 18; D-05, D-12, D-13): активные входы, запрещённые
  // компьютеры, коды приглашения. Только администратор (сайдбар + карта маршрутов +
  // серверная роль). Модалки — НА УРОВНЕ СТРАНИЦЫ (не внутри <td>: Modal наследует
  // white-space/text-align ячейки). Обновление на лету по WsEvent link_changed, без тоста.
  import { onMount } from 'svelte';
  import Button from '@trackly/kit/components/Button.svelte';
  import Modal from '@trackly/kit/components/Modal.svelte';
  import Tabs from '@trackly/kit/components/Tabs.svelte';
  import { pushToast } from '@trackly/kit/stores/toast.svelte';
  import { connectWs, onWsEvent, onWsResync } from '@trackly/transport/ws';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import type { BannedDeviceDto, InviteRowDto, LinkLoginRowDto } from '../../bindings';
  import { linkApi } from './linkApi';
  import LoginsTab from './LoginsTab.svelte';
  import InvitesTab from './InvitesTab.svelte';
  import InviteModal from './InviteModal.svelte';

  type TabKey = 'logins' | 'codes';

  function tabFromHash(): TabKey {
    return /[?&]tab=codes\b/.test(window.location.hash) ? 'codes' : 'logins';
  }

  let tab = $state<TabKey>(tabFromHash());
  let logins = $state<LinkLoginRowDto[]>([]);
  let banned = $state<BannedDeviceDto[]>([]);
  let invites = $state<InviteRowDto[]>([]);
  let loading = $state(true);
  let loginsError = $state(false);
  let invitesError = $state(false);

  let inviteOpen = $state(false);
  let revokeTarget = $state<LinkLoginRowDto | null>(null);
  let revoking = $state(false);
  let allowingId = $state<string | null>(null);

  const loginsSorted = $derived([...logins].sort((a, b) => b.createdAt - a.createdAt));
  const tabs = $derived([
    { key: 'logins', label: 'Входы', count: logins.length },
    { key: 'codes', label: 'Коды приглашения' },
  ]);

  let seq = 0;

  async function loadAll(announceErrors: boolean) {
    const my = ++seq;
    const [l, b, i] = await Promise.allSettled([
      linkApi.loginsList(),
      linkApi.devicesList(),
      linkApi.invitesList(),
    ]);
    if (my !== seq) return;
    if (l.status === 'fulfilled') logins = l.value;
    if (b.status === 'fulfilled') banned = b.value;
    if (i.status === 'fulfilled') invites = i.value;
    const lErr = l.status === 'rejected' || b.status === 'rejected';
    const iErr = i.status === 'rejected';
    if (announceErrors) {
      if (lErr && !loginsError)
        pushToast('error', 'Не удалось загрузить входы Link. Обновите страницу.');
      if (iErr && !invitesError)
        pushToast('error', 'Не удалось загрузить коды приглашения. Обновите страницу.');
    }
    loginsError = lErr;
    invitesError = iErr;
    loading = false;
  }

  function onTabChange(key: string) {
    const next: TabKey = key === 'codes' ? 'codes' : 'logins';
    tab = next;
    window.history.replaceState(null, '', next === 'codes' ? '#/link?tab=codes' : '#/link');
  }

  async function confirmRevoke() {
    if (!revokeTarget) return;
    revoking = true;
    try {
      await linkApi.loginsRevoke(revokeTarget.loginId);
      revokeTarget = null;
      pushToast('success', 'Вход отозван');
      await loadAll(false);
    } catch {
      pushToast('error', 'Не удалось отозвать вход. Повторите.');
    } finally {
      revoking = false;
    }
  }

  async function allow(device: BannedDeviceDto) {
    allowingId = device.deviceId;
    try {
      await linkApi.devicesAllow(device.deviceId);
      pushToast('success', `Компьютер ${device.hostname} снова может входить в Link`);
      await loadAll(false);
    } catch {
      pushToast('error', 'Не удалось разрешить компьютер. Повторите.');
    } finally {
      allowingId = null;
    }
  }

  onMount(() => {
    void loadAll(true);
    let disposed = false;
    let release: (() => void) | undefined;
    connectWs()
      .then((fn) => {
        if (disposed) fn();
        else release = fn;
      })
      .catch(() => {
        // WS необязателен: списки перечитываются после каждого действия.
      });
    const unsub = onWsEvent((e) => {
      if (e.type === 'link_changed') void loadAll(false);
    });
    const unsubResync = onWsResync(() => {
      void loadAll(false);
    });
    return () => {
      disposed = true;
      unsub();
      unsubResync();
      release?.();
    };
  });
</script>

<div class="link-page">
  <PageHeader title="Link">
    {#snippet actions()}
      <Button variant="primary" onclick={() => (inviteOpen = true)}
        >Выпустить код приглашения</Button
      >
    {/snippet}
  </PageHeader>

  <div class="tabs-row">
    <Tabs variant="underline" ariaLabel="Разделы Link" {tabs} active={tab} onchange={onTabChange} />
  </div>

  <div class="page-content">
    {#if tab === 'logins'}
      <LoginsTab
        logins={loginsSorted}
        {banned}
        {loading}
        error={loginsError}
        {allowingId}
        onRevoke={(row) => (revokeTarget = row)}
        onAllow={allow}
      />
    {:else}
      <InvitesTab
        {invites}
        {loading}
        error={invitesError}
        onIssue={() => (inviteOpen = true)}
        onChanged={() => loadAll(false)}
      />
    {/if}
  </div>
</div>

{#if inviteOpen}
  <InviteModal
    onClose={() => (inviteOpen = false)}
    onIssued={() => {
      void loadAll(false);
    }}
  />
{/if}

<Modal
  open={revokeTarget !== null}
  title="Отозвать вход?"
  size="md"
  initialFocus=".revoke-cancel button"
  onClose={() => {
    if (!revoking) revokeTarget = null;
  }}
>
  {#if revokeTarget}
    <p class="revoke-body">
      <strong>{revokeTarget.fullName}</strong> на компьютере
      <strong>{revokeTarget.hostname}</strong>
      будет сразу отключён от Link. Компьютер будет запрещён: войти с него не получится ни паролем, ни
      автоматически, пока вы его не разрешите.
    </p>
  {/if}
  {#snippet footer()}
    <span class="revoke-cancel">
      <Button variant="secondary" disabled={revoking} onclick={() => (revokeTarget = null)}>
        Отмена
      </Button>
    </span>
    <Button variant="destructive" loading={revoking} onclick={confirmRevoke}>Отозвать вход</Button>
  {/snippet}
</Modal>

<style lang="scss">
  .link-page {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .tabs-row {
    padding: 0 var(--tr-space-2xl);
  }

  .page-content {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: var(--tr-space-xl) var(--tr-space-2xl);
  }

  .revoke-body {
    margin: 0;
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
    line-height: 1.5;
  }
</style>
