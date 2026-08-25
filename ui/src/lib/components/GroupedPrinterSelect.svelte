<script lang="ts">
  // D-PRN-01 (Phase 11 Plan 02): printer dropdown grouped by location.
  // Server already sorts options by location then name, no-location last
  // (RequestService::printer_options ORDER BY) — this component only groups
  // for rendering, it does not re-sort.
  //
  // Phase 28 UAT (GAP-1 follow-up): the native <select> option-popup rendered
  // in the OS chrome (unstyled, ignores the app theme). Re-based on the shared
  // custom `Dropdown` primitive (variant="select", grouped drill-in) — same
  // design-system surface and location-grouping semantics (each location is a
  // drill-in group; a single location auto-flattens to its printers). The
  // Props contract is unchanged (`value` = selected printer id as string,
  // `onchange(idString)`), so the consumer (RequestFormModal) needs no edits.
  import Dropdown from '$lib/components/Dropdown.svelte';
  import type { RequestPrinterOptionDto } from '../../bindings-phase6';

  interface Props {
    options: RequestPrinterOptionDto[];
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

  const NO_LOCATION_LABEL = 'Без расположения';

  interface PrinterGroup {
    label: string;
    printers: RequestPrinterOptionDto[];
  }

  function printerName(p: RequestPrinterOptionDto): string {
    return p.name || `Принтер #${p.id}`;
  }

  // Group by location label, preserving server-provided order.
  const groups = $derived.by<PrinterGroup[]>(() => {
    const map = new Map<string, RequestPrinterOptionDto[]>();
    for (const opt of options) {
      const label = opt.place ?? NO_LOCATION_LABEL;
      const bucket = map.get(label);
      if (bucket) {
        bucket.push(opt);
      } else {
        map.set(label, [opt]);
      }
    }
    return Array.from(map.entries()).map(([label, printers]) => ({ label, printers }));
  });

  // Selected printer's display name (Dropdown's `value` is the display string).
  const selectedLabel = $derived.by(() => {
    const sel = options.find((o) => String(o.id) === value);
    return sel ? printerName(sel) : '';
  });
</script>

<Dropdown
  variant="select"
  {id}
  value={selectedLabel}
  placeholder="Выберите принтер"
  searchable={false}
  {disabled}
  {invalid}
  loading={false}
  {groups}
  getGroupId={(g) => g.label}
  getGroupName={(g) => g.label}
  getGroupCount={(g) => g.printers.length}
  isGroupExpandable={(g) => g.printers.length > 0}
  onExpandGroup={(g) => g.printers}
  getMemberId={(m) => m.id}
  getMemberName={printerName}
  onSearch={() => {}}
  onPickGroup={() => {}}
  onPickMember={(m) => onchange?.(String(m.id))}
/>
