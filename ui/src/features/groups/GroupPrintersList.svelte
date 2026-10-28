<script lang="ts">
  // Phase 41 Plan 22 (GRP-08, D-12), Phase 41.13 (PGR-01, PGR-02): ЕДИНЫЙ список
  // «Подключённых принтеров».
  //
  // Строки трёх происхождений: выведенные из `printers.usb_host_device_id`
  // (Badge «USB», вручную не правятся), явные ссылки `device_refs`
  // (Badge «Ссылка», меню «Убрать ссылку») и принтеры, входящие в состав
  // (Badge «В составе», меню нет: состав правится в разделе «Состав»).
  // Принтер с несколькими происхождениями показывается ОДНОЙ строкой с ОДНОЙ
  // пометкой по приоритету USB > ссылка > в составе. Принтер, который и
  // USB-связан, и добавлен ссылкой, получает Badge «USB», но сохраняет меню
  // «Убрать ссылку» — иначе явная ссылка стала бы невидимой и неудаляемой.
  // Дедупликацию сервер уже сделал в карточке; здесь она повторяется лишь
  // потому, что список ссылок — ЛОКАЛЬНОЕ состояние формы (добавленное до
  // сохранения сервер ещё не видел).
  import Table from '$lib/components/Table.svelte';
  import TableRow from '$lib/components/TableRow.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import ActionMenu from '$lib/components/ActionMenu.svelte';
  import Dropdown from '$lib/components/Dropdown.svelte';
  import { devices } from '$lib/api/devices';
  import type { DeviceDto, DeviceListResponse, GroupPrinterDto } from '../../bindings';

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
    /** Принтеры из card.printers как пришли с сервера (USB, ссылки, члены). */
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

  type RowOrigin = 'usb' | 'link' | 'member';

  interface Row {
    device_id: number;
    name: string;
    inventory_number: string | null;
    serial_number: string | null;
    origin: RowOrigin;
    /** Есть явная ссылка — только она включает меню «Убрать ссылку». */
    link: boolean;
  }

  const rows = $derived.by((): Row[] => {
    const linked = new Set(linkedIds);
    const out: Row[] = [];
    const seen = new Set<number>();
    for (const p of printers) {
      if (p.origin === 'link' || seen.has(p.device_id)) continue;
      seen.add(p.device_id);
      const hasLink = linked.has(p.device_id);
      // Приоритет USB > ссылка > в составе: ссылка, добавленная в форме до
      // сохранения, переключает «В составе» на «Ссылка», но не перекрывает USB.
      const origin: RowOrigin = p.origin === 'usb' ? 'usb' : hasLink ? 'link' : 'member';
      out.push({
        device_id: p.device_id,
        name: p.name,
        inventory_number: p.inventory_number,
        serial_number: p.serial_number,
        origin,
        link: hasLink,
      });
    }
    for (const linkId of linkedIds) {
      if (seen.has(linkId)) continue;
      const info: LinkedPrinterInfo | undefined =
        linkedInfo.get(linkId) ??
        printers.find((p) => p.device_id === linkId && p.origin === 'link');
      // Ссылка на не-принтер остаётся в значении свойства, но в список не
      // попадает (так же отдаёт сервер): без названия показывать нечего.
      if (!info) continue;
      seen.add(linkId);
      out.push({
        device_id: linkId,
        name: info.name,
        inventory_number: info.inventory_number,
        serial_number: info.serial_number,
        origin: 'link',
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
      // `list`, а не `listGrouped`: свёртка игнорирует `type_id`, и окно `limit`
      // резалось ДО фильтра типа, вытесняя принтеры устройствами других типов.
      // Здесь нужны отдельные устройства (ссылка ставится на конкретное), так что
      // свёртка не нужна; тип и окно применяет сервер, текст ищется тем же FTS5.
      // Флагов якорей нет намеренно: пикер видит все устройства.
      const found = await devices
        .list(
          {
            type_id: PRINTER_TYPE_ID,
            place_id: null,
            status_id: null,
            state: null,
            name_prefix: q.trim() === '' ? null : q.trim(),
            include_deleted: false,
            group_by_condition: false,
          },
          { offset: 0, limit: SEARCH_LIMIT },
        )
        .catch((): DeviceListResponse => ({ items: [], total: 0, hidden_member_count: 0 }));
      if (seq !== searchSeq) return;
      candidates = found.items.slice(0, SEARCH_LIMIT);
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

  // Пустое состояние описывает реальное поведение; «ссылкой ниже» — только
  // когда поле добавления действительно есть на экране.
  const emptyBody = $derived(
    canEdit && canLink
      ? 'Принтер появится здесь, если он входит в состав, подключён по USB к устройству состава или добавлен ссылкой. Сетевой принтер добавьте ссылкой ниже.'
      : 'Принтер появится здесь, если он входит в состав или подключён по USB к устройству состава.',
  );
</script>

<div class="printers-list">
  <Table
    columns={4}
    framed={false}
    empty={rows.length === 0}
    emptyTitle="Принтеров нет"
    {emptyBody}
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
          {#if r.origin === 'usb'}
            <Badge variant="accent">USB</Badge>
          {:else if r.origin === 'link'}
            <Badge>Ссылка</Badge>
          {:else}
            <Badge>В составе</Badge>
          {/if}
        </td>
        <td class="col-menu">
          {#if canEdit && r.link}
            <ActionMenu variant="ghost-sm" label={`Действия: ${r.name}`}>
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
