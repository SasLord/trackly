<script lang="ts">
  import type { Snippet } from 'svelte';
  import Spinner from './Spinner.svelte';

  interface Props {
    variant?: 'primary' | 'secondary' | 'destructive' | 'ghost' | 'link';
    size?: 'sm' | 'md';
    loading?: boolean;
    disabled?: boolean;
    type?: 'button' | 'submit';
    /** Квадратная кнопка-иконка (width = высота варианта, padding: 0) —
     *  UI-SPEC §1 "Кнопка-иконка". Без этого пропа разметка/стили не
     *  меняются — существующие вызовы Button не передают iconOnly. */
    iconOnly?: boolean;
    /** Обязателен вместе с iconOnly (иконка без текста — UI-SPEC §8). */
    ariaLabel?: string;
    title?: string;
    onclick?: () => void;
    children?: Snippet;
  }

  const {
    variant = 'primary',
    size = 'md',
    loading = false,
    disabled = false,
    type = 'button',
    iconOnly = false,
    ariaLabel,
    title,
    onclick,
    children,
  }: Props = $props();

  const isDisabled = $derived(disabled || loading);
  // FE-WR-14 (UI-SPEC §10 вариант б): ghost-кнопка-иконка берёт геометрию,
  // цвет и состояния ТОЛЬКО из общего `.tr-btn-ghost-icon` (global.scss), как
  // `ActionMenu variant="ghost-md"`. Scoped `.btn-ghost.svelte-x` специфичнее
  // глобального класса и перебивал бы рамку/кольцо фокуса/цвет disabled,
  // поэтому для этой комбинации `btn-ghost` не ставится.
  const ghostIcon = $derived(variant === 'ghost' && iconOnly);
</script>

<button
  {type}
  class="btn btn-{size}"
  class:btn-primary={variant === 'primary'}
  class:btn-secondary={variant === 'secondary'}
  class:btn-destructive={variant === 'destructive'}
  class:btn-ghost={variant === 'ghost' && !ghostIcon}
  class:btn-link={variant === 'link'}
  class:loading
  class:btn-icon-only={iconOnly}
  class:tr-btn-ghost-icon={ghostIcon}
  disabled={isDisabled}
  aria-label={ariaLabel}
  {title}
  {onclick}
>
  {#if loading}
    <Spinner size="sm" />
  {/if}
  {@render children?.()}
</button>

<style lang="scss">
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--tr-space-2xs);
    border: 1px solid transparent;
    border-radius: var(--tr-radius-sm);
    font-family: var(--tr-font-family);
    font-weight: var(--tr-font-weight-semibold);
    cursor: pointer;
    transition:
      background 0.12s,
      box-shadow 0.12s;
    white-space: nowrap;
    text-decoration: none;

    &:disabled {
      opacity: 0.45;
      cursor: not-allowed;
      pointer-events: none;
    }

    &.loading {
      opacity: 0.85;
      cursor: default;
      pointer-events: none;
    }
  }

  // Sizes
  .btn-md {
    height: 36px;
    padding: 0 var(--tr-space-md);
    font-size: var(--tr-font-size-body);
  }

  .btn-sm {
    height: 28px;
    padding: 0 12px;
    font-size: var(--tr-font-size-label);
  }

  // Icon-only: квадрат (width = height варианта), без внутреннего отступа
  // (UI-SPEC §1 "Кнопка-иконка"). Условный класс — без него разметка кнопок
  // не меняется.
  .btn-icon-only {
    padding: 0;
  }
  .btn-md.btn-icon-only {
    width: 36px;
  }
  .btn-sm.btn-icon-only {
    width: 28px;
  }
  // FE-WR-14: у `.btn` своя `border: 1px solid transparent` (scoped, выше по
  // специфичности, чем глобальный класс) — снимаем, как у ActionMenu ghost-md
  // (`border: none`), остальное приходит из `.tr-btn-ghost-icon`.
  .btn.tr-btn-ghost-icon {
    border: none;
  }

  // Variants
  .btn-primary {
    background: var(--tr-accent);
    color: var(--tr-on-accent);
    border-color: var(--tr-accent);

    &:hover:not(:disabled) {
      background: var(--tr-accent-hover);
    }
    &:focus-visible {
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
    &:active:not(:disabled) {
      background: var(--tr-accent-active);
      border-color: var(--tr-accent-active);
    }
  }

  .btn-secondary {
    background: var(--tr-surface);
    color: var(--tr-text-primary);
    border: 1px solid var(--tr-border-strong);

    &:hover:not(:disabled) {
      background: var(--tr-surface-sunken);
    }
    &:focus-visible {
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
      border-color: var(--tr-accent);
    }
    &:active:not(:disabled) {
      background: var(--tr-surface-sunken);
      border-color: var(--tr-text-tertiary);
    }
  }

  .btn-destructive {
    background: var(--tr-danger);
    color: var(--tr-on-accent);
    border-color: var(--tr-danger);

    &:hover:not(:disabled) {
      background: var(--tr-danger-hover);
      border-color: var(--tr-danger-hover);
    }
    &:focus-visible {
      box-shadow: 0 0 0 3px var(--tr-danger-ring);
    }
    &:active:not(:disabled) {
      background: var(--tr-danger-active);
      border-color: var(--tr-danger-active);
    }
  }

  .btn-ghost {
    background: transparent;
    color: var(--tr-text-primary);

    &:hover:not(:disabled) {
      background: var(--tr-surface-sunken);
    }
    &:focus-visible {
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
      border-color: var(--tr-accent);
    }
    &:active:not(:disabled) {
      background: var(--tr-surface-sunken);
      border-color: var(--tr-border);
    }
    &:disabled {
      color: var(--tr-text-disabled);
    }
  }

  .btn-link {
    background: transparent;
    color: var(--tr-accent);
    padding: 2px 2px;
    height: auto;
    text-decoration: underline;
    text-underline-offset: 2px;

    &:hover:not(:disabled) {
      color: var(--tr-accent-hover);
    }
    &:focus-visible {
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
      border-radius: 4px;
      text-decoration: none;
    }
    &:active:not(:disabled) {
      color: var(--tr-accent-active);
    }
    &:disabled {
      color: var(--tr-text-disabled);
      text-decoration: none;
    }
    &.loading {
      color: var(--tr-text-tertiary);
      text-decoration: none;
    }
  }
</style>
