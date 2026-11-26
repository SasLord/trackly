export interface AppError {
  code: string;
  message: string;
  details?: Record<string, unknown>;
}

/// `AppError.details` shape for `code === 'ACCESS_BLOCKED'` (09-AD-GAPS
/// restoration-flow UX). Mirrors `AppError::AccessBlocked`'s `details_value()`
/// in `crates/trackly-core/src/error.rs`.
export interface AccessBlockedDetails {
  pending: boolean;
  rejection_reason: string | null;
}

/// `AppError.details` для `code === 'ALREADY_IN_GROUP'` (D-16): ВСЕ занятые
/// элементы пакета. Зеркало `OccupiedItem` в `crates/trackly-core/src/error.rs`.
export interface AlreadyInGroupDetails {
  items: {
    kind: 'device' | 'group';
    id: number;
    name: string;
    group_id: number;
    group_name: string;
  }[];
}

/// Отказ «уже состоит в группе» распознаётся только по коду, не по тексту.
export function isAlreadyInGroup(e: unknown): e is AppError & { details: AlreadyInGroupDetails } {
  return (
    e !== null &&
    typeof e === 'object' &&
    (e as Record<string, unknown>).code === 'ALREADY_IN_GROUP' &&
    typeof (e as Record<string, unknown>).details === 'object' &&
    (e as Record<string, unknown>).details !== null
  );
}

export function parseAppError(e: unknown): AppError {
  if (
    e !== null &&
    typeof e === 'object' &&
    'code' in e &&
    'message' in e &&
    typeof (e as Record<string, unknown>).code === 'string' &&
    typeof (e as Record<string, unknown>).message === 'string'
  ) {
    return e as AppError;
  }
  return {
    code: 'UnknownError',
    message: 'Не удалось связаться с приложением. Попробуйте перезапустить.',
  };
}
