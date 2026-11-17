<script lang="ts">
  // Иконки блоков событий (UI-SPEC §9.2): inline-SVG 16px на currentColor.
  import type { IconKind } from './eventPresentation';

  interface Props {
    kind: IconKind;
  }
  const { kind }: Props = $props();
</script>

<svg
  width="16"
  height="16"
  viewBox="0 0 16 16"
  fill="none"
  stroke="currentColor"
  stroke-width="1.5"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if kind === 'plus'}
    <circle cx="8" cy="8" r="6" />
    <path d="M8 5v6M5 8h6" />
  {:else if kind === 'play'}
    <path d="M5.5 3.5l7 4.5-7 4.5z" />
  {:else if kind === 'person-plus'}
    <circle cx="6.5" cy="5.5" r="2.5" />
    <path d="M2 13c0-2.5 2-4 4.5-4s4.5 1.5 4.5 4M13 5v4M11 7h4" />
  {:else if kind === 'merge'}
    <path d="M2.5 3l4 4v6M13.5 3l-4 4M6.5 7h3" />
    <path d="M8 7v6" />
  {:else if kind === 'arrow-right'}
    <path d="M3 8h10M9 4l4 4-4 4" />
  {:else if kind === 'wrench'}
    <path
      d="M10.5 2.5a3.5 3.5 0 0 0-3.3 4.7L2.5 11.9a1.4 1.4 0 0 0 2 2l4.7-4.7a3.5 3.5 0 0 0 4.7-3.3l-2.2 2.2-2-.5-.5-2z"
    />
  {:else if kind === 'return'}
    <path d="M13 12V9a3 3 0 0 0-3-3H3M6 3L3 6l3 3" />
  {:else if kind === 'monitor'}
    <rect x="2" y="3" width="12" height="8" rx="1" />
    <path d="M6 14h4M8 11v3" />
  {:else if kind === 'check'}
    <path d="M3 8.5l3.5 3.5L13 4.5" />
  {:else if kind === 'cross'}
    <path d="M4 4l8 8M12 4l-8 8" />
  {:else if kind === 'ban'}
    <circle cx="8" cy="8" r="5.5" />
    <path d="M4 12L12 4" />
  {:else if kind === 'paperclip'}
    <path
      d="M12.5 7.5l-4.6 4.6a2.8 2.8 0 0 1-4-4l5-5a1.9 1.9 0 0 1 2.7 2.7l-5 5a.9.9 0 0 1-1.3-1.3l4.3-4.3"
    />
  {:else}
    <circle cx="8" cy="8" r="1.5" fill="currentColor" />
  {/if}
</svg>
