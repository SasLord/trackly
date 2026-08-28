<script lang="ts">
  // Phase 39 Plan 19 (PLC-01/PLC-02): create/rename modal per UI-SPEC §11.1-§11.2.
  // Shared by Plan 14's tree ActionMenu ("Создать вложенное место" / "Переименовать")
  // and the Places page's primary "Создать место" button.
  //
  // Backend surface note (Plan 12): `places_rename(id, name, version)` mutates ONLY
  // `name` — there is no `places_update` for kind/parent/level/is_storage/sort_order
  // once a place exists (those are create-time-only fields). Rendering them as
  // editable in rename mode would be dead UI whose edits are silently discarded on
  // submit — a Rule 1 bug, not a stylistic choice — so this component shows only
  // "Название" in rename mode and submits nothing else.
  //
  // Phase 39.1 Plan 09 (PLC-08): "Вариант сокращения" is the one exception to the
  // rule above — unlike kind/parent/level/is_storage/sort_order, it stays mutable
  // after a place exists (D-12), so it is shown and editable in BOTH create and
  // rename mode. It is saved by a SECOND network call (`places_set_path_variant`,
  // plan 07) issued right after `places_create`/`places_rename` inside the same
  // `handleSubmit` — one user-facing submit, two requests — rather than by
  // widening `places_rename` itself.
  //
  // No `open` prop: per this plan's props contract, the caller conditionally
  // mounts/unmounts this component ({#if}) rather than toggling visibility — each
  // mount is a fresh form instance, matching `mode`/`place`/`defaultParentId` as
  // given at mount time.
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Input from '$lib/components/Input.svelte';
  import Dropdown from '$lib/components/Dropdown.svelte';
  import Checkbox from '$lib/components/Checkbox.svelte';
  import PlacePicker from '$lib/components/PlacePicker.svelte';
  import { apiCall } from '$lib/api/client';
  import { pushToast } from '$lib/stores/toast.svelte';
  import type { AppError } from '$lib/api/errors';
  import type { PlaceDto, PlaceNewDto } from '../../bindings';

  interface Props {
    mode: 'create' | 'rename';
    place: PlaceDto | null;
    defaultParentId: number | null;
    onClose: () => void;
    onSaved: (_place: PlaceDto) => void;
  }

  const { mode, place, defaultParentId, onClose, onSaved }: Props = $props();

  interface KindOption {
    value: string;
    label: string;
  }

  // D-02: exactly six closed kind tokens, copy per UI-SPEC §14.1.
  const KIND_OPTIONS: KindOption[] = [
    { value: 'territory', label: 'Территория' },
    { value: 'zone', label: 'Зона' },
    { value: 'building', label: 'Здание' },
    { value: 'floor', label: 'Этаж' },
    { value: 'room', label: 'Помещение' },
    { value: 'outdoor', label: 'Уличный объект' },
  ];

  // D-01: suggestion only — any of the six values remains selectable regardless
  // of parent kind. UI-SPEC explicitly names two mappings (building→floor,
  // floor→room); the remaining four are this component's own reasonable
  // defaults for the same "typical next level down" pattern.
  const CHILD_KIND_SUGGESTION: Record<string, string> = {
    territory: 'zone',
    zone: 'building',
    building: 'floor',
    floor: 'room',
    room: 'room',
    outdoor: 'outdoor',
  };

  // Plain option list, no drill-in (SC #3 flat mode) — Dropdown still requires a
  // typed callback for its drill-in signature even when never invoked.
  function noExpandKind(): KindOption[] {
    return [];
  }

  interface PathVariantOption {
    value: string | null;
    label: string;
  }

  // D-05: literally "Как у родителя" — no hint/suffix naming the inherited
  // variant, in any mode, for any place (including top-level ones, D-06).
  const PATH_VARIANT_OPTIONS: PathVariantOption[] = [
    { value: null, label: 'Как у родителя' },
    { value: 'ends', label: 'Крайние' },
    { value: 'last_two', label: 'Два последних' },
    { value: 'last', label: 'Последнее' },
  ];

  function noExpandPathVariant(): PathVariantOption[] {
    return [];
  }

  const isRename = $derived(mode === 'rename');

  let name = $state(mode === 'rename' && place ? place.name : '');
  // Captured once at mount so handleSubmit can tell "user changed the variant"
  // from "user never touched it" — see the guarded second RPC below.
  const initialPathVariant =
    mode === 'rename' && place ? (place.path_variant_override ?? null) : null;
  let pathVariant = $state<string | null>(initialPathVariant);
  let kind = $state('');
  let parentId = $state<number | null>(defaultParentId);
  let level = $state('');
  let isStorage = $state(false);
  let sortOrder = $state('');

  // Not $state — read synchronously inside the suggestion effect, doesn't drive
  // any rendered output on its own.
  let kindTouched = false;

  let nameErr = $state<string | null>(null);
  let kindErr = $state<string | null>(null);
  let levelErr = $state<string | null>(null);
  let serverErr = $state<string | null>(null);
  let saving = $state(false);

  const showLevel = $derived(kind === 'floor');
  const kindLabel = $derived(KIND_OPTIONS.find((o) => o.value === kind)?.label ?? '');
  const pathVariantLabel = $derived(
    PATH_VARIANT_OPTIONS.find((o) => o.value === pathVariant)?.label ?? '',
  );
  const modalTitle = $derived(isRename ? 'Переименовать место' : 'Новое место');
  const submitLabel = $derived(isRename ? 'Сохранить' : 'Создать');

  // D-01 suggestion — create mode only, and only until the user has manually
  // picked a type (a later parent change must not clobber a deliberate choice).
  $effect(() => {
    const pid = parentId;
    if (isRename || pid === null || kindTouched) return;
    let cancelled = false;
    apiCall<PlaceDto>('places_get', { id: pid })
      .then((parent) => {
        if (cancelled || kindTouched) return;
        const suggestion = CHILD_KIND_SUGGESTION[parent.kind];
        if (suggestion) kind = suggestion;
      })
      .catch(() => {
        // Convenience-only lookup — a failed fetch must not block the form.
      });
    return () => {
      cancelled = true;
    };
  });

  function pickKind(o: KindOption) {
    kind = o.value;
    kindTouched = true;
    kindErr = null;
  }

  function pickPathVariant(o: PathVariantOption) {
    pathVariant = o.value;
  }

  function parseOptionalInt(raw: string): number | null {
    const trimmed = raw.trim();
    if (trimmed === '') return null;
    const parsed = Number(trimmed);
    return Number.isInteger(parsed) ? parsed : null;
  }

  function validate(): boolean {
    nameErr = null;
    kindErr = null;
    levelErr = null;
    serverErr = null;
    let ok = true;

    if (name.trim().length === 0) {
      nameErr = 'Укажите название места.';
      ok = false;
    }
    if (!isRename) {
      if (!kind) {
        kindErr = 'Выберите тип места.';
        ok = false;
      }
      // PLC-02: 0/negative are valid — only non-integer input is rejected, and
      // only client-side, since the wire type is a plain i64 with no friendly
      // server-side message for a malformed number.
      if (kind === 'floor' && level.trim() !== '' && !Number.isInteger(Number(level))) {
        levelErr = 'Уровень этажа — целое число. Подвал — отрицательное значение.';
        ok = false;
      }
    }
    return ok;
  }

  function mapServerError(e: unknown): void {
    const err = e as Partial<AppError> | undefined;
    const details = err?.details;
    const field =
      details && typeof details === 'object' && !Array.isArray(details) && 'field' in details
        ? (details as { field?: unknown }).field
        : undefined;
    if (err?.code === 'VALIDATION' && field === 'name') {
      nameErr = err.message ?? 'Ошибка валидации';
      return;
    }
    serverErr = err?.message ?? 'Не удалось сохранить место.';
  }

  async function handleSubmit() {
    if (!validate()) return;
    saving = true;
    serverErr = null;
    try {
      let saved: PlaceDto;
      if (isRename && place) {
        saved = await apiCall<PlaceDto>('places_rename', {
          id: place.id,
          name: name.trim(),
          version: place.version,
        });
      } else {
        const newPlace: PlaceNewDto = {
          parent_id: parentId,
          kind,
          name: name.trim(),
          level: kind === 'floor' ? parseOptionalInt(level) : null,
          is_storage: isStorage,
          sort_order: parseOptionalInt(sortOrder),
          notes: null,
        };
        saved = await apiCall<PlaceDto>('places_create', { place: newPlace });
      }
      // PLC-08 (plan 07/09): second RPC, reusing the `version` the FIRST call
      // just returned (not the stale `place.version` captured before submit).
      //
      // Only fire it when the variant actually changed. Firing unconditionally
      // wrote a no-op audit_log entry and bumped `version` on every create and
      // rename (invalidating any other open form for the same place), and — far
      // worse — a failure here would surface as a plain error toast even though
      // the create/rename had already committed, so the user would press
      // «Создать» again and get a duplicate place (code review CR-02).
      if (pathVariant !== initialPathVariant) {
        try {
          saved = await apiCall<PlaceDto>('places_set_path_variant', {
            id: saved.id,
            pathVariantOverride: pathVariant,
            version: saved.version,
          });
        } catch {
          // The primary mutation already landed. Report the PARTIAL failure and
          // close the form — re-submitting would duplicate the place.
          pushToast(
            'error',
            isRename
              ? 'Место переименовано, но вариант сокращения не применён'
              : 'Место создано, но вариант сокращения не применён',
          );
          onSaved(saved);
          return;
        }
      }
      pushToast('success', isRename ? 'Место переименовано' : 'Место создано');
      onSaved(saved);
    } catch (e) {
      mapServerError(e);
    } finally {
      saving = false;
    }
  }
