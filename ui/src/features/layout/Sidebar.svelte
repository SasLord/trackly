<script lang="ts">
  import { link } from 'svelte-spa-router';
  import active from 'svelte-spa-router/active';
  import { SIDEBAR_ITEMS } from './sidebar-config';
  import ThemeSwitcher from '$lib/components/ThemeSwitcher.svelte';
</script>

<nav class="sidebar" aria-label="Основная навигация">
  <ul class="nav-list" role="list">
    {#each SIDEBAR_ITEMS as entry}
      {#if entry.kind === 'divider'}
        <li class="divider" aria-hidden="true" role="separator"></li>
      {:else}
        <li class="nav-item">
          <a
            href={entry.route}
            use:link
            use:active={{ path: entry.route, className: 'is-active' }}
            class="nav-link"
          >
            {entry.label}
          </a>
        </li>
      {/if}
    {/each}
  </ul>

  <div class="sidebar-footer">
    <span class="theme-label">Тема</span>
    <ThemeSwitcher />
  </div>
</nav>

<style lang="scss">
  .sidebar {
    width: var(--sidebar-width);
    height: 100%;
    background: var(--color-surface);
    border-right: 1px solid var(--color-border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .nav-list {
    flex: 1;
    list-style: none;
    margin: 0;
    padding: var(--space-sm) 0;
    overflow-y: auto;
  }

  .nav-item {
    margin: 0;
    padding: 0;
  }

  .nav-link {
    display: block;
    padding: 0 var(--space-md);
    height: var(--row-height);
    line-height: var(--row-height);
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-regular);
    color: var(--color-text-secondary);
    text-decoration: none;
    border-left: 3px solid transparent;
    transition: none;

    &:hover {
      background: color-mix(in srgb, var(--color-text-primary) 5%, transparent);
      color: var(--color-text-primary);
    }

    &:focus-visible {
      outline: none;
      box-shadow: inset 0 0 0 2px var(--color-accent-focus);
    }
  }

  :global(.nav-link.is-active) {
    border-left-color: var(--color-accent);
    background: color-mix(in srgb, var(--color-accent) 10%, transparent);
    color: var(--color-text-primary);
    font-weight: var(--font-weight-medium);
  }

  .divider {
    height: 1px;
    background: var(--color-border);
    margin: var(--space-xs) var(--space-md);
  }

  .sidebar-footer {
    padding: var(--space-md);
    border-top: 1px solid var(--color-border);
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .theme-label {
    font-size: var(--font-size-label);
    color: var(--color-text-muted);
    font-weight: var(--font-weight-regular);
  }
</style>
