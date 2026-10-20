// .svelte.ts extension REQUIRED — Svelte 5 runes.
//
// Фаза 41.12 (INT-14): единый клиентский справочник типов устройств. Подписи и
// порядок приходят с сервера (`device_types_list`); в приложении не остаётся
// карт «id типа -> подпись» (прежняя неверная подпись типа жила в фильтре
// отчётов, копия — в пикере «Добавить устройства в группу»).
//
// Загрузка ленивая: ensureLoaded() вызывают экраны, которым нужны типы
// (onMount), а не корень приложения. Справочник открыт под ReadData, и
// безусловная загрузка при старте дала бы employee-сессии 403.
//
// Записи в $state делаются только из async-продолжения ensureLoaded, не
// синхронно в теле эффекта (иначе effect_update_depth_exceeded).

import { devices } from '$lib/api/devices';
import { pushToast } from '$lib/stores/toast.svelte';
import type { DeviceTypeDto } from '../../bindings';

type LoadState = 'idle' | 'loading' | 'ready' | 'error';

export const deviceTypesStore = $state<{ items: DeviceTypeDto[]; state: LoadState }>({
  items: [],
  state: 'idle',
});

let inflight: Promise<void> | null = null;
let errorToasted = false;

/** Грузит справочник один раз за сессию; параллельные вызовы делят один запрос. */
export function ensureLoaded(): Promise<void> {
  if (deviceTypesStore.state === 'ready') return Promise.resolve();
  if (inflight) return inflight;
  deviceTypesStore.state = 'loading';
  inflight = devices
    .types()
    .then((rows) => {
      deviceTypesStore.items = rows;
      deviceTypesStore.state = 'ready';
    })
    .catch(() => {
      deviceTypesStore.state = 'error';
      if (!errorToasted) {
        errorToasted = true;
        pushToast(
          'error',
          'Не удалось загрузить справочник типов. Подписи показаны кодами, обновите страницу.',
        );
      }
    })
    .finally(() => {
      inflight = null;
    });
  return inflight;
}

export function typeById(id: number): DeviceTypeDto | undefined {
  return deviceTypesStore.items.find((t) => t.id === id);
}

/** Подпись из справочника; до загрузки — «Тип {id}». */
export function typeLabel(id: number): string {
  return typeById(id)?.name ?? `Тип ${id}`;
}

/** Типы в порядке id. */
export function orderedTypes(): DeviceTypeDto[] {
  return [...deviceTypesStore.items].sort((a, b) => a.id - b.id);
}
