<script lang="ts">
  // Plan 06-04: таблица найденных принтеров при SNMP discovery.
  // Колонки: чекбокс / IP / Производитель / Модель / Имя (sysName) / Статус.
  // Дубликаты (isDuplicate=true) → Badge «Уже заведён», чекбокс disabled.
  // Header-чекбокс: выбрать все не-дубликаты.
  import Badge from '$lib/components/Badge.svelte';
  import type { DiscoveredPrinterDto } from '../../bindings-phase6';

  interface Props {
    items: DiscoveredPrinterDto[];
    selected: Set<number>;
    onToggle: (_idx: number) => void;
  }

  const { items, selected, onToggle }: Props = $props();

  const nonDuplicateCount = $derived(items.filter((it) => !it.isDuplicate).length);
  const allSelected = $derived(
    nonDuplicateCount > 0 && items.every((it, idx) => it.isDuplicate || selected.has(idx)),
  );

  function toggleAll() {
    if (allSelected) {
      // Deselect all non-duplicates.
      items.forEach((_it, idx) => {
        if (selected.has(idx)) onToggle(idx);
      });
    } else {
      // Select all non-duplicates.
      items.forEach((it, idx) => {
        if (!it.isDuplicate && !selected.has(idx)) onToggle(idx);
      });
    }
  }
</script>

{#if items.length === 0}
  <div class="empty">
    <p class="empty-text">Принтеры не найдены</p>
    <p class="empty-hint">
      В указанном диапазоне не обнаружено SNMP-устройств. Проверьте диапазон IP и community.
    </p>
  </div>
{:else}
  <div class="table-wrap">
    <table class="results-table">
      <thead>
        <tr>
          <th class="col-check">
            <input
              type="checkbox"
              checked={allSelected}
              disabled={nonDuplicateCount === 0}
              onchange={toggleAll}
              aria-label="Завести как «Принтер» — выбрать все"
            />
          </th>
          <th>IP-адрес</th>
          <th>Производитель</th>
          <th>Модель</th>
          <th>Имя (sysName)</th>
          <th>Статус</th>
        </tr>
      </thead>
      <tbody>
        {#each items as item, idx (item.ip)}
          <tr class:duplicate={item.isDuplicate}>
            <td class="col-check">
              <input
                type="checkbox"
                checked={selected.has(idx)}
                disabled={item.isDuplicate}
                onchange={() => onToggle(idx)}
                aria-label="Выбрать {item.ip}"
              />
            </td>
            <td class="col-ip" style="font-variant-numeric: tabular-nums">{item.ip}</td>
            <td>{item.vendor ?? '—'}</td>
            <td>{item.model ?? '—'}</td>
            <td>{item.sysName}</td>
            <td>
              {#if item.isDuplicate}
                <Badge variant="default">Уже заведён</Badge>
              {:else}
                <Badge variant="success">Найден</Badge>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}

<style lang="scss">
  .empty {
    text-align: center;
    padding: var(--space-xl);
    color: var(--tr-text-secondary);
  }

  .empty-text {
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--tr-text-primary);
    margin: 0 0 var(--space-xs);
  }

  .empty-hint {
    font-size: var(--font-size-body);
    margin: 0;
  }

  .table-wrap {
    overflow-x: auto;
    margin-top: var(--space-md);
  }

  .results-table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--font-size-body);

    th,
    td {
      padding: var(--space-sm) var(--space-md);
      text-align: left;
      border-bottom: 1px solid var(--tr-border);
      height: var(--row-height, 40px);
      vertical-align: middle;
    }

    th {
      font-weight: var(--font-weight-semibold);
      color: var(--tr-text-secondary);
      font-size: var(--font-size-label);
      background: var(--tr-surface);
    }

    tr:hover td {
      background: var(--tr-surface-sunken);
    }

    tr.duplicate td {
      color: var(--tr-text-tertiary);
    }

    .col-check {
      width: 40px;
      text-align: center;
    }

    .col-ip {
      white-space: nowrap;
    }
  }
</style>
