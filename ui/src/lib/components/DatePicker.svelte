<script lang="ts">
  // Phase 3.1 Plan 04 — G-2 shared DatePicker.
  //
  // Thin wrapper над native <input type="date"> с lang="ru" для RU calendar
  // в WebView2/WKWebView. Output value — ISO YYYY-MM-DD string (browser default).
  //
  // Value conversion (YYYY-MM-DD ↔ Unix seconds) — caller's responsibility:
  // используйте `isoToUnix` / `unixToIso` helpers из существующих модулей.
  // DatePicker — pure value-pass-through.
  //
  // WebKit (Tauri WKWebView на macOS, Safari): значение центрируется псевдоэлементом
  // ::-webkit-date-and-time-value (выравниваем влево), а нативного индикатора календаря
  // у input type=date нет — рисуем свою кнопку. Выбор по User-Agent (needsOwnCalendarIcon):
  // API, сообщающего о наличии индикатора, у поля нет. В Chromium/WebView2/Firefox кнопки
  // в DOM нет вовсе (не скрыта CSS) — иначе был бы дубль иконки.

  import { needsOwnCalendarIcon } from '$lib/utils/calendarIcon.js';

  interface Props {
    value: string;
    id?: string;
    min?: string;
    max?: string;
    disabled?: boolean;
    required?: boolean;
    invalid?: boolean;
  }

  let {
    value = $bindable(''),
    id,
    min,
    max,
    disabled = false,
    required = false,
    invalid = false,
  }: Props = $props();

  const ownIcon = needsOwnCalendarIcon(typeof navigator === 'undefined' ? '' : navigator.userAgent);

  let inputEl: HTMLInputElement | undefined = $state();

  function openPicker() {
    if (!inputEl || inputEl.disabled) return;
    if (typeof inputEl.showPicker === 'function') {
      try {
        inputEl.showPicker();
        return;
      } catch {
        // NotAllowedError / SecurityError — фоллбэк ниже
      }
    }
    inputEl.focus();
  }
</script>

<span class="date-picker-field">
  <input
    type="date"
    class="date-picker"
    class:invalid
    class:has-own-icon={ownIcon}
    lang="ru"
    bind:this={inputEl}
    bind:value
    {id}
    {min}
    {max}
    {disabled}
    {required}
  />
  {#if ownIcon && !disabled}
    <button
      type="button"
      class="date-picker-btn"
      aria-label="Выбрать дату"
      tabindex="-1"
      onclick={openPicker}
    >
      <svg
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
      >
        <rect x="3" y="4" width="18" height="18" rx="2" />
        <path d="M16 2v4M8 2v4M3 10h18" />
      </svg>
    </button>
  {/if}
</span>

<style lang="scss">
  .date-picker-field {
    display: block;
    position: relative;
    width: 100%;
  }

  .date-picker-btn {
    position: absolute;
    top: 0;
    bottom: 0;
    right: 6px;
    width: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: none;
    background: none;
    color: var(--tr-text-secondary);
    cursor: pointer;

    &:hover {
      color: var(--tr-text-primary);
    }

    &:focus-visible {
      outline: none;
      color: var(--tr-text-primary);
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
  }

  .date-picker {
    // Вертикальное центрирование даты (WKWebView прижимал её к верху) — колоночным
    // flex БЕЗ align-items. Строчный flex на input (display:flex / align-items:center
    // в колонке) сжимает внутренний контейнер Chromium по содержимому, и иконка
    // календаря встаёт сразу за датой. Держит гейт scripts/check-date-picker-layout.mjs.
    display: flex;
    flex-direction: column;
    justify-content: center;
    width: 100%;
    height: 36px;
    padding: 0 var(--tr-space-md);
    background: var(--tr-surface-raised);
    color: var(--tr-text-primary);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
    font-family: var(--tr-font-family);
    font-size: var(--tr-font-size-body);
    line-height: var(--tr-line-height-body);

    &::-webkit-date-and-time-value {
      text-align: left;
    }

    &.has-own-icon {
      padding-right: 36px;
    }

    &::-webkit-calendar-picker-indicator {
      cursor: pointer;
    }

    &:focus-visible {
      outline: none;
      border-color: var(--tr-accent);
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }

    &.invalid {
      border-color: var(--tr-danger);
      box-shadow: 0 0 0 3px var(--tr-danger-ring);
    }

    &:disabled {
      background: var(--tr-surface-sunken);
      color: var(--tr-text-tertiary);
      cursor: not-allowed;
    }
  }
</style>
