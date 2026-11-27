<script lang="ts">
  // Вкладка «Коды приглашения» (фаза 44-18, UI-SPEC §8.3). Сам код в таблице не
  // показывается (на сервере только хэш). Отзыв — инлайн-подтверждение в строке.
  import Badge from '@trackly/kit/components/Badge.svelte';
  import Button from '@trackly/kit/components/Button.svelte';
  import Spinner from '@trackly/kit/components/Spinner.svelte';
  import TableRow from '@trackly/kit/components/TableRow.svelte';
  import { pushToast } from '@trackly/kit/stores/toast.svelte';
  import { formatDateTimeShort } from '@trackly/kit/utils/date';
  import Table from '$lib/components/Table.svelte';
  import type { InviteRowDto } from '../../bindings';
  import { linkApi } from './linkApi';

  interface Props {
    invites: InviteRowDto[];
    loading: boolean;
    error: boolean;
    onIssue: () => void;
    onChanged: () => void | Promise<void>;
  }

  const { invites, loading, error, onIssue, onChanged }: Props = $props();

  const STATUS: Record<string, { label: string; variant: 'success' | 'default' }> = {
    active: { label: 'Активен', variant: 'success' },
    used: { label: 'Использован', variant: 'default' },
    expired: { label: 'Истёк', variant: 'default' },
    revoked: { label: 'Отозван', variant: 'default' },
  };

  const sorted = $derived([...invites].sort((a, b) => b.createdAt - a.createdAt));

  let confirmId = $state<number | null>(null);
  let busyId = $state<number | null>(null);

  async function revoke(row: InviteRowDto) {
    busyId = row.id;
    try {
      await linkApi.invitesRevoke(row.id);
      confirmId = null;
      await onChanged();
    } catch {
      confirmId = null;
      pushToast('error', 'Не удалось отозвать код. Повторите.');
    } finally {
      busyId = null;
    }
  }
</script>

{#snippet head()}
  <th>Выпущен</th>
  <th>Кем</th>
  <th>Действует до</th>
  <th>Состояние</th>
  <th>Компьютер</th>
  <th class="th-actions"></th>
{/snippet}

{#if loading}
  <div class="center"><Spinner /></div>
{:else if error}
  <p class="error-text" role="alert">Не удалось загрузить коды приглашения. Обновите страницу.</p>
{:else if sorted.length === 0}
  <div class="empty">
    <p class="empty-title">Кодов приглашения нет</p>
    <p class="empty-text">Выпустите код, чтобы подключить компьютер сотрудника к Link.</p>
    <Button variant="secondary" onclick={onIssue}>Выпустить код приглашения</Button>
  </div>
{:else}
  <Table columns={6} {head}>
    {#each sorted as row (row.id)}
      {@const st = STATUS[row.status] ?? { label: row.status, variant: 'default' as const }}
      <TableRow>
        <td>{formatDateTimeShort(row.createdAt)}</td>
        <td>{row.createdByFullName}</td>
        <td>{formatDateTimeShort(row.expiresAt)}</td>
        <td><Badge variant={st.variant}>{st.label}</Badge></td>
        <td class:tr-mono={row.usedByHostname}>{row.usedByHostname ?? '—'}</td>
        <td class="actions">
          {#if row.status === 'active'}
            {#if confirmId === row.id}
              <span class="confirm">
                <span class="confirm-text">Отозвать?</span>
                <Button
                  variant="destructive"
                  size="sm"
                  loading={busyId === row.id}
                  onclick={() => revoke(row)}
                >
                  Отозвать код
                </Button>
                <Button
                  variant="secondary"
                  size="sm"
                  disabled={busyId === row.id}
                  onclick={() => (confirmId = null)}
                >
                  Отмена
                </Button>
              </span>
            {:else}
              <span class="danger-action">
                <Button variant="ghost" size="sm" onclick={() => (confirmId = row.id)}>
                  Отозвать
                </Button>
              </span>
            {/if}
          {/if}
        </td>
      </TableRow>
    {/each}
  </Table>
{/if}

<style lang="scss">
  .center {
    display: flex;
    justify-content: center;
    padding: var(--tr-space-2xl);
  }

  .error-text {
    margin: 0;
    font-size: var(--tr-font-size-body);
    color: var(--tr-danger-text);
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--tr-space-sm);
  }

  .empty-title {
    margin: 0;
    font-size: var(--tr-font-size-body-strong);
    font-weight: var(--tr-font-weight-body-strong);
    color: var(--tr-text-primary);
  }

  .empty-text {
    margin: 0;
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-secondary);
  }

  .actions {
    text-align: right;
    white-space: nowrap;
  }

  .confirm {
    display: inline-flex;
    align-items: center;
    gap: var(--tr-space-xs);
  }

  .confirm-text {
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
  }

  .danger-action :global(.btn) {
    color: var(--tr-danger-text);
  }
</style>
