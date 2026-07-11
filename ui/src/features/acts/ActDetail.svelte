<script lang="ts">
  // Plan 03-02: detail panel (slave). Renders header + items + history-возвратов +
  // action buttons. Возврат/Печать disabled until plans 03/04.
  import Button from '$lib/components/Button.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import ActHeaderField from './ActHeaderField.svelte';
  import ActItemsTable from './ActItemsTable.svelte';
  import type { ActDto } from '../../bindings';

  interface Props {
    act: ActDto | null;
    loading: boolean;
    onCreate: () => void;
    onDelete: (_act: ActDto) => void;
    onEdit?: (_act: ActDto) => void;
    onReturn?: (_act: ActDto) => void;
    onPrint?: (_act: ActDto) => void;
  }

  const { act, loading, onCreate, onDelete, onEdit, onReturn, onPrint }: Props = $props();

  const MONTHS_RU = [
    'января',
    'февраля',
    'марта',
    'апреля',
    'мая',
    'июня',
    'июля',
    'августа',
    'сентября',
    'октября',
    'ноября',
    'декабря',
  ];

  function formatDate(utcSeconds: number | null): string | null {
    if (utcSeconds === null) return null;
    const d = new Date(utcSeconds * 1000);
    return `${d.getUTCDate()} ${MONTHS_RU[d.getUTCMonth()]} ${d.getUTCFullYear()}`;
  }

  const headerDate = $derived(act ? formatDate(act.handover_date_utc) : null);
  const deadlineLabel = $derived(act?.deadline_utc != null ? formatDate(act.deadline_utc) : null);
</script>

<div class="act-detail" aria-live="polite">
  {#if loading}
    <div class="loading">
      <Spinner size="md" />
      <span>Загружаем акт…</span>
    </div>
  {:else if act === null}
    <div class="empty">
      <h2 class="empty-heading">Выберите акт</h2>
      <p class="empty-body">Выберите акт слева, чтобы увидеть подробности, или создайте новый.</p>
      <Button variant="primary" onclick={onCreate}>+ Создать акт</Button>
    </div>
  {:else}
    <header class="detail-header">
      <h2 class="detail-title">№{act.number} от {headerDate}</h2>
      <div class="actions">
        {#if onPrint}
          <Button variant="secondary" size="sm" onclick={() => onPrint(act)}>Печать</Button>
        {:else}
          <span title="Печать недоступна">
            <Button variant="secondary" size="sm" disabled>Печать</Button>
          </span>
        {/if}
        <Button variant="secondary" size="sm" onclick={() => onEdit?.(act)} disabled={!onEdit}>
          Редактировать
        </Button>
        {#if onReturn && act.act_type === 'handover' && !act.archived}
          <Button variant="secondary" size="sm" onclick={() => onReturn(act)}>Возврат</Button>
        {:else}
          <span
            title={act.archived ? 'Акт уже в Архиве' : 'Возврат доступен только для handover-актов'}
          >
            <Button variant="secondary" size="sm" disabled>Возврат</Button>
          </span>
        {/if}
        <Button variant="destructive" size="sm" onclick={() => onDelete(act)}>Удалить</Button>
      </div>
    </header>

    <section class="section">
      <h3 class="section-heading">Шапка</h3>
      <div class="header-grid">
        <ActHeaderField label="Сдал" value={act.giver_name} />
        <ActHeaderField label="Принял" value={act.receiver_name} />
        <ActHeaderField label="Дата" value={headerDate} />
        <ActHeaderField label="Сроком до" value={deadlineLabel} />
        <ActHeaderField label="Расположение" value={act.location ?? null} />
        <ActHeaderField label="Заметки" value={act.notes ?? null} />
      </div>
    </section>

    <section class="section">
      <h3 class="section-heading">Позиции ({act.items.length})</h3>
      <ActItemsTable items={act.items} />
    </section>

    {#if act.return_ids && act.return_ids.length > 0}
      <section class="section">
        <h3 class="section-heading">История возвратов</h3>
        <ul class="returns-list">
          {#each act.return_ids as rid (rid)}
            <li>Акт возврата #{rid}</li>
          {/each}
        </ul>
        <p class="hint">Подробная история появится в plan 03.</p>
      </section>
    {/if}
  {/if}
</div>

<style lang="scss">
  .act-detail {
    height: 100%;
    overflow: auto;
    padding: var(--space-lg);
    background: var(--color-bg);
  }

  .loading,
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-md);
    min-height: 320px;
    text-align: center;
    color: var(--color-text-secondary);
  }
  .empty-heading {
    margin: 0;
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }
  .empty-body {
    margin: 0;
    max-width: 360px;
    color: var(--color-text-secondary);
  }

  .detail-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-md);
    flex-wrap: wrap;
    margin-bottom: var(--space-xl);
  }
  .detail-title {
    margin: 0;
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    font-variant-numeric: tabular-nums;
  }
  .actions {
    display: flex;
    gap: var(--space-sm);
    flex-wrap: wrap;
  }

  .section {
    margin-bottom: var(--space-xl);
  }
  .section-heading {
    margin: 0 0 var(--space-md);
    font-size: var(--font-size-subheading, var(--font-size-body));
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  .header-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-md);
  }

  .returns-list {
    margin: 0;
    padding-left: var(--space-lg);
    color: var(--color-text-primary);
    line-height: 1.6;
  }
  .hint {
    margin-top: var(--space-xs);
    color: var(--color-text-muted);
    font-size: var(--font-size-label);
  }
</style>
