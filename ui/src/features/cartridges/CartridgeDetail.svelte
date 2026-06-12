<script lang="ts">
  // Plan 04-04: детальная панель картриджа — поля + история перемещений.
  // Plan 04-05: action buttons wired (04-04 stubs → real handlers via onMenuAction callback).
  // По образцу ActDetail.svelte, паттерн из PATTERNS.md §CartridgeDetail.svelte.
  import Button from '$lib/components/Button.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import type { AuditEntryDto, CartridgeDto } from '../../bindings';

  interface Props {
    cartridge: CartridgeDto | null;
    history: AuditEntryDto[];
    loading: boolean;
    onCreate: () => void;
    onMenuAction?: (_op: string, _cartridge: CartridgeDto) => void;
  }

  const { cartridge, history, loading, onCreate, onMenuAction }: Props = $props();

  // Badge variant по status_id (UI-SPEC §Badge-цвета статусов):
  // 1→success, 2→accent, 3→warning, 4→default
  type BadgeVariant = 'success' | 'accent' | 'warning' | 'default';

  const statusVariant = $derived<BadgeVariant>(
    cartridge
      ? cartridge.status_id === 1
        ? 'success'
        : cartridge.status_id === 2
          ? 'accent'
          : cartridge.status_id === 3
            ? 'warning'
            : 'default'
      : 'default',
  );

  const modelLabel = $derived(
    cartridge && (cartridge.model_brand || cartridge.model_name)
      ? `${cartridge.model_brand ?? ''} ${cartridge.model_name ?? ''}`.trim()
      : null,
  );

  // Фотобарабан (kind 2): нет заправки; отработанный (state 6) нельзя установить.
  const isDrum = $derived(cartridge?.model_kind_id === 2);
  const isWornOut = $derived(cartridge?.state_id === 6);

  // Форматирование даты из unix seconds → «ДД.ММ.ГГГГ»
  function formatDate(utcSeconds: number): string {
    const d = new Date(utcSeconds * 1000);
    const day = String(d.getUTCDate()).padStart(2, '0');
    const month = String(d.getUTCMonth() + 1).padStart(2, '0');
    const year = d.getUTCFullYear();
    return `${day}.${month}.${year}`;
  }

  // Маппинг action → label (D-History-01, UI-SPEC §CartridgeDetail)
  function actionLabel(action: string): string {
    const labels: Record<string, string> = {
      create: 'Создан',
      'custom:cartridge_code_override': 'Создан',
      'custom:install': 'Установлен в принтер',
      'custom:return_to_stock': 'Возвращён на склад',
      'custom:to_refill': 'Отправлен на заправку',
      'custom:from_refill': 'Получен с заправки',
      'custom:write_off': 'Списан',
      update: 'Изменён',
      delete: 'Удалён',
    };
    return labels[action] ?? action;
  }

  // Парсинг payload_json для отображения подробностей
  function parsePayloadDetails(entry: AuditEntryDto): string | null {
    if (!entry.payload_json) return null;
    try {
      const p = JSON.parse(entry.payload_json) as Record<string, unknown>;
      const parts: string[] = [];
      if (p.given_by_name) parts.push(`выдал ${String(p.given_by_name)}`);
      if (p.given_to_name) parts.push(`получил ${String(p.given_to_name)}`);
      if (p.location) parts.push(String(p.location));
      return parts.length > 0 ? parts.join(', ') : null;
    } catch {
      return null;
    }
  }

  function formatHistoryEntry(entry: AuditEntryDto): string {
    const date = formatDate(entry.created_at_utc);
    const label = actionLabel(entry.action);
    const details = parsePayloadDetails(entry);
    return details ? `${date} — ${label}; ${details}` : `${date} — ${label}`;
  }
</script>

