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

const deviceTypesStore = $state<{ items: DeviceTypeDto[]; state: LoadState }>({
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
      // apiCall отдаёт text/html как строку; без проверки формы такой «успех»
      // (например, index.html вместо JSON) дал бы state='ready' и массив символов.
      if (
        !Array.isArray(rows) ||
        !rows.every((r) => typeof r?.id === 'number' && typeof r?.name === 'string')
      ) {
        throw new Error('device_types_list: неожиданная форма ответа');
      }
      deviceTypesStore.items = rows;
      deviceTypesStore.state = 'ready';
    })
    .catch(() => {
      deviceTypesStore.state = 'error';
      if (!errorToasted) {
        errorToasted = true;
        pushToast(
          'error',
          'Не удалось загрузить справочник типов. Список типов не загружен, обновите страницу.',
        );
      }
    })
    .finally(() => {
      inflight = null;
    });
  return inflight;
}

/** Типы в порядке id. */
export function orderedTypes(): DeviceTypeDto[] {
  return [...deviceTypesStore.items].sort((a, b) => a.id - b.id);
}
