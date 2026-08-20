<script module lang="ts">
  // Shared modal stack (quick 260820-rdj, defect 1): nested modals (e.g. a
  // downgrade-confirm popup atop an edit popup) each get their own Modal
  // instance. Without a stack, BOTH instances' `<svelte:window onkeydown>`
  // and backdrop-dismiss handlers would fire for the same Escape/click —
  // Escape on the top modal would also bubble logic to the one underneath,
  // and the bottom modal's backdrop could still be dismissed while a modal is
  // stacked on top of it. `openStack` tracks instance identity (in open
  // order); only the topmost entry responds to Escape/Tab-trap/backdrop
  // click, and each instance's backdrop z-index is derived from its depth so
  // a nested modal is guaranteed to render above the one it covers.
  let openStack = $state<symbol[]>([]);
</script>

<script lang="ts">
  import { untrack, type Snippet } from 'svelte';

  interface Props {
    open: boolean;
    title: string;
    size?: 'md' | 'wide' | 'xwide' | 'pdf-preview';
    onClose: () => void;
    children?: Snippet;
    footer?: Snippet;
    titleExtra?: Snippet;
  }

  const { open, title, size = 'md', onClose, children, footer, titleExtra }: Props = $props();

  const titleId = `modal-title-${Math.random().toString(36).slice(2)}`;

  // Identity for this Modal instance in the shared stack — stable for the
  // component's lifetime (not tied to `open`, so re-toggling open/closed
  // reuses the same identity).
  const instanceId = Symbol('modal');

  const stackDepth = $derived(openStack.indexOf(instanceId));
  const isTop = $derived(stackDepth >= 0 && stackDepth === openStack.length - 1);
  // Base 500 (unchanged default for the common single-modal case) + 10 per
  // nesting depth so a stacked modal's backdrop always renders above the one
  // it covers.
  const backdropZIndex = $derived(500 + Math.max(stackDepth, 0) * 10);

  // The push/remove below MUST read `openStack` inside `untrack` — this effect
  // writes to the same `$state` it reads, and every write produces a fresh
  // array reference. Without `untrack` the write re-invalidates the effect that
  // just ran, and Svelte aborts the whole component tree with
  // `effect_update_depth_exceeded` on the very first modal open (caught in
  // runtime UAT of quick 260820-rdj — compile-time gates cannot see it).
  $effect(() => {
    if (!open) return;
    untrack(() => {
      openStack = [...openStack, instanceId];
    });
    return () => {
      untrack(() => {
        openStack = openStack.filter((id) => id !== instanceId);
      });
    };
  });

  // G-1 fix (Phase 3.1 Plan 06): backdrop dismiss срабатывает ТОЛЬКО когда
  // mousedown AND mouseup произошли на backdrop element. Это защищает
  // от закрытия модала при text-selection drag (mousedown внутри → drag
  // outside → mouseup на backdrop), который ранее закрывал модал.
  let mouseDownOnBackdrop = $state(false);

  // CR-03 fix (24-10): WAI-ARIA Dialog Pattern — initial focus, Tab-trap, focus restoration.
  let dialogEl = $state<HTMLElement | null>(null);
  let prevFocus: HTMLElement | null = null;

  // CR-02/WR-02 fix (24-12): single selector source of truth for both initial focus
  // and the Tab-trap. iframe/contenteditable/audio/video/summary added so PdfPreviewModal's
  // <iframe> and similar rich content participate in the Tab-cycle instead of being skipped.
  const TRAP_FOCUSABLE_PARTS = [
    'button:not([disabled])',
    '[href]',
    'input:not([disabled])',
    'select:not([disabled])',
    'textarea:not([disabled])',
    'iframe',
    '[contenteditable]:not([contenteditable="false"])',
    'audio[controls]',
    'video[controls]',
    'summary',
    '[tabindex]:not([tabindex="-1"])',
  ];
  const TRAP_FOCUSABLE_SELECTOR = TRAP_FOCUSABLE_PARTS.join(', ');
  // CR-02 fix: use:portal-teleported content (autocomplete dropdowns, context menus) lives in
  // document.body, not inside dialogEl — map over the PARTS array (not the joined string) so
  // every comma-separated alternative is scoped, not just the first.
  const PORTAL_FOCUSABLE_SELECTOR = TRAP_FOCUSABLE_PARTS.map((p) => `[data-tr-portal] ${p}`).join(
    ', ',
  );

  function scopedFocusable(): HTMLElement[] {
    return dialogEl
      ? Array.from(dialogEl.querySelectorAll<HTMLElement>(TRAP_FOCUSABLE_SELECTOR)).filter(
          (n) => n.offsetParent !== null,
        )
      : [];
  }

  function portaledFocusable(): HTMLElement[] {
    // dropdownAnchor.ts sets position:fixed on portaled dropdowns, which always yields a
    // null layout-parent — getClientRects() is the correct visibility check here instead.
    return Array.from(document.querySelectorAll<HTMLElement>(PORTAL_FOCUSABLE_SELECTOR)).filter(
      (n) => n.getClientRects().length > 0,
    );
  }

  $effect(() => {
    if (!open) return;

    prevFocus = document.activeElement as HTMLElement | null;
    const first = scopedFocusable()[0];
    if (first) {
      first.focus();
    } else {
      dialogEl?.focus();
    }
    // WR-02 fix: verify the real outcome instead of trusting that `first` was non-null —
    // a disabled/hidden first match would previously leave focus stuck behind the backdrop.
    if (!dialogEl?.contains(document.activeElement)) {
      dialogEl?.focus();
    }

    return () => {
      prevFocus?.focus();
    };
  });

  function trapTab(e: KeyboardEvent) {
    if (e.key !== 'Tab' || !dialogEl) return;

    // Accepted limitation (WR-04, out of scope): portaledFocusable() queries the whole
    // document, not scoped per-Modal-instance — if two Modals were open simultaneously each
    // with its own open portal, either trap could pick up the other's portal node.
    const nodes = [...scopedFocusable(), ...portaledFocusable()];

    if (nodes.length === 0) return;

    const first = nodes[0];
    const last = nodes[nodes.length - 1];

    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    // quick 260820-rdj (defect 1): only the topmost modal in the stack reacts
    // to Escape/Tab-trap — a nested confirm popup must fully own keyboard
    // input while it's open, and closing it must not also close the modal
    // underneath.
    if (!isTop) return;
    if (e.key === 'Escape') {
      onClose();
      return;
    }
    trapTab(e);
  }

  function handleBackdropMousedown(e: MouseEvent) {
    if (!isTop) return;
    mouseDownOnBackdrop = e.target === e.currentTarget;
  }

  function handleBackdropMouseup(e: MouseEvent) {
    if (!isTop) return;
    if (mouseDownOnBackdrop && e.target === e.currentTarget) {
      onClose();
    }
    mouseDownOnBackdrop = false;
  }
