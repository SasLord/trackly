<script lang="ts">
  // TableSection — CMP-06 showcase gallery for Table/TableRow (Plan 25-01 primitives).
  // Static demo data only, no API calls — mirrors TabsSection.svelte's structural pattern.
  import Table from '$lib/components/Table.svelte';
  import TableRow from '$lib/components/TableRow.svelte';
  import Badge from '$lib/components/Badge.svelte';

  type BadgeVariant = 'default' | 'accent' | 'success' | 'warning' | 'destructive';

  // Same STATUS_LABELS/STATUS_VARIANTS shape as DeviceListRow.svelte (D-10: content
  // mirrors the real screen, styles come from the .dc reference).
  const STATUS_LABELS: Record<number, string> = {
    1: 'На складе',
    2: 'В работе',
    3: 'На ремонте',
    4: 'Списано',
  };

  const STATUS_VARIANTS: Record<number, BadgeVariant> = {
    1: 'default',
    2: 'accent',
    3: 'warning',
    4: 'destructive',
  };

  // Dedicated variant for the group count-pill (kept as a bound expression, not a
  // literal accent-tone attribute, so the tone-demo Badge block below stays the sole
  // source of each literal status-tone attribute).
  const countPillVariant: BadgeVariant = 'accent';

  interface DemoDevice {
    name: string;
    inventoryNo: string;
    serialNo: string;
    model: string;
    full_path: string;
    state: string;
    statusId: number;
  }

  // Block 1 — row states: normal / hover (real mouse, CSS-only) / selected.
  const stateRows: DemoDevice[] = [
    {
      name: 'Принтер HP LaserJet Pro',
      inventoryNo: 'INV-00231',
      serialNo: 'SN-88213',
      model: 'M404dn',
      full_path: 'Склад №1',
      state: 'Исправно',
      statusId: 1,
    },
    {
      name: 'МФУ Kyocera ECOSYS',
      inventoryNo: 'INV-00458',
      serialNo: 'SN-11097',
      model: 'M2040dn',
      full_path: 'Кабинет 214',
      state: 'Исправно',
      statusId: 2,
    },
    {
      name: 'Сканер Canon imageFORMULA',
      inventoryNo: 'INV-00509',
      serialNo: 'SN-73042',
      model: 'DR-C225',
      full_path: 'Кабинет 108',
      state: 'Исправно',
      statusId: 1,
    },
  ];

  // Block 2 — group row: collapsed/expanded toggle + nested device rows.
  let demoExpanded = $state(true);

  const groupDevices: DemoDevice[] = [
    {
      name: 'Принтер Pantum BM5100ADN',
      inventoryNo: 'INV-00721',
      serialNo: 'SN-40021',
      model: 'BM5100ADN',
      full_path: 'Склад №2',
      state: 'Исправно',
      statusId: 1,
    },
    {
      name: 'Принтер Pantum BM5100ADN',
      inventoryNo: 'INV-00722',
      serialNo: 'SN-40022',
      model: 'BM5100ADN',
      full_path: 'Склад №2',
      state: 'Исправно',
      statusId: 1,
    },
    {
      name: 'Принтер Pantum BM5100ADN',
      inventoryNo: 'INV-00723',
      serialNo: 'SN-40023',
      model: 'BM5100ADN',
      full_path: 'Склад №2',
      state: 'На ремонте',
      statusId: 3,
    },
  ];

  // Block 3 — all four Badge status tones + mono identifiers.
  const badgeRows: DemoDevice[] = [
    {
      name: 'Устройство на складе',
      inventoryNo: 'INV-01001',
      serialNo: 'SN-90011',
      model: 'Модель A',
      full_path: 'Склад №1',
      state: 'Исправно',
      statusId: 1,
    },
    {
      name: 'Устройство в работе',
      inventoryNo: 'INV-01002',
      serialNo: 'SN-90012',
      model: 'Модель B',
      full_path: 'Кабинет 305',
      state: 'Исправно',
      statusId: 2,
    },
    {
      name: 'Устройство на ремонте',
      inventoryNo: 'INV-01003',
      serialNo: 'SN-90013',
      model: 'Модель C',
      full_path: 'Мастерская',
      state: 'Требует ремонта',
      statusId: 3,
    },
    {
      name: 'Устройство списано',
      inventoryNo: 'INV-01004',
      serialNo: 'SN-90014',
      model: 'Модель D',
      full_path: 'Архив',
      state: 'Списано',
      statusId: 4,
    },
  ];
</script>

