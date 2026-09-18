<script lang="ts">
  // Phase 40.2 Plan 11 (NUM-12, UI-SPEC §6, D-01/D-04): "Проверьте буквы в
  // номере" — третий (последний) попап цепочки проверки, смешение
  // кириллицы/латиницы. Презентационный компонент — данные приходят
  // готовыми через пропсы (планы 13-15), включая структурированное
  // предупреждение о двойнике с сервера.
  import Modal from './Modal.svelte';
  import Button from './Button.svelte';

  // UI-SPEC Copywriting Contract: "Виды строчными: устройство, принтер,
  // картридж, фотобарабан, акт" — тот же список видов, что и в живой
  // подсказке "Занят: …" у NumberTemplateField.
  const KIND_LABEL_LOWER: Record<string, string> = {
    device: 'устройство',
    printer: 'принтер',
    cartridge: 'картридж',
    drum: 'фотобарабан',
    act: 'акт',
  };

  interface DoppelgangerRecord {
    kind: string;
    title: string;
  }

  interface Doppelganger {
    number: string;
    record: DoppelgangerRecord;
  }

  interface Props {
    number: string;
    /** null — сервер не нашёл визуального двойника, второй абзац не рендерится. */
    doppelganger: Doppelganger | null;
    /** «Поправлю» — вернуться в форму без сохранения. */
    onFix: () => void;
    /** «Продолжить» — повтор запроса с флагом подтверждения. */
    onContinue: () => void;
  }

  const { number, doppelganger, onFix, onContinue }: Props = $props();

  function kindLabel(kind: string): string {
    return KIND_LABEL_LOWER[kind] ?? kind;
  }
</script>

<Modal open={true} size="md" title="Проверьте буквы в номере" onClose={onFix}>
  <p class="body-text">
    В номере «<span class="tr-mono">{number}</span>» смешаны русские и латинские буквы. Внешне
    одинаковые буквы, например «О» и «O», считаются разными — такой номер легко перепутать.
  </p>
  {#if doppelganger !== null}
    <p class="body-text body-text-second">
      Он выглядит так же, как существующий номер «<span class="tr-mono">{doppelganger.number}</span
      >» ({kindLabel(doppelganger.record.kind)} «{doppelganger.record.title}»), но записан другими
      буквами.
    </p>
  {/if}

  {#snippet footer()}
    <Button variant="secondary" onclick={onFix}>Поправлю</Button>
    <Button variant="destructive" onclick={onContinue}>Продолжить</Button>
  {/snippet}
</Modal>

<style lang="scss">
  .body-text {
    margin: 0;
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
  }

  .body-text-second {
    margin-top: var(--tr-space-md);
  }
</style>
