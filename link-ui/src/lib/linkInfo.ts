import { invoke } from '@tauri-apps/api/core';

export interface LinkInfo {
  version: string;
  webview: string | null;
  install_path: string;
}

/** Данные служебной строки из команды `link_info`; вне Tauri — null. */
export async function loadLinkInfo(): Promise<LinkInfo | null> {
  if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) return null;
  try {
    return await invoke<LinkInfo>('link_info');
  } catch {
    return null;
  }
}

/** Строка для пилота: «Link {версия} · WebView2 {версия} · {путь}». */
export function formatStatusLine(info: LinkInfo | null): string {
  const version = info?.version || '—';
  const webview = info?.webview || '—';
  const path = info?.install_path || '—';
  return `Link ${version} · WebView2 ${webview} · ${path}`;
}
