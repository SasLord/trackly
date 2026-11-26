// Хуки доступа вложений. Пакет не знает auth: приложение регистрирует адаптер
// (setAttachmentsHost) до mount. Без регистрации пользователя «нет» — минимум прав.

export interface AttachmentsHostUser {
  id: number;
  role: string;
}

export interface AttachmentsHost {
  /** Текущий пользователь; читается внутри $derived/шаблонов, должен быть реактивным. */
  currentUser(): AttachmentsHostUser | null;
}

let host: AttachmentsHost = { currentUser: () => null };

export function setAttachmentsHost(next: AttachmentsHost): void {
  host = next;
}

export function currentUser(): AttachmentsHostUser | null {
  return host.currentUser();
}
