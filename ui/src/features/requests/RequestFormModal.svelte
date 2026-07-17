<script lang="ts">
  // Plan 06-05: модалка создания заявки.
  // Тип-переключатель: «Замена картриджа» | «Свободная форма».
  // По паттерну CartridgeFormModal.svelte.
  // D-Req-Form-01: сотрудник не выбирает модель картриджа (только принтер).
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Select from '$lib/components/Select.svelte';
  import GroupedPrinterSelect from '$lib/components/GroupedPrinterSelect.svelte';
  import Textarea from '$lib/components/Textarea.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { requests } from './api';
  import type { RequestCategoryDto, RequestPrinterOptionDto } from '../../bindings-phase6';

  interface Props {
    open: boolean;
    onClose: () => void;
    onSuccess: () => void;
  }

  const { open, onClose, onSuccess }: Props = $props();

  type RequestType = 'cartridge_replace' | 'free_form';

  // Form state
  let requestType = $state<RequestType>('cartridge_replace');
  let printerDeviceId = $state<number | null>(null);
  let description = $state('');
  let categoryId = $state<number | null>(null);
  let submitting = $state(false);

  // Validation errors
  let printerError = $state('');
  let descError = $state('');

  // Available printers list — minimal {id,name,location} DTO from the
  // CreateRequest-gated request_printer_options endpoint (D-PRN-01). This
  // replaced the closed devices listing call for printers (Phase 10 BFLA fix
  // emptied this list for Employee since ReadData/ReadPrinters got gated).
  let availablePrinters = $state<RequestPrinterOptionDto[]>([]);
  let printersLoading = $state(false);

  // Category list — loaded from the server {id, name} endpoint (D-CAT-01).
  // No more hardcoded array — request_categories rows can change without a
  // frontend redeploy.
  let categories = $state<RequestCategoryDto[]>([]);
  let categoriesLoading = $state(false);

  // Form instance counter — resets form on each open.
  let openInstanceCounter = $state(0);
  let _wasOpen = $state(false);

  $effect(() => {
    const isOpen = open;
    if (isOpen && !_wasOpen) {
      openInstanceCounter += 1;
      // Reset form state on open.
      requestType = 'cartridge_replace';
      printerDeviceId = null;
      description = '';
      categoryId = null;
      printerError = '';
      descError = '';
      // Load printers + categories list.
      loadPrinters();
      loadCategories();
    }
    _wasOpen = isOpen;
  });

  async function loadPrinters() {
    // D-PRN-01: minimal {id,name,location} list from the CreateRequest-gated
    // endpoint — every role (incl. Employee) can call this, unlike the
    // closed devices listing call which needs ReadData/ReadPrinters.
    printersLoading = true;
    try {
      availablePrinters = await requests.printerOptions();
    } catch {
      // Non-fatal — printers list stays empty.
    } finally {
      printersLoading = false;
    }
  }

  async function loadCategories() {
    // D-CAT-01: server-driven {id, name} list — no hardcoded array.
    categoriesLoading = true;
    try {
      categories = await requests.listCategories();
    } catch {
      // Non-fatal — categories list stays empty.
    } finally {
      categoriesLoading = false;
    }
  }

  function validate(): boolean {
    let valid = true;
    printerError = '';
    descError = '';

    if (requestType === 'cartridge_replace') {
      if (printerDeviceId === null) {
        printerError = 'Выберите принтер';
        valid = false;
      }
    } else {
      if (!description.trim()) {
        descError = 'Опишите вашу заявку';
        valid = false;
      }
    }
    return valid;
  }

  // D-WS-01 / Pitfall 4: ask for Notification permission only off the back of
  // a genuine user gesture (submitting a request) — never on page load/mount.
  // 'default' = user has neither granted nor denied yet; asking again after
  // 'denied' would be a no-op browsers ignore, and asking when already
  // 'granted' is pointless — so this only fires once, the first time.
  function maybeRequestNotifyPermission() {
    if (
      'Notification' in window &&
      window.isSecureContext &&
      Notification.permission === 'default'
    ) {
      void Notification.requestPermission();
    }
  }

  async function handleSubmit() {
    if (submitting) return;
    if (!validate()) return;

    submitting = true;
    try {
      await requests.create({
        requestType,
        printerDeviceId: requestType === 'cartridge_replace' ? printerDeviceId : null,
        cartridgeModelId: null,
        categoryId: requestType === 'free_form' ? categoryId : null,
        description: description.trim() || null,
      });
      pushToast('success', 'Заявка отправлена');
      maybeRequestNotifyPermission();
      onSuccess();
      onClose();
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось создать заявку. Повторите попытку.';
      pushToast('error', msg);
    } finally {
      submitting = false;
    }
  }
