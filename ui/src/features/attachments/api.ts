// Фаза 41.3, план 12: API-слой вложений.
//
// JSON-команды идут через apiCall (оба транспорта). Байты (загрузка, чтение
// файла) через apiCall НЕ ходят: на HTTP-пути он превращает бинарный ответ в
// number[]. Загрузка в Tauri — raw-invoke `attachments_upload` (команды нет в
// bindings.ts, см. 41.3-10-SUMMARY), в LAN — multipart POST (41.3-11-SUMMARY,
// query-ключи snake_case). Файл и миниатюра — схема `trackly-att` (десктоп) или
// GET /api/v1/attachments/{id}/file|thumb (браузер).

import { apiCall } from '$lib/api/client';
import { parseAppError } from '$lib/api/errors';
import type {
  AttachmentHistoryEventDto,
  AttachmentLimitDto,
  AttachmentListDto,
  IntegrityReportDto,
  AttachmentDto,
} from '../../bindings';

/** Результат загрузки одного файла (зеркало Rust `AttachmentUploadDto`). */
export interface AttachmentUploadDto {
  attachment: AttachmentDto | null;
  rejectCode: string | null;
}

export type AttachmentKind = 'file' | 'thumb';

function inTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

export const attachments = {
  list: (parentKind: string, parentId: number) =>
    apiCall<AttachmentListDto>('attachments_list', { parentKind, parentId }),
  remove: (id: number) => apiCall<null>('attachments_delete', { id }),
  restore: (id: number) => apiCall<null>('attachments_restore', { id }),
  purge: (id: number) => apiCall<null>('attachments_purge', { id }),
  history: (parentKind: string, parentId: number) =>
    apiCall<AttachmentHistoryEventDto[]>('attachments_history', { parentKind, parentId }),
  getLimit: () => apiCall<AttachmentLimitDto>('attachments_get_limit', {}),
  setLimit: (bytes: number) => apiCall<null>('attachments_set_limit', { bytes }),
  report: () => apiCall<IntegrityReportDto | null>('attachments_report', {}),
};

/** URL файла или миниатюры; работает в `<img>`/`<iframe>` обоих режимов. */
export async function attachmentUrl(id: number, kind: AttachmentKind): Promise<string> {
  if (inTauri()) {
    const { convertFileSrc } = await import('@tauri-apps/api/core');
    return convertFileSrc(`${id}/${kind}`, 'trackly-att');
  }
  return `/api/v1/attachments/${id}/${kind}`;
}

/** Ошибка чтения файла с кодом статуса (404 отличается от сетевого сбоя). */
export class AttachmentFetchError extends Error {
  readonly status: number | null;
  constructor(status: number | null, message: string) {
    super(message);
    this.name = 'AttachmentFetchError';
    this.status = status;
  }
}

export async function fetchAttachmentBytes(id: number): Promise<ArrayBuffer> {
  const url = await attachmentUrl(id, 'file');
  let res: globalThis.Response;
  try {
    res = await fetch(url, { credentials: 'same-origin' });
  } catch {
    throw new AttachmentFetchError(null, 'network');
  }
  if (!res.ok) {
    throw new AttachmentFetchError(res.status, `http ${res.status}`);
  }
  return res.arrayBuffer();
}

function rejected(code: string): AttachmentUploadDto {
  return { attachment: null, rejectCode: code };
}

/**
 * Загрузка одного файла. Не бросает: отказ приходит в `rejectCode`
 * (`too_large`, `empty`, `type_not_allowed`, `heic_failed`, `forbidden`,
 * а для транспортных сбоев `network` / `forbidden`).
 */
export async function uploadAttachment(
  parentKind: string,
  parentId: number,
  file: globalThis.File,
): Promise<AttachmentUploadDto> {
  try {
    if (inTauri()) {
      const { invoke } = await import('@tauri-apps/api/core');
      const bytes = new Uint8Array(await file.arrayBuffer());
      return await invoke<AttachmentUploadDto>('attachments_upload', bytes, {
        headers: {
          'x-att-parent-kind': parentKind,
          'x-att-parent-id': String(parentId),
          'x-att-name': encodeURIComponent(file.name),
        },
      });
    }
    const form = new FormData();
    form.append('file', file, file.name);
    const query = new URLSearchParams({
      parent_kind: parentKind,
      parent_id: String(parentId),
    });
    const res = await fetch(`/api/v1/attachments_upload?${query.toString()}`, {
      method: 'POST',
      body: form,
      credentials: 'same-origin',
    });
    if (res.status === 401 || res.status === 403) {
      return rejected('forbidden');
    }
    if (!res.ok) {
      return rejected('network');
    }
    return (await res.json()) as AttachmentUploadDto;
  } catch (e) {
    const err = parseAppError(e);
    if (err.code === 'FORBIDDEN' || err.code === 'Forbidden' || err.code === 'UNAUTHORIZED') {
      return rejected('forbidden');
    }
    return rejected('network');
  }
}
