<script lang="ts">
  // Plan 06-04: таблица найденных принтеров при SNMP discovery.
  // Колонки: чекбокс / IP / Производитель / Модель / Имя (sysName) / Статус.
  // Дубликаты (isDuplicate=true) → Badge «Уже заведён», чекбокс disabled.
  // Header-чекбокс: выбрать все не-дубликаты.
  // Plan 27-08 (D-03/D-04): сырая bespoke-таблица заменена примитивами Table/TableRow;
  // сырые checkbox-инпуты → Checkbox-примитив; select-all/per-row/dedup-подсветка
  // и вся логика выбора не изменены — только разметка.
  import Badge from '$lib/components/Badge.svelte';
  import Table from '$lib/components/Table.svelte';
  import TableRow from '$lib/components/TableRow.svelte';
  import Checkbox from '$lib/components/Checkbox.svelte';
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

{#snippet tableHead()}
  <th class="col-check">
    <Checkbox
      checked={allSelected}
      disabled={nonDuplicateCount === 0}
      onchange={toggleAll}
      id="discovery-select-all"
    >
      <span class="sr-only">Завести как «Принтер» — выбрать все</span>
    </Checkbox>
  </th>
  <th>IP-адрес</th>
  <th>Производитель</th>
  <th>Модель</th>
  <th>Имя (sysName)</th>
  <th>Статус</th>
{/snippet}

<Table
  columns={6}
  empty={items.length === 0}
  emptyTitle="Принтеры не найдены"
  emptyBody="В указанном диапазоне не обнаружено SNMP-устройств. Проверьте диапазон IP и community."
  head={tableHead}
>
  {#each items as item, idx (item.ip)}
    <TableRow class={item.isDuplicate ? 'duplicate' : undefined}>
      <td class="cell col-check">
        <Checkbox
          checked={selected.has(idx)}
          disabled={item.isDuplicate}
          onchange={() => onToggle(idx)}
          id="discovery-row-{idx}"
        >
          <span class="sr-only">Выбрать {item.ip}</span>
        </Checkbox>
      </td>
      <td class="cell col-ip tr-mono">{item.ip}</td>
      <td class="cell">{item.vendor ?? '—'}</td>
      <td class="cell">{item.model ?? '—'}</td>
      <td class="cell">{item.sysName}</td>
      <td class="cell">
        {#if item.isDuplicate}
          <Badge variant="default">Уже заведён</Badge>
        {:else}
          <Badge variant="success">Найден</Badge>
        {/if}
      </td>
    </TableRow>
  {/each}
</Table>

<style lang="scss">
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
  }

  .col-check {
    width: 40px;
    text-align: center;
  }

  .col-ip {
    white-space: nowrap;
  }

  .cell {
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
  }

  // tr.duplicate — dedup-подсветка (приглушённый текст строки), pass-through
  // класса на TableRow (:global(), т.к. .duplicate попадает на caller-<tr>
  // из другого scope-hash — тот же паттерн, что group-last-child в DeviceListRow).
  :global(tr.duplicate) > .cell {
    color: var(--tr-text-tertiary);
  }
</style>
