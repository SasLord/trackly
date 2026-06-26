<script lang="ts">
  // Plan 04-05: OperationModal — единая параметризованная модалка для 5 lifecycle-операций.
  // По образцу ReturnModal.svelte: $effect reset при open, submitting state, handleSubmit с try/catch + pushToast.
  //
  // op prop: 'install' | 'return_to_stock' | 'to_refill' | 'from_refill' | 'write_off'
  // Заголовки, поля, дефолты — по UI-SPEC §Поля OperationModal + D-Op-Fields-01.
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import DatePicker from '$lib/components/DatePicker.svelte';
  import Select from '$lib/components/Select.svelte';
  import Textarea from '$lib/components/Textarea.svelte';
  import PersonAutocomplete from '$lib/components/PersonAutocomplete.svelte';
  import LocationAutocomplete from '$lib/components/LocationAutocomplete.svelte';
  import CartridgeSelect from '$lib/components/CartridgeSelect.svelte';
  import PrinterSelect from '$lib/components/PrinterSelect.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { cartridges } from './api';
  import { printers } from '../printers/api';
  import type { CartridgeDto, CartridgeTransitionPayload, PrinterDto } from '../../bindings';
  // printers.list() (used by the new D-20/D-21 selector below) returns items
  // typed against bindings-phase6's PrinterDto (the hand-maintained type
  // printers/api.ts actually uses) — distinct from bindings.ts's generated
  // PrinterDto (used elsewhere in this file for printerContext/single-get
  // lookups). The two shapes differ only in `tonerLevels`'s exact type
  // (JsonValue vs Record<string, number|null>), which is irrelevant here;
  // aliasing avoids a structural-typing mismatch on the array assignment.
  import type { PrinterDto as PrinterListItemDto } from '../../bindings-phase6';

  type Op = 'install' | 'return_to_stock' | 'to_refill' | 'from_refill' | 'write_off';

  interface Props {
    open: boolean;
    op: Op;
    cartridge: CartridgeDto | null;
    /** Pre-fill the «Принтер» context when op='install' is opened from a request (REQ-05). */
    preFillPrinterId?: number;
    /** Filter the request-centric cartridge picker to the request's model (D-02). */
    cartridgeModelId?: number;
    /** Pre-fill «Расположение» from the request's printer location (D-05). */
    prefillLocation?: string;
    /** Pre-fill «Кому отдал» from the requester's name (D-04). */
    prefillGivenToName?: string;
    /**
     * GAP-12-04/A2: when the caller (e.g. RequestDetail's
     * handleInstallSuccess) already shows its own, more specific success
     * toast (e.g. «Заявка выполнена»), set this to `true` so the modal does
     * NOT additionally show its generic «Операция выполнена успешно.» —
     * one event, one notification. Defaults to `false`/`undefined` for the
     * cartridge-centric entry (menu → «Установить в принтер», D-08), which
     * has no caller-side toast and must keep showing its own.
     */
    suppressSuccessToast?: boolean;
    onClose: () => void;
    /**
     * WR-03: may return a Promise (e.g. the request-centric flow awaits a
     * follow-up `complete` transition). `handleSubmit` awaits this before
     * showing the modal-level success toast, so a rejected follow-up never
     * produces a false-positive "Операция выполнена успешно." alongside the
     * caller's own error toast.
     */
    onSuccess: (_cartridgeId: number) => void | Promise<void>;
  }

  const {
    open,
    op,
    cartridge,
    preFillPrinterId,
    cartridgeModelId,
    prefillLocation,
    prefillGivenToName,
    suppressSuccessToast,
    onClose,
    onSuccess,
  }: Props = $props();

  // --- Form state ---
  let dateIso = $state(''); // ISO YYYY-MM-DD (DatePicker output)
  let givenByName = $state('');
  let givenToName = $state('');
  let location = $state('');
  // DEC-B (Round 5, WR-01 fix): tracks whether `location` currently holds a
  // value auto-filled from the selected printer's deviceLocation (vs. typed by
  // the operator). Lets a printer switch refresh the auto-fill while still
  // never clobbering manual input.
  let locationAutofilled = $state(false);
  let stateId = $state(3); // default: Пустой (D-Op-Fields-01)
  let notes = $state('');
  let submitting = $state(false);

  // Validation errors
  let locationError = $state('');
  let givenByError = $state('');
  let givenToError = $state('');

  // D-01..D-08 (Phase 12 Plan 03): request-centric install flow. When the
  // caller passes `cartridge={null}` with `op='install'` (RequestDetail),
  // the modal loads a flat picker of installable stock cartridges instead
  // of operating on a pre-selected cartridge. The old cartridge-centric
  // entry (menu → «Установить в принтер», `cartridge` prop set) is
  // unaffected (D-08) — `effectiveCartridge` simply prefers the prop.
  let selectedCartridge = $state<CartridgeDto | null>(null);
  let cartridgeOptions = $state<CartridgeDto[]>([]);
  let cartridgeListLoading = $state(false);

  // D-16 (Phase 12 gap closure GAP-12-03, Plan 12-09): when installing into a
  // printer that already has a cartridge «В работе», show a «Предыдущий
  // картридж» block — read-only code+model, editable charge state (default
  // Пустой/3) and location (default empty) — both flow into the same
  // transition() call via printer_device_id/previous_cartridge_state_id/
  // previous_cartridge_location, no second API request (D-16 success
  // criterion: single transition() call).
  let previousCartridge = $state<CartridgeDto | null>(null);
  let previousCartridgeStateId = $state(3); // default: Пустой
  let previousCartridgeLocation = $state('');

  const effectiveCartridge = $derived(cartridge ?? selectedCartridge);

  // Вид расходника: фотобарабан (kind 2) использует другой набор состояний.
  const isDrum = $derived(effectiveCartridge?.model_kind_id === 2);

  // D-Op-Fields-01: from_refill → Полный (1), остальные → Пустой (3).
  // Для фотобарабана при возврате на склад по умолчанию «Отработанный» (6)
  // (UAT R4 №3). install: поле состояния не показывается.
  const defaultStateId = $derived(isDrum ? 6 : op === 'from_refill' ? 1 : 3);

  // Reset form when modal opens or when `op` changes while modal is already
  // open (WR-03: stateId must track defaultStateId whenever op changes, not
  // only on open→close cycle).
  $effect(() => {
    void op; // explicit dependency: re-run when op changes
    if (open) {
      const now = new Date();
      const y = now.getFullYear();
      const m = String(now.getMonth() + 1).padStart(2, '0');
      const d = String(now.getDate()).padStart(2, '0');
      dateIso = `${y}-${m}-${d}`;
      givenByName = '';
      givenToName = prefillGivenToName ?? '';
      location = prefillLocation ?? '';
      locationAutofilled = false;
      notes = '';
      stateId = defaultStateId;
      selectedCartridge = null;
      selectedPrinterId = undefined;
      previousCartridge = null;
      previousCartridgeStateId = 3;
      previousCartridgeLocation = '';
      printerContext = null;
      locationError = '';
      givenByError = '';
      givenToError = '';
    }
  });

  // GAP-12-05/A2 (Plan 12-12): the target printer's full DTO (deviceName +
  // ipAddress), populated by the lookup $effect below. Drives
  // printerContextHint so the hint shows the physical printer's name+IP
  // instead of an abstract #id (UX clarity — operator needs to recognize
  // the actual device, not a database key).
  let printerContext = $state<PrinterDto | null>(null);

  // D-20/D-21/D-22 (Round 4 gap-closure, Plan 12-20): the cartridge-centric
  // install entry (menu → «Установить в принтер», cartridge prop set) had no
  // UI for choosing a printer at all — preFillPrinterId only ever arrives
  // from the request-centric flow (RequestDetail). selectedPrinterId is the
  // new local choice made via the PrinterSelect rendered below; it is
  // OPTIONAL (D-20) — undefined means "no printer", identical to legacy
  // behavior (no regression).
  let selectedPrinterId = $state<number | undefined>(undefined);
  // Full printer list + reverse compatibility lookup (device_ids compatible
  // with effectiveCartridge.model_id) for the new selector (D-21).
  let printerOptions = $state<PrinterListItemDto[]>([]);
  let compatibleDeviceIds = $state<Set<number>>(new Set());

  // Single source of truth for "which printer is this install targeting":
  // request-centric flow keeps using the preFillPrinterId prop (priority);
  // cartridge-centric flow uses the new local selectedPrinterId. Both feed
  // the SAME downstream lookup/payload logic below.
  const effectivePrinterId = $derived(preFillPrinterId ?? selectedPrinterId);

  // D-20 (Round 4) / DEC-A (Round 5, Phase 12 gap closure): true when the
  // PrinterSelect selector itself is visible in the form (cartridge-centric
  // install with no incoming printer context). In that case the printer's
  // NAME is already shown by the selector's own option label — repeating it
  // in the hint below is redundant, so DEC-A drops the name there and shows
  // only #id + IP. The request-centric entry (selector never rendered) keeps
  // the original name+IP hint — no regression.
  const isSelectorVisible = $derived(
    op === 'install' && cartridge !== null && preFillPrinterId === undefined,
  );

  // REQ-05: preFillPrinterId is accepted as context when the modal is opened
  // from a request (RequestDetail). The install form is cartridge-centric;
  // we show a hint about which printer this cartridge targets when the prop is set.
  // GAP-12-05/A2: prefer printerContext.deviceName + ipAddress once the lookup
  // resolves; fall back to #{id} only while loading or if deviceName is absent.
  // DEC-A (Round 5): when the selector is visible, omit the name (already
  // shown by the selector) — hint shows only #id + IP.
  const printerContextHint = $derived(
    op === 'install' && effectivePrinterId !== undefined
      ? printerContext !== null
        ? isSelectorVisible
          ? `Устанавливается в принтер: #${effectivePrinterId}${
              printerContext.ipAddress ? ` (${printerContext.ipAddress})` : ''
            }`
          : `Устанавливается в принтер: ${
              printerContext.deviceName ?? `#${effectivePrinterId}`
            }${printerContext.ipAddress ? ` (${printerContext.ipAddress})` : ''}`
        : `Устанавливается в принтер #${effectivePrinterId}`
      : null,
  );

  // D-16 (Plan 12-09): look up the target printer's current cartridge «В
  // работе» (if any) so the «Предыдущий картридж» block can show it. Runs
  // whenever there IS a printer context (preFillPrinterId !== undefined),
  // in BOTH the request-centric (cartridge===null) and cartridge-centric
  // (cartridge!=null) install entries — GAP-12-11: the cartridge-centric
  // entry (menu → «Установить в принтер») needs the same printer name/IP
  // hint and previous-cartridge block as the request-centric one. When no
  // printer context exists (preFillPrinterId undefined), the lookup is
  // skipped entirely, avoiding a wasted API call (Test 3, D-08 regression
  // guard for the printer-less cartridge-centric flows).
  // GAP-12-05/A2: also stores the printer DTO itself into `printerContext`
  // (same lookup call — no second API request) so printerContextHint can
  // render deviceName+ipAddress.
  // GAP-12-13 (Phase 12 Round 5): effectivePrinterId is ALWAYS a device_id
  // (PrinterSelect emits String(p.deviceId); preFillPrinterId is
  // request.printerDeviceId) — printers.get() resolves by printers.id, so it
  // never matched and printerContext stayed null forever, in both install
  // entries. Switched to getByDeviceId, which resolves the actual contract.
  $effect(() => {
    if (!(open && op === 'install' && effectivePrinterId !== undefined)) {
      previousCartridge = null;
      printerContext = null;
      // DEC-B/WR-01: deselecting the printer ("Без привязки") drops an
      // auto-filled location so a stale value isn't recorded; manual input
      // (locationAutofilled === false) is left untouched.
      if (preFillPrinterId === undefined && locationAutofilled) {
        location = '';
        locationAutofilled = false;
      }
      return;
    }
    // WR-05: cancellation guard — if the operator switches/deselects the
    // printer (or reopens the modal) while this getByDeviceId round-trip is
    // in flight, ignore the late resolution so it can't overwrite
    // printerContext/previousCartridge with data for the previous printer.
    let cancelled = false;
    printers
      .getByDeviceId(effectivePrinterId)
      .then((printer) => {
        if (cancelled) return null;
        printerContext = printer;
        // DEC-B (Phase 12 Round 5): in the cartridge-centric entry (no
        // incoming preFillPrinterId — the operator just picked a printer via
        // the selector below), auto-fill «Расположение» from the printer's
        // device location. Never clobbers manual operator input: fills while
        // the field is empty OR still holds a prior auto-fill (WR-01 — so
        // switching printers refreshes the location instead of keeping the
        // first printer's value).
        if (
          preFillPrinterId === undefined &&
          printer.deviceLocation &&
          (!location.trim() || locationAutofilled)
        ) {
          location = printer.deviceLocation;
          locationAutofilled = true;
        }
        if (printer.currentCartridgeId === null) {
          previousCartridge = null;
          return null;
        }
        return cartridges.get(printer.currentCartridgeId);
      })
      .then((prev) => {
        if (cancelled) return;
        if (prev !== null && prev !== undefined) {
          previousCartridge = prev;
          // R7/D-11: kind-aware default — выставляется одновременно с
          // обнаружением previousCartridge (а не статически при открытии
          // формы), зеркалит серверный kind-aware default (Plan 13-04, D-10).
          previousCartridgeStateId = prev.model_kind_id === 2 ? 5 : 3;
        }
      })
      .catch(() => {
        if (cancelled) return;
        // Fail-safe: a failed lookup just means no previous-cartridge block
        // is shown — install can still proceed normally.
        previousCartridge = null;
      });
    return () => {
      cancelled = true;
    };
  });

  // D-13/D-14 (Phase 12 gap closure GAP-12-02): the request-centric install
  // flow (cartridge === null) narrows the picker to cartridges compatible
  // with the request's printer. When a printer context exists
  // (preFillPrinterId set) but has zero configured compatibility links, the
  // backend filter self-adjusts to show the full unfiltered kind_id=1 stock
  // (D-14) — this flag drives the warning that tells the operator
  // compatibility was never configured for this printer, so they should
  // verify the fit manually. Replaces the old WR-02 placeholder (which
  // warned on missing `cartridgeModelId`, not on missing printer↔model
  // links).
  //
  // D-21/R4 (Phase 13): source is V005 cartridge_model_compatibility via
  // printer_name matching (Plan 13-03's printers_get_compatible_aggregates),
  // not the deleted per-device junction (printers_get_compatible_models,
  // V029, removed in Plan 13-03). `models.length === 0` is the direct
  // equivalent of the old `modelIds.length === 0` check — both express
  // "this printer has zero compatible cartridge models".
  let compatibilityUnconfigured = $state(false);

  $effect(() => {
    if (!(open && op === 'install' && cartridge === null && preFillPrinterId !== undefined)) {
      compatibilityUnconfigured = false;
      return;
    }
    // WR-05: cancellation guard against a stale getCompatibleAggregates
    // resolution overwriting the flag for a printer the operator has since
    // changed away from.
    let cancelled = false;
    printers
      .getCompatibleAggregates(preFillPrinterId)
      .then((res) => {
        if (cancelled) return;
        compatibilityUnconfigured = res.models.length === 0;
      })
      .catch(() => {
        if (cancelled) return;
        // Fail-safe default (T-12-08-01): a failed compatibility check is a
        // UX hint only, not a security boundary — never show the warning
        // off of an error state.
        compatibilityUnconfigured = false;
      });
    return () => {
      cancelled = true;
    };
  });

  // D-20/D-21 (Round 4 gap-closure, Plan 12-20): load the full printer list
  // + reverse compatibility lookup for the NEW cartridge-centric printer
  // selector. Gated strictly on cartridge-centric install with NO incoming
  // printer context (preFillPrinterId === undefined) — the request-centric
  // flow (cartridge === null) and the pre-filled flow (printer context from
  // a request) never trigger this, no extra network calls there (D-08/D-20
  // regression guard, T-12-20-04).
  //
  // D-21/R3 (Phase 13): client-side derivation over V005 — cartridge.model_id
  // gives compatibility: string[] (printer names), matched against
  // printerOptions[].deviceName (case-insensitive+trim, mirrors the server-
  // side matching from Plan 13-01) to build compatibleDeviceIds, used by
  // PrinterSelect for highlighting. Replaces the deleted per-device junction
  // lookup (cartridges_models_get_compatible_devices, V029, removed in Plan
  // 13-03).
  $effect(() => {
    if (!(open && op === 'install' && cartridge !== null && preFillPrinterId === undefined)) {
      printerOptions = [];
      compatibleDeviceIds = new Set();
      return;
    }
    // WR-05: cancellation guard — if the operator switches the selected
    // cartridge (or reopens for a different one) before this list +
    // modelsGet round-trip settles, ignore the late resolution so
    // compatibleDeviceIds isn't overwritten with results for the previous
    // cartridge's model_id.
    let cancelled = false;
    Promise.all([
      printers.list({ status: null, search: null }, { offset: 0, limit: 500 }),
      cartridges.modelsGet(cartridge.model_id),
    ])
      .then(([printersRes, modelRes]) => {
        if (cancelled) return;
        printerOptions = printersRes.items;
        if (modelRes.compatibility.length === 0) {
          // D-05 pass-through: empty compatibility means "compatible with
          // any printer" — every printer in the list counts as compatible.
          compatibleDeviceIds = new Set(printerOptions.map((p) => p.deviceId));
          return;
        }
        // D-03: case-insensitive + trim matching, identical to the server-
        // side comparison in Plan 13-01.
        const normalizedNames = new Set(
          modelRes.compatibility.map((n) => n.trim().toLowerCase()),
        );
        compatibleDeviceIds = new Set(
          printerOptions
            .filter((p) => normalizedNames.has((p.deviceName ?? '').trim().toLowerCase()))
            .map((p) => p.deviceId),
        );
      })
      .catch(() => {
        if (cancelled) return;
        // Fail-safe (D-21): a failed lookup must not block install without a
        // printer — worst case the selector shows "Принтеры не найдены",
        // which is still a valid no-printer path (D-20).
        printerOptions = [];
        compatibleDeviceIds = new Set();
      });
    return () => {
      cancelled = true;
    };
  });

  // D-01/D-02 (Phase 12 Plan 03): load the installable-stock cartridge list
  // when the modal is opened for the request-centric install flow
  // (cartridge prop === null). The cartridge-centric flow (menu →
  // «Установить в принтер») never triggers this — `cartridge` is non-null
  // there, so no extra network call is made (D-08 regression guard).
  $effect(() => {
    if (!(open && op === 'install' && cartridge === null)) return;
    cartridgeListLoading = true;
    cartridges
      .list(
        {
          status_id: 1,
          installable_only: true,
          model_id: cartridgeModelId ?? null,
          kind_id: 1,
          search: null,
          include_deleted: false,
          compatible_with_printer_device_id: preFillPrinterId ?? null,
        },
        { offset: 0, limit: 200 },
      )
      .then((res) => {
        cartridgeOptions = res.items;
      })
      .catch((e: unknown) => {
        cartridgeOptions = [];
        const msg =
          e && typeof e === 'object' && 'message' in e
            ? String((e as { message: unknown }).message)
            : 'Не удалось загрузить список картриджей.';
        pushToast('error', msg);
      })
      .finally(() => {
        cartridgeListLoading = false;
      });
  });

  // Modal titles (UI-SPEC §Заголовки OperationModal)
  const MODAL_TITLES: Record<Op, string> = {
    install: 'Установка в принтер',
    return_to_stock: 'Возврат на склад',
    to_refill: 'Отправка на заправку',
    from_refill: 'Получение с заправки',
    write_off: 'Списание картриджа',
  };

  // Confirm button labels (UI-SPEC §Primary CTA)
  const CONFIRM_LABELS: Record<Op, string> = {
    install: 'Установить',
    return_to_stock: 'Вернуть на склад',
    to_refill: 'Отправить на заправку',
    from_refill: 'Вернуть с заправки',
    write_off: 'Списать',
  };

  // State options for Select — по виду расходника (V017).
  const CARTRIDGE_STATES = [
    { value: 1, label: 'Полный' },
    { value: 2, label: 'Частичный' },
    { value: 3, label: 'Пустой' },
  ];
  const DRUM_STATES = [
    { value: 4, label: 'Новый' },
    { value: 5, label: 'Изношенный' },
    { value: 6, label: 'Отработанный' },
  ];
  const stateOptions = $derived(isDrum ? DRUM_STATES : CARTRIDGE_STATES);
  const stateFieldLabel = $derived(isDrum ? 'Состояние' : 'Состояние заряда');

  // R7/D-11 (13-SPEC.md): previous-cartridge state Select больше не
  // хардкодит 1/2/3 — переиспользует stateOptions-паттерн (DRUM_STATES/
  // CARTRIDGE_STATES) по previousCartridge.model_kind_id (ДРУГОЙ картридж в
  // форме, не effectiveCartridge); дефолт 5 «Изношенный» для фотобарабана
  // зеркалит серверный kind-aware default из Plan 13-04 (D-10).
  const prevIsDrum = $derived(previousCartridge?.model_kind_id === 2);
  const prevStateOptions = $derived(prevIsDrum ? DRUM_STATES : CARTRIDGE_STATES);

  // Convert ISO date string to unix seconds
  function isoToUnix(iso: string): number {
    if (!iso) return Math.floor(Date.now() / 1000);
    return Math.floor(new Date(iso + 'T00:00:00Z').getTime() / 1000);
  }

  // Build payload from form state
  function buildPayload(): CartridgeTransitionPayload {
    const id = effectiveCartridge!.id;
    const version = effectiveCartridge!.version;

    if (op === 'install') {
      return {
        op: 'install',
        cartridge_id: id,
        version,
        date_utc: isoToUnix(dateIso),
        given_by_name: givenByName.trim(),
        given_to_name: givenToName.trim(),
        location: location.trim(),
        printer_device_id: effectivePrinterId ?? null,
        previous_cartridge_state_id: previousCartridge !== null ? previousCartridgeStateId : null,
        previous_cartridge_location:
          previousCartridge !== null ? previousCartridgeLocation : null,
      };
    } else if (op === 'return_to_stock') {
      return {
        op: 'return_to_stock',
        cartridge_id: id,
        version,
        state_id: stateId,
        location: location.trim(),
        notes: notes.trim() || null,
      };
    } else if (op === 'to_refill') {
      return {
        op: 'to_refill',
        cartridge_id: id,
        version,
        date_utc: isoToUnix(dateIso),
        given_by_name: givenByName.trim(),
        given_to_name: givenToName.trim(),
        location: location.trim(),
      };
    } else if (op === 'from_refill') {
      return {
        op: 'from_refill',
        cartridge_id: id,
        version,
        state_id: stateId,
        location: location.trim(),
        notes: notes.trim() || null,
      };
    } else {
      // write_off
      return {
        op: 'write_off',
        cartridge_id: id,
        version,
        date_utc: isoToUnix(dateIso),
        notes: notes.trim() || null,
      };
    }
  }

  function validate(): boolean {
    let valid = true;
    locationError = '';
    givenByError = '';
    givenToError = '';

    if (op === 'install' || op === 'to_refill') {
      if (!givenByName.trim()) {
        givenByError = 'Заполните это поле';
        valid = false;
      }
      if (!givenToName.trim()) {
        givenToError = 'Заполните это поле';
        valid = false;
      }
      if (!location.trim()) {
        locationError = 'Заполните это поле';
        valid = false;
      }
    } else if (op === 'return_to_stock' || op === 'from_refill') {
      if (!location.trim()) {
        locationError = 'Заполните это поле';
        valid = false;
      }
    }
    // write_off: no required fields beyond date (auto-filled)

    return valid;
  }

  async function handleSubmit() {
    if (!effectiveCartridge || submitting) return;
    if (!validate()) return;

    submitting = true;
    try {
      await cartridges.transition(buildPayload());
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось выполнить операцию. Повторите попытку.';
      pushToast('error', msg);
      submitting = false;
      return;
    }

    // WR-03: cartridge transition succeeded — now await the caller's
    // onSuccess (e.g. RequestDetail's handleInstallSuccess, which completes
    // the request). Only announce the modal-level success once onSuccess
    // resolves; if it rejects, the caller is responsible for its own
    // error toast (it already owns the more specific failure message), so
    // we just close without adding a duplicate/contradictory toast here.
    //
    // GAP-12-04/A2: skip this generic toast entirely when the caller passed
    // suppressSuccessToast={true} — it means the caller already shows its
    // own, more specific success toast (e.g. RequestDetail's «Заявка
    // выполнена») and a second toast here would be a duplicate notification
    // for the same event.
    try {
      await onSuccess(effectiveCartridge.id);
      onClose();
      if (!suppressSuccessToast) {
        pushToast('success', `Операция выполнена успешно.`);
      }
    } catch {
      onClose();
    } finally {
      submitting = false;
    }
  }

  const modalTitle = $derived(MODAL_TITLES[op] ?? 'Операция');
  const confirmLabel = $derived(CONFIRM_LABELS[op] ?? 'Подтвердить');

  // canSubmit — simple check (required validation happens in handleSubmit)
  const canSubmit = $derived(!submitting && !!effectiveCartridge);
