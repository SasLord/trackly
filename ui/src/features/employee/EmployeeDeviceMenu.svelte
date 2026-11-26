<script lang="ts">
  // Меню строки/плитки устройства сотрудника (41.6, D-09..D-11, UI-SPEC §10).
  // Обёртка над ActionMenu; проп `portal` не передаётся (портал — единственный
  // режим). Набор пунктов считает чистая buildEmployeeMenu. Компонент API не
  // вызывает и staff-команд не знает: о действиях сообщает колбэками.
  import ActionMenu from '@trackly/kit/components/ActionMenu.svelte';
  import { buildEmployeeMenu } from './employeeDeviceMenu';
  import type { EmployeeDeviceDto } from '../../bindings';

  interface Props {
    row: EmployeeDeviceDto;
    onOpen: () => void;
    onRepair: () => void;
    onCartridge: () => void;
    onGoto: (_requestId: number) => void;
    onCopy: () => void;
    trigger?: 'button' | 'none';
    variant?: 'ghost-sm' | 'ghost-md';
  }

  const {
    row,
    onOpen,
    onRepair,
    onCartridge,
    onGoto,
    onCopy,
    trigger = 'button',
    variant = 'ghost-sm',
  }: Props = $props();

  let menu = $state<ReturnType<typeof ActionMenu> | null>(null);

  const sections = $derived(buildEmployeeMenu(row));

  function activate(id: string) {
    if (id === 'open') onOpen();
    else if (id === 'repair') onRepair();
    else if (id === 'cartridge') onCartridge();
    else if (id === 'copy') onCopy();
    else if (id.startsWith('goto:')) onGoto(Number(id.slice(5)));
  }

  /** Открыть меню у точки (правый клик). */
  export function openAt(x: number, y: number) {
    menu?.openAt(x, y);
  }

  /** Открыть меню у левого-нижнего угла элемента (клавиша контекстного меню). */
  export function openFromElement(el: HTMLElement) {
    const rect = el.getBoundingClientRect();
    menu?.openAt(rect.left, rect.bottom);
  }

  /** Открыть карточку без меню (двойной клик по строке). */
  export function openView() {
    onOpen();
  }
</script>

<ActionMenu
  bind:this={menu}
  {trigger}
  {variant}
  label={'Действия: ' + row.name}
  panelMinWidth="240px"
>
  {#each sections as section, si (si)}
    {#if si > 0}
      <!-- svelte-ignore a11y_no_redundant_roles -->
      <hr role="separator" />
    {/if}
    {#each section as item (item.id)}
      <button type="button" role="menuitem" class="menu-item" onclick={() => activate(item.id)}>
        <span class="menu-text">
          <span>{item.label}</span>
          {#if item.hint}<span class="menu-hint">{item.hint}</span>{/if}
        </span>
      </button>
    {/each}
  {/each}
</ActionMenu>

<style lang="scss">
  .menu-text {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    min-width: 0;
  }

  .menu-hint {
    font-size: var(--tr-font-size-caption);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-tertiary);
  }
</style>
