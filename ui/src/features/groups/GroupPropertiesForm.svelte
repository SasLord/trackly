<script lang="ts">
  // Phase 41 Plan 22 (GRP-03, GRP-08): вкладка «Свойства» карточки группы.
  //
  // Сохранение ЯВНОЕ («Сохранить» / «Отмена»), без записи на каждое нажатие:
  // значения ip/mac проверяет и приводит к каноническому виду сервер, поэтому
  // запрос и ошибка на каждую клавишу недопустимы. Сервер — единственный
  // источник истины по формату: после сохранения форма перезаполняется из
  // ВОЗВРАЩЁННОЙ карточки (видно приведённое значение), а клиентской формулы
  // ip/mac здесь нет. Ошибка по полю приходит как details.field = «values.<id>»
  // и показывается под соответствующим полем.
  import { untrack } from 'svelte';
  import FormField from '$lib/components/FormField.svelte';
  import Input from '$lib/components/Input.svelte';
  import Button from '$lib/components/Button.svelte';
  import { groups } from '$lib/api/groups';
  import { parseAppError } from '$lib/api/errors';
  import { pushToast } from '$lib/stores/toast.svelte';
  import GroupUsersField, { type SelectedUser } from './GroupUsersField.svelte';
  import GroupPrintersList, { type LinkedPrinterInfo } from './GroupPrintersList.svelte';
  import type { GroupCardDto, GroupPropertyValueDto, GroupValueInputDto } from '../../bindings';

  interface Props {
    card: GroupCardDto;
    canEdit: boolean;
    canOpenTypeSettings: boolean;
    onSaved: (_card: GroupCardDto) => void;
    onOpenType: () => void;
  }

  const { card, canEdit, canOpenTypeSettings, onSaved, onOpenType }: Props = $props();

  const SCALAR_TYPES = ['text', 'number', 'ip', 'mac'];

  const liveProps = $derived([...card.properties].sort((a, b) => a.sort_order - b.sort_order));
  const linkPropertyId = $derived(card.link_property_id);
  const hasLinkProperty = $derived(linkPropertyId !== null);
  const usbPrinters = $derived(card.printers.filter((p) => p.origin === 'usb'));

  // --- Локальное состояние формы ---
  let texts = $state<Record<number, string>>({});
  let userLists = $state<Record<number, SelectedUser[]>>({});
  let refLists = $state<Record<number, number[]>>({});
  // Map не проксируется рунами — меняем только заменой целиком.
  let linkedInfo = $state<Map<number, LinkedPrinterInfo>>(new Map());
  let fieldErrors = $state<Record<number, string>>({});
  let saving = $state(false);
  let loadedKey = '';

  function isScalar(p: GroupPropertyValueDto): boolean {
    return SCALAR_TYPES.includes(p.data_type);
  }

  function load(c: GroupCardDto): void {
    const t: Record<number, string> = {};
    const u: Record<number, SelectedUser[]> = {};
    const r: Record<number, number[]> = {};
    for (const p of c.properties) {
      if (SCALAR_TYPES.includes(p.data_type)) t[p.property_id] = p.text ?? '';
      else if (p.data_type === 'users') u[p.property_id] = p.users.map((x) => ({ ...x }));
      else if (p.data_type === 'device_refs') r[p.property_id] = [...p.ref_device_ids];
    }
    texts = t;
    userLists = u;
    refLists = r;
    linkedInfo = new Map();
    fieldErrors = {};
    loadedKey = `${c.group.id}:${c.group.version}`;
  }

  // Повторная инициализация: эффект ЧИТАЕТ только `card`, а состояние пишет
  // внутри untrack (иначе effect_update_depth_exceeded). Несохранённые правки не
  // затираются, пока версия той же группы не изменилась.
  $effect(() => {
    const c = card;
    untrack(() => {
      const key = `${c.group.id}:${c.group.version}`;
      if (key !== loadedKey || !dirty) load(c);
    });
  });

  // --- Изменённость формы относительно карточки ---
  function serialize(
    props: GroupPropertyValueDto[],
    textOf: (_p: GroupPropertyValueDto) => string,
    usersOf: (_p: GroupPropertyValueDto) => SelectedUser[],
    refsOf: (_p: GroupPropertyValueDto) => number[],
  ): string {
    return JSON.stringify(
      props.map((p) => {
        if (SCALAR_TYPES.includes(p.data_type)) return [p.property_id, textOf(p)];
        if (p.data_type === 'users')
          return [p.property_id, usersOf(p).map((x) => [x.user_id, x.is_primary])];
        return [p.property_id, refsOf(p)];
      }),
    );
  }

  const baseline = $derived(
    serialize(
      liveProps,
      (p) => p.text ?? '',
      (p) => p.users,
      (p) => p.ref_device_ids,
    ),
  );
  const current = $derived(
    serialize(
      liveProps,
      (p) => texts[p.property_id] ?? '',
      (p) => userLists[p.property_id] ?? [],
      (p) => refLists[p.property_id] ?? [],
    ),
  );
  const dirty = $derived(current !== baseline);

  function cancel(): void {
    load(card);
  }

  function setText(id: number, v: string): void {
    texts = { ...texts, [id]: v };
  }

  function setUsers(id: number, next: SelectedUser[]): void {
    userLists = { ...userLists, [id]: next };
  }

  function addLink(id: number, d: LinkedPrinterInfo): void {
    const cur = refLists[id] ?? [];
    if (cur.includes(d.device_id)) return;
    refLists = { ...refLists, [id]: [...cur, d.device_id] };
    const info = new Map(linkedInfo);
    info.set(d.device_id, d);
    linkedInfo = info;
  }

  function removeLink(id: number, deviceId: number): void {
    refLists = { ...refLists, [id]: (refLists[id] ?? []).filter((x) => x !== deviceId) };
  }

  // --- Сохранение ---
  function buildValues(): GroupValueInputDto[] {
    return liveProps.map((p): GroupValueInputDto => {
      if (isScalar(p))
        return { property_id: p.property_id, text: texts[p.property_id] ?? '', refs: [] };
      if (p.data_type === 'users') {
        return {
          property_id: p.property_id,
          text: null,
          refs: (userLists[p.property_id] ?? []).map((u) => ({
            ref_id: u.user_id,
            is_primary: u.is_primary,
          })),
        };
      }
      return {
        property_id: p.property_id,
        text: null,
        refs: (refLists[p.property_id] ?? []).map((id) => ({ ref_id: id, is_primary: false })),
      };
    });
  }

  function fieldIdOf(details: unknown): number | null {
    if (details === null || typeof details !== 'object' || Array.isArray(details)) return null;
    const field = (details as { field?: unknown }).field;
    if (typeof field !== 'string') return null;
    const m = /^values\.(\d+)$/.exec(field);
    return m ? Number(m[1]) : null;
  }

  function mapServerError(e: unknown): void {
    const err = parseAppError(e);
    if (err.code === 'VALIDATION') {
      const propertyId = fieldIdOf(err.details);
      if (propertyId !== null) {
        fieldErrors = { ...fieldErrors, [propertyId]: err.message };
        return;
      }
    }
    if (err.code === 'OPTIMISTIC_LOCK_MISMATCH') {
      pushToast(
        'error',
        'Группа была изменена другим пользователем. Обновите карточку и повторите правки.',
      );
      return;
    }
    pushToast('error', err.message || 'Не удалось сохранить изменения.');
  }

  async function save(): Promise<void> {
    if (saving || !dirty || !canEdit) return;
    saving = true;
    fieldErrors = {};
    try {
      const next = await groups.setValues({
        id: card.group.id,
        version: card.group.version,
        values: buildValues(),
      });
      pushToast('success', 'Изменения сохранены');
      load(next);
      onSaved(next);
    } catch (e) {
      mapServerError(e);
    } finally {
      saving = false;
    }
  }

  // Input не пробрасывает aria-required, поэтому отметку на контроле ставит
  // действие контейнера (общий компонент не трогаем).
  function markRequired(node: HTMLElement, ids: string[]) {
    const apply = (list: string[]) => {
      for (const id of list)
        node.querySelector(`[id="${id}"]`)?.setAttribute('aria-required', 'true');
    };
    apply(ids);
    return {
      update(next: string[]) {
        apply(next);
      },
    };
  }

  const requiredIds = $derived(
    liveProps.filter((p) => p.is_required && isScalar(p)).map((p) => `gpf-${p.property_id}`),
  );

  const isEmptyForm = $derived(liveProps.length === 0 && usbPrinters.length === 0);

  function labelOf(p: GroupPropertyValueDto): string {
    return p.is_required ? `${p.name} *` : p.name;
  }
