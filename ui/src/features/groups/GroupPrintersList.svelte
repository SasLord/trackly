<script lang="ts">
  // Phase 41 Plan 22 (GRP-08, D-12): ЕДИНЫЙ список «Подключённых принтеров».
  //
  // Строки двух происхождений: выведенные из `printers.usb_host_device_id`
  // (Badge «USB», вручную не правятся — меню ⋯ нет) и явные ссылки
  // `device_refs` (Badge «Ссылка», меню «Убрать ссылку»). Принтер, который и
  // USB-связан, и добавлен ссылкой, показывается ОДНОЙ строкой с Badge «USB»,
  // но с меню «Убрать ссылку» — иначе явная ссылка стала бы невидимой и
  // неудаляемой. Дедупликацию сервер уже сделал в карточке; здесь она
  // повторяется лишь потому, что список ссылок — ЛОКАЛЬНОЕ состояние формы
  // (добавленное до сохранения сервер ещё не видел).
  import Table from '$lib/components/Table.svelte';
  import TableRow from '$lib/components/TableRow.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import ActionMenu from '$lib/components/ActionMenu.svelte';
  import Dropdown from '$lib/components/Dropdown.svelte';
  import { devices } from '$lib/api/devices';
  import type { DeviceDto, DeviceGroup, GroupPrinterDto } from '../../bindings';

  // Значение совпадает с PrintersPage.svelte и DeviceFormBody.svelte: общего
  // экспорта в проекте нет, и вводить его здесь значило бы трогать посторонние
  // файлы.
  const PRINTER_TYPE_ID = 2;
  const SEARCH_LIMIT = 20;

  export interface LinkedPrinterInfo {
    device_id: number;
    name: string;
    inventory_number: string | null;
    serial_number: string | null;
  }

  interface Props {
    id?: string;
    /** Принтеры из card.printers как пришли с сервера (USB и ссылки). */
    printers: GroupPrinterDto[];
    /** Id явных ссылок из локального состояния формы — источник истины до сохранения. */
    linkedIds: number[];
    /** Сведения о только что добавленных ссылках (сервер о них ещё не знает). */
    linkedInfo: Map<number, LinkedPrinterInfo>;
    canEdit: boolean;
    /** У типа есть свойство device_refs. */
    canLink: boolean;
    errorText?: string | null;
    onAddLink: (_device: LinkedPrinterInfo) => void;
    onRemoveLink: (_deviceId: number) => void;
  }

  const {
    id,
    printers,
    linkedIds,
    linkedInfo,
    canEdit,
    canLink,
    errorText = null,
    onAddLink,
    onRemoveLink,
  }: Props = $props();

  interface Row {
    device_id: number;
    name: string;
    inventory_number: string | null;
    serial_number: string | null;
    usb: boolean;
    link: boolean;
  }

  const rows = $derived.by((): Row[] => {
    const linked = new Set(linkedIds);
    const out: Row[] = [];
    const seen = new Set<number>();
    for (const p of printers) {
      if (p.origin !== 'usb' || seen.has(p.device_id)) continue;
      seen.add(p.device_id);
      out.push({
        device_id: p.device_id,
        name: p.name,
        inventory_number: p.inventory_number,
        serial_number: p.serial_number,
        usb: true,
        link: linked.has(p.device_id),
      });
    }
    for (const linkId of linkedIds) {
      if (seen.has(linkId)) continue;
      const info: LinkedPrinterInfo | undefined =
        linkedInfo.get(linkId) ??
        printers.find((p) => p.device_id === linkId && p.origin !== 'usb');
      // Ссылка на не-принтер остаётся в значении свойства, но в список не
      // попадает (так же отдаёт сервер): без названия показывать нечего.
      if (!info) continue;
      seen.add(linkId);
      out.push({
        device_id: linkId,
        name: info.name,
        inventory_number: info.inventory_number,
        serial_number: info.serial_number,
        usb: false,
        link: true,
      });
    }
    return out;
  });

  // --- Добавление ссылкой ---
  let query = $state('');
  let candidates = $state<DeviceDto[]>([]);
  let searching = $state(false);
  let searchSeq = 0;

  async function fetchCandidates(q: string): Promise<void> {
    const seq = ++searchSeq;
    searching = true;
    try {
      const groupsFound = await devices
        .listGrouped(
          {
            type_id: PRINTER_TYPE_ID,
            place_id: null,
            status_id: null,
            state: null,
            name_prefix: q.trim() === '' ? null : q.trim(),
            include_deleted: false,
            group_by_condition: true,
          },
          { offset: 0, limit: SEARCH_LIMIT },
        )
        .catch((): DeviceGroup[] => []);
      // Набор одинаковых устройств без номеров раскрываем в отдельные
      // устройства: ссылка ставится на конкретное.
      const singles = groupsFound.filter((g) => g.ids.length === 1).map((g) => g.repr);
      const multiIds = groupsFound.filter((g) => g.ids.length > 1).flatMap((g) => g.ids);
      const expanded = multiIds.length > 0 ? await devices.listByIds(multiIds).catch(() => []) : [];
      if (seq !== searchSeq) return;
      candidates = [...singles, ...expanded].slice(0, SEARCH_LIMIT);
    } finally {
      if (seq === searchSeq) searching = false;
    }
  }

  const shownIds = $derived(new Set(rows.map((r) => r.device_id)));
  const available = $derived(
    candidates.filter((d) => !shownIds.has(d.id) && !linkedIds.includes(d.id)),
  );

  function pick(d: DeviceDto): void {
    onAddLink({
      device_id: d.id,
      name: d.name,
      inventory_number: d.inventory_no,
      serial_number: d.serial_no,
    });
    query = '';
    candidates = [];
  }

  function subOf(d: DeviceDto): string | undefined {
    const parts = [
      d.inventory_no ? `инв. ${d.inventory_no}` : null,
      d.serial_no ? `SN ${d.serial_no}` : null,
    ].filter((p): p is string => p !== null);
    return parts.length > 0 ? parts.join(' · ') : undefined;
  }

  function noMembers(): DeviceDto[] {
    return [];
  }
