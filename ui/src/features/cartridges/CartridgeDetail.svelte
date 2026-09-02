<script lang="ts">
  // Plan 04-04: детальная панель картриджа — поля + история перемещений.
  // Plan 04-05: action buttons wired (04-04 stubs → real handlers via onMenuAction callback).
  // Plan 27-04 (D-01): rebuilt on the shared DetailPanel/DetailSection/DetailField
  // primitives (extracted in 27-01) per ActDetail.svelte precedent — bespoke
  // container/header/field-grid/field-item/history-list classes removed;
  // detail surface (former container `{ background: var(--tr-bg) }`)
  // dropped — the CartridgesMasterDetail wrapper now owns the panel surface (D-02).
  // Loading state has no DetailPanel equivalent (empty/filled only) — kept as a
  // sibling branch with a matching container so layout doesn't jump between states.
  //
  // Plan 40-17 (HST-02, D-16): the section above ("История перемещений") is
  // renamed to "Журнал операций" — its content/format is byte-for-byte
  // unchanged, including the known numeric-place_id display bug in
  // parsePayloadDetails below (explicitly NOT fixed by this phase). A NEW,
  // separate "Перемещения" section is added directly below it, mounting the
  // shared MovementTimeline component (Plan 40-15) fed by its OWN fetch —
  // `history` above is a prop owned by CartridgesPage.svelte (this component
  // has no existing internal fetch effect to fold into), so the new
  // movements fetch is a genuinely independent, component-owned $effect
  // keyed on `cartridge`, with its own minimal loading/error flags. Because
  // the panel's `loading` prop governs the WHOLE detail pane (not just this
  // section), a timeline-only failure here uses MovementTimeline's own
  // scoped `loadError`, not the panel's loading gate — unlike Plan 40-16's
  // modal, this page keeps rendering the rest of the cartridge's data even
  // if only the movements fetch fails.
  import Button from '$lib/components/Button.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import DetailPanel from '$lib/components/DetailPanel.svelte';
  import DetailSection from '$lib/components/DetailSection.svelte';
  import DetailField from '$lib/components/DetailField.svelte';
  import MovementTimeline from '$lib/components/MovementTimeline.svelte';
  import { apiCall } from '$lib/api/client';
  import { push } from 'svelte-spa-router';
  import type { AuditEntryDto, CartridgeDto, MovementEntryDto } from '../../bindings';

  interface Props {
    cartridge: CartridgeDto | null;
    history: AuditEntryDto[];
    loading: boolean;
    onCreate: () => void;
    onMenuAction?: (_op: string, _cartridge: CartridgeDto) => void;
  }

  const { cartridge, history, loading, onCreate, onMenuAction }: Props = $props();

  let movements = $state<MovementEntryDto[]>([]);
  let movementsLoading = $state(false);
  let movementsLoadError = $state(false);

  $effect(() => {
    const c = cartridge;
    if (c === null) {
      movements = [];
      movementsLoadError = false;
      return;
    }
    movementsLoading = true;
    movementsLoadError = false;
    apiCall<MovementEntryDto[]>('place_movements_get_timeline', {
      entityType: 'cartridge',
      entityId: c.id,
    })
      .then((entries) => {
        movements = entries;
      })
      .catch(() => {
        movements = [];
        movementsLoadError = true;
      })
      .finally(() => {
        movementsLoading = false;
      });
  });

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
      // Plan 09 Task 2 renamed the audit-log JSON key from "location" to
      // "place_id" — the value is now a numeric place id, not a readable
      // path string (known display-quality tradeoff, out of this plan's
      // scope; would need a network lookup inside a synchronous parser).
      if (p.place_id) parts.push(String(p.place_id));
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

  const panelTitle = $derived(cartridge ? cartridge.code : undefined);
</script>

{#if loading}
  <div class="detail-loading" aria-live="polite">
    <Spinner size="md" />
    <span>Загружаем картридж…</span>
  </div>
{:else}
  <DetailPanel
    title={panelTitle}
    empty={cartridge === null}
    emptyTitle="Выберите картридж"
    emptyBody="Выберите картридж слева, чтобы увидеть историю и выполнить действие, или добавьте новый."
  >
    {#snippet emptyActions()}
      <Button variant="primary" onclick={onCreate}>+ Добавить картридж/фотобарабан</Button>
    {/snippet}
    {#snippet actions()}
      {#if cartridge}
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
      {/if}
    {/snippet}

    {#if cartridge}
      <div class="title-badges">
        {#if modelLabel}
          <span class="model-label">{modelLabel}</span>
        {/if}
        <Badge variant={statusVariant}>{cartridge.status_name ?? ''}</Badge>
      </div>

      <DetailSection heading="Информация">
        <div class="info-grid">
          <DetailField label="Место" value={cartridge.full_path ?? null} />
          {#if cartridge.status_id === 2 && cartridge.holder_name}
            <DetailField label="У кого" value={cartridge.holder_name} />
          {/if}
          {#if cartridge.state_name}
            <DetailField label="Состояние заряда" value={cartridge.state_name} />
          {/if}
          {#if cartridge.notes}
            <div class="field-wide">
              <DetailField label="Примечания" value={cartridge.notes} />
            </div>
          {/if}
        </div>
      </DetailSection>

      <DetailSection heading="Журнал операций">
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
      </DetailSection>

      <DetailSection heading="Перемещения">
        <MovementTimeline
          entries={movements}
          loading={movementsLoading}
          loadError={movementsLoadError}
          onNavigateToPlace={(id) => push(`#/places?id=${id}`)}
          onNavigateToAct={(id) => push(`#/acts?id=${id}`)}
        />
      </DetailSection>
    {/if}
  </DetailPanel>
{/if}

<style lang="scss">
  .detail-loading {
    height: 100%;
    overflow: auto;
    padding: var(--tr-space-xl);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--tr-space-md);
    min-height: 320px;
    text-align: center;
    color: var(--tr-text-secondary);
  }

  .title-badges {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
    flex-wrap: wrap;
    margin-bottom: var(--tr-space-lg);
  }

  .model-label {
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-secondary);
  }

  .info-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--tr-space-md);
  }

  .field-wide {
    grid-column: 1 / -1;
  }

  .history-empty {
    margin: 0;
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-tertiary);
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
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-primary);
    border-bottom: 1px solid var(--tr-border);

    &:last-child {
      border-bottom: none;
    }
  }
</style>
