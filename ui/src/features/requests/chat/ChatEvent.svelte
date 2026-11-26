<script lang="ts">
  // Блок события жизненного цикла и блок решения (D-11, D-12; UI-SPEC §9).
  // Подписи — из общего historyLabels.ts (chatEventText), сырые коды не выводятся.
  // Текст комментария — только интерполяцией и linkify, без raw-HTML.
  import { chatEventText, entryParts } from '../historyLabels';
  import { formatDateTimeShort } from '@trackly/kit/utils/date';
  import type { ChatEventDto } from '../../../bindings';
  import ChatEventIcon from './ChatEventIcon.svelte';
  import { timeHHmm } from './chatDates';
  import { presentationFor } from './eventPresentation';
  import { linkify } from './linkify';
  import { onExternalLinkClick } from './openExternal';

  interface Props {
    item: ChatEventDto;
    resolutionNotes: string | null;
  }

  const { item, resolutionNotes }: Props = $props();

  const pres = $derived(presentationFor(item.action));
  const resolution = $derived(pres.resolution);
  const text = $derived(chatEventText(item));
  const title = $derived(resolution === 'reject' ? 'Заявка отклонена' : 'Заявка выполнена');
  const who = $derived(entryParts(item).who);
  const time = $derived(timeHHmm(item.createdAtUtc));
  const fullTime = $derived(formatDateTimeShort(item.createdAtUtc));
  const notes = $derived(item.showNotes && resolutionNotes?.trim() ? resolutionNotes : null);
  const segments = $derived(notes ? linkify(notes) : []);
  const ariaLabel = $derived(`${resolution ? `${title}${who ? ` · ${who}` : ''}` : text}, ${time}`);
</script>

<li class="event tone-{pres.tone}" class:resolution={resolution !== null} aria-label={ariaLabel}>
  <span class="icon"><ChatEventIcon kind={pres.icon} /></span>
  <div class="body">
    {#if resolution}
      <p class="head">
        <span class="title">{title}</span>{#if who}<span class="who"> · {who}</span>{/if}
      </p>
      {#if notes}
        <p class="notes">
          {#each segments as seg, i (i)}{#if seg.type === 'link'}<a
                href={seg.value}
                target="_blank"
                rel="noopener noreferrer"
                onclick={(e) => onExternalLinkClick(e, seg.value)}>{seg.value}</a
              >{:else}{seg.value}{/if}{/each}
        </p>
      {/if}
    {:else}
      <p class="head line">{text}</p>
    {/if}
  </div>
  <span class="time" title={fullTime}>{time}</span>
</li>

<style lang="scss">
  .event {
    display: flex;
    align-items: flex-start;
    gap: var(--tr-space-xs);
    box-sizing: border-box;
    width: fit-content;
    max-width: 560px;
    margin: var(--tr-space-md) auto;
    padding: var(--tr-space-xs) var(--tr-space-sm);
    list-style: none;
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    background: var(--tr-surface);
    text-align: left;
    white-space: normal;
  }
  .icon {
    display: inline-flex;
    flex: 0 0 auto;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    border-radius: var(--tr-radius-full);
    background: var(--tr-surface-sunken);
    color: var(--tr-text-secondary);
  }
  .tone-info .icon {
    background: var(--tr-info-soft);
    color: var(--tr-info-text);
  }
  .tone-success .icon {
    background: var(--tr-success-soft);
    color: var(--tr-success-text);
  }
  .tone-warning .icon {
    background: var(--tr-warning-soft);
    color: var(--tr-warning-text);
  }
  .tone-danger .icon {
    background: var(--tr-danger-muted-soft);
    color: var(--tr-danger-muted-text);
  }
  .resolution.tone-success {
    box-shadow: inset 3px 0 0 var(--tr-success);
  }
  .resolution.tone-danger {
    box-shadow: inset 3px 0 0 var(--tr-danger-muted-border);
  }
  .body {
    flex: 1 1 auto;
    min-width: 0;
  }
  p {
    margin: 0;
  }
  .line {
    font: var(--tr-text-label);
    color: var(--tr-text-primary);
    overflow-wrap: anywhere;
  }
  .title {
    font: var(--tr-text-body-strong);
    color: var(--tr-text-primary);
  }
  .who {
    font: var(--tr-text-label);
    color: var(--tr-text-secondary);
  }
  .notes {
    margin-top: var(--tr-space-2xs);
    font: var(--tr-text-body);
    color: var(--tr-text-primary);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    text-align: left;
    a {
      color: var(--tr-accent-text);
      text-decoration: underline;
    }
  }
  .time {
    flex: 0 0 auto;
    align-self: flex-end;
    font: var(--tr-text-caption);
    color: var(--tr-text-secondary);
    white-space: nowrap;
  }
</style>
