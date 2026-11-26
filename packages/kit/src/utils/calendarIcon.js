// Нужна ли полю <input type="date"> собственная иконка календаря.
//
// Почему именно по User-Agent: у input type=date нет API, сообщающего, нарисовал ли
// движок нативный индикатор. Псевдоэлемент ::-webkit-calendar-picker-indicator есть
// и в Chromium, и в WebKit, а @supports их не различает. WebKit (Tauri WKWebView на
// macOS, десктопный Safari) индикатор не рисует; Chromium/WebView2 и Firefox — рисуют;
// на мобильных платформах нативный контрол выбора даты свой.
//
// При неопределённости (пустой UA) — false: дубль иконки хуже её отсутствия,
// нативный индикатор может быть. UA влияет только на отрисовку, не на данные и права.

const NOT_WEBKIT_DESKTOP =
  /Chrome|Chromium|CriOS|Edg|EdgA|EdgiOS|OPR|Firefox|FxiOS|Android|iPhone|iPad|iPod/;

/**
 * @param {string | undefined | null} userAgent
 * @returns {boolean}
 */
export function needsOwnCalendarIcon(userAgent) {
  if (typeof userAgent !== 'string' || userAgent === '') return false;
  return userAgent.includes('AppleWebKit') && !NOT_WEBKIT_DESKTOP.test(userAgent);
}
