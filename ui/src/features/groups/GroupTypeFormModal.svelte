<script lang="ts">
  // Phase 41 Plan 19 (GRP-01, 41-UI-SPEC.md §9.4): создание и переименование
  // ТИПА группы. Монтируется родителем через {#if} (каждый показ — свежий
  // экземпляр формы), контракт как у PlaceFormModal.
  //
  // «Поведение» выбирается только при создании. При переименовании поля нет
  // ВОВСЕ — не disabled-поле: неизменяемое поле в форме правки — ложное
  // обещание (§9.4). Поэтому и запрос переименования несёт только `name`:
  // `code`/`behavior` не отправляются (T-41-19-02); сервер всё равно отклонил бы
  // отличающееся значение (DB-триггер + сервисная проверка, план 07).
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Input from '$lib/components/Input.svelte';
  import Dropdown from '$lib/components/Dropdown.svelte';
  import { groupTypes } from '$lib/api/groups';
  import { pushToast } from '$lib/stores/toast.svelte';
  import type { AppError } from '$lib/api/errors';
  import type { GroupTypeDto, GroupTypeUpdateDto } from '../../bindings';

  interface Props {
    mode: 'create' | 'rename';
    /** Тип для переименования; в режиме create не используется. */
    type?: GroupTypeDto | null;
    onClose: () => void;
    onSaved: (_type: GroupTypeDto) => void;
  }

  const { mode, type = null, onClose, onSaved }: Props = $props();

  interface BehaviorOption {
    value: string;
    label: string;
  }

  // Подписи поведения — §17.1 (Контейнер · Замещение · Разбор).
  const BEHAVIOR_OPTIONS: BehaviorOption[] = [
    { value: 'container', label: 'Контейнер' },
    { value: 'substitute', label: 'Замещение' },
    { value: 'teardown', label: 'Разбор' },
  ];

  // Плоский список без drill-in: Dropdown всё равно требует типизированный колбэк.
  function noExpand(): BehaviorOption[] {
    return [];
  }

  const isRename = $derived(mode === 'rename');

  // Начальное значение берётся один раз при монтировании (контракт {#if}: свежий экземпляр на показ).
  // svelte-ignore state_referenced_locally
  let name = $state(mode === 'rename' && type ? type.name : '');
  let behavior = $state('');
  let nameErr = $state<string | null>(null);
  let behaviorErr = $state<string | null>(null);
  let saving = $state(false);

  const behaviorLabel = $derived(BEHAVIOR_OPTIONS.find((o) => o.value === behavior)?.label ?? '');
  const modalTitle = $derived(isRename ? 'Переименовать тип' : 'Создать тип группы');
  const submitLabel = $derived(isRename ? 'Сохранить' : 'Создать');

  function pickBehavior(o: BehaviorOption) {
    behavior = o.value;
    behaviorErr = null;
  }

  // Только «обязательное поле» — UX-подсказка до запроса; формат и допустимость
  // значений решает сервер, его сообщение показывается как есть.
  function validate(): boolean {
    nameErr = null;
    behaviorErr = null;
    let ok = true;
    if (name.trim().length === 0) {
      nameErr = 'Укажите название.';
      ok = false;
    }
    if (!isRename && behavior === '') {
      behaviorErr = 'Выберите поведение типа.';
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
    if (err?.code === 'VALIDATION' && field === 'behavior') {
      behaviorErr = err.message ?? 'Ошибка валидации';
      return;
    }
    pushToast('error', err?.message ?? 'Не удалось сохранить тип.');
  }

  async function handleSubmit() {
    if (!validate()) return;
    saving = true;
    try {
      let saved: GroupTypeDto;
      if (isRename && type) {
        // Ключи code/behavior намеренно ОТСУТСТВУЮТ: сервер читает отсутствующее
        // поле как «не менять». Приведение типа нужно только потому, что
        // сгенерированный TS-тип требует эти ключи (`string | null`).
        const dto = { name: name.trim() } as GroupTypeUpdateDto;
        saved = await groupTypes.update(type.id, type.version, dto);
        pushToast('success', 'Тип переименован');
      } else {
        saved = await groupTypes.create({ name: name.trim(), behavior });
        pushToast('success', 'Тип создан');
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
  <div class="type-form">
    <div class="form-field" class:has-error={nameErr !== null}>
      <label class="form-label" for="gtf-name">Название</label>
      <Input
        id="gtf-name"
        value={name}
        invalid={nameErr !== null}
        disabled={saving}
        oninput={(v) => {
          name = v;
          nameErr = null;
        }}
      />
      {#if nameErr}
        <span class="field-error">{nameErr}</span>
      {/if}
    </div>

    {#if !isRename}
      <div class="form-field" class:has-error={behaviorErr !== null}>
        <label class="form-label" for="gtf-behavior">Поведение</label>
        <Dropdown
          id="gtf-behavior"
          variant="select"
          flat={true}
          value={behaviorLabel}
          placeholder="Выберите поведение"
          searchable={false}
          invalid={behaviorErr !== null}
          disabled={saving}
          loading={false}
          groups={BEHAVIOR_OPTIONS}
          getGroupId={(o) => o.value}
          getGroupName={(o) => o.label}
          getGroupCount={() => 0}
          isGroupExpandable={() => false}
          isGroupSelected={(o) => o.value === behavior}
          onExpandGroup={noExpand}
          getMemberId={(o) => o.value}
          getMemberName={(o) => o.label}
          onSearch={() => {}}
          onPickGroup={pickBehavior}
          onPickMember={() => {}}
        />
        <span class="field-hint">Поведение и код типа после создания не меняются.</span>
        {#if behaviorErr}
          <span class="field-error">{behaviorErr}</span>
        {/if}
      </div>
    {/if}
  </div>

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose} disabled={saving}>Отмена</Button>
    <Button variant="primary" loading={saving} onclick={handleSubmit}>{submitLabel}</Button>
  {/snippet}
</Modal>

<style lang="scss">
  .type-form {
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
</style>
