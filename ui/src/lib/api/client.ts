import { parseAppError, type AppError } from './errors';
import { authStore } from '$lib/stores/auth.svelte';
import { pushToast } from '@trackly/kit/stores/toast.svelte';

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

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
      // Tauri errors don't have HTTP status codes; check error code for auth errors.
      if (err && typeof err === 'object' && 'code' in err) {
        const code = (err as { code: string }).code;
        if (code === 'UNAUTHORIZED' || code === 'Unauthorized') {
          authStore.user = null;
          if (typeof window !== 'undefined') window.location.hash = '#/login';
        }
        if (code === 'FORBIDDEN' || code === 'Forbidden') {
          pushToast('error', 'Недостаточно прав для этого действия');
        }
      }
      throw err;
    }
  }
  // Phase 5+ HTTP path.
  const res = await fetch(`/api/v1/${name}`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(args),
  });
  if (!res.ok) {
    const body = await res.json().catch(() => ({}));
    const err = parseAppError(body);
    // 401 → clear auth and redirect to login.
    if (res.status === 401) {
      authStore.user = null;
      if (typeof window !== 'undefined') window.location.hash = '#/login';
    }
    // 403 → toast (D-DENY-01), no redirect/no authStore mutation — still throws so callers can react if needed
    if (res.status === 403) {
      pushToast('error', 'Недостаточно прав для этого действия');
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
