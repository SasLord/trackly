// Флаг «показывать историю» в попапе просмотра (41.10 D-17): один ключ на все
// виды, только localStorage, на сервер не уходит. Значение недоверенное:
// включает только строка '1'. Решение про узкий экран — за попапом (D-18).

const KEY = 'trackly:view-history';

function readStored(): boolean {
  try {
    return localStorage.getItem(KEY) === '1';
  } catch {
    return false;
  }
}

let enabled = $state(readStored());

export const historyPanel = {
  get enabled(): boolean {
    return enabled;
  },
  set(value: boolean): void {
    enabled = value;
    try {
      localStorage.setItem(KEY, value ? '1' : '0');
    } catch {
      // хранилище недоступно — флаг живёт только в памяти
    }
  },
};
