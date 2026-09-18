<script lang="ts">
  // Phase 40.2 Plan 11 (NUM-10, UI-SPEC §6, D-01/D-05): "Номер занят" — первый
  // попап цепочки проверки при сохранении. Презентационный компонент — все
  // данные приходят готовыми через пропсы от вызывающего кода (планы 13-15,
  // волна 6), который отвечает за их происхождение и оркестрацию цепочки.
  import Modal from './Modal.svelte';
  import Button from './Button.svelte';

  export interface NumberTakenRecordSummary {
    kind: 'device' | 'printer' | 'cartridge' | 'drum' | 'act';
    /**
     * Основное «имя» записи: для устройства/принтера/картриджа/фотобарабана —
     * модель; для акта — «Акт передачи» / «Акт возврата» (UI-SPEC §6 «Вид»).
     */
    title: string;
    /** Только для act: уже готовая строка «Передал: X · Принял: Y» — не
     *  разбирается на фронтенде (проще и надёжнее, см. <action> плана 11). */
    subtitle: string | null;
    /** Короткий путь места текстом (устройство/принтер/картридж/фотобарабан). */
    place: string | null;
    /** Устройство/принтер — «Статус»; картридж/фотобарабан — «Состояние»;
     *  act — «Дата» (тот же генерический слот, разный смысл по kind). */
    status: string | null;
  }

  interface Props {
    number: string;
    record: NumberTakenRecordSummary;
    /** false — форма правки / нет активного шаблона (D-05): кнопки
     *  «Взять следующий свободный» не должно быть, только «Закрыть». */
    canTakeNext: boolean;
    loadingTakeNext?: boolean;
    onTakeNext?: () => void;
    onClose: () => void;
  }

  const {
    number,
    record,
    canTakeNext,
    loadingTakeNext = false,
    onTakeNext,
    onClose,
  }: Props = $props();

  const KIND_LABEL: Record<NumberTakenRecordSummary['kind'], string> = {
    device: 'Устройство',
    printer: 'Принтер',
    cartridge: 'Картридж',
    drum: 'Фотобарабан',
    act: 'Акт',
  };

  function dash(value: string | null | undefined): string {
    return value && value.length > 0 ? value : '—';
  }

  interface CardRow {
    label: string;
    value: string;
    fullValue?: string | null;
  }

  // Карточка занявшей записи — состав строк зависит от record.kind
  // (UI-SPEC Copywriting Contract «Попапы цепочки сохранения»). Устройство/
  // принтер и картридж/фотобарабан симметричны: Вид/Модель/Место/(Статус|
  // Состояние) — локальный интерфейс пропсов даёт только один свободный
  // текстовый слот (title), поэтому «Наименование» и «Модель» из полного
  // UI-SPEC списка (5 колонок) здесь объединены в одну строку «Модель»;
  // задокументировано в SUMMARY плана 11 как решение для будущих планов
  // 13-15, которые подключат реальные данные.
  const rows = $derived.by((): CardRow[] => {
    switch (record.kind) {
      case 'device':
      case 'printer':
        return [
          { label: 'Вид', value: KIND_LABEL[record.kind] },
          { label: 'Модель', value: dash(record.title) },
          { label: 'Место', value: dash(record.place), fullValue: record.place },
          { label: 'Статус', value: dash(record.status) },
        ];
      case 'cartridge':
      case 'drum':
        return [
          { label: 'Вид', value: KIND_LABEL[record.kind] },
          { label: 'Модель', value: dash(record.title) },
          { label: 'Место', value: dash(record.place), fullValue: record.place },
          { label: 'Состояние', value: dash(record.status) },
        ];
      case 'act':
        return [
          { label: 'Вид', value: dash(record.title) },
          { label: 'Дата', value: dash(record.status) },
          { label: 'Передал / Принял', value: dash(record.subtitle) },
        ];
    }
  });
</script>

<Modal open={true} size="md" title={`Номер «${number}» уже занят`} {onClose}>
  <p class="intro">Этот номер уже есть у другой записи. Сохранить с ним нельзя.</p>
  <dl class="record-card">
    {#each rows as row (row.label)}
      <dt>{row.label}</dt>
      <dd title={row.fullValue ?? undefined}>{row.value}</dd>
    {/each}
  </dl>

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>Закрыть</Button>
    {#if canTakeNext}
      <Button variant="primary" loading={loadingTakeNext} onclick={onTakeNext}>
        Взять следующий свободный
      </Button>
    {/if}
  {/snippet}
</Modal>

<style lang="scss">
  .intro {
    margin: 0 0 var(--tr-space-md);
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
  }

  .record-card {
    display: grid;
    grid-template-columns: 120px 1fr;
    row-gap: var(--tr-space-xs);
    column-gap: var(--tr-space-sm);
    margin: 0;
  }

  dt {
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-secondary);
  }

  dd {
    margin: 0;
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
