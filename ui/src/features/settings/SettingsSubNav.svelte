<script lang="ts">
  // Plan 07-11 Task 1: Settings sub-section switch-bar (GAP-S2).
  // Splits the Settings page into per-subsection views matching the component layout.

  const SECTIONS = [
    { key: 'network', label: 'Сеть' },
    { key: 'org', label: 'Организация' },
    { key: 'storage', label: 'Хранилище' },
    { key: 'backup', label: 'Бэкапы' },
    { key: 'threshold', label: 'Порог остатка' },
    { key: 'templates', label: 'Шаблоны' },
  ] as const;

  interface Props {
    activeSection: string;
    onSectionChange: (_s: string) => void;
  }

  const { activeSection, onSectionChange }: Props = $props();
</script>

<div class="settings-sub-nav" role="tablist" aria-label="Раздел настроек">
  {#each SECTIONS as section}
    <button
      class="tab"
      class:active={section.key === activeSection}
      type="button"
      role="tab"
      aria-selected={section.key === activeSection}
      onclick={() => onSectionChange(section.key)}
    >
      {section.label}
    </button>
  {/each}
</div>

<style lang="scss">
  .settings-sub-nav {
    display: flex;
    gap: var(--space-xs);
    flex-wrap: wrap;
    flex-shrink: 0;
  }

  .tab {
    display: inline-flex;
    align-items: center;
    padding: var(--space-xs) var(--space-md);
    background: transparent;
    color: var(--color-text-primary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-family: var(--font-family-base);
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-medium);
    cursor: pointer;
    height: 32px;
    white-space: nowrap;

    &:hover {
      background: var(--color-surface-sunken);
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--color-accent-focus);
    }

    &.active {
      background: color-mix(in srgb, var(--color-accent) 10%, transparent);
      border-color: var(--color-accent);
      color: var(--color-text-primary);
    }
  }
</style>
