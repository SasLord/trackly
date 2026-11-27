<script lang="ts">
  // Вкладка «Входы» (фаза 44-18; D-12, D-13): активные входы + запрещённые компьютеры.
  // Подтверждение отзыва живёт в LinkPage (Modal не внутри <td>); здесь только колбэки.
  import Badge from '@trackly/kit/components/Badge.svelte';
  import Button from '@trackly/kit/components/Button.svelte';
  import Spinner from '@trackly/kit/components/Spinner.svelte';
  import TableRow from '@trackly/kit/components/TableRow.svelte';
  import { formatDateTimeShort } from '@trackly/kit/utils/date';
  import Table from '$lib/components/Table.svelte';
  import type { BannedDeviceDto, LinkLoginRowDto } from '../../bindings';

  interface Props {
    logins: LinkLoginRowDto[];
    banned: BannedDeviceDto[];
    loading: boolean;
    error: boolean;
    allowingId: string | null;
    onRevoke: (row: LinkLoginRowDto) => void;
    onAllow: (device: BannedDeviceDto) => void;
  }

  const { logins, banned, loading, error, allowingId, onRevoke, onAllow }: Props = $props();

  const METHOD_LABELS: Record<string, string> = { sso: 'Доменный', password: 'Пароль' };
</script>

{#snippet loginsHead()}
  <th>Сотрудник</th>
  <th>Компьютер</th>
  <th>Способ входа</th>
  <th>Вход выполнен</th>
  <th class="th-actions"></th>
{/snippet}

{#snippet bannedHead()}
  <th>Компьютер</th>
  <th>Последний сотрудник</th>
  <th>Запрещён</th>
  <th>Состояние</th>
  <th class="th-actions"></th>
{/snippet}

<section class="block">
  <h2 class="block-title">Активные входы</h2>
  {#if loading}
    <div class="center"><Spinner /></div>
  {:else if error}
    <p class="error-text" role="alert">Не удалось загрузить входы Link. Обновите страницу.</p>
  {:else if logins.length === 0}
    <div class="empty">
      <p class="empty-title">Активных входов нет</p>
      <p class="empty-text">
        Здесь появятся сотрудники, которые вошли в Link. Чтобы подключить компьютер, выпустите код
        приглашения.
      </p>
    </div>
  {:else}
    <Table columns={5} head={loginsHead}>
      {#each logins as row (row.loginId)}
        <TableRow>
          <td>
            <div>{row.fullName}</div>
            <div class="caption">{row.login}</div>
          </td>
          <td class="tr-mono">{row.hostname}</td>
          <td>{METHOD_LABELS[row.method] ?? row.method}</td>
          <td>{formatDateTimeShort(row.createdAt)}</td>
          <td class="actions">
            <span class="danger-action">
              <Button variant="ghost" size="sm" onclick={() => onRevoke(row)}>Отозвать</Button>
            </span>
          </td>
        </TableRow>
      {/each}
    </Table>
  {/if}
</section>

{#if !loading && banned.length > 0}
  <section class="block">
    <h2 class="block-title">Запрещённые компьютеры</h2>
    <Table columns={5} head={bannedHead}>
      {#each banned as dev (dev.deviceId)}
        <TableRow>
          <td class="tr-mono">{dev.hostname}</td>
          <td>{dev.lastUserFullName ?? '—'}</td>
          <td>{formatDateTimeShort(dev.revokedAt)}</td>
          <td><Badge variant="destructive">Запрещён</Badge></td>
          <td class="actions">
            <Button
              variant="secondary"
              size="sm"
              loading={allowingId === dev.deviceId}
              onclick={() => onAllow(dev)}
            >
              Разрешить
            </Button>
          </td>
        </TableRow>
      {/each}
    </Table>
  </section>
{/if}

<style lang="scss">
  .block {
    margin-bottom: var(--tr-space-2xl);
  }

  .block-title {
    margin: 0 0 var(--tr-space-md);
    font-size: var(--tr-font-size-body-strong);
    font-weight: var(--tr-font-weight-body-strong);
    color: var(--tr-text-primary);
  }

  .center {
    display: flex;
    justify-content: center;
    padding: var(--tr-space-2xl);
  }

  .caption {
    font-size: var(--tr-font-size-caption);
    color: var(--tr-text-secondary);
  }

  .error-text {
    margin: 0;
    font-size: var(--tr-font-size-body);
    color: var(--tr-danger-text);
  }

  .empty-title {
    margin: 0 0 var(--tr-space-2xs);
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

  .danger-action :global(.btn) {
    color: var(--tr-danger-text);
  }
</style>
