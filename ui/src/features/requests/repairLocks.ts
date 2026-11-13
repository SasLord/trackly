// Фаза 41.4, план 18 (D-21, D-22, D-14, D-17): общие тексты и загрузчики для
// UI-блокировок вокруг открытой ремонтной заявки. UI только показывает причину
// запрета; авторитетный отказ даёт сервер (планы 04 и 05), поэтому сбой запроса
// здесь молча означает «заявок нет» и ничего не блокирует.
import { push } from 'svelte-spa-router';
import { authStore } from '$lib/stores/auth.svelte';
import { groups } from '$lib/api/groups';
import { registerUserScopedReset } from '$lib/stores/userScoped.svelte';
import { repairs } from './api';

/** Ремонтные команды доступны менеджеру и администратору (TransitionRequests). */
export function canUseRepairs(): boolean {
  const role = authStore.user?.role;
  return role === 'admin' || role === 'manager';
}

/** deviceId -> requestId открытой ремонтной заявки; ошибка или нет доступа — пустая карта. */
export async function loadOpenRepairs(ids: number[]): Promise<Map<number, number>> {
  const map = new Map<number, number>();
  if (ids.length === 0 || !canUseRepairs()) return map;
  try {
    const rows = await repairs.openForDevices(ids);
    for (const r of rows) map.set(r.deviceId, r.requestId);
  } catch {
    // Подсказка не обязательна: серверный запрет действует и без неё.
  }
  return map;
}

export const REPAIR_STATUS_NOTE = 'Статус меняется заявкой на ремонт.';
export const REPAIR_ACT_NOTE = 'Открыта заявка на ремонт — в акт не включить.';
export const REPAIR_CONFLICT_TOAST =
  'Нельзя изменить устройство: по нему открыта заявка на ремонт.';

export function repairOpenLabel(requestId: number): string {
  return `Открыть заявку №${requestId}`;
}

export function repairMoveLockTitle(requestId: number): string {
  return `Устройство в ремонте — перемещайте его через заявку №${requestId}.`;
}

/** Серверный отказ «по устройству открыта заявка» (гонка или обход UI по LAN). */
export function isRepairConflict(e: unknown): boolean {
  if (!e || typeof e !== 'object') return false;
  const err = e as { code?: string; message?: string };
  return err.code === 'CONFLICT' && (err.message ?? '').includes('заявка на ремонт');
}

export function openRepairRequest(requestId: number): void {
  push(`#/requests?id=${requestId}`);
}

// Место группы: короткий кэш на группу (список строк делит один запрос);
// через 15 секунд запись устаревает, чтобы смена места группы не залипала.
type GroupPlace = { id: number | null; path: string | null };
const GROUP_PLACE_TTL_MS = 15_000;
const groupPlaceCache = new Map<number, { at: number; p: Promise<GroupPlace> }>();

// D-39: место группы прежнего пользователя не переносится новому.
registerUserScopedReset(() => {
  groupPlaceCache.clear();
});

export function groupPlace(groupId: number): Promise<GroupPlace> {
  const hit = groupPlaceCache.get(groupId);
  if (hit && Date.now() - hit.at < GROUP_PLACE_TTL_MS) return hit.p;
  const p = groups
    .get(groupId)
    .then((g): GroupPlace => ({ id: g.place_id, path: g.place_path }))
    .catch((): GroupPlace => {
      groupPlaceCache.delete(groupId);
      return { id: null, path: null };
    });
  groupPlaceCache.set(groupId, { at: Date.now(), p });
  return p;
}

/** Подписи статусов заявок — те же слова, что в списке и карточке заявки. */
export function requestStatusLabel(status: string): string {
  switch (status) {
    case 'open':
      return 'Создана';
    case 'in_progress':
      return 'В работе';
    case 'in_repair':
      return 'В ремонте';
    case 'completed':
      return 'Выполнена';
    case 'cancelled':
      return 'Отменена';
    default:
      return 'Отклонена';
  }
}

export function requestStatusVariant(status: string): 'default' | 'accent' | 'success' | 'warning' {
  if (status === 'open') return 'accent';
  if (status === 'in_progress' || status === 'in_repair') return 'warning';
  if (status === 'completed') return 'success';
  return 'default';
}