</script>

<form
  class="props-form"
  onsubmit={(e) => {
    e.preventDefault();
    void save();
  }}
>
  <div class="fields" use:markRequired={requiredIds}>
    {#if isEmptyForm}
      <div class="no-props">
        <p class="muted">У этого типа нет свойств.</p>
        {#if canOpenTypeSettings}
          <Button variant="link" onclick={onOpenType}>Настроить свойства типа</Button>
        {/if}
      </div>
    {/if}

    {#each liveProps as p (p.property_id)}
      {@const fid = `gpf-${p.property_id}`}
      {#if p.data_type === 'users'}
        <FormField label={labelOf(p)} id={fid}>
          {#snippet children()}
            <GroupUsersField
              id={fid}
              label={p.name}
              selected={userLists[p.property_id] ?? []}
              disabled={!canEdit || saving}
              errorText={fieldErrors[p.property_id] ?? null}
              onChange={(next) => setUsers(p.property_id, next)}
            />
          {/snippet}
        </FormField>
      {:else if p.data_type === 'device_refs'}
        <FormField label={labelOf(p)} id={fid}>
          {#snippet children()}
            <GroupPrintersList
              id={fid}
              printers={p.property_id === linkPropertyId
                ? card.printers
                : card.printers.filter((x) => x.origin !== 'usb')}
              linkedIds={refLists[p.property_id] ?? []}
              {linkedInfo}
              canEdit={canEdit && !saving}
              canLink={true}
              errorText={fieldErrors[p.property_id] ?? null}
              onAddLink={(d) => addLink(p.property_id, d)}
              onRemoveLink={(deviceId) => removeLink(p.property_id, deviceId)}
            />
          {/snippet}
        </FormField>
      {:else}
        <FormField
          label={labelOf(p)}
          id={fid}
          error={fieldErrors[p.property_id] ?? null}
          hint={p.data_type === 'ip'
            ? 'IPv4 или IPv6'
            : p.data_type === 'mac'
              ? 'Разделитель — «:», «-» или без него'
              : undefined}
        >
          {#snippet children({ describedBy, invalid })}
            <Input
              id={fid}
              type={p.data_type === 'number' ? 'number' : 'text'}
              mono={p.data_type !== 'text'}
              placeholder={p.data_type === 'ip'
                ? '192.168.1.10'
                : p.data_type === 'mac'
                  ? '00:1b:44:11:3a:b7'
                  : undefined}
              value={texts[p.property_id] ?? ''}
              {invalid}
              aria-describedby={describedBy}
              disabled={!canEdit || saving}
              oninput={(v) => setText(p.property_id, v)}
            />
          {/snippet}
        </FormField>
      {/if}
    {/each}

    <!-- У типа нет свойства-ссылки, но USB-принтеры выведены из данных. -->
    {#if !hasLinkProperty && usbPrinters.length > 0}
      <div class="readonly-printers">
        <span class="readonly-label">Подключённые принтеры</span>
        <GroupPrintersList
          printers={card.printers}
          linkedIds={[]}
          linkedInfo={new Map()}
          canEdit={false}
          canLink={false}
          onAddLink={() => {}}
          onRemoveLink={() => {}}
        />
      </div>
    {/if}
  </div>

  {#if canEdit && !isEmptyForm}
    <div class="actions">
      <Button type="submit" variant="primary" disabled={!dirty || saving} loading={saving}>
        Сохранить
      </Button>
      <Button variant="secondary" disabled={!dirty || saving} onclick={cancel}>Отмена</Button>
    </div>
  {/if}
</form>

<style lang="scss">
  .props-form {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .fields {
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-md);
    padding: var(--tr-space-md);
  }

  .no-props {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--tr-space-xs);
  }

  .muted {
    margin: 0;
    color: var(--tr-text-tertiary);
  }

  .readonly-printers {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  .readonly-label {
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-secondary);
  }

  .actions {
    flex: 0 0 auto;
    display: flex;
    gap: var(--tr-space-xs);
    padding: var(--tr-space-sm) var(--tr-space-md);
    border-top: 1px solid var(--tr-border);
    background: var(--tr-surface);
  }
</style>
