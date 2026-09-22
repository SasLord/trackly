<script lang="ts">
  // Phase 40.2 Plan 11 (NUM-12, UI-SPEC §6, D-01/D-04): "Проверьте буквы в
  // номере" — третий (последний) попап цепочки проверки, смешение
  // кириллицы/латиницы. Презентационный компонент — данные приходят
  // готовыми через пропсы (планы 13-15), включая структурированное
  // предупреждение о двойнике с сервера.
  import Modal from './Modal.svelte';
  import Button from './Button.svelte';

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
    /** FE-IN-01: серверный `NumberWarningDto.message` — выводится дословно.
     *  Сервер формулирует его под конкретный случай: смешение алфавитов,
     *  двойник без смешения (BE-CR-03) или оба сразу; номер-двойник в нём —
     *  номер существующей записи (BE-WR-09). */
    message: string;
    /** null — сервер не нашёл визуального двойника. Номер двойника нужен
     *  только для моноширинного выделения в тексте сообщения. */
    doppelganger: Doppelganger | null;
    /** «Поправлю» — вернуться в форму без сохранения. */
    onFix: () => void;
    /** «Продолжить» — повтор запроса с флагом подтверждения. */
    onContinue: () => void;
  }

  const { number, message, doppelganger, onFix, onContinue }: Props = $props();

  function escapeRegExp(s: string): string {
    return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  }

  // UI-SPEC §6: номер и номер-двойник в тексте — `.tr-mono`. Сообщение
  // остаётся дословным: только «…»-цитаты, совпадающие с одним из двух
  // номеров, рендерятся моноширинным (нечётные элементы split — номера).
  const segments = $derived.by(() => {
    const numbers = [number.trim(), doppelganger?.number.trim() ?? '']
      .filter((n) => n !== '')
      .map(escapeRegExp);
    if (numbers.length === 0) return [message];
    return message.split(new RegExp(`«(${numbers.join('|')})»`, 'g'));
  });
</script>

<!-- FE-WR-10 (а): UI-SPEC §6 — фокус по умолчанию на «Поправлю» (вторичная кнопка
     подвала), а не на «×»; destructive «Продолжить» не ближайшая к фокусу. -->
<Modal
  open={true}
  size="md"
  initialFocus=".modal-footer .btn-secondary"
  title="Проверьте буквы в номере"
  onClose={onFix}
>
  <p class="body-text">
    {#each segments as part, i (i)}{#if i % 2 === 1}«<span class="tr-mono">{part}</span
        >»{:else}{part}{/if}{/each}
  </p>

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
</style>
