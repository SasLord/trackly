<script lang="ts">
  // Plan 07-06 Task 1: Two-level navigation for Reports page.
  // Domain sub-nav (Устройства / Картриджи) + report type switch-bar.
  import Badge from '$lib/components/Badge.svelte';

  type DomainKey = 'devices' | 'cartridges';

  interface ReportConfig {
    key: string;
    label: string;
    temporal: boolean;
    cmd: string;
  }

  const DEVICE_REPORTS: ReportConfig[] = [
    { key: 'acts', label: 'Акты', temporal: true, cmd: 'reports_list_device_acts' },
    { key: 'returns', label: 'Возвраты', temporal: true, cmd: 'reports_list_device_returns' },
    { key: 'in_use', label: 'В работе', temporal: false, cmd: 'reports_list_device_in_use' },
    { key: 'in_stock', label: 'На складе', temporal: false, cmd: 'reports_list_device_in_stock' },
  ];

  const CARTRIDGE_REPORTS: ReportConfig[] = [
    {
      key: 'consumption',
      label: 'Расход',
      temporal: true,
      cmd: 'reports_list_cartridge_consumption',
    },
    {
      key: 'refills',
      label: 'История заправок',
      temporal: true,
      cmd: 'reports_list_cartridge_refills',
    },
    {
      key: 'in_use',
      label: 'В работе',
      temporal: false,
      cmd: 'reports_list_cartridge_in_use',
    },
    {
      key: 'in_stock',
      label: 'На складе',
      temporal: false,
      cmd: 'reports_list_cartridge_in_stock',
    },
  ];

  const DOMAINS = [
    { key: 'devices' as DomainKey, label: 'Устройства' },
    { key: 'cartridges' as DomainKey, label: 'Картриджи' },
  ];

  interface Props {
    activeDomain: DomainKey;
    activeReport: string;
    rowCount: number;
    onDomainChange: (_d: DomainKey) => void;
    onReportChange: (_r: string) => void;
  }

  const { activeDomain, activeReport, rowCount, onDomainChange, onReportChange }: Props = $props();

  const activeReports = $derived(activeDomain === 'devices' ? DEVICE_REPORTS : CARTRIDGE_REPORTS);
</script>

<div class="report-sub-nav">
  <nav class="domain-nav" aria-label="Домен отчётов">
    {#each DOMAINS as d}
      <button
        class="tab"
        class:active={d.key === activeDomain}
        type="button"
        onclick={() => onDomainChange(d.key)}
        aria-pressed={d.key === activeDomain}
      >
        {d.label}
      </button>
    {/each}
  </nav>

  <div class="report-nav" role="tablist" aria-label="Тип отчёта">
    {#each activeReports as r}
      <button
        class="tab"
        class:active={r.key === activeReport}
        type="button"
        role="tab"
        aria-selected={r.key === activeReport}
        onclick={() => onReportChange(r.key)}
      >
        {r.label}
        {#if r.key === activeReport}
          <Badge variant="accent" size="sm">{rowCount}</Badge>
        {/if}
      </button>
    {/each}
  </div>
</div>

<style lang="scss">
  .report-sub-nav {
    display: flex;
    flex-direction: column;
    gap: 0;
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
  }

  .domain-nav {
    display: flex;
    gap: var(--space-xs);
    padding: var(--space-sm) 0 var(--space-sm);
    border-bottom: 1px solid var(--color-border);
  }

  .report-nav {
    display: flex;
    gap: var(--space-xs);
    padding: var(--space-sm) 0 var(--space-sm);
    flex-wrap: wrap;
    // role=tablist is valid on div
  }

  .tab {
    display: inline-flex;
    align-items: center;
    gap: var(--space-xs);
    padding: var(--space-xs) var(--space-md);
    background: transparent;
    color: var(--color-text-primary);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-family: var(--font-family-base);
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-medium);
    cursor: pointer;
    height: 32px;
    white-space: nowrap;

    &:hover {
      background: var(--color-surface-sunken);
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--color-accent-focus);
    }

    &.active {
      background: color-mix(in srgb, var(--color-accent) 10%, transparent);
      border-color: var(--color-accent);
      color: var(--color-text-primary);
    }
  }
</style>
