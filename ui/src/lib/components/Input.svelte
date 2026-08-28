<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLInputAttributes } from 'svelte/elements';

  interface Props {
    type?: 'text' | 'number' | 'search' | 'password';
    value: string;
    placeholder?: string;
    disabled?: boolean;
    invalid?: boolean;
    id?: string;
    'aria-describedby'?: string;
    autocomplete?: HTMLInputAttributes['autocomplete'];
    oninput?: (_value: string) => void;
    /** Optional left icon; absent by default — no layout change when omitted. */
    iconLeft?: Snippet;
    /** Renders the value in a monospace font (e.g. for values with significant
     * leading/trailing spaces). Default `false` — no visual change for any
     * existing call site that omits this prop. */
    mono?: boolean;
  }

  let {
    type = 'text',
    value = $bindable(''),
    placeholder,
    disabled = false,
    invalid = false,
    id,
    'aria-describedby': ariaDescribedby,
    autocomplete,
    oninput,
    iconLeft,
    mono = false,
  }: Props = $props();
</script>

<div class="input-wrap">
  {#if iconLeft}
    <span class="input-icon" aria-hidden="true">{@render iconLeft()}</span>
  {/if}
  <input
    {type}
    {id}
    {placeholder}
    {disabled}
    class="input"
    class:invalid
    class:mono
    class:has-icon={!!iconLeft}
    {value}
    {autocomplete}
    aria-describedby={ariaDescribedby}
    aria-invalid={invalid || undefined}
    oninput={(e) => {
      const v = (e.currentTarget as HTMLInputElement).value;
      value = v;
      oninput?.(v);
    }}
  />
</div>

<style lang="scss">
  .input-wrap {
    display: block;
    width: 100%;
    position: relative;
  }

  .input-icon {
    position: absolute;
    left: 12px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--tr-text-tertiary);
    pointer-events: none;
    display: flex;
    align-items: center;
  }

  .input {
    display: block;
    width: 100%;
    height: 36px;
    padding: 0 var(--tr-space-md);
    background: var(--tr-surface-raised);
    color: var(--tr-text-primary);
    border: 1px solid var(--tr-border-strong);
    border-radius: var(--tr-radius-sm);
    font-family: var(--tr-font-family);
    font-size: var(--tr-font-size-body);
    line-height: var(--tr-line-height-body);

    &::placeholder {
      color: var(--tr-text-tertiary);
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

    &.mono {
      font-family: var(--tr-font-mono);
    }

    &.has-icon {
      padding-left: 34px;
    }

    &:disabled {
      background: var(--tr-surface-sunken);
      color: var(--tr-text-tertiary);
      cursor: not-allowed;
    }
  }
</style>
