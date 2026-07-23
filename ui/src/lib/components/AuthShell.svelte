<script lang="ts">
  // Phase 29, plan 01 (D-02): shared auth center-card chrome.
  // Extracted per PageHeader.svelte/DetailPanel.svelte precedent — Snippet
  // children, `const` destructure, scoped SCSS, single responsibility.
  // Covers the byte-near-identical .login-container/.login-card chrome
  // duplicated across LoginPage/FirstRunWizard/PendingScreen/BlockedScreen.
  // Does NOT render a title — each screen keeps its own heading/paragraph as
  // children, since title-to-content spacing differs per screen.
  import type { Snippet } from 'svelte';

  interface Props {
    maxWidth?: number;
    stack?: boolean;
    children?: Snippet;
  }

  const { maxWidth = 360, stack = false, children }: Props = $props();
</script>

<div class="auth-shell">
  <div class="auth-card" class:stack style:max-width="{maxWidth}px">
    {@render children?.()}
  </div>
</div>

<style lang="scss">
  .auth-shell {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    background: var(--tr-bg);
  }

  .auth-card {
    background: var(--tr-surface);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-lg);
    padding: var(--tr-space-2xl) var(--tr-space-4xl);
    width: 100%;
    box-shadow: var(--tr-elev-2);

    &.stack {
      display: flex;
      flex-direction: column;
      gap: var(--tr-space-md);
      text-align: center;
    }
  }
</style>
