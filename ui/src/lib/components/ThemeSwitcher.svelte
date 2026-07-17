<script lang="ts">
  import { themeStore, setTheme } from '$lib/stores/theme.svelte';

  const options = [
    { key: 'light' as const, label: 'Светлая', ariaLabel: 'Светлая тема' },
    { key: 'dark' as const, label: 'Тёмная', ariaLabel: 'Тёмная тема' },
    { key: 'system' as const, label: 'Системная', ariaLabel: 'Использовать системную тему' },
  ];
</script>

<div class="theme-switcher" role="group" aria-label="Переключение темы">
  {#each options as opt}
    <button
      type="button"
      class="segment"
      class:active={themeStore.preference === opt.key}
      aria-label={opt.ariaLabel}
      aria-pressed={themeStore.preference === opt.key}
      onclick={() => setTheme(opt.key)}
    >
      {opt.label}
    </button>
  {/each}
</div>

<style lang="scss">
  .theme-switcher {
    display: flex;
    width: 100%;
    height: 32px;
    background: var(--tr-surface);
    border: 1px solid var(--tr-border);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .segment {
    flex: 1;
    height: 100%;
    background: transparent;
    border: none;
    border-right: 1px solid var(--tr-border);
    cursor: pointer;
    font-family: var(--font-family-base);
    font-size: var(--font-size-label);
    color: var(--tr-text-secondary);
    padding: 0;
    transition: none;

    &:last-child {
      border-right: none;
    }

    &:hover:not(.active) {
      background: color-mix(in srgb, var(--tr-text-primary) 5%, transparent);
      color: var(--tr-text-primary);
    }

    &.active {
      background: var(--tr-surface-raised);
      color: var(--tr-text-primary);
      font-weight: var(--font-weight-medium);
    }

    &:focus-visible {
      outline: none;
      box-shadow: inset 0 0 0 2px var(--tr-focus-ring);
    }
  }
</style>
