<script module lang="ts">
  /** Пары через двоеточие, верхний регистр (как в «Настройки → Сеть»). */
  export function formatFingerprint(raw: string): string {
    const hex = raw.replace(/[^0-9a-fA-F]/g, '').toUpperCase();
    const pairs = hex.match(/.{1,2}/g);
    return pairs ? pairs.join(':') : raw;
  }
</script>

<script lang="ts">
  // Блок «Для администратора» (UI-SPEC §6.4): общий для экрана ошибки пина и первой настройки.
  // Принять новый сертификат отсюда нельзя (D-07) — только показать сведения.
  interface Props {
    server: string;
    expected: string;
    got: string;
  }

  const { server, expected, got }: Props = $props();

  const exp = $derived(formatFingerprint(expected));
  const rec = $derived(formatFingerprint(got));
</script>

<div class="admin-block">
  <p class="block-title">Для администратора</p>
  <div class="row">
    <span class="label">Сервер:</span>
    <span class="value">{server}</span>
  </div>
  <div class="row">
    <span class="label">Ожидаемый отпечаток (SPKI):</span>
    <span class="value tr-mono">{exp}</span>
  </div>
  <div class="row">
    <span class="label">Полученный отпечаток (SPKI):</span>
    <span class="value tr-mono">{rec}</span>
  </div>
</div>

<style lang="scss">
  .admin-block {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
    padding: var(--tr-space-sm) var(--tr-space-md);
    background: var(--tr-surface-sunken);
    border-radius: var(--tr-radius-md);
    text-align: left;
  }

  .block-title {
    margin: 0;
    font-size: var(--tr-font-size-caption);
    color: var(--tr-text-secondary);
  }

  .row {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  .label {
    font-size: var(--tr-font-size-caption);
    color: var(--tr-text-secondary);
  }

  .value {
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
    overflow-wrap: anywhere;
    user-select: text;
  }

  .value.tr-mono {
    font-size: var(--tr-font-size-mono);
  }
</style>
