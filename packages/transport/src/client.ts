import { parseAppError, type AppError } from './errors';
import { isTauri } from './transport.svelte';

/**
 * Реакции на отказы авторизации внедряются приложением (транспорт не знает ни
 * auth-стора, ни тостов). Без зарегистрированных хуков ничего не происходит.
 * Trackly регистрирует их в `ui/src/main.ts` до `mount`.
 */
export interface ApiHooks {
  /** 401 / UNAUTHORIZED: сессии нет или она истекла. */
  onUnauthorized?: (err: AppError) => void;
  /** 403 / FORBIDDEN: действие запрещено ролью. */
  onForbidden?: (err: AppError) => void;
}

let hooks: ApiHooks = {};

export function setApiHooks(next: ApiHooks): void {
  hooks = next;
}

/**
 * Команды, чьи HTTP-обработчики ЗАКОННО отвечают `text/html` (печатные формы
 * и предпросмотр шаблона). Перечислены от сервера: text/html-обработчики в
 * crates/trackly-app/src/http/{acts,repairs,reports,settings_org}.rs и их
 * `.route("/api/v1/<name>", …)`. Паритет держит гейт
 * `scripts/check-html-commands.mjs` (quick 261121-heg, INT-35).
 */
export const HTML_COMMANDS: ReadonlySet<string> = new Set([
  'acts_render_pdf',
  'devices_render_acceptance_pdf',
  'repairs_render_act',
  'repairs_render_device_report',
  'reports_export_pdf',
  'templates_validate_preview',
]);

export async function apiCall<R>(name: string, args: Record<string, unknown> = {}): Promise<R> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    try {
      return await invoke<R>(name, args);
    } catch (e) {
      const err = parseAppError(e);
      // У Tauri-ошибок нет HTTP-статуса: отказ авторизации определяется по коду
      // `AppError`. Сравнение нормализовано (регистр не важен), чтобы расхождение
      // `UNAUTHORIZED` / `Unauthorized` не отключало хуки молча (WR-03). Звено
      // «код из Rust → строка» держит гейт check-transport-tauri (сверка с error.rs).
      const code = err.code.toUpperCase();
      if (code === 'UNAUTHORIZED') {
        hooks.onUnauthorized?.(err);
      } else if (code === 'FORBIDDEN') {
        hooks.onForbidden?.(err);
      }
      throw err;
    }
  }
  // Phase 5+ HTTP path.
  let res: Awaited<ReturnType<typeof fetch>>;
  try {
    res = await fetch(`/api/v1/${name}`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(args),
    });
  } catch {
    // Отказ fetch (нет связи, обрыв): единственный повторяемый «сетевой» код (D-12).
    const netErr: AppError = { code: 'NETWORK_ERROR', message: 'Нет связи с сервером' };
    throw netErr;
  }
  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    let err = parseAppError(body);
    if (err.code === 'UnknownError') {
      // Тело не формы AppError (прокси, не-JSON): синтетическая ошибка с кодом статуса.
      // Тело не включается в message (T-43-15-02).
      err = {
        code: `HTTP_${res.status}`,
        message: `Сервер вернул ошибку (HTTP ${res.status})`,
        details: { status: res.status },
      };
    }
    // 401 → хук приложения (сброс пользователя, переход на вход).
    if (res.status === 401) {
      hooks.onUnauthorized?.(err);
    }
    // 403 → хук приложения (тост, D-DENY-01); ошибка всё равно бросается.
    if (res.status === 403) {
      hooks.onForbidden?.(err);
    }
    throw err;
  }
  // Binary responses (e.g. `application/pdf` from reports_export_pdf — the
  // remaining krilla-based PDF export, frozen but still live) must NOT be
  // parsed as JSON. The Tauri path returns `Vec<u8>` as `number[]`; mirror
  // that shape here so callers get the same `number[]` regardless of
  // transport.
  //
  // Phase 16 (D-09/D-10): acts_render_pdf/devices_render_acceptance_pdf now
  // respond `text/html; charset=utf-8` carrying a plain HTML string (not
  // bytes) over the HTTP transport, mirroring the Tauri path's `String`
  // return. Route `text/html` through as plain text, not the binary
  // Uint8Array conversion below.
  //
  // INT-35: text/html на команду ВНЕ HTML_COMMANDS — это не ответ, а
  // отсутствующий маршрут: SPA-fallback отдаёт index.html с 200, и прежнее
  // `as R` выдавало страницу за данные (серверная линия защиты — F17: /api/*
  // без маршрута отвечает JSON 404). Бросаем ту же ошибку, что сервер
  // отдал бы на AppError::NotFound.
  const contentType = res.headers.get('content-type') ?? '';
  if (contentType.includes('text/html')) {
    if (HTML_COMMANDS.has(name)) {
      return (await res.text()) as R;
    }
    const err: AppError = { code: 'NOT_FOUND', message: `Маршрут не найден: ${name}` };
    throw err;
  }
  if (!contentType.includes('application/json')) {
    const buf = new Uint8Array(await res.arrayBuffer());
    return Array.from(buf) as R;
  }
  return res.json();
}
