/**
 * Deliver in-memory bytes to the user as a saved file, in both webviews the
 * app runs in:
 *
 * - Tauri desktop webview: native save dialog (`@tauri-apps/plugin-dialog`)
 *   + `writeFile` (`@tauri-apps/plugin-fs`).
 * - LAN browser (server mode): Blob + detached `<a download>` click, with
 *   the anchor appended to the DOM before `click()` (required for the click
 *   to actually fire in some browsers) and `revokeObjectURL` deferred via
 *   `setTimeout` (revoking synchronously races the download start).
 *
 * Pattern copied from the existing `isTauri` + dynamic-import guard used in
 * `StorageSettings.svelte` / `OrgSettings.svelte` — not re-exported from
 * `$lib/api/client` because it isn't exported there (this is the third
 * independent copy of the same guard in the codebase).
 */

export type SaveFileResult = 'saved' | 'cancelled';

export async function saveFile(
  bytes: Uint8Array,
  suggestedName: string,
  mimeType: string,
): Promise<SaveFileResult> {
  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  if (isTauri) {
    const extension = suggestedName.includes('.') ? suggestedName.split('.').pop()! : '';

    const { save } = await import('@tauri-apps/plugin-dialog');
    const path = await save({
      defaultPath: suggestedName,
      filters: extension ? [{ name: extension.toUpperCase(), extensions: [extension] }] : [],
    });

    if (!path) {
      return 'cancelled';
    }

    const { writeFile } = await import('@tauri-apps/plugin-fs');
    await writeFile(path, bytes);
    return 'saved';
  }

  const arrayBuffer = bytes.buffer.slice(
    bytes.byteOffset,
    bytes.byteOffset + bytes.byteLength,
  ) as ArrayBuffer;
  const blob = new Blob([arrayBuffer], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = suggestedName;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
  return 'saved';
}
