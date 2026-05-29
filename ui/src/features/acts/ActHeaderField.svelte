<script lang="ts">
  // Plan 03-02: display-only header field for ActDetail.
  // Trivial NEW PATTERN per PATTERNS §«No Analog Found».
  interface Props {
    label: string;
    value: string | null;
  }
  const { label, value }: Props = $props();
</script>

<div class="field">
  <div class="label">{label}</div>
  <div class="value" class:muted={value === null || value === ''}>
    {value && value.length > 0 ? value : '—'}
  </div>
</div>

<style lang="scss">
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }
  .label {
    font-size: var(--font-size-label);
    font-weight: 500;
    color: var(--color-text-secondary);
  }
  .value {
    font-size: var(--font-size-body);
    color: var(--color-text-primary);
  }
  .value.muted {
    color: var(--color-text-muted);
  }
</style>
