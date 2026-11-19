// Системные уведомления ОС (Phase 41.5, D-G1): единая обёртка для десктопа (Tauri,
// tauri-plugin-notification) и LAN-браузера (web Notification API).
// Ни одна функция не бросает исключение наружу: при любом сбое (нет разрешения,
// Windows 7, плагин не отвечает) возвращается false, тост остаётся единственным каналом.
//
// Политика приватности (T-41.5-85): вызывающие передают ТОЛЬКО обезличенный текст
// «Новое сообщение в заявке №N · Имя», тело сообщения чата сюда не попадает.
// Заголовок и тело не логируются.

import { pushToast } from '$lib/stores/toast.svelte';

// Исход команды notify_desktop (локальный тип: bindings.ts в рантайме не импортируется).
type NotifyDesktopOutcome = 'shown' | 'suppressed' | 'unavailable';

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

/** Цель перехода по клику; обёртка не знает про заявки. */
export interface NotifyTarget {
  activate: () => void;
  /** Ссылка `#/...` для тоста-предложения (вместо перехода). */
  href: string;
  label: string;
}

const MODAL_SELECTOR = '[role="dialog"][aria-modal="true"]';
const FOCUS_FALLBACK_MS = 15_000;
const MAX_ACTIVATIONS = 20;

function isWindowsDesktop(): boolean {
  return typeof navigator !== 'undefined' && /Windows/.test(navigator.userAgent);
}

/** Тост-предложение: переход не выполняется, ввод пользователя не теряется. */
function offer(target: NotifyTarget, body: string): void {
  pushToast('info', body, { link: { label: target.label, href: target.href } });
}

/** Переход по клику; при открытом модальном окне вместо перехода — тост-предложение. */
function runOrOffer(target: NotifyTarget, body: string): void {
  try {
    if (document.querySelector(MODAL_SELECTOR)) {
      offer(target, body);
      return;
    }
    target.activate();
  } catch {
    /* не бросаем наружу */
  }
}

// Десктоп: токен -> цель. Клик ОС приходит событием trackly-notify-activated (план 28).
const activations = new Map<number, { target: NotifyTarget; body: string }>();
let tokenSeq = 0;
let activationListening = false;

function registerActivation(target: NotifyTarget, body: string): number {
  tokenSeq += 1;
  activations.set(tokenSeq, { target, body });
  while (activations.size > MAX_ACTIVATIONS) {
    const oldest = activations.keys().next().value;
    if (oldest === undefined) break;
    activations.delete(oldest);
  }
  return tokenSeq;
}

async function ensureActivationListener(): Promise<void> {
  if (activationListening) return;
  activationListening = true;
  try {
    const { listen } = await import('@tauri-apps/api/event');
    await listen<number>('trackly-notify-activated', (e) => {
      const rec = activations.get(e.payload);
      activations.delete(e.payload);
      if (rec) runOrOffer(rec.target, rec.body);
    });
  } catch {
    activationListening = false;
  }
}

// Запасной механизм (macOS/Linux, best-effort): плагин 2.4.0 не отдаёт клик на десктопе.
// Если окно получило фокус вскоре после уведомления, ПЕРЕХОД НЕ выполняется (пользователь
// мог вернуться сам) — показывается тост «Открыть заявку №N». На Windows не включать без
// решения пользователя: там свой путь через notify_desktop.
let pendingFocus: { target: NotifyTarget; body: string; at: number } | null = null;
let focusListening = false;

async function armFocusFallback(target: NotifyTarget, body: string): Promise<void> {
  pendingFocus = { target, body, at: Date.now() };
  if (focusListening) return;
  focusListening = true;
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (focused && pendingFocus && Date.now() - pendingFocus.at <= FOCUS_FALLBACK_MS) {
        const p = pendingFocus;
        pendingFocus = null;
        offer(p.target, p.body);
      }
    });
  } catch {
    focusListening = false;
  }
}

/**
 * Показывает системное уведомление, если разрешение уже выдано (без запроса).
 * target: куда вести по клику (браузер — onclick, Windows-десктоп — notify_desktop).
 */
export async function showSystemNotification(
  title: string,
  body: string,
  target?: NotifyTarget,
): Promise<boolean> {
  try {
    if (isTauri) {
      const { isPermissionGranted, sendNotification } =
        await import('@tauri-apps/plugin-notification');
      if (!(await isPermissionGranted())) return false;
      if (target) {
        const token = registerActivation(target, body);
        void ensureActivationListener();
        try {
          // apiCall, а не сгенерированный bindings.ts: на Windows-CI он не создаётся
          // (export_bindings там отключён), а динамический import ломает vite build.
          const { apiCall } = await import('$lib/api/client');
          const outcome = await apiCall<NotifyDesktopOutcome>('notify_desktop', {
            title,
            body,
            token,
          });
          if (outcome === 'shown') return true;
          if (outcome === 'suppressed') {
            // Уведомления отключены в Windows пользователем или политикой: путь плагина не вызывать.
            activations.delete(token);
            return false;
          }
        } catch {
          /* команда недоступна — остаётся путь плагина */
        }
        activations.delete(token);
      }
      sendNotification({ title, body });
      if (target && !isWindowsDesktop()) void armFocusFallback(target, body);
      return true;
    }
    if (!browserSupported() || Notification.permission !== 'granted') return false;
    const n = new Notification(title, { body });
    if (target) {
      n.onclick = () => {
        try {
          window.focus();
        } catch {
          /* не бросаем */
        }
        n.close();
        runOrOffer(target, body);
      };
    }
    return true;
  } catch {
    return false;
  }
}
