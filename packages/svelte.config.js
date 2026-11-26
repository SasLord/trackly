// svelte-check ищет конфиг вверх от файла; пакеты используют тот же препроцессор, что и ui
// (иначе fallback на svelte-preprocess без typescript-трансформера — ошибка svelte-check).
export { default } from '../ui/svelte.config.js';
