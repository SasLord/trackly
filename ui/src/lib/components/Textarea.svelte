<script lang="ts">
  interface Props {
    value: string;
    placeholder?: string;
    disabled?: boolean;
    invalid?: boolean;
    id?: string;
    rows?: number;
    oninput?: (_value: string) => void;
  }

  const {
    value = $bindable(''),
    placeholder,
    disabled = false,
    invalid = false,
    id,
    rows = 3,
    oninput,
  }: Props = $props();
</script>

<textarea
  {id}
  {placeholder}
  {disabled}
  {rows}
  class="textarea"
  class:invalid
  {value}
  aria-invalid={invalid || undefined}
  oninput={(e) => {
    const v = (e.currentTarget as HTMLTextAreaElement).value;
    oninput?.(v);
  }}
></textarea>

<style lang="scss">
  .textarea {
    display: block;
    width: 100%;
    min-height: 80px;
    padding: var(--space-sm) var(--space-md);
    background: var(--color-bg);
    color: var(--color-text-primary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-family: var(--font-family-base);
    font-size: var(--font-size-body);
    line-height: var(--line-height-body);
    resize: vertical;

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
    }

    &:disabled {
      background: var(--color-surface-sunken);
      color: var(--color-text-muted);
      cursor: not-allowed;
    }
  }
</style>