<div class="cartridge-detail" aria-live="polite">
  {#if loading}
    <div class="loading">
      <Spinner size="md" />
      <span>Загружаем картридж…</span>
    </div>
  {:else if cartridge === null}
    <div class="empty">
      <h2 class="empty-heading">Выберите картридж</h2>
      <p class="empty-body">
        Выберите картридж слева, чтобы увидеть историю и выполнить действие, или добавьте новый.
      </p>
      <Button variant="primary" onclick={onCreate}>+ Добавить картридж/фотобарабан</Button>
    </div>
  {:else}
    <header class="detail-header">
      <div class="title-row">
        <h2 class="detail-title" style="font-variant-numeric: tabular-nums">
          {cartridge.code}
        </h2>
        {#if modelLabel}
          <span class="model-label">{modelLabel}</span>
        {/if}
        <Badge variant={statusVariant}>{cartridge.status_name ?? ''}</Badge>
      </div>
      <div class="actions">
        {#if cartridge.status_id === 1}
          <!-- На складе: установить (если не отработанный барабан),
               отправить на заправку (только картриджи) -->
          {#if !(isDrum && isWornOut)}
            <Button
              variant="secondary"
              size="sm"
              onclick={() => onMenuAction?.('install', cartridge!)}>Установить</Button
            >
          {/if}
          {#if !isDrum}
            <Button
              variant="secondary"
              size="sm"
              onclick={() => onMenuAction?.('to_refill', cartridge!)}>На заправку</Button
            >
          {/if}
        {:else if cartridge.status_id === 2}
          <!-- В работе: вернуть на склад -->
          <Button
            variant="secondary"
            size="sm"
            onclick={() => onMenuAction?.('return_to_stock', cartridge!)}>Вернуть на склад</Button
          >
        {:else if cartridge.status_id === 3}
          <!-- На заправке: забрать с заправки -->
          <Button
            variant="secondary"
            size="sm"
            onclick={() => onMenuAction?.('from_refill', cartridge!)}>Забрать с заправки</Button
          >
        {/if}
        <Button variant="secondary" size="sm" onclick={() => onMenuAction?.('edit', cartridge!)}
          >Редактировать</Button
        >
        <Button variant="destructive" size="sm" onclick={() => onMenuAction?.('delete', cartridge!)}
          >Удалить</Button
        >
      </div>
    </header>

    <section class="section">
      <h3 class="section-heading">Информация</h3>
      <div class="fields-grid">
        <div class="field">
          <span class="field-label">Расположение</span>
          <span class="field-value">{cartridge.location ?? '—'}</span>
        </div>
        {#if cartridge.status_id === 2 && cartridge.holder_name}
          <div class="field">
            <span class="field-label">У кого</span>
            <span class="field-value">{cartridge.holder_name}</span>
          </div>
        {/if}
        {#if cartridge.state_name}
          <div class="field">
            <span class="field-label">Состояние заряда</span>
            <span class="field-value">{cartridge.state_name}</span>
          </div>
        {/if}
        {#if cartridge.notes}
          <div class="field field-wide">
            <span class="field-label">Примечания</span>
            <span class="field-value">{cartridge.notes}</span>
          </div>
        {/if}
      </div>
    </section>

    <section class="section">
      <h3 class="section-heading">История перемещений</h3>
      {#if history.length === 0}
        <p class="history-empty">История пуста</p>
      {:else}
        <ul class="history-list">
          {#each history as entry (entry.id)}
            <li class="history-row">
              {formatHistoryEntry(entry)}
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  {/if}
</div>

<style lang="scss">
  .cartridge-detail {
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
    margin-bottom: var(--space-xl);
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    flex-wrap: wrap;
    margin-bottom: var(--space-sm);
  }

  .detail-title {
    margin: 0;
    font-size: var(--font-size-display);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    line-height: var(--line-height-display);
    font-variant-numeric: tabular-nums;
  }

  .model-label {
    font-size: var(--font-size-body);
    color: var(--color-text-secondary);
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
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  .fields-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-md);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .field-wide {
    grid-column: 1 / -1;
  }

  .field-label {
    font-size: var(--font-size-label);
    color: var(--color-text-muted);
  }

  .field-value {
    font-size: var(--font-size-body);
    color: var(--color-text-primary);
  }

  .history-empty {
    margin: 0;
    font-size: var(--font-size-body);
    color: var(--color-text-muted);
    font-style: italic;
  }

  .history-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .history-row {
    display: flex;
    align-items: center;
    min-height: var(--row-height-dense, 32px);
    padding: 0 0;
    font-size: var(--font-size-label);
    color: var(--color-text-primary);
    border-bottom: 1px solid var(--color-border);

    &:last-child {
      border-bottom: none;
    }
  }
</style>
