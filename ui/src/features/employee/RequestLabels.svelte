<script lang="ts">
  // Метки открытых заявок по устройству (41.6, D-07, UI-SPEC §11).
  // Своя заявка — ссылка «Заявка №N»; чужая — нессылочная метка без номера (BOPLA).
  import Badge from '@trackly/kit/components/Badge.svelte';
  import { requestTargetHref } from '../requests/requestTarget';

  interface Props {
    myOpenRequests: { id: number; requestType: string; status: string }[];
    otherOpenRequest: boolean;
  }

  const { myOpenRequests, otherOpenRequest }: Props = $props();

  const mine = $derived([...myOpenRequests].sort((a, b) => a.id - b.id));

  // Клик по метке не должен доходить до строки/плитки. Двойной клик по ссылке
  // отсекает обработчик строки (`closest('button, a')`).
  function stopPropagation(e: MouseEvent) {
    e.stopPropagation();
  }
</script>

{#if mine.length > 0 || otherOpenRequest}
  <span class="request-labels">
    {#each mine as r (r.id)}
      <a
        class="label-link"
        href={requestTargetHref(r.id, 'info')}
        title={'Открыть заявку №' + r.id}
        aria-label={'Открыть заявку №' + r.id}
        onclick={stopPropagation}
      >
        <Badge variant="accent" appearance="soft" size="sm"
          >Заявка №<span class="tr-mono">{r.id}</span></Badge
        >
      </a>
    {/each}
    {#if otherOpenRequest}
      <span class="label-other" title="По устройству уже открыта заявка">
        <Badge variant="default" appearance="soft" size="sm">Есть заявка</Badge>
      </span>
    {/if}
  </span>
{/if}

<style lang="scss">
  .request-labels {
    display: inline-flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--tr-space-2xs);
    flex-shrink: 0;
  }

  .label-link {
    display: inline-flex;
    text-decoration: none;

    &:hover {
      text-decoration: underline;
    }
  }

  .label-other {
    display: inline-flex;
  }
</style>
