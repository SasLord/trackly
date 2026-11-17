// Тон и иконка блока события по коду действия (UI-SPEC §9.2).
// Чистый TS без импортов и enum — Node импортирует напрямую (check-chat-dates.mjs).

export type EventTone = 'info' | 'success' | 'warning' | 'danger' | 'neutral';
export type IconKind =
  | 'plus'
  | 'play'
  | 'person-plus'
  | 'merge'
  | 'arrow-right'
  | 'wrench'
  | 'return'
  | 'monitor'
  | 'check'
  | 'cross'
  | 'ban'
  | 'paperclip'
  | 'dot';

export interface EventPresentation {
  tone: EventTone;
  icon: IconKind;
  resolution: 'complete' | 'reject' | null;
}

const P = (
  tone: EventTone,
  icon: IconKind,
  resolution: 'complete' | 'reject' | null = null,
): EventPresentation => ({ tone, icon, resolution });

export function presentationFor(action: string): EventPresentation {
  const code = action.startsWith('custom:') ? action.slice('custom:'.length) : action;
  switch (code) {
    case 'create':
      return P('info', 'plus');
    case 'accept':
    case 'repair_accept':
      return P('info', 'play');
    case 'repair_join':
      return P('info', 'person-plus');
    case 'repair_merge':
      return P('info', 'merge');
    case 'repair_merged_into':
      return P('neutral', 'merge');
    case 'repair_move':
      return P('warning', 'arrow-right');
    case 'repair_send':
      return P('warning', 'wrench');
    case 'repair_receive':
      return P('success', 'return');
    case 'repair_assign_device':
      return P('neutral', 'monitor');
    case 'complete':
    case 'repair_complete':
      return P('success', 'check', 'complete');
    case 'reject':
      return P('danger', 'cross', 'reject');
    case 'cancel':
    case 'repair_cancel':
      return P('danger', 'ban');
    default:
      return P('neutral', 'dot');
  }
}
