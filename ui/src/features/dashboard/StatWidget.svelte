<script lang="ts">
  // Plan 07-05: StatWidget — универсальная карточка-виджет для дашборда.
  // Показывает число, подпись, список разбивки и опциональное предупреждение о низком остатке.
  import Spinner from '$lib/components/Spinner.svelte';

  interface BreakdownRow {
    label: string;
    count: number;
  }

  interface Props {
    id: string;
    title: string;
    mainNumber: number | null;
    mainLabel: string;
    breakdown: BreakdownRow[];
    loading: boolean;
    error: string | null;
    warningItems?: string[];
  }

  const {
    id,
    title,
    mainNumber,
    mainLabel,
    breakdown,
    loading,
    error,
    warningItems = [],
  }: Props = $props();
</script>

<section class="stat-widget" aria-labelledby="widget-title-{id}">
  <h2 class="widget-title" id="widget-title-{id}">{title}</h2>
  {#if loading}
    <div class="widget-loading">
      <Spinner size="sm" />
    </div>
  {:else if error}
    <div class="widget-error">Не удалось загрузить. Смените период или обновите страницу.</div>
  {:else}
    <div class="stat-value-row">
      <span class="stat-number">{mainNumber ?? '—'}</span>
      <span class="stat-unit">{mainLabel}</span>
    </div>
    {#if breakdown.length > 0}
      <div class="pill-row">
        {#each breakdown as row}
          <span class="pill">{row.label}: <strong>{row.count}</strong></span>
        {/each}
      </div>
    {/if}
    {#if warningItems && warningItems.length > 0}
      <div class="widget-warning">
        <span>Низкий остаток:</span>
        <ul>
          {#each warningItems as m}
            <li>{m}</li>
          {/each}
        </ul>
      </div>
    {/if}
  {/if}
</section>

<style lang="scss">
  .stat-widget {
    background: var(--tr-surface);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    padding: 16px;
    box-shadow: var(--tr-elev-1);
    min-width: 0;
    min-height: 120px;
  }

  .widget-title {
    margin: 0 0 var(--tr-space-xs);
    font-size: 13px;
    color: var(--tr-text-secondary);
  }

  .stat-value-row {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-top: 6px;
  }

  .stat-number {
    font-size: 30px;
    font-weight: 700;
    line-height: 1;
    font-variant-numeric: tabular-nums;
    color: var(--tr-text-primary);
  }

  .stat-unit {
    font-size: 13px;
    color: var(--tr-text-tertiary);
  }

  .pill-row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 14px;
  }

  .pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 3px 9px;
    border-radius: 11px;
    background: var(--tr-surface-sunken);
    font-size: 12px;
    color: var(--tr-text-secondary);
    white-space: nowrap;

    strong {
      color: var(--tr-text-primary);
      font-variant-numeric: tabular-nums;
    }
  }

  .widget-loading,
  .widget-error {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 80px;
  }

  .widget-error {
    color: var(--tr-text-tertiary);
    font-size: var(--tr-font-size-label);
  }

  .widget-warning {
    margin-top: var(--tr-space-xs);
    padding: var(--tr-space-2xs) var(--tr-space-xs);
    background: var(--tr-warning-soft);
    border: 1px solid transparent;
    border-radius: var(--tr-radius-xs);
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-primary);

    span {
      font-weight: 600;
      color: var(--tr-warning-text);
    }

    ul {
      list-style: disc;
      margin: var(--tr-space-2xs) 0 0 var(--tr-space-md);
      padding: 0;

      li {
        line-height: 1.6;
      }
    }
  }
</style>
