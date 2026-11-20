<script lang="ts">
  // Компактный индикатор тонера для строки списка (41.6, D-18, UI-SPEC §12).
  // Показывает минимальный известный уровень среди цветов; формула заливки общая
  // с TonerGauge (tonerFillColor). API не вызывает — данные приходят в DTO строки.
  import { tonerFillColor } from '../printers/tonerFill';
  import { formatDateTimeShort } from '$lib/utils/date';

  interface Props {
    toner: {
      levels: { label: string; percent: number | null }[];
      polledAtUtc: number | null;
    } | null;
  }

  const { toner }: Props = $props();

  const noPoll = $derived(
    toner === null || (toner.polledAtUtc === null && toner.levels.length === 0),
  );

  // Минимум среди известных уровней (null и < 0 = неизвестно).
  const min = $derived.by<{ label: string; percent: number } | null>(() => {
    let best: { label: string; percent: number } | null = null;
    for (const l of toner?.levels ?? []) {
      if (l.percent === null || l.percent < 0) continue;
      if (best === null || l.percent < best.percent) best = { label: l.label, percent: l.percent };
    }
    return best;
  });

  const pct = $derived(min === null ? null : Math.min(100, Math.max(0, min.percent)));

  const title = $derived.by(() => {
    if (noPoll) return 'Нет данных опроса';
    if (min === null) return 'Уровень тонера неизвестен';
    const polled = toner?.polledAtUtc ?? null;
    const suffix = polled !== null ? ` · опрос ${formatDateTimeShort(polled)}` : '';
    return `Тонер: минимум ${pct}% (${min.label})${suffix}`;
  });
</script>

<span class="toner-mini" role="img" aria-label={title} {title}>
  {#if !noPoll}
    <span class="track">
      {#if pct !== null}
        <span class="fill" style="width: {pct}%; background: {tonerFillColor(pct)}"></span>
      {/if}
    </span>
  {/if}
  <span class="value">{pct !== null ? pct + '%' : '—'}</span>
</span>

<style lang="scss">
  .toner-mini {
    display: inline-flex;
    align-items: center;
    gap: var(--tr-space-2xs);
  }

  .track {
    display: inline-block;
    position: relative;
    width: 32px;
    height: 8px;
    box-sizing: border-box;
    overflow: hidden;
    background: var(--tr-surface-sunken);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
  }

  .fill {
    display: block;
    height: 100%;
  }

  .value {
    font-size: 12px;
    color: var(--tr-text-secondary);
    font-variant-numeric: tabular-nums;
  }
</style>
