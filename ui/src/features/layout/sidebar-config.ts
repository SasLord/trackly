export type SidebarItem = { kind: 'item'; route: string; label: string; phase?: number | string };
export type SidebarDivider = { kind: 'divider' };
export type SidebarEntry = SidebarItem | SidebarDivider;

// PINNED: 10 items + 4 dividers = 14 entries — source of truth per UI-SPEC §Copywriting Sidebar.
// Dividers after: Карта (pos 3), Акты (pos 6), Заявки (pos 10), Пользователи (pos 13).
export const SIDEBAR_ITEMS: SidebarEntry[] = [
  { kind: 'item', route: '/', label: 'Дашборд', phase: 7 },
  { kind: 'item', route: '/map', label: 'Карта', phase: 'v2' },
  { kind: 'divider' },
  { kind: 'item', route: '/devices', label: 'Устройства' },
  { kind: 'item', route: '/acts', label: 'Акты' },
  { kind: 'divider' },
  { kind: 'item', route: '/printers', label: 'Принтеры', phase: 6 },
  { kind: 'item', route: '/cartridges', label: 'Картриджи' },
  { kind: 'item', route: '/requests', label: 'Заявки', phase: 6 },
  { kind: 'divider' },
  { kind: 'item', route: '/reports', label: 'Отчёты', phase: 7 },
  { kind: 'item', route: '/users', label: 'Пользователи', phase: 5 },
  { kind: 'divider' },
  { kind: 'item', route: '/settings', label: 'Настройки', phase: 7 },
];
