// Фаза 41.10 (D-23, D-20, D-06): единый словарь подписей событий ленты истории.
// Каждый вариант Rust-enum `HistoryEventKind` и `HistoryReason`
// (crates/trackly-core/src/domain/history_events.rs) обязан иметь здесь подпись.
// Паритет держит гейт scripts/check-history-event-labels.mjs (в `pnpm lint`).
//
// Нераспознанный вид -> «событие не определено», нераспознанное основание ->
// «причина не определена»: значение, записанное более новой версией программы,
// никогда не роняет ленту. Сырой токен и сырой код статуса пользователю не показываются.
//
// Модуль исполним после ts.transpileModule без рантайм-импортов (гейт грузит его
// из строки), поэтому тип из bindings — type-only, подписи оснований перемещений
// (movementReason.ts) сюда не импортируются, а запасная подпись статуса приходит
// функцией в контексте. Примеры в комментариях вымышленные.
import type { HistoryEntryDto, HistorySegmentDto } from '../bindings';

export type HistoryEventKindToken =
  | 'group_joined'
  | 'group_left'
  | 'group_transferred'
  | 'group_formed'
  | 'group_disbanded'
  | 'group_nested'
  | 'group_unnested'
  | 'status_changed';

export type HistoryReasonToken =
  | 'manual'
  | 'act'
  | 'repair'
  | 'group_move'
  | 'group_create'
  | 'group_disband'
  | 'device_write_off'
  | 'group_write_off'
  | 'teardown';

export const UNKNOWN_HISTORY_EVENT = 'событие не определено';
// Дубль UNKNOWN_MOVEMENT_REASON из movementReason.ts (рантайм-импорт запрещён, см. шапку);
// равенство строк закрывает гейт check-history-event-labels [H4].
export const UNKNOWN_HISTORY_REASON = 'причина не определена';
export const UNKNOWN_STATUS_LABEL = 'неизвестный статус';

export type HistoryKindIcon = 'move' | 'membership' | 'status' | 'unknown';

export interface HistoryTextCtx {
  /** Лента устройства или лента группы (UI-SPEC §7.2 / §9). */
  variant: 'device' | 'group';
  /** Открытая группа (для варианта group): ссылка на неё не нужна, текст отличается. */
  currentGroupId?: number;
  /** Запасная подпись статуса по коду из стора статусов; undefined — нет такой. */
  statusLabel?: (code: string) => string | undefined;
}

function quoted(label: string | null | undefined): string {
  const s = label?.trim() ?? '';
  return s.length > 0 ? ` «${s}»` : '';
}

function statusName(
  label: string | null | undefined,
  code: string | null | undefined,
  ctx: HistoryTextCtx,
): string {
  const snap = label?.trim() ?? '';
  if (snap.length > 0) return snap;
  const c = code?.trim() ?? '';
  if (c.length > 0) {
    const fb = ctx.statusLabel?.(c)?.trim();
    if (fb) return fb;
  }
  return UNKNOWN_STATUS_LABEL;
}

type KindFn = (seg: HistorySegmentDto, ctx: HistoryTextCtx) => string;

/** Подпись вида события (середина строки), UI-SPEC §7.2 и §9. */
export const HISTORY_KIND_LABELS: Record<HistoryEventKindToken, KindFn> = {
  // «Включено в группу «АРМ-5»» / «Включено устройство «Монитор #2»»
  group_joined: (seg, ctx) =>
    ctx.variant === 'group'
      ? `Включено устройство${quoted(seg.subject_label)}`
      : `Включено в группу${quoted(seg.to_group_label)}`,
  group_left: (seg, ctx) =>
    ctx.variant === 'group'
      ? `Выведено устройство${quoted(seg.subject_label)}`
      : `Выведено из группы${quoted(seg.from_group_label ?? seg.to_group_label)}`,
  group_transferred: (seg, ctx) => {
    if (ctx.variant === 'group') {
      const who = `Устройство${quoted(seg.subject_label)}`;
      const hereIsTarget =
        ctx.currentGroupId !== undefined && seg.to_group_id === ctx.currentGroupId;
      return hereIsTarget
        ? `${who} перемещено сюда из группы${quoted(seg.from_group_label)}`
        : `${who} перемещено в группу${quoted(seg.to_group_label)}`;
    }
    const from = seg.from_group_label?.trim() ?? '';
    const to = seg.to_group_label?.trim() ?? '';
    if (from.length === 0 && to.length === 0) return 'Перемещено между группами';
    return `Перемещено из группы${quoted(from)} в${quoted(to)}`;
  },
  group_formed: (seg, ctx) => {
    if (ctx.variant === 'group') {
      return (seg.subject_label?.trim() ?? '').length > 0
        ? `Группа образована от устройства${quoted(seg.subject_label)}`
        : 'Группа образована';
    }
    return `Образована группа${quoted(seg.to_group_label ?? seg.subject_label)}`;
  },
  group_disbanded: (seg, ctx) =>
    ctx.variant === 'group'
      ? 'Группа расформирована'
      : `Группа${quoted(seg.from_group_label ?? seg.subject_label)} расформирована`,
  group_nested: (seg, ctx) =>
    ctx.variant === 'group' && seg.subject_type === 'group' && seg.subject_id !== ctx.currentGroupId
      ? `Вложена группа${quoted(seg.subject_label)}`
      : `Вложена в группу${quoted(seg.to_group_label)}`,
  group_unnested: (seg, ctx) =>
    ctx.variant === 'group' && seg.subject_type === 'group' && seg.subject_id !== ctx.currentGroupId
      ? `Выведена группа${quoted(seg.subject_label)}`
      : `Выведена из группы${quoted(seg.from_group_label ?? seg.to_group_label)}`,
  // «Статус: В работе → На списание»
  status_changed: (seg, ctx) =>
    `Статус: ${statusName(seg.status_from_label, seg.status_from_code, ctx)} → ${statusName(seg.status_to_label, seg.status_to_code, ctx)}`,
};