</script>

<Modal {open} title={modalTitle} size="md" {onClose}>
  <form
    class="form"
    onsubmit={(e) => {
      e.preventDefault();
      handleSubmit();
    }}
  >
    <!-- Поля по op (UI-SPEC §Поля OperationModal) -->

    {#if op === 'install' || op === 'to_refill'}
      {#if op === 'install' && cartridge !== null && preFillPrinterId === undefined}
        <!-- D-20/D-21 (Round 4 gap-closure, Plan 12-20): optional printer
             selector for the cartridge-centric install entry — only shown
             when there's no incoming printer context (request-centric flow
             already has its own preFillPrinterId, no regression there). -->
        <div class="field">
          <label class="label" for="op-printer">Принтер (опционально)</label>
          <PrinterSelect
            options={printerOptions}
            {compatibleDeviceIds}
            value={selectedPrinterId !== undefined ? String(selectedPrinterId) : ''}
            id="op-printer"
            onchange={(v) => {
              selectedPrinterId = v ? parseInt(v, 10) : undefined;
            }}
          />
        </div>
      {/if}
      {#if printerContextHint}
        <!-- GAP-12-05/A2: printer context (deviceName+ipAddress) renders
             FIRST in the form, before the cartridge-select, so the operator
             immediately sees which physical printer they're installing
             into — not buried after the picker. -->
        <p class="field-hint">{printerContextHint}</p>
      {/if}
      {#if op === 'install' && cartridge === null}
        <!-- D-01/D-02/D-03/D-08: request-centric install flow — pick a
             physical cartridge from the installable-stock list. Not shown
             when `cartridge` is already set (old cartridge-centric entry). -->
        <div class="field">
          <label class="label" for="op-cartridge">Картридж</label>
          <CartridgeSelect
            options={cartridgeOptions}
            value={selectedCartridge ? String(selectedCartridge.id) : ''}
            disabled={cartridgeListLoading}
            id="op-cartridge"
            onchange={(v) => {
              selectedCartridge = cartridgeOptions.find((c) => String(c.id) === v) ?? null;
            }}
          />
          {#if compatibilityUnconfigured}
            <span class="field-warning">Совместимость не задана — проверьте вручную</span>
          {/if}
        </div>
      {/if}
      {#if previousCartridge}
        <!-- D-16 (Plan 12-09): previous-cartridge block — shown only when the
             target printer already has a cartridge «В работе». Read-only
             code+model; editable charge state/location flow into the SAME
             transition() call via buildPayload(), no second request. -->
        <div class="field field-full previous-cartridge-block">
          <p class="field-hint">
            Сейчас в принтере: {previousCartridge.code} ({previousCartridge.model_brand}
            {previousCartridge.model_name})
          </p>
          <!-- GAP-12-05/A2: purely informational hint explaining the
               inverted Кто/Кому semantics — no new fields, the existing
               «Кто выдал»/«Кому выдал» inputs below already apply to the
               NEW cartridge and (by the reverse-role logic) to the
               returned previous cartridge. -->
          <p class="field-hint">
            Поля «Кто выдал» / «Кому выдал» ниже относятся к НОВОМУ картриджу. При возврате этого
            картриджа на склад роли переворачиваются: кто устанавливает новый — принимает этот
            обратно; кому устанавливают новый — тот и сдаёт этот на возврат.
          </p>
          <label class="label" for="op-prev-state">Состояние заряда (предыдущий картридж)</label>
          <Select
            value={String(previousCartridgeStateId)}
            id="op-prev-state"
            onchange={(v) => (previousCartridgeStateId = parseInt(v, 10))}
          >
            {#each prevStateOptions as opt (opt.value)}
              <option value={String(opt.value)}>{opt.label}</option>
            {/each}
          </Select>
          <label class="label" for="op-prev-location">Расположение (предыдущий картридж)</label>
          <LocationAutocomplete
            value={previousCartridgeLocation}
            placeholder="Расположение"
            id="op-prev-location"
            onChange={(v) => (previousCartridgeLocation = v)}
          />
        </div>
      {/if}
      <!-- Дата -->
      <div class="field">
        <label class="label" for="op-date">Дата</label>
        <DatePicker bind:value={dateIso} id="op-date" required />
      </div>

      <!-- Кто выдал -->
      <div class="field">
        <label class="label" for="op-given-by">Кто выдал</label>
        <PersonAutocomplete
          field="giver"
          bind:value={givenByName}
          placeholder="ФИО выдавшего"
          id="op-given-by"
          invalid={!!givenByError}
        />
        {#if givenByError}
          <span class="field-error">{givenByError}</span>
        {/if}
      </div>

      <!-- Кому выдал -->
      <div class="field">
        <label class="label" for="op-given-to">Кому выдал</label>
        <PersonAutocomplete
          field="receiver"
          bind:value={givenToName}
          placeholder="ФИО получившего"
          id="op-given-to"
          invalid={!!givenToError}
        />
        {#if givenToError}
          <span class="field-error">{givenToError}</span>
        {/if}
      </div>

      <!-- Расположение -->
      <div class="field">
        <label class="label" for="op-location">Расположение</label>
        <LocationAutocomplete
          value={location}
          placeholder="Расположение"
          id="op-location"
          invalid={!!locationError}
          onChange={(v) => {
            location = v;
            // WR-01: a manual edit unmarks the auto-fill so a later printer
            // switch will not overwrite what the operator typed.
            locationAutofilled = false;
          }}
        />
        {#if locationError}
          <span class="field-error">{locationError}</span>
        {:else if op === 'install'}
          <span class="field-hint">Укажите рабочее место или кабинет (не склад)</span>
        {/if}
      </div>
    {:else if op === 'return_to_stock' || op === 'from_refill'}
      <!-- Состояние (заряда — для картриджей; для фотобарабанов — состояние) -->
      <div class="field">
        <label class="label" for="op-state">{stateFieldLabel}</label>
        <Select value={String(stateId)} id="op-state" onchange={(v) => (stateId = parseInt(v, 10))}>
          {#each stateOptions as opt (opt.value)}
            <option value={String(opt.value)}>{opt.label}</option>
          {/each}
        </Select>
      </div>

      <!-- Расположение -->
      <div class="field">
        <label class="label" for="op-location">Расположение</label>
        <LocationAutocomplete
          value={location}
          placeholder="Расположение"
          id="op-location"
          invalid={!!locationError}
          onChange={(v) => (location = v)}
        />
        {#if locationError}
          <span class="field-error">{locationError}</span>
        {:else if op === 'return_to_stock'}
          <span class="field-hint">Укажите склад или место хранения</span>
        {/if}
      </div>

      <!-- Примечание (optional) -->
      <div class="field">
        <label class="label" for="op-notes">Примечание</label>
        <Textarea
          value={notes}
          placeholder="Необязательно"
          id="op-notes"
          oninput={(v) => (notes = v)}
        />
      </div>
    {:else if op === 'write_off'}
      <!-- Дата -->
      <div class="field">
        <label class="label" for="op-date">Дата</label>
        <DatePicker bind:value={dateIso} id="op-date" required />
      </div>

      <!-- Причина / Примечание (optional) -->
      <div class="field">
        <label class="label" for="op-notes">Причина / Примечание</label>
        <Textarea
          value={notes}
          placeholder="Необязательно"
          id="op-notes"
          oninput={(v) => (notes = v)}
        />
      </div>
    {/if}
  </form>

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>Отмена</Button>
    <Button variant="primary" loading={submitting} disabled={!canSubmit} onclick={handleSubmit}>
      {confirmLabel}
    </Button>
  {/snippet}
</Modal>

<style lang="scss">
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .label {
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
    font-weight: var(--font-weight-regular);
  }

  .field-hint {
    font-size: var(--font-size-label);
    color: var(--color-text-muted);
  }

  .field-error {
    font-size: var(--font-size-label);
    color: var(--color-destructive);
  }

  .field-warning {
    font-size: var(--font-size-label);
    color: var(--color-warning);
  }

  .field-full {
    width: 100%;
  }

  .previous-cartridge-block {
    padding: var(--space-sm);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    background: var(--color-surface);
  }
</style>
