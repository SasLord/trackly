<script lang="ts">
  import type { Snippet } from 'svelte';
  import Sidebar from './Sidebar.svelte';

  interface Props {
    children?: Snippet;
  }

  const { children }: Props = $props();
</script>

<a href="#main" class="skip-link">Перейти к основному содержимому</a>

<div class="app-layout">
  <aside class="sidebar-container">
    <Sidebar />
  </aside>
  <main id="main" class="content">
    {@render children?.()}
  </main>
</div>

<style lang="scss">
  .app-layout {
    display: grid;
    grid-template-columns: var(--sidebar-width) 1fr;
    min-height: 100vh;
  }

  .sidebar-container {
    position: sticky;
    top: 0;
    height: 100vh;
    overflow: hidden;
  }

  .content {
    padding: var(--tr-space-xl);
    overflow: auto;
    min-height: 100vh;
    background: var(--tr-bg);
  }

  .skip-link {
    position: absolute;
    left: -9999px;
    top: -9999px;
    z-index: 9999;
    padding: var(--tr-space-md);
    background: var(--tr-accent);
    color: var(--tr-text-inverse);
    font-size: var(--font-size-body);
    text-decoration: none;
    border-radius: var(--radius-sm);

    &:focus {
      left: 0;
      top: 0;
    }
  }
</style>
