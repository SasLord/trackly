import type { UserRole } from '$lib/stores/auth.svelte';

export type SidebarItem = {
  kind: 'item';
  route: string;
  label: string;
  phase?: number | string;
  /** If set, only users with one of these roles see this item. Omit = visible to all. */
  roles?: UserRole[];
};
export type SidebarDivider = { kind: 'divider' };
export type SidebarEntry = SidebarItem | SidebarDivider;

// PINNED: 12 items + 4 dividers = 16 entries — source of truth per UI-SPEC §Copywriting Sidebar.
// Dividers after: Карта (pos 3 — now after Места, per 39-UI-SPEC.md §7), Акты (pos 7),
// Заявки (pos 11), Пользователи (pos 14).
export const SIDEBAR_ITEMS: SidebarEntry[] = [
  { kind: 'item', route: '/', label: 'Дашборд', phase: 7 },
  { kind: 'item', route: '/map', label: 'Карта', phase: 'v2' },
  { kind: 'item', route: '/places', label: 'Места', phase: 39, roles: ['admin', 'manager'] },
  { kind: 'divider' },
  { kind: 'item', route: '/devices', label: 'Устройства' },
  { kind: 'item', route: '/acts', label: 'Акты' },
  { kind: 'divider' },
  { kind: 'item', route: '/printers', label: 'Принтеры', phase: 6 },
  { kind: 'item', route: '/cartridges', label: 'Картриджи' },
  { kind: 'item', route: '/requests', label: 'Заявки', phase: 6 },
  { kind: 'divider' },
  { kind: 'item', route: '/reports', label: 'Отчёты', phase: 7 },
  { kind: 'item', route: '/users', label: 'Пользователи', phase: 5, roles: ['admin'] },
  { kind: 'divider' },
  { kind: 'item', route: '/settings', label: 'Настройки', phase: 7, roles: ['admin'] },
  { kind: 'item', route: '/showcase', label: 'Витрина компонентов', roles: ['admin'] },
];

/**
 * Returns sidebar entries visible to the given role.
 * Items without a `roles` field are visible to everyone.
 * Dividers adjacent to hidden items are preserved (cosmetic—layout handles them).
 */
export function getVisibleItems(role: UserRole | null): SidebarEntry[] {
  return SIDEBAR_ITEMS.filter((entry) => {
    if (entry.kind === 'divider') return true;
    if (!entry.roles) return true;
    if (role === null) return false;
    return entry.roles.includes(role);
  });
}
