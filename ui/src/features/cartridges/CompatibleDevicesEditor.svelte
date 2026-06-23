<script lang="ts">
  // Plan 12-07: чеклист совместимых принтеров для модели картриджа.
  // Закрывает GAP-12-02 (фронтенд-часть, D-12) — пишет/читает printer_cartridge_models
  // через дуал-транспорт команды из 12-05 (cartridge_models_get_compatible_devices /
  // cartridge_models_set_compatible_devices).
  import Button from '$lib/components/Button.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { cartridges } from './api';
  import { printers } from '../printers/api';
  import type { PrinterDto } from '../../bindings-phase6';

  interface Props {
    cartridgeModelId: number;
  }

  const { cartridgeModelId }: Props = $props();

  let loading = $state(true);
  let saving = $state(false);
  // Полный ростер принтеров (devices type_id=2, через printers.list).
  let printerRoster = $state<PrinterDto[]>([]);
  // Текущее выделение чекбоксов (device id принтеров).
  let checkedIds = $state<Set<number>>(new Set());

  async function load() {
    loading = true;
    try {
      const [list, compat] = await Promise.all([
        printers.list({ status: null, search: null }, { offset: 0, limit: 500 }),
        cartridges.modelsGetCompatibleDevices(cartridgeModelId),
      ]);
      printerRoster = list.items;
      checkedIds = new Set(compat.device_ids);
    } catch {
      printerRoster = [];
      checkedIds = new Set();
      pushToast('error', 'Не удалось загрузить совместимые принтеры');
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void cartridgeModelId;
    void load();
  });

  function toggle(deviceId: number, checked: boolean) {
    const next = new Set(checkedIds);
    if (checked) {
      next.add(deviceId);
    } else {
      next.delete(deviceId);
    }
    checkedIds = next;
  }

  async function handleSave() {
    saving = true;
    try {
      const result = await cartridges.modelsSetCompatibleDevices(
        cartridgeModelId,
        Array.from(checkedIds),
      );
      checkedIds = new Set(result.device_ids);
      pushToast('success', 'Совместимые принтеры сохранены');
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось сохранить совместимые принтеры';
      pushToast('error', msg);
    } finally {
      saving = false;
    }
  }
</script>

<div class="compat-devices-editor">
  {#if loading}
    <div class="loading-row"><Spinner size="sm" /></div>
  {:else if printerRoster.length === 0}
    <p class="muted">Принтеры не найдены — заведите принтер в разделе «Принтеры»</p>
  {:else}
    <ul class="checklist">
      {#each printerRoster as p (p.deviceId)}
        <li class="checklist-row">
          <label class="checklist-label">
            <input
              type="checkbox"
              checked={checkedIds.has(p.deviceId)}
              onchange={(e) => toggle(p.deviceId, (e.currentTarget as HTMLInputElement).checked)}
            />
            <span>{p.deviceName ?? `Принтер #${p.id}`}{p.deviceLocation ? ` (${p.deviceLocation})` : ''}</span>
          </label>
        </li>
      {/each}
    </ul>
    <Button variant="primary" size="sm" loading={saving} onclick={handleSave}>
      {#if saving}Сохранение…{:else}Сохранить{/if}
    </Button>
  {/if}
</div>

<style lang="scss">
  .compat-devices-editor {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
  }

  .loading-row {
    display: flex;
    justify-content: center;
    padding: var(--space-md);
  }

  .muted {
    color: var(--color-text-muted);
    font-size: var(--font-size-body);
    margin: 0;
  }

  .checklist {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
    max-height: 240px;
    overflow-y: auto;
  }

  .checklist-row {
    border-bottom: 1px solid var(--color-border);

    &:last-child {
      border-bottom: none;
    }
  }

  .checklist-label {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    padding: var(--space-xs) 0;
    font-size: var(--font-size-body);
    color: var(--color-text-primary);
    cursor: pointer;
  }
</style>
