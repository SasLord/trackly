import { parseAppError } from './errors';
import { authStore } from '$lib/stores/auth.svelte';
import { pushToast } from '$lib/stores/toast.svelte';

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

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
  return res.json();
}
