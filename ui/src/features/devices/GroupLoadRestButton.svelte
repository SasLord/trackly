<script lang="ts">
  // Кнопка «загрузить весь состав» усечённой группы (фаза 41.9, D-09/D-10,
  // UI-SPEC §11). Одна и та же для строки АРМ и якорной строки.
  import Button from '$lib/components/Button.svelte';
  import RowReload from '$lib/components/icons/RowReload.svelte';

  interface Props {
    groupName: string;
    shown: number;
    total: number;
    loading: boolean;
    onclick: () => void;
  }

  const { groupName, shown, total, loading, onclick }: Props = $props();

  // Строка не должна реагировать на клик по кнопке.
  function stop(e: MouseEvent) {
    e.stopPropagation();
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<span class="load-rest" onclick={stop}>
  <Button
    variant="ghost"
    size="sm"
    iconOnly
    ariaLabel={'Загрузить весь состав группы «' + groupName + '»'}
    title={'Загрузить весь состав: показано ' + shown + ' из ' + total}
    {loading}
    {onclick}
  >
    <RowReload />
  </Button>
</span>

<style lang="scss">
  .load-rest {
    display: inline-flex;
    flex-shrink: 0;
  }
</style>
