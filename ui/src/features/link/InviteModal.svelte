<script lang="ts">
  // Попап «Код приглашения» (фаза 44-18, UI-SPEC §8.4; D-05). Два шага в одном окне.
  // Код живёт ТОЛЬКО в локальном состоянии этого компонента: LinkPage рендерит его
  // через {#if}, закрытие размонтирует и стирает код (T-44-70); в таблицу он не попадает.
  import { onMount } from 'svelte';
  import Button from '@trackly/kit/components/Button.svelte';
  import Dropdown from '@trackly/kit/components/Dropdown.svelte';
  import FormField from '@trackly/kit/components/FormField.svelte';
  import Input from '@trackly/kit/components/Input.svelte';
  import Modal from '@trackly/kit/components/Modal.svelte';
  import { pushToast } from '@trackly/kit/stores/toast.svelte';
  import { formatDateTimeShort } from '@trackly/kit/utils/date';
  import { apiCall } from '@trackly/transport/client';
  import type { InviteIssuedDto, NetworkSettingsDto } from '../../bindings';
  import { linkApi } from './linkApi';

  interface Props {
    onClose: () => void;
    /** Код выпущен — перечитать список (сам код наружу не передаётся). */
    onIssued: () => void;
  }

  const { onClose, onIssued }: Props = $props();

  const TTL_OPTIONS = [
    { id: '1', label: '1 час' },
    { id: '24', label: '24 часа' },
    { id: '168', label: '7 дней' },
  ];

  let host = $state('');
  let ttl = $state('24');
  let submitting = $state(false);
  let failed = $state(false);
  let serverDown = $state(false);
  let issued = $state<InviteIssuedDto | null>(null);
  let copied = $state(false);
  let copiedTimer: ReturnType<typeof setTimeout> | null = null;

  const ttlLabel = $derived(TTL_OPTIONS.find((o) => o.id === ttl)?.label ?? '');
  const hostIsIp = $derived(isIpAddress(host.trim()));

  function isDigits(p: string): boolean {
    return p.length > 0 && p.length <= 3 && [...p].every((c) => c >= '0' && c <= '9');
  }

  /** IPv4 (четыре группы цифр) или IPv6 (есть двоеточие, только hex/`:`/скобки). */
  function isIpAddress(h: string): boolean {
    const parts = h.split('.');
    if (parts.length === 4 && parts.every(isDigits)) return true;
    return h.includes(':') && [...h].every((c) => '0123456789abcdefABCDEF:[]'.includes(c));
  }

  function noExpand(): { id: string; label: string }[] {
    return [];
  }

  onMount(() => {
    // Адрес по умолчанию — из настроек сети (имя, если известно); необязателен.
    apiCall<NetworkSettingsDto>('settings_get_network', {})
      .then((s) => {
        if (host === '' && s.server_url) {
          try {
            host = new URL(s.server_url).hostname;
          } catch {
            // Нечитаемый URL — поле остаётся пустым, сервер подставит свой адрес.
          }
        }
      })
      .catch(() => {});
    return () => {
      if (copiedTimer) clearTimeout(copiedTimer);
    };
  });

  async function issue() {
    failed = false;
    submitting = true;
    try {
      issued = await linkApi.invitesCreate({ host: host.trim(), ttlHours: Number(ttl) });
      onIssued();
    } catch (e: unknown) {
      const code =
        e && typeof e === 'object' && 'code' in e ? String((e as { code: unknown }).code) : '';
      if (code.toUpperCase() === 'CONFLICT') serverDown = true;
      else failed = true;
    } finally {
      submitting = false;
    }
  }

  async function copy() {
    if (!issued) return;
    try {
      await navigator.clipboard.writeText(issued.code);
      copied = true;
      if (copiedTimer) clearTimeout(copiedTimer);
      copiedTimer = setTimeout(() => (copied = false), 2000);
    } catch {
      pushToast('error', 'Не удалось скопировать. Выделите код и скопируйте вручную.');
    }
  }
</script>

