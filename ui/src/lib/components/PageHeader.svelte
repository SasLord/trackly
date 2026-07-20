<script lang="ts">
  import type { Snippet } from 'svelte';
  import { sidebarNav, openNav, closeNav } from '../../features/layout/layout-state.svelte';

  interface Props {
    title: string;
    actions?: Snippet;
  }

  const { title, actions }: Props = $props();

  function toggleNav() {
    if (sidebarNav.open) {
      closeNav();
    } else {
      openNav();
    }
  }
</script>

<header class="page-header">
  <div class="page-header-left">
    <button
      type="button"
      class="nav-toggle"
      aria-expanded={sidebarNav.open}
      aria-controls="app-sidebar"
      aria-label={sidebarNav.open ? 'Закрыть меню' : 'Открыть меню'}
      onclick={toggleNav}
    >
      <svg width="18" height="18" viewBox="0 0 18 18" fill="none" aria-hidden="true">
        <path
          d="M2 5H16M2 9H16M2 13H16"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
        />
      </svg>
    </button>
    <h1 class="page-title">{title}</h1>
  </div>
  {#if actions}
    <div class="page-header-actions">
      {@render actions()}
    </div>
  {/if}
</header>

<style lang="scss">
  @use '../../styles/_breakpoints' as bp;

  .page-header {
    position: sticky;
    top: 0;
    z-index: 20;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--tr-space-md);
    height: var(--header-height);
    padding: 0 24px;
    flex: none;
    background: var(--tr-surface);
    border-bottom: 1px solid var(--tr-border);
  }

  .page-header-left {
    display: flex;
    align-items: center;
    gap: var(--tr-space-sm);
    min-width: 0;
  }

  .page-title {
    margin: 0;
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .page-header-actions {
    display: flex;
    align-items: center;
    gap: 10px;
    flex: none;
  }

  .nav-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    flex-shrink: 0;
    border-radius: var(--tr-radius-sm);
    background: transparent;
    border: none;
    color: var(--tr-text-secondary);
    cursor: pointer;

    &:hover {
      background: var(--tr-row-hover);
    }
    &:focus-visible {
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
      outline: none;
    }

    @media (min-width: bp.$bp-lg) {
      display: none;
    }
  }
</style>
