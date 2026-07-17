<script lang="ts">
  import { toastStore, removeToast } from '$lib/stores/toast.svelte';
  import Toast from './Toast.svelte';
</script>

<div class="toast-host" aria-live="polite" aria-atomic="false">
  {#each toastStore.items as item (item.id)}
    <Toast kind={item.kind} message={item.message} onClose={() => removeToast(item.id)} />
  {/each}
</div>

<style lang="scss">
  .toast-host {
    position: fixed;
    bottom: var(--tr-space-xl);
    right: var(--tr-space-xl);
    z-index: 1000;
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
    max-width: 400px;
    pointer-events: none;

    :global(.toast) {
      pointer-events: all;
    }
  }
</style>
