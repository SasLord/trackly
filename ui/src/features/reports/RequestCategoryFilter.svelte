<script lang="ts">
  // Quick task 260821-w18 (CATF-01..04): funnel-фильтр домена «Заявки» —
  // кнопка-воронка + попап с 8 чекбоксами (Все + 7 категорий/типов).
  //
  // `selectedKeys === null` => «Все» (backend: без WHERE-ограничения).
  // `selectedKeys === []` => явный пустой выбор (backend: 0 строк).
  // `selectedKeys` — allow-list ключей чекбоксов, тот же wire-контракт, что
  // `category_filter_clause` в report_service.rs (см. interfaces PLAN).
  import Button from '$lib/components/Button.svelte';
  import Checkbox from '$lib/components/Checkbox.svelte';
  import { portal } from '$lib/utils/portal';
  import { dropdownAnchor } from '$lib/utils/dropdownAnchor';

  interface Props {
    selectedKeys: string[] | null;
    onChange: (_keys: string[] | null) => void;
  }

  const { selectedKeys, onChange }: Props = $props();

  const CHECKBOX_DEFS = [
    { key: 'ad_register', label: 'Регистрации' },
    { key: 'cartridge_replace', label: 'Замена картриджа' },
    { key: 'repair', label: 'Ремонт техники' },
    { key: 'consumables', label: 'Расходные материалы' },
    { key: 'software', label: 'Программное обеспечение' },
    { key: 'no_category', label: 'Без категорий' },
    { key: 'other', label: 'Прочее' },
  ] as const;

  const ALL_KEYS = CHECKBOX_DEFS.map((d) => d.key);

  const allChecked = $derived(selectedKeys === null);

  function isChecked(key: string): boolean {
    return allChecked || (selectedKeys?.includes(key) ?? false);
  }

  function toggleAll(next: boolean) {
    if (next) {
      onChange(null);
    } else {
      onChange([...ALL_KEYS]);
    }
  }

  function toggleKey(key: string, next: boolean) {
    if (allChecked) return; // disabled в этом состоянии — защита на всякий случай
    const current = selectedKeys ?? [];
    const updated = next ? [...current, key] : current.filter((k) => k !== key);
    onChange(updated);
  }

  let open = $state(false);
  let rootEl = $state<HTMLElement | null>(null);
  let triggerEl = $state<HTMLElement | null>(null);
  let panelEl = $state<HTMLElement | null>(null);

  $effect(() => {
    function onDown(e: MouseEvent) {
      const target = e.target as Node;
      if (open && !rootEl?.contains(target) && !panelEl?.contains(target)) {
        open = false;
      }
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') open = false;
    }
    document.addEventListener('mousedown', onDown);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onDown);
      document.removeEventListener('keydown', onKey);
    };
  });
</script>

<div class="request-category-filter" bind:this={rootEl}>
  <div class="trigger-wrap" bind:this={triggerEl}>
    <Button variant="secondary" size="md" onclick={() => (open = !open)}>
      <svg
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
        aria-hidden="true"
      >
        <polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3"></polygon>
      </svg>
      <span class="sr-only">Фильтр по категориям заявок</span>
    </Button>
    {#if !allChecked}
      <span class="active-dot" aria-hidden="true"></span>
    {/if}
  </div>

  {#if open}
    <div
      class="popover-panel"
      role="dialog"
      aria-label="Фильтр по категориям заявок"
      use:portal
      use:dropdownAnchor={{ anchorEl: triggerEl, gap: 4 }}
      bind:this={panelEl}
    >
      <Checkbox checked={allChecked} onchange={toggleAll}>Все</Checkbox>
      <div class="divider"></div>
      {#each CHECKBOX_DEFS as def (def.key)}
        <Checkbox
          checked={isChecked(def.key)}
          disabled={allChecked}
          onchange={(c) => toggleKey(def.key, c)}
        >
          {def.label}
        </Checkbox>
      {/each}
    </div>
  {/if}
</div>

<style lang="scss">
  .request-category-filter {
    position: relative;
    display: inline-flex;
  }

  .trigger-wrap {
    position: relative;
    display: inline-flex;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  .active-dot {
    position: absolute;
    top: -2px;
    right: -2px;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--tr-accent);
    border: 1.5px solid var(--tr-surface);
  }

  .popover-panel {
    z-index: 1000;
    min-width: 240px;
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
    padding: var(--tr-space-sm);
    background: var(--tr-surface-raised);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-md);
    box-shadow: var(--tr-elev-2);
  }

  .divider {
    height: 1px;
    background: var(--tr-border);
    margin: var(--tr-space-2xs) 0;
  }
</style>
