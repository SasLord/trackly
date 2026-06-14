<script lang="ts">
  // Plan 06-05: модалка создания заявки.
  // Тип-переключатель: «Замена картриджа» | «Свободная форма».
  // По паттерну CartridgeFormModal.svelte.
  // D-Req-Form-01: сотрудник не выбирает модель картриджа (только принтер).
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Select from '$lib/components/Select.svelte';
  import Textarea from '$lib/components/Textarea.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { requests } from './api';
  import { printers } from '../printers/api';
  import type { PrinterDto } from '../../bindings-phase6';

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

  // Available printers list
  let availablePrinters = $state<PrinterDto[]>([]);
  let printersLoading = $state(false);

  // Fixed category list (D-Req-Categories-01)
  const CATEGORIES = [
    { id: 1, label: 'Ремонт техники' },
    { id: 2, label: 'Расходные материалы' },
    { id: 3, label: 'Программное обеспечение' },
    { id: 4, label: 'Прочее' },
  ];

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
      // Load printers list.
      loadPrinters();
    }
    _wasOpen = isOpen;
  });

  async function loadPrinters() {
    printersLoading = true;
    try {
      const resp = await printers.list({ status: null, search: null }, { offset: 0, limit: 200 });
      availablePrinters = resp.items;
    } catch {
      // Non-fatal — printers list stays empty.
    } finally {
      printersLoading = false;
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
          <Select
            value={printerDeviceId !== null ? String(printerDeviceId) : ''}
            id="req-printer"
            invalid={!!printerError}
            onchange={(v) => {
              printerDeviceId = v ? parseInt(v, 10) : null;
              printerError = '';
            }}
          >
            <option value="">Выберите принтер</option>
            {#each availablePrinters as p (p.id)}
              <option value={String(p.id)}>{p.deviceName ?? p.ipAddress ?? `Принтер #${p.id}`}</option>
            {/each}
          </Select>
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
            {#each CATEGORIES as cat (cat.id)}
              <option value={String(cat.id)}>{cat.label}</option>
            {/each}
          </Select>
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
    gap: var(--space-md);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .label {
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
    font-weight: var(--font-weight-regular);
  }

  .type-toggle {
    display: flex;
    gap: 0;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .type-btn {
    flex: 1;
    padding: var(--space-xs) var(--space-md);
    background: transparent;
    color: var(--color-text-primary);
    border: none;
    font-family: var(--font-family-base);
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-regular);
    cursor: pointer;
    height: 36px;
    transition: background 0.1s;

    &:not(:last-child) {
      border-right: 1px solid var(--color-border);
    }

    &:hover {
      background: var(--color-surface-sunken);
    }

    &:focus-visible {
      outline: none;
      box-shadow: inset 0 0 0 2px var(--color-accent);
    }

    &.active {
      background: color-mix(in srgb, var(--color-accent) 12%, transparent);
      color: var(--color-text-primary);
      font-weight: var(--font-weight-semibold);
    }
  }

  .field-hint {
    font-size: var(--font-size-label);
    color: var(--color-text-muted);
  }

  .field-error {
    font-size: var(--font-size-label);
    color: var(--color-destructive);
  }
</style>
