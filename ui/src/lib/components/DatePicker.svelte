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
    padding: 0 var(--tr-space-md);
    background: var(--tr-bg);
    color: var(--tr-text-primary);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
    font-family: var(--font-family-base);
    font-size: var(--font-size-body);
    line-height: var(--line-height-body);

    &:focus-visible {
      outline: none;
      border-color: var(--tr-accent);
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }

    &.invalid {
      border-color: var(--tr-danger);
      box-shadow: 0 0 0 3px rgba(220, 38, 38, 0.2);
    }

    &:disabled {
      background: var(--tr-surface-sunken);
      color: var(--tr-text-tertiary);
      cursor: not-allowed;
    }
  }
</style>
