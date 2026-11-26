import { parseAppError } from './errors';

/// Паузы перед повторами (D-12): три повтора, 1 с, 2 с, 4 с.
export const RETRY_DELAYS_MS = [1000, 2000, 4000];

/// Сетевой сбой = значение без формы AppError (отказ fetch, обрыв).
/// Любой AppError с кодом — доменный отказ сервера, повторять его бессмысленно.
export function isNetworkFailure(e: unknown): boolean {
  return parseAppError(e).code === 'UnknownError';
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
  const c = (globalThis as { crypto?: Crypto }).crypto;
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
