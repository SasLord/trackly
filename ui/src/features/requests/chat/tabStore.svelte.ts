// Активная вкладка карточки заявки («Информация» / «Чат») с запоминанием
// в localStorage (41.5 D-03). Значение недоверенное: whitelist info|chat.

const KEY = 'trackly:request-detail-tab';

export type RequestTab = 'info' | 'chat';

function readStored(): RequestTab {
  try {
    const v = localStorage.getItem(KEY);
    return v === 'chat' ? 'chat' : 'info';
  } catch {
    return 'info';
  }
}

export const tabStore = $state<{ active: RequestTab }>({ active: readStored() });

export function setTab(tab: string): void {
  const next: RequestTab = tab === 'chat' ? 'chat' : 'info';
  tabStore.active = next;
  try {
    localStorage.setItem(KEY, next);
  } catch {
    // хранилище недоступно — вкладка живёт только в памяти
  }
}
