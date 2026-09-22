<script lang="ts">
  // Phase 40.2 Plan 11 (NUM-11, UI-SPEC §6, D-01/D-04): "Не соответствует
  // шаблону" — второй попап цепочки проверки, показывается только при
  // активном шаблоне и только в попапах создания. Презентационный
  // компонент — данные приходят готовыми через пропсы (планы 13-15).
  import Modal from './Modal.svelte';
  import Button from './Button.svelte';

  interface Props {
    number: string;
    mask: string;
    /** Название попапа-контекста для второго предложения тела, например
     *  «Новый акт» (см. UI-SPEC «Названия контекстов», ровно 5 вариантов). */
    contextLabel: string;
    /** «Поправлю» — вернуться в форму без сохранения. */
    onFix: () => void;
    /** «Продолжить» — сохранить как есть, цепочка идёт дальше. */
    onContinue: () => void;
  }

  const { number, mask, contextLabel, onFix, onContinue }: Props = $props();
</script>

<!-- FE-WR-10 (а): UI-SPEC §6 — фокус по умолчанию на «Поправлю» (вторичная кнопка
     подвала), а не на «×»; destructive «Продолжить» не ближайшая к фокусу. -->
<Modal
  open={true}
  size="md"
  initialFocus=".modal-footer .btn-secondary"
  title="Номер не соответствует шаблону"
  onClose={onFix}
>
  <p class="body-text">
    Номер «<span class="tr-mono">{number}</span>» не подходит под шаблон «<span class="tr-mono"
      >{mask}</span
    >». Сохранить как есть? Следующее окно «{contextLabel}» откроется без шаблона.
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
