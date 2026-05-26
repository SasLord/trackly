import './styles/global.scss';
import { mount } from 'svelte';
import App from './App.svelte';
import { initTheme } from './lib/stores/theme.svelte';

// Apply theme before mount to avoid a flash — must run before Svelte renders.
initTheme();

const target = document.getElementById('app');
if (!target) {
  throw new Error('Элемент #app не найден в index.html');
}

const app = mount(App, { target });

export default app;
