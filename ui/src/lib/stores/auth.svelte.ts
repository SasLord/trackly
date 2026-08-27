// .svelte.ts extension REQUIRED — Svelte 5 runes are only processed in .svelte/.svelte.ts files.

import type { PlacePathDisplay } from '$lib/utils/placePath';

export type UserRole = 'admin' | 'manager' | 'employee';

export interface CurrentUser {
  id: number;
  login: string;
  fullName: string;
  role: UserRole;
}

export const authStore = $state({
  user: null as CurrentUser | null,
  // Вариант сокращения пути места (quick 260827-ui3). Дефолт 'ends' совпадает
  // с бэкенд-дефолтом (PlacePathDisplay::Ends) — до завершения boot-фетча
  // auth_status или при его ошибке рендер уже показывает правильный дефолт,
  // а не пустую ячейку.
  placePathDisplay: 'ends' as PlacePathDisplay,
});

export function isAuthenticated(): boolean {
  return authStore.user !== null;
}
