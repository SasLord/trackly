// Фаза 41.3, G2: подпись автора / удалившего вложение.
//
// id = null — действие сделано в десктопном приложении без учётной записи
// (Identity::trusted_admin при выключенной блокировке рабочего стола). Это НЕ
// удалённый пользователь: учётная запись удалена, только когда id есть, а имени
// по нему уже нет.

export const APP_ACTOR_LABEL = 'Администратор (приложение)';
export const DELETED_USER_LABEL = 'Пользователь удалён';

export function actorLabel(name: string | null, id: number | null): string {
  if (name) return name;
  return id === null ? APP_ACTOR_LABEL : DELETED_USER_LABEL;
}