</script>

<Modal {open} title="Новая заявка" size="md" {onClose}>
  {#key openInstanceCounter}
    <form
      class="form"
      onsubmit={(e) => {
        e.preventDefault();
        handleSubmit();
      }}
    >
      <!-- Тип-переключатель -->
      <div class="field">
        <span class="label" id="req-type-label">Тип заявки</span>
        <div class="type-toggle" role="group" aria-labelledby="req-type-label">
          <button
            type="button"
            class="type-btn"
            class:active={requestType === 'cartridge_replace'}
            onclick={() => {
              requestType = 'cartridge_replace';
              printerError = '';
              descError = '';
            }}
            aria-pressed={requestType === 'cartridge_replace'}
          >
            Замена картриджа
          </button>
          <button
            type="button"
            class="type-btn"
            class:active={requestType === 'free_form'}
            onclick={() => {
              requestType = 'free_form';
              printerError = '';
              descError = '';
            }}
            aria-pressed={requestType === 'free_form'}
          >
            Свободная форма
          </button>
        </div>
      </div>

      {#if requestType === 'cartridge_replace'}
        <!-- Принтер (обязательно) -->
        <div class="field">
          <label class="label" for="req-printer">Принтер</label>
          <GroupedPrinterSelect
            options={availablePrinters}
            value={printerDeviceId !== null ? String(printerDeviceId) : ''}
            id="req-printer"
            invalid={!!printerError}
            onchange={(v) => {
              printerDeviceId = v ? parseInt(v, 10) : null;
              printerError = '';
            }}
          />
          {#if printerError}
            <span class="field-error">{printerError}</span>
          {/if}
          {#if printersLoading && availablePrinters.length === 0}
            <span class="field-hint">Загрузка принтеров…</span>
          {/if}
        </div>

        <!-- Комментарий (опционально) -->
        <div class="field">
          <label class="label" for="req-comment">Комментарий</label>
          <Textarea
            value={description}
            placeholder="Опишите проблему (необязательно)"
            id="req-comment"
            oninput={(v) => (description = v)}
          />
        </div>
      {:else}
        <!-- Категория (опционально) -->
        <div class="field">
          <label class="label" for="req-category">Категория</label>
          <Select
            value={categoryId !== null ? String(categoryId) : ''}
            id="req-category"
            onchange={(v) => {
              categoryId = v ? parseInt(v, 10) : null;
            }}
          >
            <option value="">Без категории</option>
            {#each categories as cat (cat.id)}
              <option value={String(cat.id)}>{cat.name}</option>
            {/each}
          </Select>
          {#if categoriesLoading && categories.length === 0}
            <span class="field-hint">Загрузка категорий…</span>
          {/if}
        </div>

        <!-- Описание (обязательно) -->
        <div class="field">
          <label class="label" for="req-desc">Описание</label>
          <Textarea
            value={description}
            placeholder="Опишите вашу заявку"
            id="req-desc"
            invalid={!!descError}
            oninput={(v) => {
              description = v;
              descError = '';
            }}
          />
          {#if descError}
            <span class="field-error">{descError}</span>
          {/if}
        </div>
      {/if}
    </form>
  {/key}

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>Отмена</Button>
    <Button variant="primary" loading={submitting} onclick={handleSubmit}>Отправить заявку</Button>
  {/snippet}
</Modal>

<style lang="scss">
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  .label {
    font-size: var(--font-size-label);
    color: var(--tr-text-secondary);
    font-weight: var(--font-weight-regular);
  }

  .type-toggle {
    display: flex;
    gap: 0;
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
    overflow: hidden;
  }

  .type-btn {
    flex: 1;
    padding: var(--tr-space-2xs) var(--tr-space-md);
    background: transparent;
    color: var(--tr-text-primary);
    border: none;
    font-family: var(--font-family-base);
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-regular);
    cursor: pointer;
    height: 36px;
    transition: background 0.1s;

    &:not(:last-child) {
      border-right: 1px solid var(--tr-border);
    }

    &:hover {
      background: var(--tr-surface-sunken);
    }

    &:focus-visible {
      outline: none;
      box-shadow: inset 0 0 0 2px var(--tr-accent);
    }

    &.active {
      background: color-mix(in srgb, var(--tr-accent) 12%, transparent);
      color: var(--tr-text-primary);
      font-weight: var(--font-weight-semibold);
    }
  }

  .field-hint {
    font-size: var(--font-size-label);
    color: var(--tr-text-tertiary);
  }

  .field-error {
    font-size: var(--font-size-label);
    color: var(--tr-danger);
  }
</style>
