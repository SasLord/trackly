<script lang="ts">
  interface Tab {
    key: string;
    label: string;
    count?: number;
    disabled?: boolean;
  }

  interface Props {
    variant?: 'underline' | 'segmented';
    tabs: Tab[];
    active: string;
    onchange?: (_key: string) => void;
    ariaLabel?: string;
  }

  let { variant = 'underline', tabs, active = $bindable(), onchange, ariaLabel }: Props = $props();

  function selectTab(tab: Tab) {
    if (tab.disabled) return;
    active = tab.key;
    onchange?.(tab.key);
  }
</script>

{#snippet tabButtons()}
  {#each tabs as tab (tab.key)}
    <button
      type="button"
      class="tab"
      class:active={tab.key === active}
      disabled={tab.disabled}
      role={variant === 'segmented' ? undefined : 'tab'}
      aria-selected={variant === 'segmented' ? undefined : tab.key === active}
      aria-pressed={variant === 'segmented' ? tab.key === active : undefined}
      onclick={() => selectTab(tab)}
    >
      {tab.label}
      {#if variant === 'underline' && tab.count != null}
        <span class="tab-count">{tab.count}</span>
      {/if}
    </button>
  {/each}
{/snippet}

{#if variant === 'segmented'}
  <div class="tabs tabs-segmented" role="group" aria-label={ariaLabel}>
    {@render tabButtons()}
  </div>
{:else}
  <div class="tabs tabs-underline" role="tablist" aria-label={ariaLabel}>
    {@render tabButtons()}
  </div>
{/if}

<style lang="scss">
  // Underline variant (switch-bar): transcribed from Tabs.dc.html tabStyle(state)
  .tabs-underline {
    display: inline-flex;
    align-items: center;
    gap: 4px;

    .tab {
      display: inline-flex;
      align-items: center;
      gap: 6px;
      height: 34px;
      padding: 0 12px;
      background: transparent;
      border: none;
      border-bottom: 2px solid transparent;
      margin-bottom: -1px;
      font-size: 14px;
      font-weight: 500;
      color: var(--tr-text-secondary);
      cursor: pointer;
      white-space: nowrap;
      border-radius: 6px 6px 0 0;
      outline: none;
      transition:
        background 0.12s,
        box-shadow 0.12s;

      &.active {
        color: var(--tr-accent-text);
        border-bottom-color: var(--tr-accent);
        font-weight: 600;
      }

      &:hover:not(.active):not(:disabled) {
        color: var(--tr-text-primary);
        background: var(--tr-surface-sunken);
      }

      &:focus-visible {
        color: var(--tr-text-primary);
        box-shadow: 0 0 0 3px var(--tr-focus-ring);
      }

      &:disabled {
        color: var(--tr-text-disabled);
        cursor: not-allowed;
      }
    }

    .tab-count {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      min-width: 18px;
      height: 18px;
      padding: 0 5px;
      border-radius: 9px;
      font-size: 11px;
      font-weight: 600;
      line-height: 1;
      background: var(--tr-surface-sunken);
      color: var(--tr-text-secondary);
    }

    .tab.active .tab-count {
      background: var(--tr-accent-soft);
      color: var(--tr-accent-text);
    }

    .tab:disabled .tab-count {
      color: var(--tr-text-disabled);
    }
  }

  // Segmented variant (pill group): transcribed from Tabs.dc.html segStyle(act)
  .tabs-segmented {
    display: inline-flex;
    gap: 3px;
    padding: 3px;
    background: var(--tr-surface-sunken);
    border-radius: 7px;

    .tab {
      display: inline-flex;
      align-items: center;
      height: 28px;
      padding: 0 12px;
      border-radius: 5px;
      font-size: 13px;
      font-weight: 600;
      cursor: pointer;
      background: transparent;
      border: none;
      color: var(--tr-text-secondary);
      transition:
        background 0.12s,
        box-shadow 0.12s;

      &.active {
        background: var(--tr-surface);
        color: var(--tr-accent-text);
        box-shadow: var(--tr-elev-1);
      }

      &:focus-visible {
        box-shadow: 0 0 0 3px var(--tr-focus-ring);
      }
    }
  }
</style>
