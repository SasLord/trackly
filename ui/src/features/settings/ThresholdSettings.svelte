<script lang="ts">
  import { onMount } from 'svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { apiCall } from '$lib/api/client';

  let threshold = $state(2);

  onMount(async () => {
    try {
      threshold = await apiCall<number>('settings_get_low_stock_threshold', {});
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось загрузить порог остатка';
      pushToast('error', msg);
    }
  });

  async function saveThreshold() {
    try {
      await apiCall<void>('settings_set_low_stock_threshold', { threshold });
      pushToast('success', 'Порог обновлён');
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось сохранить порог остатка';
      pushToast('error', msg);
    }
  }
</script>

<section class="settings-section">
  <h2 class="section-title">Порог низкого остатка</h2>

  <div class="threshold-row">
    <label class="form-label" for="threshold-input">
      Уведомлять, когда остаток картриджей модели меньше
    </label>
    <div class="input-group">
      <input
        id="threshold-input"
        class="form-input"
        type="number"
        min="1"
        max="999"
        bind:value={threshold}
        onblur={saveThreshold}
      />
      <span class="input-suffix">штук</span>
    </div>
    <p class="helper-text">
      Значение сохраняется автоматически при потере фокуса.
    </p>
  </div>
</section>

<style lang="scss">
  .settings-section {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-lg);
    max-width: 640px;
  }

  .section-title {
    margin: 0 0 var(--space-md);
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
  }

  .threshold-row {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .form-label {
    font-size: var(--font-size-label);
    font-weight: var(--font-weight-medium);
    color: var(--color-text-secondary);
  }

  .input-group {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
  }

  .form-input {
    width: 80px;
    padding: var(--space-xs) 2px var(--space-xs) var(--space-sm);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-body);
    background: var(--color-bg);
    color: var(--color-text-primary);
    text-align: right;
    appearance: auto;

    &:focus {
      outline: none;
      border-color: var(--color-accent);
      box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-accent) 20%, transparent);
    }
  }

  .input-suffix {
    font-size: var(--font-size-body);
    color: var(--color-text-secondary);
  }

  .helper-text {
    margin: 0;
    font-size: var(--font-size-label);
    color: var(--color-text-muted);
    line-height: 1.5;
  }
</style>
