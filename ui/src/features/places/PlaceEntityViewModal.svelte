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
  import { untrack, tick } from 'svelte';
  import { push } from 'svelte-spa-router';
  import { openRequestTarget } from '../requests/requestTarget';
  import Modal from '@trackly/kit/components/Modal.svelte';
  import Button from '@trackly/kit/components/Button.svelte';
  import Spinner from '@trackly/kit/components/Spinner.svelte';
  import DetailSection from '$lib/components/DetailSection.svelte';
  import DetailField from '$lib/components/DetailField.svelte';
  import MovementTimeline from '$lib/components/MovementTimeline.svelte';
  import IconHistory from '$lib/components/icons/IconHistory.svelte';
  import { historyPanel } from '$lib/stores/historyPanel.svelte';
  import { placeContentEventsStore } from '$lib/stores/placeContentEvents.svelte';
  import DeviceFormBody from '../devices/DeviceFormBody.svelte';
  import DeviceFormModal from '../devices/DeviceFormModal.svelte';
  import CartridgeFormBody from '../cartridges/CartridgeFormBody.svelte';
  import CartridgeFormModal from '../cartridges/CartridgeFormModal.svelte';
  import AttachmentsBlock from '../attachments/AttachmentsBlock.svelte';
  import AttachmentHistory from '../attachments/AttachmentHistory.svelte';
  import { createAttachmentEvents } from '../attachments/attachmentEvents.svelte';
  import { devices } from '../devices/api';
  import { cartridges } from '../cartridges/api';
  import RepairDeviceHistoryModal from '../requests/RepairDeviceHistoryModal.svelte';
  import { repairs } from '../requests/api';
  import { canUseRepairs } from '../requests/repairLocks';
  import { apiCall } from '$lib/api/client';
  import type {
    PlaceContentDto,
    DeviceDto,
    CartridgeDto,
    CartridgeModelDto,
    HistoryEntryDto,
  } from '../../bindings';

  interface Props {
    row: PlaceContentDto;
    onClose: () => void;
    /**
     * Fired after a successful save from the «Редактировать» edit modal so
     * the caller (PlaceContents) can reload its table — the row's
     * name/place/status may have just changed.
     *
     * WARNING-1 gap-closure round 2 (live UAT, 2026-09-18): the first fix
     * (`onChanged?: () => void`, no arguments) invalidated only the
     * currently VIEWED root (`PlaceContents`'s own `place.id`) — safe for
     * that node's own counter, but the row's ACTUAL old/new place (e.g. a
     * device moved between two floors under the viewed building) sits on a
     * different tree branch and never got evicted, matching exactly what
     * live UAT reported ("only the currently open section updates").
     * `deviceDto`/`cartridgeDto` (loaded BEFORE the edit, unchanged while
     * `editOpen`) already hold the row's OLD `place_id`; `DeviceFormModal`'s
     * `onSaved` result / `CartridgeFormModal`'s `onSuccess` entity already
     * carry the NEW one (same values `DevicesPage`/`CartridgesPage` already
     * use for their own producers, Task 1/Task 2 of this plan) — no extra
     * request needed. `onChanged` now forwards that array so the caller can
     * invalidate BOTH the exact old and new place, not just the viewed root.
     */
    onChanged?: (changedPlaceIds: number[]) => void;
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

  const isCartridge = $derived(row.kind === 'cartridge');

  // 41.3-14: вложения карточки (устройство и принтер — одна сущность device).
  const attParentKind = $derived(row.kind === 'cartridge' ? 'cartridge' : 'device');
  let attachmentsBlock = $state<ReturnType<typeof AttachmentsBlock> | null>(null);
  const attEvents = createAttachmentEvents(
    () => attParentKind,
    () => row.id,
  );

  let loading = $state(true);
  let loadError = $state(false);
  let deviceDto = $state<DeviceDto | null>(null);
  let cartridgeDto = $state<CartridgeDto | null>(null);
  let cartridgeModels = $state<CartridgeModelDto[]>([]);
  // 41.10 D-19: лента истории грузится ОТДЕЛЬНО от основной загрузки (своё
  // состояние, свой токен) — ошибка ленты не превращает попап в «Не удалось
  // загрузить данные». Принтер и устройство — одна и та же запись devices.
  let historyEntries = $state<HistoryEntryDto[]>([]);
  let timelineLoading = $state(false);
  let timelineError = $state(false);
  let timelineToken = 0;
  let timelineLoadedFor: string | null = null;

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
      }
      loading = false;
    })();
  });

  // ---- 41.10 D-14..D-19: переключатель «История» ----
  let wide = $state(false);
  let narrowPanelOpen = $state(false);
  let toggleHost = $state<HTMLElement | null>(null);
  let panelEl = $state<HTMLElement | null>(null);
  let savedScroll = 0;

  $effect(() => {
    if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return;
    const mq = window.matchMedia('(min-width: 1024px)');
    wide = mq.matches;
    const onChange = (e: MediaQueryListEvent) => {
      wide = e.matches;
      // узкий→широкий: панель закрыта, размер определяет флаг; широкий→узкий:
      // панель и так закрыта, флаг не трогаем (D-18).
      narrowPanelOpen = false;
    };
    mq.addEventListener('change', onChange);
    return () => mq.removeEventListener('change', onChange);
  });

  // D-18: на узком экране флаг не читается — pressed берётся из narrowPanelOpen.
  const splitMode = $derived(wide && historyPanel.enabled);
  const pressed = $derived(wide ? historyPanel.enabled : narrowPanelOpen);

  function toggleHistory() {
    if (wide) {
      historyPanel.set(!historyPanel.enabled);
    } else {
      toggleNarrowPanel(!narrowPanelOpen);
    }
  }

  function scrollHost(): HTMLElement | null {
    return (
      toggleHost?.closest('.modal-container')?.querySelector<HTMLElement>('.modal-body') ?? null
    );
  }

  async function toggleNarrowPanel(open: boolean) {
    if (open) {
      savedScroll = scrollHost()?.scrollTop ?? 0;
      narrowPanelOpen = true;
      await tick();
      panelEl?.focus();
    } else {
      narrowPanelOpen = false;
      await tick();
      const host = scrollHost();
      if (host) host.scrollTop = savedScroll;
    }
  }

  function handlePanelKeydown(e: KeyboardEvent) {
    if (e.key !== 'Escape') return;
    // Esc закрывает панель, а не попап: Modal пропускает defaultPrevented.
    e.preventDefault();
    toggleNarrowPanel(false).then(() => toggleHost?.querySelector('button')?.focus());
  }

  async function loadTimeline(r: PlaceContentDto) {
    const key = `${r.kind}:${r.id}`;
    if (timelineLoadedFor === key) return;
    timelineLoadedFor = key;
    const token = ++timelineToken;
    timelineLoading = true;
    timelineError = false;
    try {
      const list = await apiCall<HistoryEntryDto[]>('history_get_timeline', {
        entityType: r.kind === 'cartridge' ? 'cartridge' : 'device',
        entityId: r.id,
      });
      if (token !== timelineToken) return;
      historyEntries = list;
    } catch {
      if (token !== timelineToken) return;
      timelineError = true;
      timelineLoadedFor = null;
    } finally {
      if (token === timelineToken) timelineLoading = false;
    }
  }

  // Смена строки: сброс ленты и панели (объявлен РАНЬШЕ загрузчика — порядок эффектов).
  $effect(() => {
    void row;
    untrack(() => {
      timelineToken++;
      timelineLoadedFor = null;
      historyEntries = [];
      timelineError = false;
      timelineLoading = false;
      narrowPanelOpen = false;
    });
  });

  // D-19: лента — при первом включении или сразу, если флаг включён на широком.
  $effect(() => {
    const r = row;
    if (pressed) untrack(() => void loadTimeline(r));
  });

  // R8-01: событие другого клиента — перечитать открытую ленту (кэш timelineLoadedFor
  // иначе выходит сразу). Читаем ТОЛЬКО reloadSeq; всё остальное — в untrack,
  // lastReloadSeq не $state (иначе effect_update_depth_exceeded).
  let lastReloadSeq = placeContentEventsStore.reloadSeq;
  $effect(() => {
    const seq = placeContentEventsStore.reloadSeq;
    untrack(() => {
      if (seq === lastReloadSeq) return;
      lastReloadSeq = seq;
      timelineLoadedFor = null;
      if (pressed) void loadTimeline(row);
    });
  });

  // Фаза 41.4, план 32: счётчик заходов в сервис — отдельная независимая загрузка
  // (только device/printer, только персонал); сбой или 0 — кнопки «Ремонты» нет,
  // просмотр не затрагивается (loading/loadError основной загрузки не делятся).
  let visitCount = $state(0);
  let historyOpen = $state(false);
  let historyToken = 0;

  $effect(() => {
    const r = row;
    const token = ++historyToken;
    visitCount = 0;
    if (r.kind === 'cartridge' || !canUseRepairs()) return;
    repairs
      .deviceHistory(r.id)
      .then((h) => {
        if (token === historyToken) visitCount = h.visitCount;
      })
      .catch(() => {
        if (token === historyToken) visitCount = 0;
      });
    return () => {
      historyToken++;
    };
  });

  const loadFailed = $derived(
    loadError || (!loading && isCartridge ? cartridgeDto === null : deviceDto === null),
  );

  // GAP-9 (39-UAT.md, Прогон 5): «Перейти к…» previously navigated via raw
  // `window.location.hash = …` assignment, closing the modal (which unmounts
  // THIS component, via the caller's `viewRow = null`) BEFORE writing the
  // new hash. That ordering is a real hazard — closing first means the
  // navigation write depends on this still-executing synchronous handler
  // completing on a component that is simultaneously being torn down, and
  // nothing here would guarantee it survives a future change to `onClose`
  // (e.g. making it async, or adding a confirm step). svelte-spa-router's own
  // `push()` — the library's supported programmatic-navigation API, unlike
  // raw hash assignment which is only ever used elsewhere in this codebase
  // for simple "go to a fixed static route" cases (login redirects, `#/`
  // fallback) with no modal/state teardown in flight — is used here instead,
  // and the navigation is sequenced to complete (awaited) BEFORE closing the
  // popup, not after. That way the hash write is never at risk of being
  // swallowed by whatever `onClose()` ends up doing.
  async function handleGoTo() {
    const target = `${SECTION_HASH_BY_KIND[row.kind] ?? '#/'}?id=${row.id}`;
    await push(target);
    onClose();
  }

  function handleEdit() {
    viewOpen = false;
    editOpen = true;
  }

  function handleEditClose() {
    editOpen = false;
    viewOpen = true;
  }

  // Two separate handlers (not one shared `handleEditSaved`) because the two
  // edit modals report the row's NEW place differently: `CartridgeFormModal`'s
  // `onSuccess` hands back the full saved `CartridgeDto` (new `place_id` is a
  // field on it); `DeviceFormModal`'s `onSaved` hands back only the fields it
  // was explicitly extended with (`{ typeId, placeId }`, Task 2 of this plan) —
  // same asymmetry `CartridgesPage.svelte`/`DevicesPage.svelte` already live
  // with for their own producers.
  function handleDeviceEditSaved(result?: { typeId: number; placeId: number | null }) {
    const oldPlaceId = deviceDto?.place_id ?? null;
    const newPlaceId = result?.placeId ?? null;
    editOpen = false;
    onChanged?.(
      Array.from(new Set([oldPlaceId, newPlaceId].filter((id): id is number => id !== null))),
    );
    onClose();
  }

  function handleCartridgeEditSaved(cart: CartridgeDto) {
    const oldPlaceId = cartridgeDto?.place_id ?? null;
    const newPlaceId = cart.place_id ?? null;
    editOpen = false;
    onChanged?.(
      Array.from(new Set([oldPlaceId, newPlaceId].filter((id): id is number => id !== null))),
    );
    onClose();
  }