type ReasonFn = (entry: HistoryEntryDto) => string;

function refBase(prefix: string, entry: HistoryEntryDto): string {
  return `${prefix}${quoted(entry.ref_group_label)}`;
}

/** Подписи оснований событий (UI-SPEC §7.5). Ссылки на акт/заявку/группу рисует шаблон. */
export const HISTORY_REASON_LABELS: Record<HistoryReasonToken, ReasonFn> = {
  manual: () => 'вручную',
  act: (e) => (e.act_number ? `актом №${e.act_number}` : 'актом'),
  repair: (e) =>
    e.request_id !== null && e.request_id !== undefined
      ? `по ремонтной заявке №${e.request_id}`
      : 'по ремонтной заявке',
  group_move: (e) => refBase('при переносе группы', e),
  group_create: () => 'при создании группы',
  group_disband: () => 'при расформировании группы',
  device_write_off: () => 'при списании',
  group_write_off: (e) => refBase('при списании группы', e),
  teardown: (e) => refBase('при разборе', e),
};

/**
 * Основание записи. Для событий — словарь выше; для чистого перемещения
 * (reason_is_event=false) подпись даёт `movementReason.ts`, её передаёт вызывающий
 * (`fromMovement`), здесь без неё — нейтральная деградация.
 */
export function reasonLabel(
  entry: HistoryEntryDto,
  fromMovement?: (movement: NonNullable<HistorySegmentDto['movement']>) => string,
): string {
  if (entry.reason_is_event) {
    if (Object.hasOwn(HISTORY_REASON_LABELS, entry.reason_source)) {
      return HISTORY_REASON_LABELS[entry.reason_source as HistoryReasonToken](entry);
    }
    return UNKNOWN_HISTORY_REASON;
  }
  const mv = entry.segments.find((s) => s.kind === 'move')?.movement;
  if (mv && fromMovement) return fromMovement(mv);
  return UNKNOWN_HISTORY_REASON;
}

function placeName(path: string | null | undefined): string {
  const s = path?.trim() ?? '';
  return s.length > 0 ? s : 'без места';
}

/** Текст сегмента как первого в строке (с заглавной «Статус:»). */
export function kindText(seg: HistorySegmentDto, ctx: HistoryTextCtx): string {
  if (seg.kind === 'move') {
    const m = seg.movement;
    if (!m) return UNKNOWN_HISTORY_EVENT;
    return `${placeName(m.from_place_path_short ?? m.from_place_path)} → ${placeName(m.to_place_path_short ?? m.to_place_path)}`;
  }
  const k = seg.event_kind;
  if (typeof k === 'string' && Object.hasOwn(HISTORY_KIND_LABELS, k)) {
    return HISTORY_KIND_LABELS[k as HistoryEventKindToken](seg, ctx);
  }
  return UNKNOWN_HISTORY_EVENT;
}

const SEGMENT_ORDER: Record<string, number> = { membership: 0, move: 1, status: 2 };

function orderOf(seg: HistorySegmentDto): number {
  return Object.hasOwn(SEGMENT_ORDER, seg.kind) ? SEGMENT_ORDER[seg.kind] : 3;
}

/** Раскладка сегментов по шаблону состав → место → статус через «; » (UI-SPEC §7.4). */
export function composeEntryText(entry: HistoryEntryDto, ctx: HistoryTextCtx): string {
  const segs = entry.segments
    .map((s, i) => ({ s, i }))
    .sort((a, b) => orderOf(a.s) - orderOf(b.s) || a.i - b.i)
    .map((x) => x.s);
  if (segs.length === 0) return UNKNOWN_HISTORY_EVENT;
  return segs
    .map((seg, idx) => {
      const text = kindText(seg, ctx);
      if (idx === 0) return text;
      if (seg.kind === 'move') return `место: ${text}`;
      if (seg.kind === 'status' && text.startsWith('Статус:')) {
        return `статус:${text.slice('Статус:'.length)}`;
      }
      return text;
    })
    .join('; ');
}

/** Значок строки — вид первого сегмента (после упорядочивания). */
export function kindIconOf(seg: HistorySegmentDto | undefined): HistoryKindIcon {
  if (!seg) return 'unknown';
  if (seg.kind === 'move') return 'move';
  if (seg.kind === 'membership') return 'membership';
  if (seg.kind === 'status') return 'status';
  return 'unknown';
}

/** Значок записи: по первому сегменту в порядке состав → место → статус. */
export function entryIcon(entry: HistoryEntryDto): HistoryKindIcon {
  const first = [...entry.segments].sort((a, b) => orderOf(a) - orderOf(b))[0];
  return kindIconOf(first);
}
