// usbHostPicker — чистая логика выбора USB-хоста принтера (фаза 41.13, PGR-03).
//
// Зачем отдельный модуль: поле «Устройство-хост (USB)» живёт в
// DeviceFormBody.svelte (файл на 1300+ строк), а разводка секций списка и обход
// поддерева состава читаются лучше отдельно от разметки. Модуль не зависит от
// Svelte и от окружения браузера: только данные на входе, данные на выходе.
//
// ЧЕСТНО О ПРОВЕРКЕ: JS-тест-раннера в проекте нет. Компиляция
// (`svelte-check`, `pnpm build`) и линт не доказывают поведение: что секции
// встают в нужном порядке, что заголовок второй секции рисуется, что запрос,
// равный строке значения, считается пустым. Поведение закрепляется структурным
// гейтом (отдельный план) и живой проверкой — до неё оно UNVERIFIED.

/** Элемент списка выбора хоста. */
export type HostItem =
  | { kind: 'clear' }
  | {
      kind: 'device';
      id: number;
      name: string;
      inventoryNo: string | null;
      serialNo: string | null;
      /** 'group' — секция «В составе …»; 'all' — секция «Все устройства». */
      section: 'group' | 'all';
    };

/** Устройство, как его отдаёт состав (подмножество GroupMemberDeviceDto). */
export interface CompositionDevice {
  device_id: number;
  name: string;
  inventory_number: string | null;
  serial_number: string | null;
}

/** Состав (подмножество GroupCompositionDto): прямые члены и вложенные узлы. */
export interface CompositionLike {
  devices: CompositionDevice[];
  child_groups: { id: number }[];
}

/** Потолок числа обходимых узлов поддерева: защита от разросшегося дерева.
 *  Граница осознанная: секция «Все устройства» и поиск всё равно достигают
 *  любого устройства. */
export const SUBTREE_CAP = 50;

/** Текст ошибки загрузки списка. */
export const HOST_LIST_ERROR = 'Не удалось загрузить список устройств. Попробуйте ещё раз.';
/** Текст отказа сервера выбрать это устройство хостом. */
export const HOST_REJECTED_ERROR = 'Это устройство нельзя выбрать хостом. Выберите другое.';
/** Подсказка под полем. */
export const HOST_FIELD_HINT = 'Системный блок, в который воткнут принтер. Необязательно.';
/** Заголовок второй секции. */
export const SECTION_ALL = 'Все устройства';

/** Подпись хоста: «Имя» либо «Имя · инв. 114» (одноимённые иначе неразличимы).
 *  Единая для поля, для сравнения «запрос равен значению» и для детали. */
export function hostLabel(name: string, inventoryNo: string | null | undefined): string {
  const inv = (inventoryNo ?? '').trim();
  return inv === '' ? name : `${name} · инв. ${inv}`;
}

/** Подстрока пункта: «инв. 114 · SN AB123» либо undefined. */
export function hostSub(item: HostItem): string | undefined {
  if (item.kind !== 'device') return undefined;
  const parts = [
    item.inventoryNo ? `инв. ${item.inventoryNo}` : null,
    item.serialNo ? `SN ${item.serialNo}` : null,
  ].filter((p): p is string => p !== null);
  return parts.length > 0 ? parts.join(' · ') : undefined;
}

/** Обход поддерева в ширину через переданную функцию загрузки состава.
 *  Якорь входит в результат: состав отдаёт прямых членов, включая якорь.
 *  Ошибка одного узла молча пропускает его, остальные продолжаются. */
