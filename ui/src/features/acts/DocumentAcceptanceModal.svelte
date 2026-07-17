<script lang="ts">
  // Plan 03-05 (DEV-14): intermediate modal — собирает «Кто передал»,
  // «Кто принял» и дату для документа приёма устройства на склад,
  // затем триггерит onSubmit, после чего родитель открывает
  // PdfPreviewModal в режиме mode='acceptance'.
  //
  // Backend (devices_render_acceptance_pdf) сделан в plan 03-04 — UI здесь
  // только собирает payload и передаёт его дальше. PDF-рендеринг — в
  // PdfPreviewModal.
  //
  // W-9 (timezone): UI работает в локали Europe/Moscow (UTC+3 без DST,
  // single-tz приложение). `<input type="date">` отдаёт строку
  // `YYYY-MM-DD` локального дня. Backend `render_acceptance_pdf` использует
  // `time::OffsetDateTime::from_unix_timestamp(...)` — UTC. Чтобы выбранный
  // календарный день не «съехал» назад при отображении (формирование
  // «28 мая 2026 г.»), мы кодируем «полночь MSK выбранного дня»: вычитаем
  // 3 часа из UTC-полуночи. Подробности — комментарий внутри
  // dateLocalToUtcSeconds.

  import Button from '$lib/components/Button.svelte';
  import PersonAutocomplete from '$lib/components/PersonAutocomplete.svelte';
  import Modal from '$lib/components/Modal.svelte';
  import type { DeviceDto } from '../../bindings';

  interface Props {
    open: boolean;
    device: DeviceDto | null;
    onClose: () => void;
    onSubmit: (_payload: {
      deviceId: number;
      giverName: string;
      receiverName: string;
      dateUtc: number;
    }) => void;
  }

  const { open, device, onClose, onSubmit }: Props = $props();

  function todayLocalIso(): string {
    const d = new Date();
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const day = String(d.getDate()).padStart(2, '0');
    return `${y}-${m}-${day}`;
  }

  /**
   * Converts a local `YYYY-MM-DD` (interpreted as Europe/Moscow / UTC+3 single-tz)
   * to a unix-seconds value representing midnight MSK of that calendar day.
   * Backend uses `OffsetDateTime::from_unix_timestamp(...)` (UTC) and we want
   * `format_ru_date` to render the same calendar day the user picked.
   *
   * Midnight MSK = UTC midnight − 3 hours, so subtract 3 * 3600 * 1000 ms
   * from `Date.UTC(...)` before dividing by 1000.
   */
  function dateLocalToUtcSeconds(dateStr: string): number {
    const [y, m, d] = dateStr.split('-').map(Number);
    const utcMs = Date.UTC(y, (m ?? 1) - 1, d ?? 1, 0, 0, 0) - 3 * 3600 * 1000;
    return Math.floor(utcMs / 1000);
  }

  let giverName = $state('');
  let receiverName = $state('');
  let dateLocal = $state(todayLocalIso());
  let submitting = $state(false);

  // Reset fields when modal re-opens.
  $effect(() => {
    if (open) {
      giverName = '';
      receiverName = '';
      dateLocal = todayLocalIso();
      submitting = false;
    }
  });

  const canSubmit = $derived(
    !!device && giverName.trim().length > 0 && receiverName.trim().length > 0 && !!dateLocal,
  );

  function handleSubmit() {
    if (!device || !canSubmit) return;
    submitting = true;
    const dateUtc = dateLocalToUtcSeconds(dateLocal);
    onSubmit({
      deviceId: device.id,
      giverName: giverName.trim(),
      receiverName: receiverName.trim(),
      dateUtc,
    });
    // Parent закрывает intermediate-модал и открывает preview-модал.
  }
</script>

<Modal {open} title="Документ приёма устройства" {onClose}>
  <div class="acceptance-form">
    {#if device}
      <p class="device-line">
        Устройство: <strong>{device.name}</strong>
        {#if device.inventory_no}
          (инв. № {device.inventory_no})
        {/if}
      </p>
    {/if}

    <label class="field">
      <span class="field-label">Кто передал</span>
      <PersonAutocomplete field="giver" bind:value={giverName} placeholder="ФИО передающего" />
    </label>

    <label class="field">
      <span class="field-label">Кто принял</span>
      <PersonAutocomplete
        field="receiver"
        bind:value={receiverName}
        placeholder="ФИО принимающего"
      />
    </label>

    <label class="field">
      <span class="field-label">Дата</span>
      <input
        class="date-input"
        type="date"
        value={dateLocal}
        oninput={(e) => (dateLocal = (e.currentTarget as HTMLInputElement).value)}
      />
    </label>
  </div>

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>Отмена</Button>
    <Button variant="primary" disabled={!canSubmit || submitting} onclick={handleSubmit}>
      Сформировать PDF
    </Button>
  {/snippet}
</Modal>

<style lang="scss">
  .acceptance-form {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
  }

  .device-line {
    margin: 0 0 var(--tr-space-2xs);
    color: var(--tr-text-secondary);
    font-size: var(--tr-font-size-body);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  .field-label {
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-primary);
  }

  .date-input {
    height: 32px;
    padding: 0 var(--tr-space-xs);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
    background: var(--tr-surface);
    color: var(--tr-text-primary);
    font-family: var(--tr-font-family);
    font-size: var(--tr-font-size-body);

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
  }
</style>
