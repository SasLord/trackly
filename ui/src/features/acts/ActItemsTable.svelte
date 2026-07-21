<script lang="ts">
  // Plan 03-02: read-only items table for ActDetail.
  // Columns: Устройство · Инв. № · Серийный № · Количество · Состояние · Возврат
  // Plan 03 заполнит «Возврат» ссылками на return-акты.
  import type { ActItemDto } from '../../bindings';

  interface Props {
    items: ActItemDto[];
  }
  const { items }: Props = $props();
</script>

<div class="items-table" role="table" aria-label="Позиции акта">
  <div class="thead" role="row">
    <div class="th col-device">Устройство</div>
    <div class="th col-inv">Инв. №</div>
    <div class="th col-serial">Серийный №</div>
    <div class="th col-qty">Количество</div>
    <div class="th col-state">Состояние</div>
    <div class="th col-return">Возврат</div>
  </div>

  {#if items.length === 0}
    <div class="empty">Позиций пока нет.</div>
  {:else}
    {#each items as item (item.id)}
      <div class="tr" role="row">
        <div class="td col-device">
          <span class="device-name">{item.device_name}</span>
          {#if item.model}
            <span class="device-model">{item.model}</span>
          {/if}
        </div>
        <div class="td col-inv" class:muted={!item.inventory_no}>
          <span class="tr-mono">{item.inventory_no ?? '—'}</span>
        </div>
        <div class="td col-serial" class:muted={!item.serial_no}>
          <span class="tr-mono">{item.serial_no ?? '—'}</span>
        </div>
        <div class="td col-qty tabular">{item.quantity}</div>
        <div class="td col-state" class:muted={!item.condition_at_time}>
          {item.condition_at_time ?? '—'}
        </div>
        <div class="td col-return muted">—</div>
      </div>
    {/each}
  {/if}
</div>

<style lang="scss">
  .items-table {
    width: 100%;
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
    overflow: hidden;
  }

  .thead {
    display: grid;
    grid-template-columns: 25% 15% 15% 10% 15% 20%;
    background: var(--tr-surface-sunken);
    border-bottom: 1px solid var(--tr-border);
  }
  .th {
    padding: var(--tr-space-xs) var(--tr-space-md);
    font-size: var(--tr-font-size-label);
    font-weight: 500;
    color: var(--tr-text-secondary);
  }

  .tr {
    display: grid;
    grid-template-columns: 25% 15% 15% 10% 15% 20%;
    border-bottom: 1px solid var(--tr-border);
    min-height: 40px;
    align-items: center;

    &:last-child {
      border-bottom: none;
    }
  }
  .td {
    padding: var(--tr-space-xs) var(--tr-space-md);
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
  }
  .col-qty.tabular {
    font-variant-numeric: tabular-nums;
  }
  .col-device {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-3xs);
  }
  .device-name {
    font-weight: 500;
  }
  .device-model {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
  }
  .muted {
    color: var(--tr-text-tertiary);
  }
  .empty {
    padding: var(--tr-space-2xl);
    text-align: center;
    color: var(--tr-text-tertiary);
    font-size: var(--tr-font-size-body);
  }
</style>
