import '@trackly/design/global.scss';
import { mount } from 'svelte';
import App from './App.svelte';
import { initTheme } from '@trackly/kit/stores/theme.svelte';
import { pushToast } from '@trackly/kit/stores/toast.svelte';
import { setApiHooks } from '@trackly/transport/client';
import { setAttachmentsHost } from '@trackly/attachments/host';
import { authStore } from '$lib/stores/auth.svelte';

// Реакции на 401/403 транспорта живут здесь (пакет transport не знает auth и тосты).
// Регистрация строго до mount: первый же apiCall уже должен их видеть.
setApiHooks({
  onUnauthorized: () => {
    authStore.user = null;
    if (typeof window !== 'undefined') window.location.hash = '#/login';
  },
  onForbidden: () => {
    pushToast('error', 'Недостаточно прав для этого действия');
  },
});

// Права вложений читаются из auth через хост (пакет attachments не знает auth).
// Регистрация строго до mount; чтение authStore.user остаётся реактивным.
setAttachmentsHost({
  currentUser: () => {
    const u = authStore.user;
    return u ? { id: u.id, role: u.role } : null;
  },
});

// Apply theme before mount to avoid a flash — must run before Svelte renders.
initTheme();

const target = document.getElementById('app');
if (!target) {
  throw new Error('Элемент #app не найден в index.html');
}

const app = mount(App, { target });

export default app;
