<script lang="ts">
  // Plan 07-06 Task 1: Two-level navigation for Reports page.
  // Domain sub-nav (Устройства / Картриджи) + report type switch-bar.
  // Plan 07-10 Task 2: GAP-R2 — both navs share one row on desktop.
  //                    GAP-R5 — badges on ALL tabs (active: real count; inactive: –).
  // Plan 28-03 Task 1 (D-06): both levels moved onto the shared Tabs primitive
  // (segmented for domain, underline+count for report type) — no bespoke tab markup.
  import Tabs from '$lib/components/Tabs.svelte';

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
     *  When absent, active tab shows rowCount and inactive tabs show 0
     *  (Plan 28-03: Tabs.count is typed number, no string dash fallback). */
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
  <Tabs
    variant="segmented"
    tabs={DOMAINS.map((d) => ({ key: d.key, label: d.label }))}
    active={activeDomain}
    ariaLabel="Домен отчётов"
    onchange={(key) => onDomainChange(key as DomainKey)}
  />

  <Tabs
    variant="underline"
    tabs={activeReports.map((r) => ({
      key: r.key,
      label: r.label,
      count: statusCounts ? (statusCounts[r.key] ?? 0) : r.key === activeReport ? rowCount : 0,
    }))}
    active={activeReport}
    ariaLabel="Тип отчёта"
    onchange={onReportChange}
  />
</div>

<style lang="scss">
  // GAP-R2: single flex row on desktop; wraps on narrow screens
  .report-sub-nav {
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
    gap: var(--tr-space-md);
    border-bottom: 1px solid var(--tr-border);
    flex-shrink: 0;
    flex-wrap: wrap;
    padding: var(--tr-space-xs) 0;
  }
</style>
