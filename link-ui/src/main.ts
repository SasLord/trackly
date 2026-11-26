import '@trackly/design/global.scss';
import { mount } from 'svelte';
import App from './App.svelte';
import { initTheme } from '@trackly/kit/stores/theme.svelte';

// Тема применяется до mount, чтобы не мигало.
initTheme();

const target = document.getElementById('app');
if (!target) {
  throw new Error('Элемент #app не найден в index.html');
}

const app = mount(App, { target });

export default app;
