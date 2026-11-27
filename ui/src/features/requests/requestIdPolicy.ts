// Политика clientRequestId формы заявки (фаза 43, D-11/D-12, уточнение WR-02).
// Чистый модуль: без рун и компонентов, исполняется гейтом check-request-create-idempotency.

export function requestSignature(
  _requestType: string,
  _printerDeviceId: number | null,
  _categoryId: number | null,
  _description: string,
): string {
  throw new Error('не реализовано');
}

export function createRequestIdIssuer(_newId: () => string): {
  idFor(signature: string): string;
  reset(): void;
} {
  throw new Error('не реализовано');
}
