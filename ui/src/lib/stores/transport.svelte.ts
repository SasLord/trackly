// Transport detection — evaluated once at module load time.
// isTauri: true when running inside Tauri webview (desktop app),
// false when served to a LAN browser (Phase 5+ server mode).
export const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
