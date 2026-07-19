<script lang="ts">
  import type { Snippet } from 'svelte';
  import { sidebarNav, openNav, closeNav } from '../../features/layout/layout-state.svelte';

  interface Props {
    title: string;
    variant?: 'fixed' | 'wrap';
    actions?: Snippet;
  }

  const { title, variant = 'fixed', actions }: Props = $props();

  function toggleNav() {
    if (sidebarNav.open) {
      closeNav();
    } else {
      openNav();
    }
  }
</script>

<header class="page-header page-header--{variant}">
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
  {#if actions}
    <div class="page-header-actions">
      {@render actions()}
    </div>
  {/if}
</header>

<style lang="scss">
  @use '../../styles/_breakpoints' as bp;

  .page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--tr-space-md);
    flex: none;
    border-bottom: 1px solid var(--tr-border);
  }

  .page-header--fixed {
    height: var(--header-height);
    padding: 0 var(--tr-space-xl);
  }

  .page-header--wrap {
    padding: var(--tr-space-md) var(--tr-space-xl);
    flex-wrap: wrap;
  }

  .page-title {
    margin: 0;
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
  }

  .page-header-actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .page-header--wrap .page-header-actions {
    gap: 8px;
    flex-wrap: wrap;
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
