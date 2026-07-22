<script lang="ts">
  // D-01/D-02/D-03 (Phase 12 Plan 03): flat cartridge selector for
  // OperationModal's request-centric install flow.
  //
  // Phase 28 UAT (GAP-1 follow-up): the native <select> option-popup rendered
  // in the OS chrome (unstyled, ignores the app theme). Re-based on the shared
  // custom `Dropdown` primitive (flat + variant="select") — same design-system
  // surface as every other picker in the app. The Props contract is unchanged
  // (`value` = selected cartridge id as string, `onchange(idString)`), so the
  // consumer (OperationModal) needs no edits.
  import Dropdown from '$lib/components/Dropdown.svelte';
  import type { CartridgeDto } from '../../bindings';

  interface Props {
    options: CartridgeDto[];
    value: string;
    disabled?: boolean;
    invalid?: boolean;
    id?: string;
    onchange?: (_value: string) => void;
  }

  const {
    options,
    value = $bindable(''),
    disabled = false,
    invalid = false,
    id,
    onchange,
  }: Props = $props();

  function optionLabel(o: CartridgeDto): string {
    const model = [o.model_brand, o.model_name].filter(Boolean).join(' ');
    return `${o.code} — ${model} (${o.state_name ?? '—'})`;
  }

  const selectedLabel = $derived.by(() => {
    const sel = options.find((o) => String(o.id) === value);
    return sel ? optionLabel(sel) : '';
  });

  // Flat list — no drill-in; onExpandGroup is never really called
  // (isGroupExpandable === false) but Dropdown needs a typed member fn.
  function noExpand(): CartridgeDto[] {
    return [];
  }
</script>

<Dropdown
  variant="select"
  flat={true}
  {id}
  value={selectedLabel}
  placeholder="Выберите картридж"
  {disabled}
  {invalid}
  loading={false}
  groups={options}
  getGroupId={(o) => o.id}
  getGroupName={optionLabel}
  getGroupCount={() => 0}
  isGroupExpandable={() => false}
  isGroupSelected={(o) => String(o.id) === value}
  onExpandGroup={noExpand}
  getMemberId={(o) => o.id}
  getMemberName={optionLabel}
  onSearch={() => {}}
  onPickGroup={(o) => onchange?.(String(o.id))}
  onPickMember={() => {}}
/>
