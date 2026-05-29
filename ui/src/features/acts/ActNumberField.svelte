<script lang="ts">
  // Plan 03-02: специальный input для поля № в ActFormModal.
  // Badge «авто» / «override» (warning) + ссылка «Следующий» (только в override-mode).
  // На mount — запрашивает acts.peekNextNumber().
  import { onMount } from 'svelte';
  import Input from '$lib/components/Input.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import Button from '$lib/components/Button.svelte';
  import { acts } from './api';
  import { pushToast } from '$lib/stores/toast.svelte';

  interface Props {
    /** When `null` → "auto" mode (use server-suggested number). When `number` → override. */
    value: number | null;
    onChange: (_v: number | null) => void;
    invalid?: boolean;
    errorMessage?: string | null;
  }

  let { value = $bindable(null), onChange, invalid = false, errorMessage = null }: Props = $props();

  let predicted = $state<number | null>(null);
  let displayValue = $state('');
  let loadingPredicted = $state(false);

  const isOverride = $derived(value !== null);

  onMount(async () => {
    loadingPredicted = true;
    try {
      const n = await acts.peekNextNumber();
      predicted = n;
      if (value === null) {
        displayValue = String(n);
      } else {
        displayValue = String(value);
      }
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось получить следующий номер';
      pushToast('error', msg);
    } finally {
      loadingPredicted = false;
    }
  });

  function handleInput(v: string) {
    displayValue = v;
    const parsed = parseInt(v, 10);
    if (!Number.isFinite(parsed) || parsed <= 0) {
      onChange(null);
      return;
    }
    if (predicted !== null && parsed === predicted) {
      // User typed exactly the predicted value → still auto.
      onChange(null);
    } else {
      onChange(parsed);
    }
  }

  function resetToAuto() {
    if (predicted === null) return;
    displayValue = String(predicted);
    onChange(null);
  }
</script>

<div class="num-field" class:has-error={invalid}>
  <div class="control-row">
    <div class="input-wrap">
      <Input
        id="act-number"
        type="number"
        value={displayValue}
        placeholder={loadingPredicted ? 'Загрузка…' : 'Например, 42'}
        oninput={handleInput}
        {invalid}
      />
    </div>
    <span class="badge">
      {#if isOverride}
        <Badge variant="warning">override</Badge>
      {:else}
        <Badge variant="default">авто</Badge>
      {/if}
    </span>
    {#if isOverride && predicted !== null}
      <Button variant="link" size="sm" onclick={resetToAuto}>Следующий</Button>
    {/if}
  </div>
  <p class="hint">
    {#if isOverride}
      Будет записано в журнал событий.
    {:else}
      Следующий по порядку. Можно изменить.
    {/if}
  </p>
  {#if invalid && errorMessage}
    <p class="error">{errorMessage}</p>
  {/if}
</div>

<style lang="scss">
  .num-field {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }
  .control-row {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }
  .input-wrap {
    flex: 1;
    max-width: 200px;
  }
  .badge {
    flex-shrink: 0;
  }
  .hint {
    margin: 0;
    font-size: var(--font-size-label);
    color: var(--color-text-muted);
  }
  .error {
    margin: 0;
    font-size: var(--font-size-label);
    color: var(--color-destructive);
  }
</style>
