<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '@trackly/kit/components/Button.svelte';
  import { formatStatusLine, loadLinkInfo, type LinkInfo } from '../lib/linkInfo';

  let info = $state<LinkInfo | null>(null);
  const text = $derived(formatStatusLine(info));

  onMount(async () => {
    info = await loadLinkInfo();
  });

  async function copy() {
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      // Буфер обмена недоступен — молча пропускаем, строку можно выделить вручную.
    }
  }
</script>

<div class="status-line">
  <span class="text">{text}</span>
  <Button variant="secondary" size="sm" onclick={copy}>Скопировать диагностику</Button>
</div>

<style lang="scss">
  .status-line {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    color: var(--tr-text-secondary);
    font-size: var(--tr-font-size-caption);
  }

  .text {
    font-family: var(--tr-font-mono);
    user-select: text;
    overflow-wrap: anywhere;
  }
</style>