</script>

{#snippet dataBlock()}
  {#if loading}
    <div class="view-state" aria-live="polite">
      <Spinner size="md" />
      <span>Загружаем данные…</span>
    </div>
  {:else if loadFailed}
    <p class="view-error">Не удалось загрузить данные. Закройте окно и попробуйте ещё раз.</p>
  {:else}
    {#if isCartridge && cartridgeDto}
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

    <!-- Фаза 41.1 (D-16/D-19): происхождение — ссылка на запись разбора, не на
         устройство; в таймлайн перемещений не добавляется (41.5). -->
    {#if !isCartridge && deviceDto && deviceDto.origin_group_id !== null}
      {@const originId = deviceDto.origin_group_id}
      <DetailField label="Из разбора">
        <Button
          variant="link"
          size="sm"
          onclick={async () => {
            await push(`#/groups?id=${originId}`);
            onClose();
          }}>{deviceDto.origin_group_name ?? ''}</Button
        >
      </DetailField>
    {/if}

    <AttachmentsBlock parentKind={attParentKind} parentId={row.id} bind:this={attachmentsBlock} />

    {#if attEvents.events.length > 0}
      <DetailSection heading="История вложений">
        <AttachmentHistory events={attEvents.events} onOpen={(id) => attachmentsBlock?.open(id)} />
      </DetailSection>
    {/if}
  {/if}
{/snippet}

{#snippet historyBlock()}
  <DetailSection heading="История">
    {#if timelineLoading}
      <div class="history-state" aria-live="polite">
        <Spinner size="sm" />
        <span>Загружаем историю…</span>
      </div>
    {:else if timelineError}
      <p class="view-error">
        Не удалось загрузить историю. Проверьте подключение и откройте карточку снова.
      </p>
    {:else}
      <MovementTimeline
        entries={historyEntries}
        loading={false}
        loadError={false}
        variant={isCartridge ? 'cartridge' : 'device'}
        onNavigateToPlace={async (id) => {
          await push(`#/places?id=${id}`);
          onClose();
        }}
        onNavigateToAct={async (id) => {
          await push(`#/acts?id=${id}`);
          onClose();
        }}
        onNavigateToGroup={async (id) => {
          await push(`#/groups?id=${id}`);
          onClose();
        }}
        onNavigateToRequest={(id) => {
          openRequestTarget(id, 'info');
          onClose();
        }}
      />
    {/if}
  </DetailSection>
{/snippet}

<Modal
  open={viewOpen}
  title={TITLE_BY_KIND[row.kind] ?? 'Просмотр'}
  size={splitMode ? 'split' : 'md'}
  {onClose}
>
  {#snippet headerActions()}
    <span class="history-toggle" bind:this={toggleHost}>
      <Button
        variant="ghost"
        size="sm"
        iconOnly
        ariaLabel="История"
        title="История"
        ariaPressed={pressed}
        onclick={toggleHistory}
      >
        <IconHistory />
      </Button>
    </span>
  {/snippet}

  {#if splitMode}
    <div class="view-split">
      <div class="view-split-data">
        {@render dataBlock()}
      </div>
      <section class="view-split-history">
        {@render historyBlock()}
      </section>
    </div>
  {:else}
    <div class="view-data" hidden={!wide && narrowPanelOpen}>
      {@render dataBlock()}
    </div>
    {#if !wide && narrowPanelOpen}
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <section
        class="view-panel"
        tabindex="-1"
        aria-label="История"
        bind:this={panelEl}
        onkeydown={handlePanelKeydown}
      >
        {@render historyBlock()}
      </section>
    {/if}
  {/if}

  {#snippet footer()}
    <Button variant="secondary" onclick={handleGoTo}>
      {GOTO_LABEL_BY_KIND[row.kind] ?? 'Перейти'}
    </Button>
    {#if !isCartridge && visitCount > 0}
      <Button variant="secondary" onclick={() => (historyOpen = true)}>
        Ремонты ({visitCount})
      </Button>
    {/if}
    <Button variant="primary" onclick={handleEdit} disabled={loading || loadFailed}>
      Редактировать
    </Button>
  {/snippet}
</Modal>

{#if !isCartridge && visitCount > 0}
  <RepairDeviceHistoryModal
    open={historyOpen}
    deviceId={row.id}
    deviceName={deviceDto?.name ?? row.name}
    onClose={() => (historyOpen = false)}
    onNavigate={onClose}
  />
{/if}

{#if isCartridge}
  <CartridgeFormModal
    open={editOpen}
    target={cartridgeDto}
    models={cartridgeModels}
    onClose={handleEditClose}
    onSuccess={handleCartridgeEditSaved}
  />
{:else}
  <DeviceFormModal
    open={editOpen}
    target={deviceDto}
    onClose={handleEditClose}
    onSaved={handleDeviceEditSaved}
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

  .history-state {
    display: flex;
    align-items: center;
    gap: var(--tr-space-sm);
    color: var(--tr-text-secondary);
  }

  .history-toggle {
    display: inline-flex;
  }

  // Тело split-попапа без отступов (Modal): каждая колонка прокручивается сама
  // и держит отступы внутри, поэтому полоса прокрутки данных стоит вплотную к
  // разделителю, а полоса истории — к правому краю; между контентом и полосой
  // остаётся --tr-space-xl, как в узком попапе (41.10 UAT).
  .view-split {
    display: grid;
    grid-template-columns: minmax(0, var(--modal-max-width)) minmax(360px, 1fr);
    height: 100%;
    min-height: 0;
  }

  .view-split-data,
  .view-split-history {
    min-height: 0;
    overflow-y: auto;
    padding: var(--tr-space-xl);
  }

  .view-split-history {
    border-left: 1px solid var(--tr-border);
  }

  .view-error {
    margin: 0;
    padding: var(--tr-space-md);
    color: var(--tr-danger-text);
  }
</style>
