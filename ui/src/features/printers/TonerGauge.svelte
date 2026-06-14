<script lang="ts">
  // Plan 06-04: TonerGauge — горизонтальный progressbar уровня тонера/чернил.
  // Цвет по порогам (UI-SPEC §Toner-gauge семантика): ≥25% accent, 10-24% warning, <10% destructive.
  // Неизвестно (-2/-3 или null): пустой серый track, без fill.
  // role="progressbar", aria-valuenow/min/max (UI-SPEC §Accessibility §TonerGauge).

  interface Props {
    label: string;
    level: number | null;
    max?: number;
    encoding: 'percent' | 'level_over_max';
  }

  const { label, level, max = 100, encoding }: Props = $props();

  // Compute percentage.
  // SNMP unknown values: -2 (hrDeviceRunning:undetectable) or -3 (unknown).
  const pct = $derived<number | null>(
    level === null || level < 0
      ? null
      : encoding === 'percent'
        ? Math.min(100, Math.max(0, level))
        : max <= 0
          ? null
          : Math.min(100, Math.round((level * 100) / max)),
  );

  // Color by threshold (UI-SPEC §Toner-gauge семантика).
  const fillColor = $derived<string>(
    pct === null
      ? 'var(--color-surface-sunken)'
      : pct >= 25
        ? 'var(--color-accent)'
        : pct >= 10
          ? 'var(--color-warning)'
          : 'var(--color-destructive)',
  );

  const ariaLabel = $derived(
    `Уровень ${label}: ${pct !== null ? pct + '%' : 'неизвестно'}`,
  );
</script>

<div class="toner-gauge-row">
  <span class="gauge-label">{label}</span>
  <div
    class="gauge-track"
    role="progressbar"
    aria-valuenow={pct ?? 0}
    aria-valuemin={0}
    aria-valuemax={100}
    aria-label={ariaLabel}
  >
    {#if pct !== null}
      <div
        class="gauge-fill"
        style="width: {pct}%; background: {fillColor}"
      ></div>
    {/if}
  </div>
  <span class="gauge-pct" style="font-variant-numeric: tabular-nums">
    {pct !== null ? pct + '%' : '—'}
  </span>
</div>

<style lang="scss">
  .toner-gauge-row {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: var(--space-sm);
  }

  .gauge-label {
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
    white-space: nowrap;
    min-width: 60px;
  }

  .gauge-track {
    height: 8px;
    background: var(--color-surface);
    border-radius: var(--radius-sm);
    overflow: hidden;
    position: relative;
    border: 1px solid var(--color-border);
  }

  .gauge-fill {
    height: 100%;
    border-radius: var(--radius-sm);
    transition: width 0.3s ease;
  }

  .gauge-pct {
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
    min-width: 36px;
    text-align: right;
  }
</style>
