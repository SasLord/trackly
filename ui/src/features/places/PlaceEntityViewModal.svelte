<script lang="ts">
  // GAP-8 (39-UAT.md, Прогон 3): the read-only «Просмотр» popup opened from a
  // PlaceContents.svelte content row. Replaces the previous behaviour (row
  // click navigated straight to the entity's OWN section, where the record
  // was invisible among many others and nothing was highlighted — 39-20-PLAN
  // shipped that knowingly as a limitation this gap closes).
  //
  // Reuses the existing *FormBody components in `readonly` mode (see
  // DeviceFormBody.svelte / CartridgeFormBody.svelte's own readonly
  // doc-comments) instead of forking a second copy of these forms — a
  // read-only mirror that drifted from the real form on the next edit would
  // be its own class of bug.
  //
  // Printer asymmetry (documented per the gap's explicit instruction): there
  // is NO dedicated printer edit modal anywhere in this codebase.
  // `places_contents` (place_service.rs) returns the underlying `devices.id`
  // for BOTH kind='device' and kind='printer' rows — a printer's editable
  // data genuinely IS its `devices` row (name/inventory/serial/place/status),
  // exactly the same data PrinterDetail.svelte's own «Данные устройства»
  // section already edits via DeviceFormModal. So «Просмотр принтера» /
  // «Редактировать» for a printer reuse DeviceFormBody/DeviceFormModal too —
  // not a new, invented printer form. `DeviceFormModal` already renders the
  // title "Редактирование принтера" once `target.type_id === 2`, so nothing
  // extra is needed to get that right.
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import DeviceFormBody from '../devices/DeviceFormBody.svelte';
  import DeviceFormModal from '../devices/DeviceFormModal.svelte';
  import CartridgeFormBody from '../cartridges/CartridgeFormBody.svelte';
  import CartridgeFormModal from '../cartridges/CartridgeFormModal.svelte';
  import { devices } from '../devices/api';
  import { cartridges } from '../cartridges/api';
  import type { PlaceContentDto, DeviceDto, CartridgeDto, CartridgeModelDto } from '../../bindings';

  interface Props {
    row: PlaceContentDto;
    onClose: () => void;
    /** Fired after a successful save from the «Редактировать» edit modal so
     *  the caller (PlaceContents) can reload its table — the row's
     *  name/place/status may have just changed. */
    onChanged?: () => void;
  }

  const { row, onClose, onChanged }: Props = $props();

  const SECTION_HASH_BY_KIND: Record<string, string> = {
    device: '#/devices',
    printer: '#/printers',
    cartridge: '#/cartridges',
  };

  const TITLE_BY_KIND: Record<string, string> = {
    device: 'Просмотр устройства',
    printer: 'Просмотр принтера',
    cartridge: 'Просмотр картриджа',
  };

  const GOTO_LABEL_BY_KIND: Record<string, string> = {
    device: 'Перейти к устройству',
    printer: 'Перейти к принтеру',
    cartridge: 'Перейти к картриджу',
  };

  const isCartridge = row.kind === 'cartridge';

  let loading = $state(true);
  let loadError = $state(false);
  let deviceDto = $state<DeviceDto | null>(null);
  let cartridgeDto = $state<CartridgeDto | null>(null);
  let cartridgeModels = $state<CartridgeModelDto[]>([]);

  // View popup and edit popup are mutually exclusive, never stacked — a
  // second dim backdrop behind the edit form would just be visual noise.
  // «Редактировать» hides this one and opens the real edit modal; cancelling
  // the edit modal (not saving) brings this view back with the same
  // already-loaded data — no need to re-fetch just because the user backed
  // out of editing.
  let viewOpen = $state(true);
  let editOpen = $state(false);

  $effect(() => {
    const r = row;
    loading = true;
    loadError = false;
    (async () => {
      try {
        if (r.kind === 'cartridge') {
          const [cart, modelList] = await Promise.all([
            cartridges.get(r.id),
            cartridges.modelsList(),
          ]);
          cartridgeDto = cart;
          cartridgeModels = modelList;
        } else {
          // device AND printer content rows both carry a `devices` row id —
          // see the file-header comment.
          deviceDto = await devices.get(r.id);
        }
      } catch {
        loadError = true;
      } finally {
        loading = false;
      }
    })();
  });

  const loadFailed = $derived(
    loadError || (!loading && isCartridge ? cartridgeDto === null : deviceDto === null),
  );

  function handleGoTo() {
    onClose();
    window.location.hash = `${SECTION_HASH_BY_KIND[row.kind] ?? '#/'}?id=${row.id}`;
  }

  function handleEdit() {
    viewOpen = false;
    editOpen = true;
  }

  function handleEditClose() {
    editOpen = false;
    viewOpen = true;
  }

  function handleEditSaved() {
    editOpen = false;
    onChanged?.();
    onClose();
  }
</script>

<Modal open={viewOpen} title={TITLE_BY_KIND[row.kind] ?? 'Просмотр'} size="md" {onClose}>
  {#if loading}
    <div class="view-state" aria-live="polite">
      <Spinner size="md" />
      <span>Загружаем данные…</span>
    </div>
  {:else if loadFailed}
    <p class="view-error">Не удалось загрузить данные. Закройте окно и попробуйте ещё раз.</p>
  {:else if isCartridge && cartridgeDto}
    <CartridgeFormBody
      target={cartridgeDto}
      models={cartridgeModels}
      readonly={true}
      onClose={() => {}}
      onSuccess={() => {}}
      onLoading={() => {}}
      onCanSubmitChange={() => {}}
      onRegisterSubmit={() => {}}
    />
  {:else if deviceDto}
    <DeviceFormBody
      target={deviceDto}
      stateHints={[]}
      typeId={deviceDto.type_id}
      readonly={true}
      onSaved={() => {}}
      onLoading={() => {}}
      onCanSubmitChange={() => {}}
      onRegisterSubmit={() => {}}
    />
  {/if}

  {#snippet footer()}
    <Button variant="secondary" onclick={handleGoTo}>
      {GOTO_LABEL_BY_KIND[row.kind] ?? 'Перейти'}
    </Button>
    <Button variant="primary" onclick={handleEdit} disabled={loading || loadFailed}>
      Редактировать
    </Button>
  {/snippet}
</Modal>

{#if isCartridge}
  <CartridgeFormModal
    open={editOpen}
    target={cartridgeDto}
    models={cartridgeModels}
    onClose={handleEditClose}
    onSuccess={handleEditSaved}
  />
{:else}
  <DeviceFormModal
    open={editOpen}
    target={deviceDto}
    onClose={handleEditClose}
    onSaved={handleEditSaved}
  />
{/if}

<style lang="scss">
  .view-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--tr-space-md);
    padding: var(--tr-space-2xl);
    color: var(--tr-text-secondary);
  }

  .view-error {
    margin: 0;
    padding: var(--tr-space-md);
    color: var(--tr-danger-text);
  }
</style>
