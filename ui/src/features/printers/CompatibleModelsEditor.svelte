<script lang="ts">
  // Plan 12-07: чеклист совместимых моделей картриджей (kind_id=1) для принтера.
  // Закрывает GAP-12-02 (фронтенд-часть, D-12) — пишет/читает printer_cartridge_models
  // через дуал-транспорт команды из 12-05 (printers_get_compatible_models /
  // printers_set_compatible_models).
  import Button from '$lib/components/Button.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { printers } from './api';
  import { cartridges } from '../cartridges/api';
  import type { CartridgeModelDto } from '../../bindings';

  interface Props {
    deviceId: number;
  }

  const { deviceId }: Props = $props();

  let loading = $state(true);
  let saving = $state(false);
  // Полный ростер моделей картриджей (kind_id=1, фотобарабаны исключены — D-12/D-13).
  let models = $state<CartridgeModelDto[]>([]);
  // Текущее выделение чекбоксов (id моделей).
  let checkedIds = $state<Set<number>>(new Set());

  async function load() {
    loading = true;
    try {
      const [allModels, compat] = await Promise.all([
        cartridges.modelsList(),
        printers.getCompatibleModels(deviceId),
      ]);
      // Клиентский фильтр kind_id === 1 — фотобарабаны (kind_id=2) не участвуют
      // в принтер-картридж совместимости (D-12/D-13 alignment).
      models = allModels.filter((m) => m.kind_id === 1);
      checkedIds = new Set(compat.modelIds);
    } catch {
      models = [];
      checkedIds = new Set();
      pushToast('error', 'Не удалось загрузить совместимые модели картриджей');
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void deviceId;
    void load();
  });

  function toggle(modelId: number, checked: boolean) {
    const next = new Set(checkedIds);
    if (checked) {
      next.add(modelId);
    } else {
      next.delete(modelId);
    }
    checkedIds = next;
  }

  async function handleSave() {
    saving = true;
    try {
      const result = await printers.setCompatibleModels({
        deviceId,
        modelIds: Array.from(checkedIds),
      });
      checkedIds = new Set(result.modelIds);
      pushToast('success', 'Совместимые модели картриджей сохранены');
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось сохранить совместимые модели картриджей';
      pushToast('error', msg);
    } finally {
      saving = false;
    }
  }
</script>

<div class="compat-models-editor">
  {#if loading}
    <div class="loading-row"><Spinner size="sm" /></div>
  {:else if models.length === 0}
    <p class="muted">Нет моделей картриджей — добавьте модель в разделе «Картриджи»</p>
  {:else}
    <ul class="checklist">
      {#each models as m (m.id)}
        <li class="checklist-row">
          <label class="checklist-label">
            <input
              type="checkbox"
              checked={checkedIds.has(m.id)}
              onchange={(e) => toggle(m.id, (e.currentTarget as HTMLInputElement).checked)}
            />
            <span>{m.brand} {m.model}</span>
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
  .compat-models-editor {
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