</script>

<svelte:window onkeydown={open ? handleKeydown : undefined} />

{#if open}
  <div
    class="modal-backdrop"
    style:z-index={backdropZIndex}
    onmousedown={handleBackdropMousedown}
    onmouseup={handleBackdropMouseup}
    aria-modal="true"
    role="dialog"
    aria-labelledby={titleId}
    tabindex="-1"
  >
    <div class="modal-container modal-{size}" bind:this={dialogEl} tabindex="-1">
      <header class="modal-header">
        <div class="modal-title-group">
          <h2 id={titleId} class="modal-title">{title}</h2>
          {#if titleExtra}
            {@render titleExtra()}
          {/if}
        </div>
        <button type="button" class="modal-close" onclick={onClose} aria-label="Закрыть">×</button>
      </header>
      <div class="modal-body">
        {@render children?.()}
      </div>
      {#if footer}
        <footer class="modal-footer">
          {@render footer()}
        </footer>
      {/if}
    </div>
  </div>
{/if}

<svelte:head>
  {#if open}
    <style>
      body {
        overflow: hidden;
      }
    </style>
  {/if}
</svelte:head>

<style lang="scss">
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: var(--tr-overlay);
    backdrop-filter: blur(2px);
    display: flex;
    align-items: center;
    justify-content: center;
    // z-index set inline via style:z-index — depth-based (see instance script)
    // so a nested modal always renders above the one it covers.
  }

  .modal-container {
    background: var(--tr-surface);
    border-radius: var(--tr-radius-lg);
    box-shadow: var(--tr-elev-3);
    display: flex;
    flex-direction: column;
    max-height: calc(100vh - 64px);
    animation: modal-in 150ms ease-out;
  }

  .modal-md {
    width: var(--modal-max-width);
    max-width: var(--modal-max-width);
  }
  .modal-wide {
    width: var(--modal-max-width-wide);
    max-width: var(--modal-max-width-wide);
  }
  .modal-xwide {
    width: 100%;
    max-width: 1000px;
  }
  .modal-pdf-preview {
    width: 100%;
    max-width: min(95vw, 1100px);
    height: min(90vh, 920px);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--tr-space-md) var(--tr-space-xl);
    border-bottom: 1px solid var(--tr-border);
    flex-shrink: 0;
  }

  .modal-title-group {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
    min-width: 0;
    flex: 1 1 auto;
  }

  .modal-title {
    margin: 0;
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    line-height: var(--tr-line-height-h3);
    color: var(--tr-text-primary);
  }

  .modal-close {
    background: transparent;
    border: none;
    cursor: pointer;
    color: var(--tr-text-secondary);
    font-size: 20px;
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--tr-radius-xs);
    padding: 0;
    line-height: 1;

    &:hover {
      background: var(--tr-surface);
      color: var(--tr-text-primary);
    }
    &:focus-visible {
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
      outline: none;
    }
  }

  .modal-body {
    padding: var(--tr-space-xl);
    overflow-y: auto;
    overflow-x: hidden; // prevent horizontal scroll from long unbreakable strings
    flex: 1;
    // Ensure text inside modal body always wraps.
    overflow-wrap: anywhere;
    word-break: break-word;
  }

  .modal-footer {
    padding: var(--tr-space-md) var(--tr-space-xl);
    border-top: 1px solid var(--tr-border);
    display: flex;
    justify-content: flex-end;
    gap: var(--tr-space-xs);
    flex-shrink: 0;
  }

  @keyframes modal-in {
    from {
      opacity: 0;
      transform: scale(0.98);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
</style>
