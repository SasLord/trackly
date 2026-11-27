// Политика clientRequestId формы заявки (фаза 43, D-11/D-12, уточнение WR-02).
// Чистый модуль: без рун и компонентов, исполняется гейтом check-request-create-idempotency.

/**
 * Подпись тела requests.create: только поля, реально уходящие в тело для данного типа
 * (принтер — у cartridge_replace, категория — у free_form), описание с trim, как в запросе.
 */
export function requestSignature(
  requestType: string,
  printerDeviceId: number | null,
  categoryId: number | null,
  description: string,
): string {
  return JSON.stringify([
    requestType,
    requestType === 'cartridge_replace' ? printerDeviceId : null,
    requestType === 'free_form' ? categoryId : null,
    description.trim(),
  ]);
}

/**
 * Выдатчик id: пока подпись тела не менялась, возвращает один и тот же id (повтор после
 * потери ответа не создаёт дубль, D-11); правка тела после сбоя выпускает новый id, иначе
 * сервер по ключу идемпотентности вернул бы исходную заявку и правка пропала бы (WR-02).
 */
export function createRequestIdIssuer(newId: () => string): {
  idFor(signature: string): string;
  reset(): void;
} {
  let current: string | null = null;
  let lastSignature: string | null = null;
  return {
    idFor(signature: string): string {
      if (current === null || signature !== lastSignature) {
        current = newId();
        lastSignature = signature;
      }
      return current;
    },
    reset(): void {
      current = null;
      lastSignature = null;
    },
  };
}
