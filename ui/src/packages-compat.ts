// Проверка дрейфа типов между bindings и структурными копиями в packages/*.
// Пакеты не импортируют bindings (D-17), поэтому их DTO объявлены заново; здесь
// взаимная присваиваемость. Если поле разошлось, `svelte-check` краснеет.
import type { WsEvent as BindingsWsEvent } from './bindings-phase6';
import type { WsEvent as PackageWsEvent } from '@trackly/transport/ws';

type Assert<T extends true> = T;
type Mutual<A, B> = [A] extends [B] ? ([B] extends [A] ? true : false) : false;

export type WsEventCompat = Assert<Mutual<BindingsWsEvent, PackageWsEvent>>;

// Вложения (план 17): DTO bindings <-> структурные типы packages/attachments.
import type {
  AttachmentDto as BAttachmentDto,
  AttachmentHistoryEventDto as BAttachmentHistoryEventDto,
  AttachmentLimitDto as BAttachmentLimitDto,
  AttachmentListDto as BAttachmentListDto,
  IntegrityReportDto as BIntegrityReportDto,
} from './bindings';
import type {
  AttachmentDto as PAttachmentDto,
  AttachmentHistoryEventDto as PAttachmentHistoryEventDto,
  AttachmentLimitDto as PAttachmentLimitDto,
  AttachmentListDto as PAttachmentListDto,
  IntegrityReportDto as PIntegrityReportDto,
} from '@trackly/attachments/types';

export type AttachmentDtoCompat = Assert<Mutual<BAttachmentDto, PAttachmentDto>>;
export type AttachmentListDtoCompat = Assert<Mutual<BAttachmentListDto, PAttachmentListDto>>;
export type AttachmentHistoryEventDtoCompat = Assert<
  Mutual<BAttachmentHistoryEventDto, PAttachmentHistoryEventDto>
>;
export type AttachmentLimitDtoCompat = Assert<Mutual<BAttachmentLimitDto, PAttachmentLimitDto>>;
export type IntegrityReportDtoCompat = Assert<Mutual<BIntegrityReportDto, PIntegrityReportDto>>;
