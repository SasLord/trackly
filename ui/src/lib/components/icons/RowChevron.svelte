<script lang="ts">
  // RowChevron — общий шеврон раскрытия строки таблицы (TableRow group-режим,
  // строки-свёртки устройств/групп). Вид 1:1 с прежней inline-кнопкой TableRow.
  interface Props {
    expanded: boolean;
    onclick: (e: MouseEvent) => void;
    /** По умолчанию «Развернуть»/«Свернуть» по expanded. */
    label?: string;
    class?: string;
  }

  const { expanded, onclick, label, class: className = '' }: Props = $props();
</script>

<button
  type="button"
  class="tr-row-chevron {className}"
  class:expanded
  aria-expanded={expanded}
  aria-label={label ?? (expanded ? 'Свернуть' : 'Развернуть')}
  {onclick}
>
  <svg width="14" height="14" viewBox="0 0 16 16" fill="none" aria-hidden="true">
    <path
      d="M6 4l4 4-4 4"
      stroke="currentColor"
      stroke-width="1.75"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  </svg>
</button>

<style lang="scss">
  .tr-row-chevron {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    padding: 0;
    border-radius: var(--tr-radius-xs);
    margin-right: var(--tr-space-2xs);
    background: transparent;
    border: none;
    color: var(--tr-text-secondary);
    cursor: pointer;
    transform: none;
    // This ONE transition stays at .15s (TableRows.dc value, verbatim) — not the
    // usual .12s micro-transition used elsewhere in the design system.
    transition: transform 0.15s;

    &.expanded {
      transform: rotate(90deg);
    }

    &:focus-visible {
      outline: none;
      // Same unified focus-ring token as the row's first-cell ring in TableRow.
      box-shadow: inset 0 0 0 2px var(--tr-focus-ring);
    }
  }
</style>
