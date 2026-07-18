<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    checked?: boolean;
    disabled?: boolean;
    invalid?: boolean;
    id?: string;
    onchange?: (_checked: boolean) => void;
    children?: Snippet;
  }

  let {
    checked = $bindable(false),
    disabled = false,
    invalid = false,
    id,
    onchange,
    children,
  }: Props = $props();
</script>

<label class="check-row" class:disabled>
  <span class="box-wrap">
    <input
      type="checkbox"
      bind:checked
      {disabled}
      {id}
      class="native-input"
      onchange={() => onchange?.(checked)}
    />
    <span class="box" class:invalid aria-hidden="true"></span>
  </span>
  {@render children?.()}
</label>

<style lang="scss">
  .check-row {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
    cursor: pointer;

    &.disabled {
      color: var(--tr-text-disabled);
      cursor: not-allowed;
    }
  }

  .box-wrap {
    position: relative;
    display: inline-flex;
    flex: none;
  }

  .native-input {
    position: absolute;
    inset: 0;
    width: 18px;
    height: 18px;
    margin: 0;
    opacity: 0;
    cursor: pointer;

    &:disabled {
      cursor: not-allowed;
    }
  }

  .box {
    width: 18px;
    height: 18px;
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1.5px solid var(--tr-border-strong);
    background: var(--tr-surface);
    box-sizing: border-box;
    border-radius: 5px;

    &::after {
      content: '';
      width: 10px;
      height: 6px;
      border-left: 2px solid var(--tr-on-accent);
      border-bottom: 2px solid var(--tr-on-accent);
      transform: rotate(-45deg) translate(0, -1px);
      opacity: 0;
    }

    &.invalid {
      border-color: var(--tr-danger);
      box-shadow: 0 0 0 3px var(--tr-danger-ring);
    }
  }

  .native-input:checked ~ .box {
    background: var(--tr-accent);
    border-color: var(--tr-accent);

    &::after {
      opacity: 1;
    }
  }

  .native-input:focus-visible ~ .box {
    box-shadow: 0 0 0 3px var(--tr-focus-ring);
    border-color: var(--tr-accent);
  }

  .native-input:disabled ~ .box {
    background: var(--tr-surface-sunken);
    border-color: var(--tr-border);
  }
</style>
