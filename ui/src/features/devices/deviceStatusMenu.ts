// Фаза 41.8, план 29 (D-35, D-26): чистая логика подменю «Статус» в меню строки
// устройства. Какие статусы показать, какие неактивны и кому виден сам пункт.
// Правила зеркалят форму устройства и серверные отказы; сервер остаётся границей
// безопасности (DeviceService::update).
//
// Модуль БЕЗ импортов: golden-гейт check-device-status-menu.mjs транспилирует его
// изолированно. Локальные литералы (код «списано», роли, тексты причин) сверяются
// гейтом с STATUS_WRITTEN_OFF / MUTATE_ROLES (deviceRowMenu.ts),
// anchorWriteOffRefusal (writeOffTexts.ts) и REPAIR_STATUS_NOTE (repairLocks.ts).
const WRITTEN_OFF_CODE = 'списано';
const STATUS_MENU_ROLES = ['admin', 'manager'];
const REPAIR_REASON = 'Статус меняется заявкой на ремонт.';

export interface StatusMenuStatus {
  id: number;
  code: string;
  name: string;
}

export interface StatusOptionSpec {
  id: number;
  code: string;
  label: string;
  current: boolean;
  disabled: boolean;
  disabledReason: string | null;
}

export interface StatusMenuInput {
  statuses: StatusMenuStatus[];
  currentStatusId: number | null;
  isAnchor: boolean;
  anchorGroupName: string | null;
  /** Номер открытой ремонтной заявки по устройству; null или отсутствие поля: правило не действует. */
  openRepairRequestId?: number | null;
}

function anchorReason(groupName: string | null): string {
  return `Устройство — якорь группы «${groupName ?? ''}»; спишите его через группу.`;
}

/** Варианты подменю «Статус» в порядке справочника. */
export function statusMenuOptions(input: StatusMenuInput): StatusOptionSpec[] {
  const repairLocked =
    input.openRepairRequestId !== null && input.openRepairRequestId !== undefined;
  return input.statuses.map((s): StatusOptionSpec => {
    let disabled = false;
    let disabledReason: string | null = null;
    if (repairLocked) {
      disabled = true;
      disabledReason = REPAIR_REASON;
    } else if (input.isAnchor && s.code === WRITTEN_OFF_CODE) {
      disabled = true;
      disabledReason = anchorReason(input.anchorGroupName);
    }
    return {
      id: s.id,
      code: s.code,
      label: s.name,
      current: s.id === input.currentStatusId,
      disabled,
      disabledReason,
    };
  });
}

/** Виден ли пункт «Статус»: право MutateDevices (admin, manager). */
export function canChangeStatus(input: { role: string | null }): boolean {
  return input.role !== null && STATUS_MENU_ROLES.includes(input.role);
}
