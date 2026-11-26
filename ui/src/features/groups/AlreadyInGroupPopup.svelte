<script lang="ts">
  // Фаза 41.8, план 09 (D-16, UI-SPEC §10.1): подтверждение «уже в группе».
  // Два режима: отдельная Modal (embedded=false) или встроенный шаг внутри чужой
  // Modal (embedded=true) — модалка над модалкой запрещена. Операцию переноса
  // выполняет родитель в onConfirm; здесь только текст, фокус и состояние запроса.
  //
  // Тело задаёт white-space: normal и text-align: left: Modal может рендериться
  // внутри <td>, где наследуются nowrap и выравнивание по центру.
  import Modal from '@trackly/kit/components/Modal.svelte';
  import Button from '@trackly/kit/components/Button.svelte';
  import { onMount } from 'svelte';

  interface Props {
    kind: 'device' | 'group' | 'detail';
    subjectName: string;
    currentGroupName: string;
    targetName: string;
    /** Место цели; callout показывается только если оно задано. */
    targetPlaceLabel?: string | null;
    onConfirm: () => Promise<void>;
    onCancel: () => void;
    embedded?: boolean;
  }

  const {
    kind,
    subjectName,
    currentGroupName,
    targetName,
    targetPlaceLabel = null,
    onConfirm,
    onCancel,
    embedded = false,
  }: Props = $props();

  let saving = $state(false);
  let serverErr = $state<string | null>(null);
  let cancelBtn = $state<HTMLElement | null>(null);

  const title = $derived(
    kind === 'device'
      ? 'Устройство уже в группе'
      : kind === 'group'
        ? 'Группа уже вложена'
        : 'Деталь уже в группе',
  );

  const body = $derived(
    kind === 'device'
      ? `Устройство «${subjectName}» сейчас входит в группу «${currentGroupName}». Переместить его в «${targetName}»?`
      : kind === 'group'
        ? `Группа «${subjectName}» сейчас вложена в «${currentGroupName}». Переместить её в «${targetName}»?`
        : `Деталь «${subjectName}» сейчас входит в группу «${currentGroupName}». Переместить её обратно в «${targetName}»?`,
  );

  const callout = $derived(
    targetPlaceLabel
      ? kind === 'group'
        ? `Группа и её состав переедут в «${targetPlaceLabel}».`
        : `${kind === 'detail' ? 'Деталь' : 'Устройство'} переедет в «${targetPlaceLabel}».`
      : null,
  );

  onMount(() => {
    // Встроенный шаг не пересоздаёт Modal, поэтому фокус ставим сами.
    if (embedded) cancelBtn?.querySelector('button')?.focus();
  });

  async function confirm() {
    if (saving) return;
    saving = true;
    serverErr = null;
    try {
      await onConfirm();
    } catch (e) {
      const err = e as { message?: string } | undefined;
      serverErr = err?.message ?? 'Не удалось переместить. Повторите попытку.';
    } finally {
      saving = false;
    }
  }
</script>

{#snippet content()}
  <div class="already">
    <p class="already-body">{body}</p>
    {#if callout}
      <div class="callout" role="note">{callout}</div>
    {/if}
    {#if serverErr}
      <div class="server-error">{serverErr}</div>
    {/if}
  </div>
{/snippet}

{#snippet buttons()}
  <span bind:this={cancelBtn} class="btn-slot">
    <Button variant="secondary" onclick={onCancel} disabled={saving}>Отменить</Button>
  </span>
  <Button variant="destructive" loading={saving} onclick={confirm}>Переместить</Button>
{/snippet}

{#if embedded}
  <div class="embedded">
    <h3 class="embedded-title">{title}</h3>
    {@render content()}
    <div class="embedded-footer">{@render buttons()}</div>
  </div>
{:else}
  <Modal
    open={true}
    {title}
    size="md"
    onClose={onCancel}
    initialFocus=".modal-footer .btn-secondary"
  >
    {@render content()}
    {#snippet footer()}
      {@render buttons()}
    {/snippet}
  </Modal>
{/if}

<style lang="scss">
  .already,
  .embedded {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
    white-space: normal;
    text-align: left;
  }

  .embedded-title {
    margin: 0;
    font: var(--tr-text-h3);
    color: var(--tr-text-primary);
  }

  .embedded-footer {
    display: flex;
    justify-content: flex-end;
    gap: var(--tr-space-xs);
  }

  .btn-slot {
    display: inline-flex;
  }

  .already-body {
    margin: 0;
    color: var(--tr-text-primary);
    overflow-wrap: anywhere;
    white-space: normal;
    text-align: left;
  }

  .callout {
    padding: var(--tr-space-xs) var(--tr-space-md);
    background: var(--tr-warning-soft);
    border-radius: var(--tr-radius-xs);
    font: var(--tr-text-label);
    color: var(--tr-warning-text);
  }

  .server-error {
    padding: var(--tr-space-xs) var(--tr-space-md);
    background: color-mix(in srgb, var(--tr-danger) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--tr-danger) 30%, transparent);
    border-radius: var(--tr-radius-xs);
    color: var(--tr-danger);
  }
</style>
