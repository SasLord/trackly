// Структурные DTO вложений. Пакет не импортирует bindings (D-17): типы объявлены
// заново, а их совместимость с bindings проверяет ui/src/packages-compat.ts.

export interface AttachmentDto {
  id: number;
  originalName: string;
  mime: string;
  sizeBytes: number;
  hasThumbnail: boolean;
  viewable: boolean;
  missing: boolean;
  uploadedByName: string | null;
  uploadedById: number | null;
  createdAtUtc: number;
  deletedByName: string | null;
  deletedById: number | null;
  deletedAtUtc: number | null;
  fromMessage: boolean;
}

export interface AttachmentHistoryEventDto {
  attachmentId: number;
  name: string;
  action: string;
  state: string;
  userName: string | null;
  createdAtUtc: number;
}

export interface AttachmentListDto {
  items: AttachmentDto[];
  deleted: AttachmentDto[];
  maxSizeBytes: number;
  canAttach: boolean;
  canDeleteAny: boolean;
  canManageDeleted: boolean;
  readonly: boolean;
}

export interface AttachmentLimitDto {
  maxBytes: number;
  defaultBytes: number;
  ceilingBytes: number;
  path: string;
}

export interface IntegrityReportDto {
  checkedAtUtc: number;
  files: number;
  bytes: number;
  lost: number;
  orphansRemoved: number;
  orphansYoung: number;
  deletedRows: number;
  deletedBytes: number;
  status: string;
}