</script>

<Modal open={true} title={modalTitle} {onClose}>
  <div class="place-form">
    <div class="form-field" class:has-error={nameErr !== null}>
      <label class="form-label" for="pf-name">Название</label>
      <Input
        id="pf-name"
        value={name}
        invalid={nameErr !== null}
        disabled={saving}
        placeholder="Например, 214"
        oninput={(v) => {
          name = v;
          nameErr = null;
        }}
      />
      {#if nameErr}
        <span class="field-error">{nameErr}</span>
      {/if}
    </div>

    <div class="form-field">
      <label class="form-label" for="pf-path-variant">Вариант сокращения</label>
      <Dropdown
        id="pf-path-variant"
        variant="select"
        flat={true}
        value={pathVariantLabel}
        placeholder="Как у родителя"
        searchable={false}
        disabled={saving}
        loading={false}
        groups={PATH_VARIANT_OPTIONS}
        getGroupId={(o) => o.value ?? '__inherit__'}
        getGroupName={(o) => o.label}
        getGroupCount={() => 0}
        isGroupExpandable={() => false}
        isGroupSelected={(o) => o.value === pathVariant}
        onExpandGroup={noExpandPathVariant}
        getMemberId={(o) => o.value ?? '__inherit__'}
        getMemberName={(o) => o.label}
        onSearch={() => {}}
        onPickGroup={pickPathVariant}
        onPickMember={() => {}}
      />
    </div>

    {#if !isRename}
      <div class="form-field" class:has-error={kindErr !== null}>
        <label class="form-label" for="pf-kind">Тип</label>
        <Dropdown
          id="pf-kind"
          variant="select"
          flat={true}
          value={kindLabel}
          placeholder="Выберите тип"
          searchPlaceholder="Поиск"
          invalid={kindErr !== null}
          disabled={saving}
          loading={false}
          groups={KIND_OPTIONS}
          getGroupId={(o) => o.value}
          getGroupName={(o) => o.label}
          getGroupCount={() => 0}
          isGroupExpandable={() => false}
          isGroupSelected={(o) => o.value === kind}
          onExpandGroup={noExpandKind}
          getMemberId={(o) => o.value}
          getMemberName={(o) => o.label}
          onSearch={() => {}}
          onPickGroup={pickKind}
          onPickMember={() => {}}
        />
        {#if kindErr}
          <span class="field-error">{kindErr}</span>
        {/if}
      </div>

      <div class="form-field">
        <label class="form-label" for="pf-parent">Родительское место</label>
        <PlacePicker
          id="pf-parent"
          value={parentId}
          onChange={(id) => (parentId = id)}
          disabled={saving}
        />
      </div>

      {#if showLevel}
        <div class="form-field" class:has-error={levelErr !== null}>
          <label class="form-label" for="pf-level">Уровень</label>
          <Input
            id="pf-level"
            type="number"
            value={level}
            invalid={levelErr !== null}
            disabled={saving}
            oninput={(v) => {
              level = v;
              levelErr = null;
            }}
          />
          <span class="field-hint">Подвал — отрицательное значение; 0 допустим</span>
          {#if levelErr}
            <span class="field-error">{levelErr}</span>
          {/if}
        </div>
      {/if}

      <div class="form-field">
        <Checkbox
          id="pf-storage"
          checked={isStorage}
          disabled={saving}
          onchange={(c) => (isStorage = c)}
        >
          Складское место
        </Checkbox>
        <span class="field-hint"
          >Место используется как склад — будет подставляться в возвратах и в фильтре «на складе»</span
        >
      </div>

      <div class="form-field">
        <label class="form-label" for="pf-order">Порядок</label>
        <Input
          id="pf-order"
          type="number"
          value={sortOrder}
          disabled={saving}
          oninput={(v) => (sortOrder = v)}
        />
        <span class="field-hint">Пусто — автоматический порядок</span>
      </div>
    {/if}

    {#if serverErr}
      <div class="server-error">{serverErr}</div>
    {/if}
  </div>

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose} disabled={saving}>Отмена</Button>
    <Button variant="primary" loading={saving} onclick={handleSubmit}>
      {#if saving}Сохранение…{:else}{submitLabel}{/if}
    </Button>
  {/snippet}
</Modal>

<style lang="scss">
  .place-form {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
    padding: var(--tr-space-md) 0;
  }

  .form-field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  .form-label {
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-secondary);
  }

  .field-hint {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
  }

  .field-error {
    font-size: var(--tr-font-size-label);
    color: var(--tr-danger);
  }

  .server-error {
    padding: var(--tr-space-xs) var(--tr-space-md);
    background: color-mix(in srgb, var(--tr-danger) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--tr-danger) 30%, transparent);
    border-radius: var(--tr-radius-xs);
    font-size: var(--tr-font-size-body);
    color: var(--tr-danger);
  }
</style>
