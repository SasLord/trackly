// Системные уведомления ОС (Phase 41.5, D-G1): единая обёртка для десктопа (Tauri,
// tauri-plugin-notification) и LAN-браузера (web Notification API).
// Ни одна функция не бросает исключение наружу: при любом сбое (нет разрешения,
// Windows 7, плагин не отвечает) возвращается false, тост остаётся единственным каналом.
//
// Политика приватности (T-41.5-85): вызывающие передают ТОЛЬКО обезличенный текст
// «Новое сообщение в заявке №N · Имя», тело сообщения чата сюда не попадает.
// Заголовок и тело не логируются.

const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

let promptArmed = false;

/**
 * Окно приложения в фокусе? document.hidden НЕ использовать: при переключении между
 * приложениями (а не вкладками) он остаётся false.
 */
export function appHasFocus(): boolean {
  return typeof document !== 'undefined' && document.hasFocus();
}

function browserSupported(): boolean {
  return typeof window !== 'undefined' && 'Notification' in window && window.isSecureContext;
}

/** Проверяет разрешение и при необходимости запрашивает его. Вызывать из жеста пользователя. */
export async function ensureNotifyPermission(): Promise<boolean> {
  try {
    if (isTauri) {
      const { isPermissionGranted, requestPermission } =
        await import('@tauri-apps/plugin-notification');
      if (await isPermissionGranted()) return true;
      return (await requestPermission()) === 'granted';
    }
    if (!browserSupported()) return false;
    if (Notification.permission === 'default') {
      return (await Notification.requestPermission()) === 'granted';
    }
    return Notification.permission === 'granted';
  } catch {
    return false;
  }
}

/**
 * Готовит запрос разрешения один раз: в браузере — по первому клику (жест пользователя),
 * в десктопе — сразу. Возвращает функцию снятия слушателя.
 */
export function armPermissionPrompt(): () => void {
  if (promptArmed) return () => {};
  try {
    if (isTauri) {
      promptArmed = true;
      void ensureNotifyPermission();
      return () => {};
    }
    if (!browserSupported() || Notification.permission !== 'default') return () => {};
    promptArmed = true;
    const handler = () => {
      void ensureNotifyPermission();
    };
    document.addEventListener('pointerdown', handler, { capture: true, once: true });
    return () => document.removeEventListener('pointerdown', handler, { capture: true });
  } catch {
    return () => {};
  }
}

/** Показывает системное уведомление, если разрешение уже выдано (без запроса). */
export async function showSystemNotification(title: string, body: string): Promise<boolean> {
  try {
    if (isTauri) {
      const { isPermissionGranted, sendNotification } =
        await import('@tauri-apps/plugin-notification');
      if (!(await isPermissionGranted())) return false;
      sendNotification({ title, body });
      return true;
    }
    if (!browserSupported() || Notification.permission !== 'granted') return false;
    new Notification(title, { body });
    return true;
  } catch {
    return false;
  }
}
