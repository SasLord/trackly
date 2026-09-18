<script lang="ts">
  import type { Snippet } from 'svelte';
  import { portal } from '$lib/utils/portal';

  interface Props {
    label?: string;
    /** 'default' — текущий бордер-триггер (36×36, используется в DevicesPage
     *  «Импорт и экспорт» — НЕ трогать). 'ghost-sm' — без бордера, 28px,
     *  ghost-стиль как Button variant="ghost" size="sm" (quick 260820-rdj).
     *  'ghost-md' — 36×36, визуально идентична Button variant="ghost"
     *  size="md" iconOnly через ОБЩИЙ класс .tr-btn-ghost-icon
     *  (global.scss) — план 40.2-09, UI-SPEC §10 вариант б. */
    variant?: 'default' | 'ghost-sm' | 'ghost-md';
    /** Заменяет «три точки» произвольной иконкой (например
     *  IconInsertTemplate) — используется вместе с 'ghost-md'. */
    icon?: Snippet;
    /** Inline min-width панели поверх дефолтных 280px (`.action-menu-panel`). */
    panelMinWidth?: string;
    /** Рендерит панель в `<body>` через `use:portal` и переключает её на
     *  `position: fixed`, чтобы `overflow-y: auto` тела `Modal` её не
     *  обрезал (UI-SPEC §3, Discretion #3). Правый край панели выравнивается
     *  по правому краю кнопки-триггера; при нехватке места снизу панель
     *  открывается вверх. По умолчанию false — существующие вызовы
     *  ('default'/'ghost-sm') не меняют поведения. */
    portal?: boolean;
    /** UI-SPEC §2: поле номера показывает кнопку «Вставка» всегда, но
     *  делает её disabled в readonly/disabled-формах (plan 12 usage).
     *  Rule 2 (executor-examples.md) — минимально необходимо для того,
     *  чтобы ghost-md был полноценной заменой Button.iconOnly, у которого
     *  disabled уже есть; также нужно для витрины этого плана
     *  ("обычная и disabled версии обеих"). Не меняет 'default'/'ghost-sm'
     *  вызовы — ни один существующий не передаёт disabled. */
    disabled?: boolean;
    children: Snippet;
  }

  const {
    label = 'Действия',
    variant = 'default',
    icon,
    panelMinWidth,
    portal: usePortal = false,
    disabled = false,
    children,
  }: Props = $props();

  let open = $state(false);
  let rootEl = $state<HTMLElement | null>(null);
  let triggerEl = $state<HTMLButtonElement | null>(null);
  let panelEl = $state<HTMLElement | null>(null);

  function menuItems(): HTMLElement[] {
    return panelEl ? Array.from(panelEl.querySelectorAll<HTMLElement>('[role="menuitem"]')) : [];
  }

  function close(returnFocus = false) {
    open = false;
    if (returnFocus) triggerEl?.focus();
  }

  // Move focus to the first menu item whenever the panel opens.
  $effect(() => {
    if (open && panelEl) menuItems()[0]?.focus();
  });

  $effect(() => {
    function onDown(e: MouseEvent) {
      // Outside pointer click — close without forcing focus back to the
      // trigger; focus should follow wherever the user clicked.
      const target = e.target as Node;
      const insideRoot = rootEl?.contains(target) ?? false;
      // Portal mode moves the panel out of rootEl (into <body>), so the
      // click-outside check must also cover the portaled panel.
      const insidePanel = usePortal ? (panelEl?.contains(target) ?? false) : false;
      if (open && !insideRoot && !insidePanel) open = false;
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') close(true);
    }
    document.addEventListener('mousedown', onDown);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onDown);
      document.removeEventListener('keydown', onKey);
    };
  });

  function onTriggerKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      open = true;
    }
  }

  function onPanelKeydown(e: KeyboardEvent) {
    const its = menuItems();
    if (its.length === 0) return;
    const idx = its.indexOf(document.activeElement as HTMLElement);
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      its[(idx + 1) % its.length]?.focus();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      its[(idx - 1 + its.length) % its.length]?.focus();
    } else if (e.key === 'Home') {
      e.preventDefault();
      its[0]?.focus();
    } else if (e.key === 'End') {
      e.preventDefault();
      its[its.length - 1]?.focus();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      close(true);
    }
  }

  /** Portal-mode positioning (UI-SPEC §3): right edge of the panel aligns
   *  with the right edge of the trigger button, 4px below; flips to open
   *  upward when there isn't enough room below. Unlike
   *  `lib/utils/dropdownAnchor.ts` (which left-aligns and pins the panel's
   *  width to the anchor's width for the combobox/select field), this
   *  action right-aligns and leaves width to the panel's own
   *  min/max-width CSS (280–420px, UI-SPEC §3) — kept local to this
   *  component rather than generalizing dropdownAnchor for a single
   *  consumer. */
  function actionMenuPortalPosition(node: HTMLElement, anchor: HTMLElement | null) {
    let anchorEl = anchor;

    function reposition() {
      if (!anchorEl) return;
      const rect = anchorEl.getBoundingClientRect();
      const gap = 4;
      const maxHeight = 280;

      node.style.position = 'fixed';
      node.style.left = 'auto';
      node.style.right = `${window.innerWidth - rect.right}px`;

      const spaceBelow = window.innerHeight - rect.bottom;
      const neededHeight = Math.min(maxHeight, node.scrollHeight || maxHeight);

      if (spaceBelow >= neededHeight) {
        node.style.top = `${rect.bottom + gap}px`;
        node.style.bottom = 'auto';
      } else {
        node.style.bottom = `${window.innerHeight - rect.top + gap}px`;
        node.style.top = 'auto';
      }
    }

    reposition();
    window.addEventListener('scroll', reposition, true);
    window.addEventListener('resize', reposition);

    return {
      update(newAnchor: HTMLElement | null) {
        anchorEl = newAnchor;
        reposition();
      },
      destroy() {
        window.removeEventListener('scroll', reposition, true);
        window.removeEventListener('resize', reposition);
      },
    };
  }
