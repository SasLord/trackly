<script lang="ts">
  import { onMount } from 'svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { apiCall } from '$lib/api/client';
  import Input from '$lib/components/Input.svelte';
  import Radio from '$lib/components/Radio.svelte';

  let threshold = $state(2);
  // Дефолт на непронастроенной БД — «по модели принтера» (CONTEXT «Хранение настройки»).
  let basis = $state<'cartridge_model' | 'printer_model'>('printer_model');

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

    try {
      const loaded = await apiCall<string>('settings_get_low_stock_basis', {});
      basis = loaded === 'cartridge_model' ? 'cartridge_model' : 'printer_model';
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось загрузить базу подсчёта остатка';
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

  async function saveBasis() {
    try {
      await apiCall<void>('settings_set_low_stock_basis', { basis });
      pushToast('success', 'База подсчёта обновлена');
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось сохранить базу подсчёта остатка';
      pushToast('error', msg);
    }
  }
</script>

<section class="settings-section">
  <h2 class="section-title">Порог низкого остатка</h2>

  <div class="basis-row">
    <span class="form-label">База подсчёта низкого остатка</span>
    <!--
      Radio.svelte's bind:group listens on the native "change" event, which
      DOES bubble (unlike "blur") — Svelte updates the bound value
      synchronously on that same event before this wrapper's onchange runs,
      so saveBasis reads the already-updated value. Same wrapper-listens-to-
      bubbled-native-event trick as the threshold <Input>'s onfocusout below.
    -->
    <div class="radio-group" onchange={saveBasis}>
      <div class="radio-label">
        <Radio bind:group={basis} value="printer_model">
          <span class="radio-text">
            <span class="radio-title">По модели принтера</span>
            <span class="helper-text">
              Считать нехватку по совместимым моделям принтеров — разные картриджи, подходящие
              одному принтеру, суммируются вместе.
            </span>
          </span>
        </Radio>
      </div>
      <div class="radio-label">
        <Radio bind:group={basis} value="cartridge_model">
          <span class="radio-text">
            <span class="radio-title">По модели картриджа</span>
            <span class="helper-text">Считать нехватку отдельно для каждой модели картриджа.</span
            >
          </span>
        </Radio>
      </div>
    </div>
  </div>

  <div class="threshold-row">
    <label class="form-label" for="threshold-input">
      {basis === 'cartridge_model'
        ? 'Уведомлять, когда остаток картриджей модели меньше'
        : 'Уведомлять, когда остаток картриджей, совместимых с моделью принтера, меньше'}
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

  .basis-row {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
    margin-bottom: var(--tr-space-lg);
  }

  .radio-group {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
  }

  .radio-label {
    margin-top: var(--tr-space-xs);
  }

  .radio-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .radio-title {
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-medium);
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
