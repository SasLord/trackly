<script lang="ts">
  // Plan 06-04: PrinterAlertBanner — предупреждение о проблемном состоянии принтера.
  // ТОЧНО по паттерну LowStockBanner.svelte: role="alert", aria-live="polite",
  // warning SVG icon, color-mix background (UI-SPEC §PrinterAlertBanner, PRN-06, D-Alert-01).

  interface Props {
    hasAlert: boolean;
    alertType: 'offline' | 'error' | null;
    lastSeenUtc: number | null;
  }

  const { hasAlert, alertType, lastSeenUtc }: Props = $props();

  // Relative time: seconds → «Н минут назад» / «Н часов назад» / «Н дней назад».
  function relativeTime(utcSeconds: number | null): string {
    if (utcSeconds === null) return 'никогда';
    const nowSec = Math.floor(Date.now() / 1000);
    const diff = nowSec - utcSeconds;
    if (diff < 60) return 'только что';
    if (diff < 3600) return `${Math.floor(diff / 60)} мин. назад`;
    if (diff < 86400) return `${Math.floor(diff / 3600)} ч. назад`;
    return `${Math.floor(diff / 86400)} дн. назад`;
  }

  const statusText = $derived<string>(
    alertType === 'offline' ? 'Принтер не в сети' : 'Ошибка принтера',
  );

  const borderColor = $derived<string>(
    alertType === 'error' ? 'var(--color-destructive)' : 'var(--color-warning)',
  );

  const lastSeenText = $derived<string>(relativeTime(lastSeenUtc));
</script>

{#if hasAlert}
  <div class="alert-banner" role="alert" aria-live="polite" style="border-color: {borderColor}">
    <span class="alert-icon" aria-hidden="true">
      <svg
        width="16"
        height="16"
        viewBox="0 0 16 16"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        aria-hidden="true"
      >
        <path
          d="M8 1.5L14.5 13H1.5L8 1.5Z"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linejoin="round"
        />
        <path d="M8 6V9" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
        <circle cx="8" cy="11" r="0.75" fill="currentColor" />
      </svg>
    </span>
    <div class="alert-content">
      <span class="alert-title">{statusText}.</span>
      <span class="alert-time">Последний успешный опрос: {lastSeenText}.</span>
    </div>
  </div>
{/if}

<style lang="scss">
  .alert-banner {
    display: flex;
    align-items: flex-start;
    gap: var(--space-sm);
    padding: var(--space-sm) var(--space-md);
    margin-bottom: var(--space-md);
    background: color-mix(in srgb, var(--color-warning) 10%, transparent);
    border: 1px solid var(--color-warning);
    border-radius: var(--radius-md);
    color: var(--color-text-primary);
  }

  .alert-icon {
    color: var(--color-warning);
    flex-shrink: 0;
    margin-top: 2px;
    display: flex;
    align-items: center;
    width: 16px;
    height: 16px;
  }

  .alert-content {
    flex: 1;
    font-size: var(--font-size-body);
    line-height: var(--line-height-body);
  }

  .alert-title {
    font-weight: var(--font-weight-semibold);
    margin-right: var(--space-xs);
  }

  .alert-time {
    color: var(--color-text-secondary);
    font-size: var(--font-size-label);
  }
</style>
