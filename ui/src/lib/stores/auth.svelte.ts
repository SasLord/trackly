// .svelte.ts extension REQUIRED — Svelte 5 runes are only processed in .svelte/.svelte.ts files.

export type UserRole = 'admin' | 'manager' | 'employee';

export interface CurrentUser {
  id: number;
  login: string;
  fullName: string;
  role: UserRole;
}

export const authStore = $state({
  user: null as CurrentUser | null,
});

export function isAuthenticated(): boolean {
  return authStore.user !== null;
}
