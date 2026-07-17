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
    padding: 0 var(--tr-space-md);
    background: var(--tr-bg);
    color: var(--tr-text-primary);
    border: 1px solid var(--tr-border);
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

    &:disabled {
      background: var(--tr-surface-sunken);
      color: var(--tr-text-tertiary);
      cursor: not-allowed;
    }
  }
</style>
