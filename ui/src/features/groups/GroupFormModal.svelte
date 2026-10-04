<script lang="ts">
  // Phase 41 Plan 19 (GRP-04, 41-UI-SPEC.md §7.4): создание и переименование
  // ГРУППЫ. Монтируется родителем через {#if} (каждый показ — свежий экземпляр).
  //
  // Создание: «Тип группы» (Dropdown, обязателен; при вызове из меню узла-типа
  // предзаполнен и disabled — `presetTypeId`) и «Название». Пустое название
  // уходит на сервер как `name: null` — имя по умолчанию «{тип} #{seq}»
  // формирует СЕРВЕР. Число N в подсказке — только подсказка (max(seq)+1 по
  // группам выбранного типа из groups.list()); клиент имя не формирует и номер
  // из названий не разбирает.
  //
  // Переименование: только «Название».
  import { onMount } from 'svelte';
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Input from '$lib/components/Input.svelte';
  import Dropdown from '$lib/components/Dropdown.svelte';
  import { groupTypes, groups } from '$lib/api/groups';
  import { pushToast } from '$lib/stores/toast.svelte';
  import type { AppError } from '$lib/api/errors';
  import type { GroupDto, GroupTypeDto } from '../../bindings';

  interface Props {
    mode: 'create' | 'rename';
    /** Группа для переименования; в режиме create не используется. */
    group?: GroupDto | null;
    /** Тип, предзаполненный и заблокированный (вызов из меню узла-типа). */
    presetTypeId?: number | null;
    onClose: () => void;
    onSaved: (_group: GroupDto) => void;
  }

  const { mode, group = null, presetTypeId = null, onClose, onSaved }: Props = $props();

  const isRename = $derived(mode === 'rename');

  // Начальные значения берутся один раз при монтировании (контракт {#if}: свежий экземпляр на показ).
  // svelte-ignore state_referenced_locally
  let name = $state(mode === 'rename' && group ? group.name : '');
  // svelte-ignore state_referenced_locally
  let typeId = $state<number | null>(presetTypeId);
  let typeOptions = $state<GroupTypeDto[]>([]);
  let allGroups = $state<GroupDto[]>([]);
  let loadingOptions = $state(false);
  let nameErr = $state<string | null>(null);
  let typeErr = $state<string | null>(null);
  let serverErr = $state<string | null>(null);
  let saving = $state(false);

  const selectedType = $derived(typeOptions.find((t) => t.id === typeId) ?? null);
  const typeLabel = $derived(selectedType?.name ?? '');
  const typeLocked = $derived(presetTypeId !== null);

  // Подсказка «{тип} #N»: N = max(seq)+1 среди групп выбранного типа. Только подсказка.
  const nextSeq = $derived(
    allGroups.filter((g) => g.type_id === typeId).reduce((m, g) => Math.max(m, g.seq), 0) + 1,
  );
  const defaultNameHint = $derived(
    selectedType ? `Если оставить пустым — «${selectedType.name} #${nextSeq}»` : '',
  );

  const modalTitle = $derived(isRename ? 'Переименовать группу' : 'Создать группу');
  const submitLabel = $derived(isRename ? 'Сохранить' : 'Создать');

  function noExpand(): GroupTypeDto[] {
    return [];
  }

  onMount(() => {
    if (isRename) return;
    loadingOptions = true;
    // Подсказка и список типов — удобство; сбой не блокирует форму (создать можно и без подсказки).
    Promise.all([groupTypes.list(false), groups.list()])
      .then(([types, list]) => {
        typeOptions = types;
        allGroups = list;
      })
      .catch(() => {
        serverErr = 'Не удалось загрузить типы групп. Закройте окно и повторите.';
      })
      .finally(() => {
        loadingOptions = false;
      });
  });

  function pickType(t: GroupTypeDto) {
    typeId = t.id;
    typeErr = null;
  }

  function validate(): boolean {
    nameErr = null;
    typeErr = null;
    serverErr = null;
    let ok = true;
    if (isRename && name.trim().length === 0) {
      nameErr = 'Укажите название.';
      ok = false;
    }
    if (!isRename && typeId === null) {
      typeErr = 'Выберите тип группы.';
      ok = false;
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
    serverErr = err?.message ?? 'Не удалось сохранить группу.';
    pushToast('error', serverErr);
  }

  async function handleSubmit() {
    if (!validate()) return;
    saving = true;
    try {
      let saved: GroupDto;
      if (isRename && group) {
        saved = await groups.update(group.id, group.version, name.trim());
        pushToast('success', 'Группа переименована');
      } else {
        const trimmed = name.trim();
        saved = await groups.create({
          type_id: typeId as number,
          // Пусто — имя по умолчанию формирует сервер.
          name: trimmed === '' ? null : trimmed,
          place_id: null,
        });
        pushToast('success', 'Группа создана');
      }
      onSaved(saved);
    } catch (e) {
      mapServerError(e);
    } finally {
      saving = false;
    }
  }
</script>

<Modal open={true} title={modalTitle} size="md" {onClose}>
  <div class="group-form">
    {#if !isRename}
      <div class="form-field" class:has-error={typeErr !== null}>
        <label class="form-label" for="gf-type">Тип группы</label>
        <Dropdown
          id="gf-type"
          variant="select"
          flat={true}
          value={typeLabel}
          placeholder="Выберите тип группы"
          searchable={false}
          invalid={typeErr !== null}
          disabled={saving || typeLocked}
          loading={loadingOptions}
          groups={typeOptions}
          getGroupId={(t) => t.id}
          getGroupName={(t) => t.name}
          getGroupCount={() => 0}
          isGroupExpandable={() => false}
          isGroupSelected={(t) => t.id === typeId}
          onExpandGroup={noExpand}
          getMemberId={(t) => t.id}
          getMemberName={(t) => t.name}
          onSearch={() => {}}
          onPickGroup={pickType}
          onPickMember={() => {}}
        />
        {#if typeErr}
          <span class="field-error">{typeErr}</span>
        {/if}
      </div>
    {/if}

    <div class="form-field" class:has-error={nameErr !== null}>
      <label class="form-label" for="gf-name">Название</label>
      <Input
        id="gf-name"
        value={name}
        invalid={nameErr !== null}
        disabled={saving}
        placeholder={selectedType ? `${selectedType.name} #${nextSeq}` : 'Название группы'}
        oninput={(v) => {
          name = v;
          nameErr = null;
        }}
      />
      {#if !isRename && defaultNameHint}
        <span class="field-hint">{defaultNameHint}</span>
      {/if}
      {#if nameErr}
        <span class="field-error">{nameErr}</span>
      {/if}
    </div>

    {#if serverErr}
      <div class="server-error">{serverErr}</div>
    {/if}
  </div>

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose} disabled={saving}>Отмена</Button>
    <Button variant="primary" loading={saving} onclick={handleSubmit}>{submitLabel}</Button>
  {/snippet}
</Modal>

<style lang="scss">
  .group-form {
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
