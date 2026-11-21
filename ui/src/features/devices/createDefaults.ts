// Последние выбранные «Место» и «Статус» формы СОЗДАНИЯ устройства —
// подставляются по умолчанию при следующем создании (правка не затрагивается).
//
// Хранение — localStorage зрителя: работает одинаково в десктопе (WebView
// Tauri) и в LAN-браузере, ничего не пишет в БД. Ключ разведён по
// пользователю, чтобы общий браузер не подсовывал чужой выбор. Любая ошибка
// хранилища (приватный режим, квота, мусор в значении) молча даёт «нет
// запомненного» — форма никогда не падает из-за удобства.

const KEY_PREFIX = 'trackly:device-create-defaults';

export interface DeviceCreateDefaults {
  placeId: number | null;
  statusId: number | null;
}

const EMPTY: DeviceCreateDefaults = { placeId: null, statusId: null };

function keyFor(userId: number | null | undefined): string {
  return userId == null ? KEY_PREFIX : `${KEY_PREFIX}:${userId}`;
}

function positiveIntOrNull(v: unknown): number | null {
  return typeof v === 'number' && Number.isInteger(v) && v > 0 ? v : null;
}

export function loadDeviceCreateDefaults(userId: number | null | undefined): DeviceCreateDefaults {
  try {
    const raw = localStorage.getItem(keyFor(userId));
    if (!raw) return EMPTY;
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== 'object' || parsed === null) return EMPTY;
    const o = parsed as Record<string, unknown>;
    return { placeId: positiveIntOrNull(o.placeId), statusId: positiveIntOrNull(o.statusId) };
  } catch {
    return EMPTY;
  }
}

export function saveDeviceCreateDefaults(
  userId: number | null | undefined,
  value: DeviceCreateDefaults,
): void {
  try {
    localStorage.setItem(keyFor(userId), JSON.stringify(value));
  } catch {
    // Удобство, не данные: сбой хранилища не мешает сохранению устройства.
  }
}
