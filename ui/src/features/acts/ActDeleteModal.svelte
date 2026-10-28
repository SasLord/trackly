<script lang="ts">
  // Подтверждение удаления акта. Заменяет нативный диалог браузера: в
  // десктопном webview он до пользователя не доходит, акт удалялся без
  // подтверждения, а предупреждение о последствиях оставалось невидимым.
  //
  // Презентационный компонент: сам ничего не удаляет и в API не ходит.
  // Решение принимает родитель (ActsPage): onConfirm запускает удаление, а
  // deleting блокирует кнопки и закрытие окна, пока запрос не завершён.
  // Монтируется родителем через {#if}.
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import type { ActDto } from '../../bindings';

  interface Props {
    act: ActDto;
    /** Идёт запрос на удаление: кнопки заблокированы, окно не закрывается. */
    deleting?: boolean;
    onCancel: () => void;
    onConfirm: () => void;
  }

  const { act, deleting = false, onCancel, onConfirm }: Props = $props();

  const isReturn = $derived(act.act_type === 'return');
  const title = $derived(
    isReturn ? `Удалить акт возврата №${act.number}?` : `Удалить акт №${act.number}?`,
  );
  const body = $derived(
    isReturn
      ? 'Акт будет помечен как удалённый. Состояние и Расположение устройств вернутся к значениям на момент выдачи. Если parent был в Архиве — выйдет из архива. Устройства, выведенные этим возвратом из состава групп, вернутся в свои группы.'
      : 'Акт будет помечен как удалённый. Все устройства из акта вернутся на склад в исходные Состояние и Расположение (на момент выдачи). Связанные возвраты также будут отменены. Устройства, выведенные этим актом из состава групп, вернутся в свои группы. Действие можно отменить только восстановлением из бэкапа БД.',
  );

  // Во время запроса Esc, клик по фону и крестик окно не закрывают.
  function handleClose() {
    if (!deleting) onCancel();
  }
</script>

<!-- Начальный фокус на «Отмена»: действие деструктивное, случайный Enter не
     должен удалять акт. -->
<Modal
  open={true}
  {title}
  size="md"
  initialFocus=".modal-footer .btn-secondary"
  onClose={handleClose}
>
  <p class="modal-body-text">{body}</p>

  {#snippet footer()}
    <Button variant="secondary" disabled={deleting} onclick={onCancel}>Отмена</Button>
    <Button variant="destructive" loading={deleting} disabled={deleting} onclick={onConfirm}>
      Удалить
    </Button>
  {/snippet}
</Modal>

<style lang="scss">
  .modal-body-text {
    margin: 0 0 var(--tr-space-md) 0;
    color: var(--tr-text-primary);
  }
</style>
