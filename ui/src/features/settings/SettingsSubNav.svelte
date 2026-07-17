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
    { key: 'ad', label: 'Active Directory' },
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
    gap: var(--tr-space-2xs);
    flex-wrap: wrap;
    flex-shrink: 0;
  }

  .tab {
    display: inline-flex;
    align-items: center;
    padding: var(--tr-space-2xs) var(--tr-space-md);
    background: transparent;
    color: var(--tr-text-primary);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
    font-family: var(--tr-font-family);
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-medium);
    cursor: pointer;
    height: 32px;
    white-space: nowrap;

    &:hover {
      background: var(--tr-surface-sunken);
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }

    &.active {
      background: color-mix(in srgb, var(--tr-accent) 10%, transparent);
      border-color: var(--tr-accent);
      color: var(--tr-text-primary);
    }
  }
</style>
