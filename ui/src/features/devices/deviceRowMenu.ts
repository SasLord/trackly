// Фаза 41.1, план 16 (GRD-01, GRD-02, GRD-05, D-22, D-24, D-25, D-26, D-38):
// чистая логика меню строки устройства. Модуль самодостаточен (ни одного
// import, кроме типов) и не знает про DOM-типы: его транспилирует и гоняет по
// golden-фикстуре Node-гейт `scripts/check-device-row-menu.mjs`, потому что
// JS-тест-раннера в проекте нет, а компиляционные гейты не ловят ни
// расходящихся правил видимости, ни забытой охраны события.
//
// Что здесь решается: какие пункты и в каком порядке (buildMenuSections), у
// каких устройств групповые пункты бессмысленны (isGroupActionAllowed), какие
// быстрые действия порождают типы групп (quickActionsFromTypes /
// quickActionsForDevice) и какие события строки принадлежат вложенным
// модалкам или меню, а не самой строке (isRowOverlayEvent / isOwnKeyEvent).
//
// Правила видимости зеркалят отказы сервера (план 11): UI только заранее
// прячет то, что сервер всё равно отклонит. Сервер остаётся границей
// безопасности; общей фикстуры с Rust у этого модуля нет.

export type MenuItemKind = 'poll' | 'view' | 'edit' | 'print' | 'addToGroup' | 'quick' | 'delete';

export interface MenuItemSpec {
  kind: MenuItemKind;
  label: string;
  /** Только для kind === 'quick': id типа группы, по которому создаётся группа. */
  typeId?: number;
  danger?: boolean;
}

export interface QuickAction {
  typeId: number;
  label: string;
}

/** Минимум полей типа группы, нужный меню (подмножество GroupTypeDto). */
export interface QuickActionType {
  id: number;
  sort_order: number;
  quick_action_enabled: boolean;
  quick_action_label: string | null;
}

export interface MenuInput {
  hasPoll: boolean;
  hasPrint: boolean;
  groupActionsVisible: boolean;
  quickActions: QuickAction[];
}

export interface GroupActionInput {
  statusCode: string;
  isMember: boolean;
  isAnchor: boolean;
  isNetworkPrinter: boolean;
}

/** type_id типа «Устройство»: тот же, что DevicesPage передаёт в фильтр списка. */
export const ANCHOR_DEVICE_TYPE_ID = 1;

/** Всё, что живёт ВНУТРИ строки, но не является самой строкой: модалки и меню. */
export const ROW_OVERLAY_SELECTOR = '.modal-backdrop, [role="dialog"], [role="menu"]';

const LABEL_POLL = 'Опросить сейчас';
const LABEL_VIEW = 'Просмотр';
const LABEL_EDIT = 'Редактировать';
const LABEL_PRINT = 'Печать документа приёма';
const LABEL_ADD_TO_GROUP = 'Добавить в группу';
const LABEL_DELETE = 'Удалить';

/** Машинные коды статусов (не подписи), при которых групповые действия бессмысленны. */
const BLOCKED_STATUS_CODES = ['списано', 'на_списание'];

/**
 * Секции меню сверху вниз: [опрос] → [просмотр, правка, печать] →
 * [в группу, быстрые действия] → [удалить]. Пустая секция не возвращается, а
 * значит и граница-разделитель перед ней рисоваться не будет: меню не
 * начинается и не заканчивается разделителем, двух подряд не бывает.
 */
export function buildMenuSections(input: MenuInput): MenuItemSpec[][] {
  const poll: MenuItemSpec[] = input.hasPoll ? [{ kind: 'poll', label: LABEL_POLL }] : [];
  const main: MenuItemSpec[] = [
    { kind: 'view', label: LABEL_VIEW },
    { kind: 'edit', label: LABEL_EDIT },
  ];
  if (input.hasPrint) main.push({ kind: 'print', label: LABEL_PRINT });
  const group: MenuItemSpec[] = input.groupActionsVisible
    ? [
        { kind: 'addToGroup', label: LABEL_ADD_TO_GROUP },
        ...input.quickActions.map(
          (q): MenuItemSpec => ({ kind: 'quick', label: q.label, typeId: q.typeId }),
        ),
      ]
    : [];
  const del: MenuItemSpec[] = [{ kind: 'delete', label: LABEL_DELETE, danger: true }];
  return [poll, main, group, del].filter((s) => s.length > 0);
}

/**
 * Показывать ли групповые пункты («Добавить в группу», быстрые действия).
 * Решение по `code` статуса, а не по подписи и не по числовому id (D-02).
 */
export function isGroupActionAllowed(input: GroupActionInput): boolean {
  if (input.isMember || input.isAnchor || input.isNetworkPrinter) return false;
  return !BLOCKED_STATUS_CODES.includes(input.statusCode);
}

/**
 * Быстрые действия, порождённые типами групп: только включённые и с непустой
 * подписью, по возрастанию sort_order типа, затем id. Подпись берётся ТОЛЬКО из
 * quick_action_label (D-22): литералов встроенных подписей в UI нет.
 */
export function quickActionsFromTypes(types: QuickActionType[]): QuickAction[] {
  return types
    .filter((t) => t.quick_action_enabled === true)
    .map((t) => ({ t, label: (t.quick_action_label ?? '').trim() }))
    .filter((x) => x.label !== '')
    .sort((a, b) => a.t.sort_order - b.t.sort_order || a.t.id - b.t.id)
    .map((x) => ({ typeId: x.t.id, label: x.label }));
}

/**
 * Быстрые действия, СОЗДАЮЩИЕ группу от устройства, доступны только
 * устройствам type_id = 1 (D-38): у принтера остаётся лишь «Добавить в группу».
 * Сервер дублирует правило отказом (план 11).
 */
export function quickActionsForDevice(
  types: QuickActionType[],
  deviceTypeId: number,
): QuickAction[] {
  if (deviceTypeId !== ANCHOR_DEVICE_TYPE_ID) return [];
  return quickActionsFromTypes(types);
}

/**
 * Событие пришло из модалки или меню, живущих внутри строки (Modal не
 * портальный и рендерится на месте). Duck-typing: модуль без DOM-типов, цель
 * без closest (текстовый узел) и null не считаются оверлеем.
 */
export function isRowOverlayEvent(target: unknown): boolean {
  if (typeof target !== 'object' || target === null) return false;
  const closest = (target as { closest?: unknown }).closest;
  if (typeof closest !== 'function') return false;
  return closest.call(target, ROW_OVERLAY_SELECTOR) != null;
}

/**
 * Клавиша адресована самой ячейке строки, а не вложенному полю модалки:
 * цель события — сам currentTarget. Пустая цель (null / undefined) не своя,
 * даже если ячейка тоже пуста.
 */
export function isOwnKeyEvent(target: unknown, currentTarget: unknown): boolean {
  if (target === null || target === undefined) return false;
  return target === currentTarget;
}
