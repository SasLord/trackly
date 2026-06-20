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
