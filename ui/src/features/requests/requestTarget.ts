// Цель перехода из уведомления (Phase 41.5, gap R2-1): заявка + вкладка.
//
// Ссылка ВСЕГДА уникальна (параметр n): повторный переход на тот же hash не вызывает
// hashchange в svelte-spa-router, а выбор заявки в списке hash не меняет. Без n клик по
// второму уведомлению или переход на уже выбранную заявку, но другую вкладку, терялся бы.
// RequestsPage параметр n игнорирует.
import { push } from 'svelte-spa-router';

export type RequestTargetTab = 'info' | 'chat';

let seq = 0;

export function requestTargetHref(requestId: number, tab: RequestTargetTab): string {
  seq += 1;
  return `#/requests?id=${Number(requestId)}&tab=${tab}&n=${Date.now().toString(36)}${seq.toString(36)}`;
}

export function openRequestTarget(requestId: number, tab: RequestTargetTab): void {
  void push(requestTargetHref(requestId, tab));
}
