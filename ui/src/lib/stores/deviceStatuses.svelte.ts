// .svelte.ts extension REQUIRED — Svelte 5 runes.
//
// Фаза 41.1 (D-06, D-09): единый клиентский справочник статусов устройств.
// Подписи и порядок приходят с сервера (`device_statuses_list`), в приложении
// не остаётся карт «id -> подпись» и «имя -> тон». Тон бейджа решается по
// машинному `code`, поэтому переименование подписи тон не меняет (D-02).
//
// Загрузка ленивая: ensureLoaded() вызывают экраны, которым нужны статусы
// (onMount), а не корень приложения. Справочник открыт под ReadData, и
// безусловная загрузка при старте дала бы employee-сессии 403 (A7).
//
// Записи в $state делаются только из async-продолжения ensureLoaded, не
// синхронно в теле $effect (иначе effect_update_depth_exceeded).

import { devices } from '$lib/api/devices';
import { pushToast } from '$lib/stores/toast.svelte';
import type { DeviceStatusDto } from '../../bindings';

export type StatusVariant = 'default' | 'accent' | 'success' | 'warning' | 'destructive';
export type StatusAppearance = 'soft' | 'solid' | 'dot' | 'count';
export interface StatusTone {
  variant: StatusVariant;
  appearance: StatusAppearance;
  /** Приглушённый чип: статус «ещё в очереди/помечен», тон тот же, ярче только действующий. */
  muted?: boolean;
}

type LoadState = 'idle' | 'loading' | 'ready' | 'error';

export const deviceStatusesStore = $state<{ items: DeviceStatusDto[]; state: LoadState }>({
  items: [],
  state: 'idle',
});

// Единственная в приложении таблица тонов; ключ — code из справочника
// (UI-SPEC §10.2, пересмотрено 2026-10-17: R-06 отменён). Все шесть статусов —
// soft-чипы. Тон = насколько плохо, muted = ещё только в очереди/помечено:
// пара «действующий / ожидающий» делит variant и различается яркостью.
const TONES: Record<string, StatusTone> = {
  на_складе: { variant: 'default', appearance: 'soft' },
  в_работе: { variant: 'accent', appearance: 'soft' },
  на_ремонте: { variant: 'warning', appearance: 'soft', muted: true },
  в_ремонте: { variant: 'warning', appearance: 'soft' },
  на_списание: { variant: 'destructive', appearance: 'soft', muted: true },
  списано: { variant: 'destructive', appearance: 'soft' },
};
const FALLBACK_TONE: StatusTone = { variant: 'default', appearance: 'soft' };

let inflight: Promise<void> | null = null;
let errorToasted = false;

/** Грузит справочник один раз за сессию; параллельные вызовы делят один запрос. */
export function ensureLoaded(): Promise<void> {
  if (deviceStatusesStore.state === 'ready') return Promise.resolve();
  if (inflight) return inflight;
  deviceStatusesStore.state = 'loading';
  inflight = devices
    .statuses()
    .then((rows) => {
      // apiCall отдаёт text/html как строку; без проверки формы такой «успех»
      // (например, index.html вместо JSON) дал бы state='ready' и мусор в items.
      if (
        !Array.isArray(rows) ||
        !rows.every(
          (r) =>
            typeof r?.id === 'number' &&
            typeof r?.code === 'string' &&
            typeof r?.name === 'string' &&
            typeof r?.sort_order === 'number',
        )
      ) {
        throw new Error('device_statuses_list: неожиданная форма ответа');
      }
      deviceStatusesStore.items = rows;
      deviceStatusesStore.state = 'ready';
    })
    .catch(() => {
      deviceStatusesStore.state = 'error';
      if (!errorToasted) {
        errorToasted = true;
        pushToast(
          'error',
          'Не удалось загрузить справочник статусов. Подписи показаны кодами, обновите страницу.',
        );
      }
    })
    .finally(() => {
      inflight = null;
    });
  return inflight;
}

export function statusById(id: number): DeviceStatusDto | undefined {
  return deviceStatusesStore.items.find((s) => s.id === id);
}

export function statusByCode(code: string): DeviceStatusDto | undefined {
  return deviceStatusesStore.items.find((s) => s.code === code);
}

/** id статуса по коду; null, пока справочник не загружен. */
export function statusIdByCode(code: string): number | null {
  return statusByCode(code)?.id ?? null;
}

/** Подпись из справочника; до загрузки — «Статус {id}». */
export function statusLabel(id: number): string {
  return statusById(id)?.name ?? `Статус ${id}`;
}

/** Статусы в порядке жизненного цикла (sort_order). */
export function orderedStatuses(): DeviceStatusDto[] {
  return [...deviceStatusesStore.items].sort((a, b) => a.sort_order - b.sort_order || a.id - b.id);
}

/** Тон бейджа по коду статуса; неизвестный или пустой код — нейтральный. */
export function statusTone(code: string | null | undefined): StatusTone {
  return (code ? TONES[code] : undefined) ?? FALLBACK_TONE;
}