<Modal open={true} title="Код приглашения" size="md" {onClose}>
  {#if issued}
    <div class="code-block tr-mono">{issued.code}</div>
    <p class="caption">
      Действует до {formatDateTimeShort(issued.expiresAt)}. Используется один раз. После закрытия
      окна код больше не показывается.
    </p>
    <p class="caption">Сервер: {issued.host}</p>
  {:else}
    <p class="lead">
      Код подключает один компьютер к этому серверу Trackly. Он не входит в учётную запись: после
      подключения сотрудник войдёт сам — автоматически или паролем.
    </p>
    {#if serverDown}
      <div class="callout callout-danger" role="alert">
        Сервер Trackly не запущен. Запустите его в разделе «Настройки → Сеть» и выпустите код снова.
      </div>
    {:else}
      <FormField
        label="Адрес сервера для Link"
        id="link-invite-host"
        hint="Для автоматического входа укажите имя сервера, а не IP-адрес."
      >
        {#snippet children({ describedBy, invalid })}
          <Input
            id="link-invite-host"
            value={host}
            disabled={submitting}
            {invalid}
            aria-describedby={describedBy}
            oninput={(v) => (host = v)}
          />
        {/snippet}
      </FormField>
      {#if hostIsIp}
        <div class="callout callout-warning" role="note">
          По IP-адресу автоматический вход не работает: сотрудники будут входить по паролю.
        </div>
      {/if}
      <div class="field">
        <span class="field-label">Срок действия</span>
        <Dropdown
          variant="select"
          flat={true}
          searchable={false}
          value={ttlLabel}
          placeholder="Выберите срок"
          disabled={submitting}
          loading={false}
          groups={TTL_OPTIONS}
          getGroupId={(o) => o.id}
          getGroupName={(o) => o.label}
          getGroupCount={() => 0}
          isGroupExpandable={() => false}
          isGroupSelected={(o) => o.id === ttl}
          onExpandGroup={noExpand}
          getMemberId={(o) => o.id}
          getMemberName={(o) => o.label}
          onSearch={() => {}}
          onPickGroup={(o) => (ttl = o.id)}
          onPickMember={() => {}}
        />
      </div>
      {#if failed}
        <p class="error-text" role="alert">Не удалось выпустить код. Повторите.</p>
      {/if}
    {/if}
  {/if}
  {#snippet footer()}
    {#if issued}
      <Button variant="primary" onclick={copy}>{copied ? 'Скопировано' : 'Скопировать код'}</Button>
      <Button variant="secondary" onclick={onClose}>Готово</Button>
    {:else}
      <Button variant="secondary" disabled={submitting} onclick={onClose}>Отмена</Button>
      <Button variant="primary" loading={submitting} disabled={serverDown} onclick={issue}>
        Выпустить код
      </Button>
    {/if}
  {/snippet}
</Modal>

<style lang="scss">
  .lead {
    margin: 0 0 var(--tr-space-md);
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-secondary);
    line-height: 1.5;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
    margin-top: var(--tr-space-md);
  }

  .field-label {
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-label);
    color: var(--tr-text-primary);
  }

  .callout {
    margin-top: var(--tr-space-sm);
    padding: var(--tr-space-sm) var(--tr-space-md);
    border-radius: var(--tr-radius-md);
    font-size: var(--tr-font-size-body);
  }

  .callout-warning {
    background: var(--tr-warning-soft);
    color: var(--tr-warning-text);
  }

  .callout-danger {
    background: var(--tr-danger-soft);
    color: var(--tr-danger-text);
  }

  .error-text {
    margin: var(--tr-space-sm) 0 0;
    font-size: var(--tr-font-size-body);
    color: var(--tr-danger-text);
  }

  .code-block {
    padding: var(--tr-space-md);
    background: var(--tr-surface-sunken);
    border-radius: var(--tr-radius-md);
    overflow-wrap: anywhere;
    user-select: all;
  }

  .caption {
    margin: var(--tr-space-sm) 0 0;
    font-size: var(--tr-font-size-caption);
    color: var(--tr-text-secondary);
  }
</style>
