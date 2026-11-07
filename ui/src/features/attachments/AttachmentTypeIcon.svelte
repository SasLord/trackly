<script module lang="ts">
  export type AttachmentIconKind = 'image' | 'document' | 'table' | 'archive';

  /** Иконка по типу (UI-SPEC §6.4): изображение, документ, таблица, архив. */
  export function iconKindForMime(mime: string, name: string): AttachmentIconKind {
    const m = mime.toLowerCase();
    const ext = (name.includes('.') ? name.split('.').pop()! : '').toLowerCase();
    if (m.startsWith('image/')) return 'image';
    if (
      ext === 'xls' ||
      ext === 'xlsx' ||
      ext === 'ods' ||
      m.includes('spreadsheet') ||
      m.includes('ms-excel')
    ) {
      return 'table';
    }
    if (ext === 'zip' || ext === '7z' || m.includes('zip') || m.includes('7z')) return 'archive';
    return 'document';
  }
</script>

<script lang="ts">
  interface Props {
    kind: AttachmentIconKind;
  }
  const { kind }: Props = $props();
</script>

<svg
  class="att-icon"
  width="32"
  height="32"
  viewBox="0 0 32 32"
  fill="none"
  stroke="currentColor"
  stroke-width="1.75"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if kind === 'image'}
    <rect x="4" y="6" width="24" height="20" rx="2" />
    <circle cx="11" cy="13" r="2" />
    <path d="M4 22l7-6 5 4 4-3 8 6" />
  {:else if kind === 'table'}
    <rect x="5" y="5" width="22" height="22" rx="2" />
    <path d="M5 12h22M5 19h22M14 5v22" />
  {:else if kind === 'archive'}
    <rect x="6" y="4" width="20" height="24" rx="2" />
    <path d="M16 4v6M14 10h4M14 14h4M14 18h4" />
    <rect x="13" y="21" width="6" height="4" rx="1" />
  {:else}
    <path d="M8 4h11l6 6v18H8z" />
    <path d="M19 4v6h6M12 16h9M12 20h9M12 24h6" />
  {/if}
</svg>

<style lang="scss">
  .att-icon {
    display: block;
    flex-shrink: 0;
  }
</style>
