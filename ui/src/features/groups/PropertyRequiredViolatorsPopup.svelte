<script lang="ts">
  // Phase 41 Plan 20 (D-14, UI-SPEC 9.5): попап «Нельзя сделать свойство
  // обязательным». Презентационный (образец — NumberTakenPopup): все данные
  // приходят через props, собственных запросов нет. Список нарушителей приходит
  // от сервера готовым, UI его не вычисляет.
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import { pluralizeRu } from '$lib/utils/pluralize';

  interface Props {
    /** Группы-нарушители (первые, что есть); показывается не более MAX_ROWS. */
    groups: { id: number; name: string }[];
    /** Сколько нарушителей всего (может быть больше groups.length). */
    total: number;
    /** «Открыть»: страница переходит-фокусом на группу и закрывает попап. */
    onOpen: (_id: number) => void;
    onClose: () => void;
  }

  const { groups, total, onOpen, onClose }: Props = $props();

  const MAX_ROWS = 20;

  const shown = $derived(groups.slice(0, MAX_ROWS));
  const rest = $derived(Math.max(0, total - shown.length));
  const totalWord = $derived(pluralizeRu(total, ['группы', 'групп', 'групп']));
  const restWord = $derived(pluralizeRu(rest, ['группа', 'группы', 'групп']));
</script>

<Modal open={true} size="md" title="Нельзя сделать свойство обязательным" {onClose}>
  <p class="intro">
    У {total}
    {totalWord} это свойство не заполнено. Заполните его или оставьте свойство необязательным.
  </p>
  <ul class="violators">
    {#each shown as g (g.id)}
      <li class="violator">
        <span class="violator-name" title={g.name}>{g.name}</span>
        <Button variant="link" onclick={() => onOpen(g.id)}>Открыть</Button>
      </li>
    {/each}
  </ul>
  {#if rest > 0}
    <p class="rest">и ещё {rest} {restWord}</p>
  {/if}

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>Закрыть</Button>
  {/snippet}
</Modal>

<style lang="scss">
  .intro {
    margin: 0 0 var(--tr-space-md);
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
  }

  .violators {
    list-style: none;
    margin: 0;
    padding: 0;
    // Не более 20 строк: свой скролл, чтобы попап не вырастал за экран.
    max-height: 320px;
    overflow-y: auto;
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-sm);
  }

  .violator {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--tr-space-sm);
    padding: var(--tr-space-2xs) var(--tr-space-sm);
    border-bottom: 1px solid var(--tr-border);

    &:last-child {
      border-bottom: none;
    }
  }

  .violator-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
  }

  .rest {
    margin: var(--tr-space-xs) 0 0;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
  }
</style>
