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
    <div class="widget-error">Ошибка загрузки</div>
  {:else}
    <p class="stat-number">{mainNumber ?? '—'}</p>
    <p class="stat-label">{mainLabel}</p>
    {#if breakdown.length > 0}
      <ul class="breakdown-list">
        {#each breakdown as row}
          <li>{row.label}: {row.count}</li>
        {/each}
      </ul>
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
    padding: var(--tr-space-xl);
    min-height: 120px;
  }

  .widget-title {
    margin: 0 0 var(--tr-space-xs);
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .stat-number {
    font-size: var(--font-size-display);
    font-weight: var(--font-weight-semibold);
    margin: 0;
    color: var(--tr-text-primary);
    line-height: 1.2;
  }

  .stat-label {
    font-size: var(--font-size-label);
    color: var(--tr-text-secondary);
    margin: var(--tr-space-2xs) 0 0;
  }

  .breakdown-list {
    list-style: none;
    padding: 0;
    margin: var(--tr-space-xs) 0 0;
    font-size: var(--font-size-label);
    color: var(--tr-text-secondary);

    li {
      line-height: 1.6;
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
    font-size: var(--font-size-label);
  }

  .widget-warning {
    margin-top: var(--tr-space-xs);
    padding: var(--tr-space-2xs) var(--tr-space-xs);
    background: color-mix(in srgb, var(--tr-warning) 10%, transparent);
    border: 1px solid var(--tr-warning);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-label);
    color: var(--tr-text-primary);

    span {
      font-weight: var(--font-weight-medium);
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
