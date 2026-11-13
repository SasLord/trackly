// .svelte.ts extension REQUIRED — файл живёт рядом со стором-реестром рун.
//
// Фаза 41.4, план 42 (D-39): единый реестр сброса клиентских кэшей, зависящих
// от пользователя. Модульное состояние JS переживает смену входа в той же
// вкладке: кэш справочников администратора иначе достался бы следующему
// пользователю (в том числе «ready» поверх ошибок, которые он должен увидеть).
//
// Каждый модуль с модульным состоянием, которое грузится с сервера, регистрирует
// здесь свой сброс на уровне модуля. App.svelte вызывает resetUserScoped() при
// смене authStore.user?.id. Гейт: ui/scripts/check-user-scoped-reset.mjs.

const resets = new Set<() => void>();

/** Регистрирует сброс; повторная регистрация той же функции — нет-оп. Возвращает отписку. */
export function registerUserScopedReset(fn: () => void): () => void {
  resets.add(fn);
  return () => {
    resets.delete(fn);
  };
}

/** Вызывает все сбросы; ошибка одного не мешает остальным. */
export function resetUserScoped(): void {
  for (const fn of [...resets]) {
    try {
      fn();
    } catch (e) {
      console.error('resetUserScoped: сброс завершился ошибкой', e);
    }
  }
}
