import { parseAppError } from './errors';

interface CryptoLike {
  randomUUID?: () => string;
  getRandomValues?: <T extends ArrayBufferView>(array: T) => T;
}

/// Паузы перед повторами (D-12): три повтора, 1 с, 2 с, 4 с.
export const RETRY_DELAYS_MS = [1000, 2000, 4000];

/// Коды, при которых повтор осмыслен (D-12): отказ fetch (его в apiCall превращают в
/// NETWORK_ERROR) и шлюзовые ответы 502/503/504. Остальные HTTP-ответы (400/413/422 …)
/// и доменные AppError повторять бессмысленно.
export const RETRYABLE_CODES: ReadonlySet<string> = new Set([
  'NETWORK_ERROR',
  'HTTP_502',
  'HTTP_503',
  'HTTP_504',
]);

/// Прямой отказ fetch у вызывающих, обходящих apiCall.
const FETCH_FAILURE_RE = /fetch|network|load failed/i;

/// Сетевой сбой: NETWORK_ERROR, HTTP 502/503/504 или TypeError отказа fetch.
/// Программные ошибки (TypeError иного рода) и любой иной AppError не повторяются.
export function isNetworkFailure(e: unknown): boolean {
  const code = parseAppError(e).code;
  return RETRYABLE_CODES.has(code) || (e instanceof TypeError && FETCH_FAILURE_RE.test(e.message));
}

const defaultSleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

export interface SendWithRetryOptions {
  delaysMs?: number[];
  sleep?: (ms: number) => Promise<void>;
  isRetryable?: (e: unknown) => boolean;
}

/// Вызывает fn; при сетевом сбое повторяет ТОТ ЖЕ fn (замыкание держит один клиентский id).
export async function sendWithRetry<T>(
  fn: () => Promise<T>,
  opts: SendWithRetryOptions = {},
): Promise<T> {
  const delays = opts.delaysMs ?? RETRY_DELAYS_MS;
  const sleep = opts.sleep ?? defaultSleep;
  const retryable = opts.isRetryable ?? isNetworkFailure;
  let attempt = 0;
  for (;;) {
    try {
      return await fn();
    } catch (e) {
      if (!retryable(e) || attempt >= delays.length) throw e;
      await sleep(delays[attempt]);
      attempt += 1;
    }
  }
}

/// Клиентский id идемпотентности: ^[A-Za-z0-9_-]{1,64}$ (серверная проверка is_valid_client_id).
export function newClientId(): string {
  const c = (globalThis as { crypto?: CryptoLike }).crypto;
  if (c && typeof c.randomUUID === 'function') return c.randomUUID();
  if (c && typeof c.getRandomValues === 'function') {
    const bytes = c.getRandomValues(new Uint8Array(16));
    return Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
  }
  // Запасной путь (контекст без crypto): не криптостойкий, но уникальный в пределах практики.
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}${Math.random().toString(36).slice(2)}`.slice(
    0,
    64,
  );
}
