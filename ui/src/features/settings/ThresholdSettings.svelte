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
    <p class="helper-text">Значение сохраняется автоматически при потере фокуса.</p>
  </div>
</section>

<style lang="scss">
  .settings-section {
    background: var(--tr-surface);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    padding: var(--tr-space-xl);
    max-width: 640px;
  }

  .section-title {
    margin: 0 0 var(--tr-space-md);
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .threshold-row {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
  }

  .form-label {
    font-size: var(--font-size-label);
    font-weight: var(--font-weight-medium);
    color: var(--tr-text-secondary);
  }

  .input-group {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
  }

  .form-input {
    width: 80px;
    padding: var(--tr-space-2xs) 2px var(--tr-space-2xs) var(--tr-space-xs);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
    font-size: var(--font-size-body);
    background: var(--tr-bg);
    color: var(--tr-text-primary);
    text-align: right;
    appearance: auto;

    &:focus {
      outline: none;
      border-color: var(--tr-accent);
      box-shadow: 0 0 0 2px color-mix(in srgb, var(--tr-accent) 20%, transparent);
    }
  }

  .input-suffix {
    font-size: var(--font-size-body);
    color: var(--tr-text-secondary);
  }

  .helper-text {
    margin: 0;
    font-size: var(--font-size-label);
    color: var(--tr-text-tertiary);
    line-height: 1.5;
  }
</style>
