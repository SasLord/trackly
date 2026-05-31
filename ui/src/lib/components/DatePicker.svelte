<script lang="ts">
  // Phase 3.1 Plan 04 — G-2 shared DatePicker.
  //
  // Thin wrapper над native <input type="date"> с lang="ru" для RU calendar
  // в WebView2/WKWebView. Output value — ISO YYYY-MM-DD string (browser default).
  //
  // Value conversion (YYYY-MM-DD ↔ Unix seconds) — caller's responsibility:
  // используйте `isoToUnix` / `unixToIso` helpers из существующих модулей.
  // DatePicker — pure value-pass-through.

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
</script>

<input
  type="date"
  class="date-picker"
  class:invalid
  lang="ru"
  bind:value
  {id}
  {min}
  {max}
  {disabled}
  {required}
/>

<style lang="scss">
  .date-picker {
    display: block;
    width: 100%;
    height: 36px;
    padding: 0 var(--space-md);
    background: var(--color-bg);
    color: var(--color-text-primary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-family: var(--font-family-base);
    font-size: var(--font-size-body);
    line-height: var(--line-height-body);

    &:focus-visible {
      outline: none;
      border-color: var(--color-accent);
      box-shadow: 0 0 0 3px var(--color-accent-focus);
    }

    &.invalid {
      border-color: var(--color-destructive);
      box-shadow: 0 0 0 3px rgba(220, 38, 38, 0.2);
    }

    &:disabled {
      background: var(--color-surface-muted);
      color: var(--color-text-muted);
      cursor: not-allowed;
    }
  }
</style>
