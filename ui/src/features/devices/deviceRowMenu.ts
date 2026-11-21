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

export type MenuItemKind =
  | 'poll'
  | 'view'
  | 'edit'
  | 'print'
  | 'addToGroup'
  | 'quick'
  | 'removeFromGroup'
  | 'moveToGroup'
  | 'writeOff'
  | 'returnToTeardown'
  | 'delete';

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
  /** Показать «Вывести из группы» (результат canRemoveFromGroup); без поля — не показывать. */
  removeFromGroup?: boolean;
  /** Показать «Переместить в группу» (результат canMoveToGroup); без поля — не показывать. */
  moveToGroup?: boolean;
  /** Показать «Списать» (результат canWriteOff); без поля — не показывать. */
  writeOff?: boolean;
}

export interface GroupActionInput {
  statusCode: string;
  isMember: boolean;
  isAnchor: boolean;
}

/** type_id типа «Устройство»: быстрые действия строки только для него; фильтр страницы тип больше не задаёт (фаза 41.9, D-18). */
export const ANCHOR_DEVICE_TYPE_ID = 1;

/** Всё, что живёт ВНУТРИ строки, но не является самой строкой: модалки и меню. */
export const ROW_OVERLAY_SELECTOR = '.modal-backdrop, [role="dialog"], [role="menu"]';

const LABEL_POLL = 'Опросить сейчас';
const LABEL_VIEW = 'Просмотр';
const LABEL_EDIT = 'Редактировать';
const LABEL_PRINT = 'Печать документа приёма';
const LABEL_ADD_TO_GROUP = 'Добавить в группу';
const LABEL_REMOVE_FROM_GROUP = 'Вывести из группы';
const LABEL_MOVE_TO_GROUP = 'Переместить в группу';
const LABEL_WRITE_OFF = 'Списать';
const LABEL_RETURN_TO_TEARDOWN = 'Вернуть в разбор';
const LABEL_DELETE = 'Удалить';

/** Машинные коды статусов (не подписи), при которых групповые действия бессмысленны. */
export const STATUS_WRITTEN_OFF = 'списано';
export const STATUS_TO_WRITE_OFF = 'на_списание';
const BLOCKED_STATUS_CODES = [STATUS_WRITTEN_OFF, STATUS_TO_WRITE_OFF];

/** Роли с правом MutateGroups / списания: зеркало серверных отказов. */
const MUTATE_ROLES = ['admin', 'manager'];
const WRITE_OFF_ROLES = ['admin', 'manager'];

/**
 * Секции меню сверху вниз (D-03): [опрос] → [просмотр, правка, печать] →
 * [вывести из группы, переместить в группу] → [в группу, быстрые действия] →
 * [списать, удалить]. «Списать» не danger, danger только у «Удалить». Пустая
 * секция не возвращается, а значит и граница-разделитель перед ней рисоваться
 * не будет: меню не начинается и не заканчивается разделителем, двух подряд
 * не бывает.
 */
export function buildMenuSections(input: MenuInput): MenuItemSpec[][] {
  const poll: MenuItemSpec[] = input.hasPoll ? [{ kind: 'poll', label: LABEL_POLL }] : [];
  const main: MenuItemSpec[] = [
    { kind: 'view', label: LABEL_VIEW },
    { kind: 'edit', label: LABEL_EDIT },
  ];
  if (input.hasPrint) main.push({ kind: 'print', label: LABEL_PRINT });
  const membership: MenuItemSpec[] = [];
  if (input.removeFromGroup === true) {
    membership.push({ kind: 'removeFromGroup', label: LABEL_REMOVE_FROM_GROUP });
  }
  if (input.moveToGroup === true) {
    membership.push({ kind: 'moveToGroup', label: LABEL_MOVE_TO_GROUP });
  }
  const group: MenuItemSpec[] = input.groupActionsVisible
    ? [
        { kind: 'addToGroup', label: LABEL_ADD_TO_GROUP },
        ...input.quickActions.map(
          (q): MenuItemSpec => ({ kind: 'quick', label: q.label, typeId: q.typeId }),
        ),
      ]
    : [];
  const tail: MenuItemSpec[] = [];
  if (input.writeOff === true) tail.push({ kind: 'writeOff', label: LABEL_WRITE_OFF });
  tail.push({ kind: 'delete', label: LABEL_DELETE, danger: true });
  return [poll, main, membership, group, tail].filter((s) => s.length > 0);
}

/**
 * Показывать ли групповые пункты («Добавить в группу», быстрые действия).
 * Решение по `code` статуса, а не по подписи и не по числовому id (D-02).
 */
export function isGroupActionAllowed(input: GroupActionInput): boolean {
  if (input.isMember || input.isAnchor) return false;
  return !BLOCKED_STATUS_CODES.includes(input.statusCode);
}

/**
 * Показывать ли «Вывести из группы» (GRD-12, D-01): роль admin или manager (право
 * MutateGroups), устройство состоит в группе и не является якорем ЛЮБОЙ группы.
 * Тип устройства не ограничивает: пункт виден любому прямому участнику.
 * Зеркалит отказ сервера: `release_inner` отказывает для якоря любой группы
 * (D-14 фазы 41.1). Сервер остаётся границей безопасности, UI заранее прячет пункт.
 */
export function canRemoveFromGroup(input: {
  role: string | null;
  groupId: number | null;
  anchorGroupId: number | null;
}): boolean {
  return (
    input.role !== null &&
    MUTATE_ROLES.includes(input.role) &&
    input.groupId !== null &&
    input.anchorGroupId === null
  );
}

/**
 * Показывать ли «Переместить в группу» (D-04): admin/manager, участник группы,
 * не якорь, статус не «Списано»/«На списание» (сервер не вводит в группу такие
 * устройства). Решение по `code` статуса.
 */
export function canMoveToGroup(input: {
  role: string | null;
  groupId: number | null;
  anchorGroupId: number | null;
  statusCode: string;
}): boolean {
  if (input.anchorGroupId !== null) return false;
  if (BLOCKED_STATUS_CODES.includes(input.statusCode)) return false;
  if (input.groupId === null) return false;
  return input.role !== null && MUTATE_ROLES.includes(input.role);
}

/**
 * Показывать ли «Списать» (D-19): admin/manager и статус с кодом `на_списание`,
 * в том числе у якоря и участника группы. Решение по `code`, не по подписи.
 */
export function canWriteOff(input: { role: string | null; statusCode: string }): boolean {
  return (
    input.role !== null &&
    WRITE_OFF_ROLES.includes(input.role) &&
    input.statusCode === STATUS_TO_WRITE_OFF
  );
}

/**
 * Можно ли вернуть деталь призрака в разбор (D-12, D-15): admin/manager, деталь
 * не удалена и не списана. Одна функция для меню призрака и строки возврата.
 */
export function canReturnToTeardown(input: {
  role: string | null;
  isDeleted: boolean;
  statusCode: string;
}): boolean {
  return (
    input.role !== null &&
    MUTATE_ROLES.includes(input.role) &&
    !input.isDeleted &&
    input.statusCode !== STATUS_WRITTEN_OFF
  );
}

/** Меню призрака (D-12): ровно один пункт «Вернуть в разбор» либо пусто. */
export function ghostMenuSections(input: {
  role: string | null;
  isDeleted: boolean;
  statusCode: string;
}): MenuItemSpec[][] {
  return canReturnToTeardown(input)
    ? [[{ kind: 'returnToTeardown', label: LABEL_RETURN_TO_TEARDOWN }]]
    : [];
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
