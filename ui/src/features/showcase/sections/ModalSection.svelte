<script lang="ts">
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Input from '$lib/components/Input.svelte';

  let open = $state(false);
  let demoValue = $state('Пример значения');
</script>

<section class="modal-section">
  <h2>Модальное окно</h2>

  <Button onclick={() => (open = true)}>Показать модал</Button>

  <Modal {open} title="Демонстрация модального окна" onClose={() => (open = false)}>
    <p>
      Это тело модального окна — здесь может находиться произвольный контент, например форма или
      подтверждающий текст.
    </p>
    <Input bind:value={demoValue} placeholder="Введите значение" />

    {#snippet footer()}
      <Button variant="secondary" onclick={() => (open = false)}>Отмена</Button>
      <Button variant="primary" onclick={() => (open = false)}>Подтвердить</Button>
    {/snippet}
  </Modal>
</section>

<style lang="scss">
  .modal-section {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-lg);
    align-items: flex-start;
  }

  h2 {
    margin: 0;
    font-size: var(--tr-font-size-h2);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }
</style>
