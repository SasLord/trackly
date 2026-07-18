<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    group?: string | number | null;
    value: string | number;
    disabled?: boolean;
    invalid?: boolean;
    id?: string;
    children?: Snippet;
  }

  let {
    group = $bindable(null),
    value,
    disabled = false,
    invalid = false,
    id,
    children,
  }: Props = $props();
</script>

<label class="check-row" class:disabled>
  <span class="box-wrap">
    <input type="radio" bind:group {value} {disabled} {id} class="native-input" />
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
    border-radius: 50%;

    &::after {
      content: '';
      width: 8px;
      height: 8px;
      border-radius: 50%;
      background: var(--tr-on-accent);
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
