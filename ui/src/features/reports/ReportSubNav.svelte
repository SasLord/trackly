<script lang="ts">
  // Plan 07-06 Task 1: Two-level navigation for Reports page.
  // Domain sub-nav (Устройства / Картриджи) + report type switch-bar.
  // Plan 07-10 Task 2: GAP-R2 — both navs share one row on desktop.
  //                    GAP-R5 — badges on ALL tabs (active: real count; inactive: –).
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
    /** Real per-tab counts from reports_get_report_counts (G2-5b).
     *  When provided, all tabs show statusCounts[key] ?? 0.
     *  When absent, active tab shows rowCount and inactive tabs show '–'. */
    statusCounts?: Record<string, number>;
    onDomainChange: (_d: DomainKey) => void;
    onReportChange: (_r: string) => void;
  }

  const {
    activeDomain,
    activeReport,
    rowCount,
    statusCounts,
    onDomainChange,
    onReportChange,
  }: Props = $props();

  const activeReports = $derived(activeDomain === 'devices' ? DEVICE_REPORTS : CARTRIDGE_REPORTS);
</script>

<!-- GAP-R2: domain-nav (left) and report-nav (right) on the same flex row on desktop -->
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
        <!-- G2-5b: when statusCounts provided, show real count for ALL tabs;
             otherwise fall back to rowCount (active) / '–' (inactive) for compat -->
        <Badge variant={r.key === activeReport ? 'accent' : 'default'} size="sm">
          {statusCounts ? (statusCounts[r.key] ?? 0) : r.key === activeReport ? rowCount : '–'}
        </Badge>
      </button>
    {/each}
  </div>
</div>

<style lang="scss">
  // GAP-R2: single flex row on desktop; wraps on narrow screens
  .report-sub-nav {
    display: flex;
    flex-direction: row;
    align-items: center;
    gap: var(--tr-space-md);
    border-bottom: 1px solid var(--tr-border);
    flex-shrink: 0;
    flex-wrap: wrap;
  }

  .domain-nav {
    display: flex;
    gap: var(--tr-space-2xs);
    padding: var(--tr-space-xs) 0;
    flex-shrink: 0;
    // No border-bottom here — parent .report-sub-nav owns the single bottom border
  }

  .report-nav {
    display: flex;
    gap: var(--tr-space-2xs);
    padding: var(--tr-space-xs) 0;
    flex: 1;
    justify-content: flex-end;
    flex-wrap: wrap;
    // role=tablist is valid on div
  }

  .tab {
    display: inline-flex;
    align-items: center;
    gap: var(--tr-space-2xs);
    padding: var(--tr-space-2xs) var(--tr-space-md);
    background: transparent;
    color: var(--tr-text-primary);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
    font-family: var(--tr-font-family);
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-medium);
    cursor: pointer;
    height: 32px;
    white-space: nowrap;

    &:hover {
      background: var(--tr-surface-sunken);
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }

    &.active {
      background: color-mix(in srgb, var(--tr-accent) 10%, transparent);
      border-color: var(--tr-accent);
      color: var(--tr-text-primary);
    }
  }
</style>
