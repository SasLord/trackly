// Обёртки invoke для команд Link (кроме link_info — он в linkInfo.ts).
// Сетевых вызовов в webview нет: всё идёт через Rust (ADR 0002).
import { invoke } from '@tauri-apps/api/core';
import type { CmdError, CmdErrorCode, LinkEvent } from './linkState';

const KNOWN: ReadonlySet<string> = new Set<CmdErrorCode>([
  'empty_code',
  'invalid_code',
  'expired_code',
  'used_code',
  'server_unreachable',
  'pin_mismatch',
  'bad_credentials',
  'user_blocked',
  'device_banned',
  'rate_limited',
  'no_network',
  'auth_unavailable',
  'internal',
]);

const INTERNAL_MESSAGE = 'Внутренняя ошибка Link. Повторите попытку.';

/** Приводит что угодно, пришедшее из invoke, к { code, message }. */
export function normalizeError(e: unknown): CmdError {
  if (e && typeof e === 'object') {
    const o = e as { code?: unknown; message?: unknown };
    if (typeof o.code === 'string' && KNOWN.has(o.code)) {
      return {
        code: o.code as CmdErrorCode,
        message: typeof o.message === 'string' ? o.message : INTERNAL_MESSAGE,
      };
    }
  }
  return { code: 'internal', message: INTERNAL_MESSAGE };
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) {
    throw normalizeError(null);
  }
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw normalizeError(e);
  }
}

export const getState = () => call<LinkEvent>('link_get_state');
export const submitInvite = (code: string) => call<void>('link_submit_invite', { code });
export const login = (login: string, password: string) =>
  call<void>('link_login', { login, password });
export const retrySso = () => call<void>('link_retry_sso');
export const logout = () => call<void>('link_logout');
export const recheckPin = () => call<void>('link_recheck_pin');
export const retryNow = () => call<void>('link_retry_now');
export const quit = () => call<void>('link_quit');