</script>

<div class="printers-list">
  <Table
    columns={4}
    framed={false}
    empty={rows.length === 0}
    emptyTitle="Принтеров нет"
    emptyBody="USB-принтеры появятся здесь сами, как только будут привязаны к устройству состава. Сетевой принтер добавьте ссылкой ниже."
  >
    {#snippet head()}
      <th>Название</th>
      <th>Инв. № / Серийный №</th>
      <th>Связь</th>
      <th class="col-menu" aria-label="Действия"></th>
    {/snippet}
    {#each rows as r (r.device_id)}
      <TableRow>
        <td class="cell" title={r.name}>{r.name}</td>
        <td class="cell">
          {#if r.inventory_number || r.serial_number}
            {#if r.inventory_number}<span class="tr-mono line">{r.inventory_number}</span>{/if}
            {#if r.serial_number}<span class="tr-mono line">{r.serial_number}</span>{/if}
          {:else}
            —
          {/if}
        </td>
        <td class="cell">
          {#if r.usb}
            <Badge variant="accent">USB</Badge>
          {:else}
            <Badge>Ссылка</Badge>
          {/if}
        </td>
        <td class="col-menu">
          {#if canEdit && r.link}
            <ActionMenu variant="ghost-sm" portal label={`Действия: ${r.name}`}>
              <button type="button" role="menuitem" onclick={() => onRemoveLink(r.device_id)}>
                Убрать ссылку
              </button>
            </ActionMenu>
          {/if}
        </td>
      </TableRow>
    {/each}
  </Table>

  {#if canEdit && canLink}
    <div class="add-row">
      <Dropdown
        {id}
        variant="combobox"
        flat={true}
        value={query}
        placeholder="Добавить принтер по названию или инвентарному номеру"
        invalid={!!errorText}
        loading={searching}
        groups={available}
        getGroupId={(d: DeviceDto) => d.id}
        getGroupName={(d: DeviceDto) => d.name}
        getGroupSub={subOf}
        getGroupCount={() => 0}
        isGroupExpandable={() => false}
        onExpandGroup={noMembers}
        getMemberId={(d: DeviceDto) => d.id}
        getMemberName={(d: DeviceDto) => d.name}
        onSearch={(q) => void fetchCandidates(q)}
        onQueryInput={(v) => (query = v)}
        onPickGroup={pick}
        onPickMember={() => {}}
      />
    </div>
  {/if}

  {#if errorText}
    <span class="field-error" role="alert">{errorText}</span>
  {/if}
</div>

<style lang="scss">
  .printers-list {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
  }

  .line {
    display: block;
  }

  .col-menu {
    width: 44px;
    text-align: right;
    white-space: nowrap;
  }

  .add-row {
    min-width: 0;
  }

  .field-error {
    font-size: var(--tr-font-size-label);
    color: var(--tr-danger-text);
  }
</style>
