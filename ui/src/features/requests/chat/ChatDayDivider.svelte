<script lang="ts">
  // Разделитель дня в ленте (UI-SPEC §8.3): «таблетка», липнет к верху ленты.
  interface Props {
    label: string;
  }
  const { label }: Props = $props();
</script>

<li class="day-divider" role="separator">
  <span class="pill">{label}</span>
</li>

<style lang="scss">
  .day-divider {
    position: sticky;
    top: 8px;
    z-index: 1;
    display: flex;
    justify-content: center;
    list-style: none;
    margin: var(--tr-space-md) 0;
    pointer-events: none;
  }
  .pill {
    display: inline-flex;
    align-items: center;
    height: 24px;
    padding: 0 var(--tr-space-sm);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-full);
    background: var(--tr-surface);
    font: var(--tr-text-caption);
    color: var(--tr-text-secondary);
    white-space: nowrap;
    text-align: center;
  }
</style>
