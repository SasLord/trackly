<script lang="ts">
  // Переключатель оповещений чата по одной заявке (D-G2). Состояние — серверное
  // (unreadStore.mutedIds, D-G3); эффектов нет, только $derived.
  import { isMuted, toggleMute } from './unreadStore.svelte';

  interface Props {
    requestId: number;
  }

  const { requestId }: Props = $props();

  const muted = $derived(isMuted(requestId));
  const label = $derived(
    muted ? 'Включить оповещения по этой заявке' : 'Отключить оповещения по этой заявке',
  );
</script>

<button
  type="button"
  class="mute"
  class:muted
  aria-pressed={muted}
  aria-label={label}
  title={label}
  onclick={() => void toggleMute(requestId, !muted)}
>
  <svg
    width="18"
    height="18"
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    stroke-width="1.5"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path d="M4 11V7a4 4 0 018 0v4l1 1.5H3L4 11zM6.5 14a1.5 1.5 0 003 0" />
    {#if muted}
      <path d="M2 2l12 12" />
    {/if}
  </svg>
</button>

<style lang="scss">
  .mute {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: none;
    width: 34px;
    height: 34px;
    margin-left: auto;
    padding: 0;
    background: transparent;
    border: none;
    border-radius: var(--tr-radius-md);
    color: var(--tr-text-secondary);
    cursor: pointer;
    outline: none;
    transition:
      background 0.12s,
      box-shadow 0.12s;

    &:hover {
      color: var(--tr-text-primary);
      background: var(--tr-surface-sunken);
    }

    &:focus-visible {
      color: var(--tr-text-primary);
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }

    &.muted {
      color: var(--tr-accent-text);
      background: var(--tr-accent-soft);
    }
  }
</style>
