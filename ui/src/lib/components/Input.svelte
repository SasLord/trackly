<script lang="ts">
  interface Props {
    type?: 'text' | 'number' | 'search';
    value: string;
    placeholder?: string;
    disabled?: boolean;
    invalid?: boolean;
    id?: string;
    'aria-describedby'?: string;
    oninput?: (_value: string) => void;
  }

  const {
    type = 'text',
    value = $bindable(''),
    placeholder,
    disabled = false,
    invalid = false,
    id,
    'aria-describedby': ariaDescribedby,
    oninput,
  }: Props = $props();
</script>

<input
  {type}
  {id}
  {placeholder}
  {disabled}
  class="input"
  class:invalid
  {value}
  aria-describedby={ariaDescribedby}
  aria-invalid={invalid || undefined}
  oninput={(e) => {
    const v = (e.currentTarget as HTMLInputElement).value;
    oninput?.(v);
  }}
/>

<style lang="scss">
  .input {
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

    &::placeholder {
      color: var(--color-text-muted);
    }

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
      background: var(--color-surface-sunken);
      color: var(--color-text-muted);
      cursor: not-allowed;
    }
  }
</style>
