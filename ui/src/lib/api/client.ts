import { parseAppError } from './errors';

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

export async function apiCall<R>(name: string, args: Record<string, unknown> = {}): Promise<R> {
  if (isTauri) {
    const { invoke } = await import('@tauri-apps/api/core');
    try {
      return await invoke<R>(name, args);
    } catch (e) {
      throw parseAppError(e);
    }
  }
  // Phase 5+ HTTP path. Phase 2 stub — never executed in Tauri runtime.
  const res = await fetch(`/api/v1/${name}`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(args),
  });
  if (!res.ok) throw parseAppError(await res.json().catch(() => ({})));
  return res.json();
}
