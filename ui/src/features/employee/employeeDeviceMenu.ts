// Фаза 41.6, план 08 (D-09, D-10, D-11): чистая логика меню строки устройства
// сотрудника. Модуль самодостаточен (только `import type`): его транспилирует и
// гоняет по golden-фикстуре Node-гейт `scripts/check-employee-device-menu.mjs`.
//
// Правила видимости только заранее прячут то, что сервер всё равно решит сам;
// границей безопасности остаётся сервер.

export interface EmployeeMenuRow {
  isPrinter: boolean;
  inventoryNumber: string | null;
  /** Есть открытая ремонтная заявка (независимо от участия). */
  inRepair: boolean;
  /** Открытая ремонтная заявка, где сотрудник участник. */
  myRepairRequestId: number | null;
  /** Открытые заявки по устройству, где сотрудник участник. */
  myOpenRequests: { id: number; requestType: string; status: string }[];
}

export type EmployeeMenuItemId = 'open' | 'repair' | 'cartridge' | 'copy' | `goto:${number}`;

export interface EmployeeMenuItem {
  id: EmployeeMenuItemId;
  label: string;
  hint?: string;
}

/** Подсказка пункта «Заявка на ремонт» (D-11); номер чужой заявки не показывается. */
export const JOIN_REPAIR_HINT = 'Уже в ремонте — вы присоединитесь к заявке';
/** Подпись в футере карточки (D-11). */
export const JOIN_REPAIR_FOOTER_NOTE = 'Устройство уже в ремонте — вы присоединитесь к заявке';

export function buildEmployeeMenu(row: EmployeeMenuRow): EmployeeMenuItem[][] {
  const first: EmployeeMenuItem[] = [{ id: 'open', label: 'Открыть' }];

  const second: EmployeeMenuItem[] = [];
  if (row.myRepairRequestId === null) {
    const repair: EmployeeMenuItem = { id: 'repair', label: 'Заявка на ремонт' };
    if (row.inRepair) repair.hint = JOIN_REPAIR_HINT;
    second.push(repair);
  }
  if (row.isPrinter) second.push({ id: 'cartridge', label: 'Замена картриджа' });
  const gotos = [...row.myOpenRequests].sort((a, b) => a.id - b.id);
  for (const r of gotos) {
    second.push({ id: `goto:${r.id}`, label: `Перейти к заявке №${r.id}` });
  }

  const third: EmployeeMenuItem[] = [];
  if ((row.inventoryNumber ?? '').trim() !== '') {
    third.push({ id: 'copy', label: 'Скопировать инв. номер' });
  }

  return [first, second, third].filter((s) => s.length > 0);
}