export async function walkGroupSubtree(
  fetchComposition: (groupId: number) => Promise<CompositionLike>,
  rootGroupId: number,
  options: { excludeDeviceId?: number | null } = {},
): Promise<HostItem[]> {
  const visited = new Set<number>([rootGroupId]);
  const seenDevices = new Set<number>();
  const out: HostItem[] = [];
  let level: number[] = [rootGroupId];
  while (level.length > 0) {
    const results = await Promise.all(
      level.map((gid) => fetchComposition(gid).catch((): CompositionLike | null => null)),
    );
    const next: number[] = [];
    for (const comp of results) {
      if (comp === null) continue;
      for (const d of comp.devices) {
        if (seenDevices.has(d.device_id)) continue;
        if (options.excludeDeviceId != null && d.device_id === options.excludeDeviceId) continue;
        seenDevices.add(d.device_id);
        out.push({
          kind: 'device',
          id: d.device_id,
          name: d.name,
          inventoryNo: d.inventory_number,
          serialNo: d.serial_number,
          section: 'group',
        });
      }
      for (const child of comp.child_groups) {
        if (visited.has(child.id)) continue;
        if (visited.size >= SUBTREE_CAP) continue;
        visited.add(child.id);
        next.push(child.id);
      }
    }
    level = next;
  }
  return out;
}

/** Клиентский фильтр первой секции: подстрока без учёта регистра по названию,
 *  инв. и серийному №. Пустой запрос возвращает всё. */
export function filterSectionOne(items: HostItem[], query: string): HostItem[] {
  const q = query.trim().toLowerCase();
  if (q === '') return items;
  return items.filter((it) => {
    if (it.kind !== 'device') return false;
    return [it.name, it.inventoryNo, it.serialNo].some(
      (v) => v != null && v.toLowerCase().includes(q),
    );
  });
}

/** Запрос, равный строке текущего значения, считается пустым: иначе открытие
 *  панели искало бы «Системный блок #4 · инв. 114» и ничего не находило. */
export function effectiveQuery(raw: string, currentLabel: string): string {
  const q = raw.trim();
  if (q === '') return '';
  return q === currentLabel.trim() ? '' : q;
}

/** Собирает список в порядке секций: [«Не указан» только при пустом запросе],
 *  секция 1 после фильтра, секция 2 = результат сервера без устройств секции 1
 *  и без самого редактируемого устройства. */
export function buildHostList(args: {
  query: string;
  currentLabel: string;
  sectionOne: HostItem[];
  searchResults: HostItem[];
  selfId: number | null;
}): HostItem[] {
  const q = effectiveQuery(args.query, args.currentLabel);
  const first = filterSectionOne(args.sectionOne, q);
  const taken = new Set<number>();
  for (const it of args.sectionOne) if (it.kind === 'device') taken.add(it.id);
  const second = args.searchResults.filter(
    (it) => it.kind === 'device' && !taken.has(it.id) && it.id !== args.selfId,
  );
  return [...(q === '' ? [{ kind: 'clear' } as HostItem] : []), ...first, ...second];
}

/** Заголовок секции для `getGroupSection`.
 *  Условие «заголовок второй секции только если есть строки первой» живёт
 *  ЗДЕСЬ, внутри функции: Dropdown не рисует заголовок при undefined, и
 *  безусловный undefined для второй секции означал бы, что «Все устройства»
 *  не появится никогда. */
export function sectionFor(
  item: HostItem,
  ctx: { hasSectionOne: boolean; sectionOneTitle: string },
): string | undefined {
  if (item.kind === 'clear') return undefined;
  if (item.section === 'group') return ctx.sectionOneTitle;
  return ctx.hasSectionOne ? SECTION_ALL : undefined;
}

/** Видимость поля по таблице UI-SPEC 4.1. */
export function isUsbHostFieldVisible(ctx: {
  typeIsPrinter: boolean;
  isEdit: boolean;
  readonly: boolean;
  ipAddress: string;
  printerLoaded: boolean;
  loadedPrinterIp: string | null;
}): boolean {
  if (!ctx.typeIsPrinter) return false;
  if (ctx.readonly) return ctx.printerLoaded;
  if (!ctx.isEdit) return ctx.ipAddress.trim() === '';
  return ctx.printerLoaded && (ctx.loadedPrinterIp ?? '').trim() === '';
}
