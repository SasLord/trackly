<script lang="ts">
  // Phase 27, plan 01 (D-01): shared detail-panel primitive.
  // Field label/value widget — covers the field-grid item duplicated across
  // ActDetail/CartridgeDetail/PrinterDetail, and is contract-compatible with
  // the acts window's former field-widget component (label/value, null -> '—').
  interface Props {
    label: string;
    value?: string | null;
  }

  const { label, value }: Props = $props();
</script>

<div class="field">
  <span class="field-label">{label}</span>
  <span class="field-value">{value ?? '—'}</span>
</div>

<style lang="scss">
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-3xs);
  }
  .field-label {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
  }
  .field-value {
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
  }
</style>
