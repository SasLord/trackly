<script lang="ts">
  // Plan 07-11 Task 1: Settings sub-section switch-bar (GAP-S2).
  // Splits the Settings page into per-subsection views matching the component layout.
  // Plan 28-05 Task 1 (D-06): bespoke tab-bar replaced with the shared Tabs primitive
  // (variant="underline", no count — 7 sections have no counters).
  import Tabs from '$lib/components/Tabs.svelte';

  const SECTIONS = [
    { key: 'network', label: 'Сеть' },
    { key: 'org', label: 'Организация' },
    { key: 'storage', label: 'Хранилище' },
    { key: 'backup', label: 'Бэкапы' },
    { key: 'threshold', label: 'Порог остатка' },
    { key: 'templates', label: 'Шаблоны' },
    { key: 'ad', label: 'Active Directory' },
  ] as const;

  interface Props {
    activeSection: string;
    onSectionChange: (_s: string) => void;
  }

  const { activeSection, onSectionChange }: Props = $props();
</script>

<Tabs
  variant="underline"
  tabs={SECTIONS.map((s) => ({ key: s.key, label: s.label }))}
  active={activeSection}
  ariaLabel="Раздел настроек"
  onchange={onSectionChange}
/>
