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

  let {
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
  bind:value
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
    padding: var(--tr-space-xs) var(--tr-space-md);
    background: var(--tr-surface);
    color: var(--tr-text-primary);
    border: 1px solid var(--tr-border-strong);
    border-radius: var(--tr-radius-sm);
    font-family: var(--tr-font-family);
    font-size: var(--tr-font-size-body);
    line-height: var(--tr-line-height-body);
    resize: vertical;

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
