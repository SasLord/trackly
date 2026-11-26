<script lang="ts">
  // Счётчик непрочитанного (UI-SPEC §11.1): единственный вид во всех местах.
  // Ноль и меньше — ничего; больше 99 — «99+».
  import Badge from './Badge.svelte';

  interface Props {
    count: number;
    /** Шаблон aria-label, например «Непрочитанных в чате». */
    label?: string;
  }

  const { count, label = 'Непрочитанных' }: Props = $props();

  const text = $derived(count > 99 ? '99+' : String(count));
</script>

{#if count > 0}
  <span class="unread-badge" aria-label={`${label}: ${count}`}>
    <Badge appearance="solid" variant="accent" size="sm">{text}</Badge>
  </span>
{/if}

<style lang="scss">
  .unread-badge {
    display: inline-flex;
    align-items: center;
    flex: none;
  }
</style>
