<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import DeviceList from './DeviceList.svelte';
  import DeviceFormModal from './DeviceFormModal.svelte';
  import { devices } from './api';
  import type { DeviceDto, DeviceFilter, Pagination } from '../../bindings';

  // ---------------------------------------------------------------------------
  // State
  // ---------------------------------------------------------------------------
  let items = $state<DeviceDto[]>([]);
  let total = $state(0);
  let loading = $state(false);
  let modalOpen = $state(false);
  let editTarget = $state<DeviceDto | null>(null);

  // type_id=1 ("Устройство") hardcoded — /devices section shows only Устройства.
  // /printers (Phase 6) will use type_id=2 ("Принтер").
  const filter = $state<DeviceFilter>({
    type_id: 1,
    location_id: null,
    status_id: null,
    state: null,
    name_prefix: null,
    include_deleted: false,
  });

  const pagination = $state<Pagination>({ offset: 0, limit: 50 });

  // ---------------------------------------------------------------------------
  // Data loading
  // ---------------------------------------------------------------------------
  async function refresh() {
    loading = true;
    try {
      const resp = await devices.list(filter, pagination);
      items = resp.items;
      total = resp.total;
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось загрузить устройства';
      pushToast('error', msg);
    } finally {
      loading = false;
    }
  }

  onMount(refresh);

  // ---------------------------------------------------------------------------
  // Modal handlers
  // ---------------------------------------------------------------------------
  function openCreate() {
    editTarget = null;
    modalOpen = true;
  }

  function openEdit(d: DeviceDto) {
    editTarget = d;
    modalOpen = true;
  }

  function onSaved() {
    modalOpen = false;
    refresh();
  }
</script>

<div class="devices-page">
  <header class="page-header">
    <h1 class="page-title">Устройства</h1>
    <div class="header-actions">
      <Button variant="primary" onclick={openCreate}>+ Создать устройство</Button>
      <Button variant="secondary" disabled>Импорт CSV</Button>
      <Button variant="secondary" disabled>Экспорт CSV</Button>
    </div>
  </header>

  <div class="page-content">
    <DeviceList {items} {total} {loading} onEdit={openEdit} onDelete={refresh} />
  </div>
</div>

<DeviceFormModal
  open={modalOpen}
  target={editTarget}
  onClose={() => (modalOpen = false)}
  {onSaved}
/>

<style lang="scss">
  .devices-page {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-lg) var(--space-xl);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
    gap: var(--space-md);
    flex-wrap: wrap;
  }

  .page-title {
    margin: 0;
    font-size: var(--font-size-page-title, var(--font-size-heading));
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    line-height: var(--line-height-heading);
  }

  .header-actions {
    display: flex;
    gap: var(--space-sm);
    align-items: center;
    flex-wrap: wrap;
  }

  .page-content {
    flex: 1;
    overflow: auto;
    padding: var(--space-lg) var(--space-xl);
  }
</style>