</script>

<div class="action-menu" bind:this={rootEl}>
  <button
    type="button"
    class:action-menu-trigger={variant !== 'ghost-md'}
    class:action-menu-trigger--ghost-sm={variant === 'ghost-sm'}
    class:tr-btn-ghost-icon={variant === 'ghost-md'}
    aria-haspopup="menu"
    aria-expanded={open}
    aria-label={label}
    {disabled}
    bind:this={triggerEl}
    onclick={() => (open = !open)}
    onkeydown={onTriggerKeydown}
  >
    {#if icon}
      {@render icon()}
    {:else}
      <svg width="18" height="18" viewBox="0 0 18 18" aria-hidden="true">
        <circle cx="9" cy="3.5" r="1.5" fill="currentColor" />
        <circle cx="9" cy="9" r="1.5" fill="currentColor" />
        <circle cx="9" cy="14.5" r="1.5" fill="currentColor" />
      </svg>
    {/if}
  </button>
  {#if open}
    {#if usePortal}
      <div
        class="action-menu-panel action-menu-panel--portal"
        role="menu"
        tabindex="-1"
        style:min-width={panelMinWidth}
        bind:this={panelEl}
        use:portal
        use:actionMenuPortalPosition={triggerEl}
        onkeydown={onPanelKeydown}
        onclick={() => close(true)}
      >
        {@render children()}
      </div>
    {:else}
      <div
        class="action-menu-panel"
        role="menu"
        tabindex="-1"
        style:min-width={panelMinWidth}
        bind:this={panelEl}
        onkeydown={onPanelKeydown}
        onclick={() => close(true)}
      >
        {@render children()}
      </div>
    {/if}
  {/if}
</div>

<!--
  Пункты меню с aria-disabled="true" (например, переполненная маска в меню
  «Вставка», план 12): клик по `.action-menu-panel` закрывает меню
  БЕЗУСЛОВНО (`onclick={() => close(true)}` выше) — это применяется к любому
  клику внутри панели, включая недоступные пункты. Элементы, которые не
  должны закрывать меню по клику, обязаны вызывать `event.stopPropagation()`
  в своём собственном обработчике — ответственность потребителя компонента,
  ActionMenu это не отслеживает.
-->

<style lang="scss">
  .action-menu {
    position: relative;
    display: inline-flex;
  }

  .action-menu-trigger {
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--tr-radius-sm);
    background: transparent;
    border: 1px solid var(--tr-border-strong);
    color: var(--tr-text-secondary);
    cursor: pointer;

    &:hover {
      background: var(--tr-row-hover);
    }
    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
    &:disabled {
      opacity: 0.45;
      cursor: not-allowed;
      pointer-events: none;
    }
  }

  .action-menu-trigger--ghost-sm {
    width: 28px;
    height: 28px;
    border: none;
    background: transparent;
    color: var(--tr-text-primary);

    &:hover {
      background: var(--tr-surface-sunken);
    }
  }

  // 'ghost-md' trigger intentionally does NOT get the scoped
  // `.action-menu-trigger` base class above (see class:action-menu-trigger
  // in the markup) — Svelte's per-component style scoping compiles that
  // selector to `.action-menu-trigger.svelte-xxxx`, which has HIGHER CSS
  // specificity than the plain global `.tr-btn-ghost-icon` class regardless
  // of source order, so its `border`/`color` would silently win over the
  // shared class and reintroduce the exact divergence UI-SPEC §10 (вариант
  // б) is meant to prevent. Geometry/color for ghost-md therefore comes
  // ENTIRELY from `.tr-btn-ghost-icon` (global.scss) — no styles duplicated
  // here.

  .action-menu-panel {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    z-index: 1000;
    min-width: 180px;
    display: flex;
    flex-direction: column;
    padding: 4px;
    background: var(--tr-surface-raised);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    box-shadow: var(--tr-elev-2);
  }

  // Portal mode (UI-SPEC §3): positioned via `actionMenuPortalPosition`
  // (inline `position: fixed`/`top`/`bottom`/`right` set on the node), so
  // the static `top`/`right` offsets above must be cleared here. max-width
  // 420px / max-height 280px + scroll are scoped to the portal panel only
  // — the pre-existing 'default'/'ghost-sm' panels keep their unconstrained
  // size (no behavior change, acceptance criterion of plan 40.2-09 task 2).
  .action-menu-panel--portal {
    top: auto;
    right: auto;
    max-width: 420px;
    max-height: 280px;
    overflow: auto;
  }

  .action-menu-panel :global(button) {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 12px;
    text-align: left;
    background: transparent;
    border: none;
    border-radius: var(--tr-radius-sm);
    color: var(--tr-text-primary);
    font-family: inherit;
    font-size: 14px;
    cursor: pointer;

    &:hover {
      background: var(--tr-row-hover);
    }
  }
</style>
