<script lang="ts">
  import { themeStore, setTheme } from '$lib/stores/theme.svelte';

  const options = [
    { key: 'light' as const, label: 'Светлая', ariaLabel: 'Светлая тема' },
    { key: 'system' as const, label: 'Системная', ariaLabel: 'Использовать системную тему' },
    { key: 'dark' as const, label: 'Тёмная', ariaLabel: 'Тёмная тема' },
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
    padding: 2px;
    gap: 2px;
    background: var(--tr-surface-sunken);
    border: 1px solid var(--tr-border);
    border-radius: 8px;
  }

  .segment {
    flex: 1;
    height: 26px;
    background: transparent;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-family: var(--tr-font-family);
    font-size: 12px;
    font-weight: 600;
    color: var(--tr-text-tertiary);
    box-shadow: none;
    padding: 0 4px;
    transition:
      background 0.12s,
      color 0.12s;

    &:hover:not(.active) {
      background: color-mix(in srgb, var(--tr-text-primary) 5%, transparent);
      color: var(--tr-text-primary);
    }

    &.active {
      color: var(--tr-text-primary);
      background: var(--tr-surface-raised);
      box-shadow: var(--tr-elev-1);
    }

    &:focus-visible {
      outline: none;
      box-shadow: inset 0 0 0 2px var(--tr-focus-ring);
    }
  }
</style>
