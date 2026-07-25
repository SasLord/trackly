<script lang="ts">
  import type { Snippet } from 'svelte';
  import { router } from 'svelte-spa-router';
  import Sidebar from './Sidebar.svelte';
  import { sidebarNav, closeNav } from './layout-state.svelte';

  interface Props {
    children?: Snippet;
  }

  const { children }: Props = $props();

  // Reactive desktop/mobile flag — the ONE intentional matchMedia exception in this plan:
  // `inert` is a DOM attribute, not stylable via pure CSS (UI-SPEC §6.1).
  let isDesktop = $state(true);
  let asideEl = $state<HTMLElement | null>(null);
  let prevFocus: HTMLElement | null = null;

  $effect(() => {
    if (typeof window === 'undefined') return;
    const mql = window.matchMedia('(min-width: 1024px)');
    isDesktop = mql.matches;

    function handleChange(e: MediaQueryListEvent) {
      isDesktop = e.matches;
      // Crossing back to desktop while the drawer was open — close it so state
      // does not desync across a resize.
      if (e.matches && sidebarNav.open) {
        closeNav();
      }
    }

    mql.addEventListener('change', handleChange);
    return () => mql.removeEventListener('change', handleChange);
  });

  // Auto-close on route change — covers both "click any nav link" (a nav-link click
  // always triggers a route change via use:link) and direct hash navigation.
  $effect(() => {
    void router.location;
    closeNav();
  });

  // Focus management — mirrors Modal.svelte's focus-trap-entry pattern: capture the
  // previously-focused element, move focus into the drawer, restore on close.
  $effect(() => {
    if (!sidebarNav.open || isDesktop) return;

    prevFocus = document.activeElement as HTMLElement | null;
    const focusable = asideEl
      ? Array.from(asideEl.querySelectorAll<HTMLElement>('a[href], button:not([disabled])')).filter(
          (n) => n.offsetParent !== null,
        )
      : [];
    (focusable[0] ?? asideEl)?.focus();

    return () => {
      prevFocus?.focus();
    };
  });

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      closeNav();
    }
  }

  let mouseDownOnBackdrop = $state(false);

  function handleBackdropMousedown(e: MouseEvent) {
    mouseDownOnBackdrop = e.target === e.currentTarget;
  }

  function handleBackdropMouseup(e: MouseEvent) {
    if (mouseDownOnBackdrop && e.target === e.currentTarget) {
      closeNav();
    }
    mouseDownOnBackdrop = false;
  }
</script>

<svelte:window onkeydown={sidebarNav.open ? handleKeydown : undefined} />

<a href="#main" class="skip-link">Перейти к основному содержимому</a>

<div class="app-layout">
  {#if sidebarNav.open && !isDesktop}
    <div
      class="nav-backdrop"
      role="presentation"
      onmousedown={handleBackdropMousedown}
      onmouseup={handleBackdropMouseup}
    ></div>
  {/if}
  <aside
    id="app-sidebar"
    class="sidebar-container"
    class:open={sidebarNav.open}
    bind:this={asideEl}
    inert={!isDesktop && !sidebarNav.open}
  >
    <Sidebar />
  </aside>
  <main id="main" class="content" inert={!isDesktop && sidebarNav.open}>
    {@render children?.()}
  </main>
</div>

<svelte:head>
  {#if sidebarNav.open && !isDesktop}
    <style>
      body {
        overflow: hidden;
      }
    </style>
  {/if}
</svelte:head>

<style lang="scss">
  @use '../../styles/_breakpoints' as bp;

  .app-layout {
    display: grid;
    grid-template-columns: var(--sidebar-width) 1fr;
    grid-template-rows: minmax(0, 1fr);
    height: 100vh;
    overflow: hidden;
    background: var(--tr-surface);
  }

  .sidebar-container {
    position: sticky;
    top: 0;
    height: 100vh;
    overflow: hidden;
  }

  @media (max-width: (bp.$bp-lg - 1px)) {
    .app-layout {
      grid-template-columns: 1fr;
    }

    .sidebar-container {
      position: fixed;
      inset-block: 0;
      left: 0;
      width: 236px;
      z-index: 60;
      transform: translateX(-100%);
      transition: transform 0.18s ease;
      box-shadow: var(--tr-elev-3);

      &.open {
        transform: translateX(0);
      }

      @media (prefers-reduced-motion: reduce) {
        transition: none;
      }
    }
  }

  .nav-backdrop {
    position: fixed;
    inset: 0;
    background: var(--tr-overlay);
    z-index: 55;
  }

  .content {
    display: flex;
    flex-direction: column;
    overflow: auto;
    min-height: 0;
    background: var(--tr-surface);
  }

  .skip-link {
    position: absolute;
    left: -9999px;
    top: -9999px;
    z-index: 9999;
    padding: var(--tr-space-md);
    background: var(--tr-accent);
    color: var(--tr-text-inverse);
    font-size: var(--tr-font-size-body);
    text-decoration: none;
    border-radius: var(--tr-radius-xs);

    &:focus {
      left: 0;
      top: 0;
    }
  }
</style>
