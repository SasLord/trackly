<script lang="ts">
  // Plan 06-04: DiscoveryModal — SNMP discovery с 2-step flow (scan → review → admit).
  // По паттерну OperationModal.svelte (Modal + form state + try/catch + pushToast).
  // size="wide" (960px). Reset на open. handleScan → printers.discover. handleCreate → printers.admit.
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Input from '$lib/components/Input.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import DiscoveryResultsTable from './DiscoveryResultsTable.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { printers } from './api';
  import type { DiscoveredPrinterDto } from '../../bindings-phase6';

  interface Props {
    open: boolean;
    onClose: () => void;
    onSuccess: (_created: number) => void;
  }

  const { open, onClose, onSuccess }: Props = $props();

  let ipStart = $state('');
  let ipEnd = $state('');
  let community = $state('public');
  let scanning = $state(false);
  let creating = $state(false);
  let discovered = $state<DiscoveredPrinterDto[]>([]);
  let selected = $state<Set<number>>(new Set());
  let scanned = $state(false);

  // Reset on open.
  $effect(() => {
    if (open) {
      ipStart = '';
      ipEnd = '';
      community = 'public';
      scanning = false;
      creating = false;
      discovered = [];
      selected = new Set();
      scanned = false;
    }
  });

  function toggleSelect(idx: number) {
    const next = new Set(selected);
    if (next.has(idx)) next.delete(idx);
    else next.add(idx);
    selected = next;
  }

  const selectedCount = $derived(selected.size);
  const selectedIps = $derived(
    Array.from(selected)
      .map((idx) => discovered[idx]?.ip)
      .filter(Boolean) as string[],
  );

  async function handleScan() {
    if (!ipStart.trim() || !ipEnd.trim()) {
      pushToast('error', 'Укажите корректный диапазон IP-адресов');
      return;
    }
    scanning = true;
    scanned = false;
    discovered = [];
    selected = new Set();
    try {
      discovered = await printers.discover(
        ipStart.trim(),
        ipEnd.trim(),
        community.trim() || 'public',
      );
      scanned = true;
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось завершить поиск. Проверьте сеть и повторите.';
      pushToast('error', msg);
    } finally {
      scanning = false;
    }
  }

  async function handleCreate() {
    if (selectedIps.length === 0) return;
    creating = true;
    try {
      const admitted = await printers.admit(selectedIps, community.trim() || 'public');
      const count = admitted.length;
      pushToast('success', `Заведено принтеров: ${count}`);
      onSuccess(count);
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось выполнить операцию. Повторите попытку.';
      pushToast('error', msg);
    } finally {
      creating = false;
    }
  }
</script>

<Modal {open} title="Поиск принтеров в сети" size="wide" {onClose}>
  <div class="scan-form">
    <div class="fields">
      <div class="field">
        <label class="field-label" for="ip-start">Диапазон IP-адресов</label>
        <Input
          id="ip-start"
          value={ipStart}
          placeholder="Например: 192.168.1.1"
          oninput={(v) => (ipStart = v)}
        />
      </div>
      <div class="field">
        <label class="field-label" for="ip-end">до</label>
        <Input id="ip-end" value={ipEnd} placeholder="192.168.1.254" oninput={(v) => (ipEnd = v)} />
      </div>
      <div class="field">
        <label class="field-label" for="community">SNMP Community</label>
        <Input
          id="community"
          value={community}
          placeholder="По умолчанию: public"
          oninput={(v) => (community = v)}
        />
      </div>
      <Button variant="primary" loading={scanning} onclick={handleScan}>Начать поиск</Button>
    </div>
  </div>

  {#if scanning}
    <div class="scanning-state">
      <Spinner size="md" />
      <span>Поиск принтеров… найдено: {discovered.length}</span>
    </div>
  {:else if scanned}
    <div class="results-section">
      <h3 class="results-heading">Найденные принтеры</h3>
      <DiscoveryResultsTable items={discovered} {selected} onToggle={toggleSelect} />
    </div>
  {/if}

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>Отмена</Button>
    {#if scanned}
      <Button
        variant="primary"
        loading={creating}
        disabled={selectedCount === 0}
        onclick={handleCreate}
      >
        Завести выбранные ({selectedCount})
      </Button>
    {/if}
  {/snippet}
</Modal>

<style lang="scss">
  .scan-form {
    margin-bottom: var(--space-md);
  }

  .fields {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr auto;
    gap: var(--space-md);
    align-items: end;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .field-label {
    font-size: var(--font-size-label);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-secondary);
  }

  .scanning-state {
    display: flex;
    align-items: center;
    gap: var(--space-md);
    padding: var(--space-xl);
    justify-content: center;
    color: var(--color-text-secondary);
    font-size: var(--font-size-body);
  }

  .results-section {
    margin-top: var(--space-md);
  }

  .results-heading {
    font-size: var(--font-size-body);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    margin: 0 0 var(--space-sm);
  }
</style>
