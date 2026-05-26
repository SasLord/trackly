export interface AppError {
  code: string;
  message: string;
  details?: Record<string, unknown>;
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