{#snippet tableHead()}
  <th>Наименование</th>
  <th>Инвентарный №</th>
  <th>Серийный №</th>
  <th>Модель</th>
  <th>Место</th>
  <th>Состояние</th>
  <th>Статус</th>
  <th>Действия</th>
{/snippet}

<section class="table-section">
  <h2>Таблицы</h2>

  <div class="variant-block">
    <h3 class="variant-label">Состояния строки</h3>
    <Table columns={8} head={tableHead}>
      <TableRow>
        <td>{stateRows[0].name}</td>
        <td class="tr-mono">{stateRows[0].inventoryNo}</td>
        <td class="tr-mono">{stateRows[0].serialNo}</td>
        <td>{stateRows[0].model}</td>
        <td>{stateRows[0].full_path}</td>
        <td>{stateRows[0].state}</td>
        <td
          ><Badge variant={STATUS_VARIANTS[stateRows[0].statusId]}
            >{STATUS_LABELS[stateRows[0].statusId]}</Badge
          ></td
        >
        <td>—</td>
      </TableRow>
      <TableRow selected={true}>
        <td>{stateRows[1].name}</td>
        <td class="tr-mono">{stateRows[1].inventoryNo}</td>
        <td class="tr-mono">{stateRows[1].serialNo}</td>
        <td>{stateRows[1].model}</td>
        <td>{stateRows[1].full_path}</td>
        <td>{stateRows[1].state}</td>
        <td
          ><Badge variant={STATUS_VARIANTS[stateRows[1].statusId]}
            >{STATUS_LABELS[stateRows[1].statusId]}</Badge
          ></td
        >
        <td>—</td>
      </TableRow>
      <TableRow last={true}>
        <td>{stateRows[2].name}</td>
        <td class="tr-mono">{stateRows[2].inventoryNo}</td>
        <td class="tr-mono">{stateRows[2].serialNo}</td>
        <td>{stateRows[2].model}</td>
        <td>{stateRows[2].full_path}</td>
        <td>{stateRows[2].state}</td>
        <td
          ><Badge variant={STATUS_VARIANTS[stateRows[2].statusId]}
            >{STATUS_LABELS[stateRows[2].statusId]}</Badge
          ></td
        >
        <td>—</td>
      </TableRow>
    </Table>
  </div>

  <div class="variant-block">
    <h3 class="variant-label">Строка-группа</h3>
    <Table columns={8} head={tableHead}>
      <TableRow
        group
        groupExpanded={demoExpanded}
        groupName="Принтер Pantum BM5100ADN"
        groupColspan={4}
        onToggleGroup={() => (demoExpanded = !demoExpanded)}
      >
        <td>Склад №2</td>
        <td>Разное</td>
        <td class="group-pill-cell">
          <Badge variant={countPillVariant} appearance="count">{groupDevices.length} шт.</Badge>
        </td>
        <td>—</td>
      </TableRow>
      {#if demoExpanded}
        {#each groupDevices as device, i (device.inventoryNo)}
          <TableRow indent last={i === groupDevices.length - 1}>
            <td>{device.name}</td>
            <td class="tr-mono">{device.inventoryNo}</td>
            <td class="tr-mono">{device.serialNo}</td>
            <td>{device.model}</td>
            <td>{device.full_path}</td>
            <td>{device.state}</td>
            <td
              ><Badge variant={STATUS_VARIANTS[device.statusId]}
                >{STATUS_LABELS[device.statusId]}</Badge
              ></td
            >
            <td>—</td>
          </TableRow>
        {/each}
      {/if}
    </Table>
  </div>

  <div class="variant-block">
    <h3 class="variant-label">Бейджи статуса и моно-идентификаторы</h3>
    <Table columns={8} head={tableHead}>
      <TableRow>
        <td>{badgeRows[0].name}</td>
        <td class="tr-mono">{badgeRows[0].inventoryNo}</td>
        <td class="tr-mono">{badgeRows[0].serialNo}</td>
        <td>{badgeRows[0].model}</td>
        <td>{badgeRows[0].full_path}</td>
        <td>{badgeRows[0].state}</td>
        <td><Badge variant="default">{STATUS_LABELS[1]}</Badge></td>
        <td>—</td>
      </TableRow>
      <TableRow>
        <td>{badgeRows[1].name}</td>
        <td class="tr-mono">{badgeRows[1].inventoryNo}</td>
        <td class="tr-mono">{badgeRows[1].serialNo}</td>
        <td>{badgeRows[1].model}</td>
        <td>{badgeRows[1].full_path}</td>
        <td>{badgeRows[1].state}</td>
        <td><Badge variant="accent">{STATUS_LABELS[2]}</Badge></td>
        <td>—</td>
      </TableRow>
      <TableRow>
        <td>{badgeRows[2].name}</td>
        <td class="tr-mono">{badgeRows[2].inventoryNo}</td>
        <td class="tr-mono">{badgeRows[2].serialNo}</td>
        <td>{badgeRows[2].model}</td>
        <td>{badgeRows[2].full_path}</td>
        <td>{badgeRows[2].state}</td>
        <td><Badge variant="warning">{STATUS_LABELS[3]}</Badge></td>
        <td>—</td>
      </TableRow>
      <TableRow last={true}>
        <td>{badgeRows[3].name}</td>
        <td class="tr-mono">{badgeRows[3].inventoryNo}</td>
        <td class="tr-mono">{badgeRows[3].serialNo}</td>
        <td>{badgeRows[3].model}</td>
        <td>{badgeRows[3].full_path}</td>
        <td>{badgeRows[3].state}</td>
        <td><Badge variant="destructive">{STATUS_LABELS[4]}</Badge></td>
        <td>—</td>
      </TableRow>
    </Table>
  </div>
</section>

<style lang="scss">
  .table-section {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-lg);
  }

  h2 {
    margin: 0;
    font-size: var(--tr-font-size-h2);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .variant-block {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--tr-space-sm);
    width: 100%;
  }

  .variant-label {
    margin: 0;
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-secondary);
    text-transform: uppercase;
  }

  .group-pill-cell {
    text-align: center;
  }
</style>
