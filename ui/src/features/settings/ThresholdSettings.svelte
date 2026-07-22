<script lang="ts">
  import { onMount } from 'svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { apiCall } from '$lib/api/client';
  import Input from '$lib/components/Input.svelte';

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
    <!--
      Input.svelte has no onblur prop and does not forward arbitrary DOM events
      to the inner <input>. Native "blur" does not bubble, so onblur on this
      wrapper would never fire. "focusout" DOES bubble by spec, so wrapping
      the primitive in onfocusout preserves the "save on blur" behavior.
    -->
    <div class="input-group" onfocusout={saveThreshold}>
      <div class="threshold-input-wrap">
        <Input
          id="threshold-input"
          type="number"
          value={String(threshold)}
          oninput={(v) => (threshold = Number(v) || 0)}
        />
      </div>
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
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .threshold-row {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
  }

  .form-label {
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-secondary);
  }

  .input-group {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
  }

  .threshold-input-wrap {
    width: 80px;
  }

  .input-suffix {
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-secondary);
  }

  .helper-text {
    margin: 0;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
    line-height: 1.5;
  }
</style>
