<script lang="ts">
  // Phase 41 Plan 20 (D-09, D-13, UI-SPEC 9.1-9.2): правая панель выбранного
  // типа группы — описание типа (только чтение: код и поведение неизменяемы) и
  // таблица свойств. Для не-admin (canEdit=false) панель только для чтения.
  // Единственный скролл-регион — сам DetailPanel (оболочка .content скроллит
  // только внутренние регионы страниц).
  import { untrack } from 'svelte';
  import DetailPanel from '$lib/components/DetailPanel.svelte';
  import DetailSection from '$lib/components/DetailSection.svelte';
  import DetailField from '$lib/components/DetailField.svelte';
  import Checkbox from '$lib/components/Checkbox.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import GroupTypePropertiesTable from './GroupTypePropertiesTable.svelte';
  import { groupTypes } from '$lib/api/groups';
  import { pushToast } from '$lib/stores/toast.svelte';
  import type { AppError } from '$lib/api/errors';
  import type { GroupTypeDto } from '../../bindings';

  interface Props {
    typeId: number;
    /** admin: правка свойств, «Показать скрытые». Иначе — только чтение. */
    canEdit: boolean;
    /** Смена токена — тихая перезагрузка (без сброса таблицы на спиннер). */
    refreshToken?: number;
    /** Фокус на группе в дереве (из попапа нарушителей обязательности). */
    onNavigateToGroup: (_groupId: number) => void;
    /** Данные типа изменились (свойства) — страница может обновить дерево. */
    onChanged?: () => void;
  }

  const { typeId, canEdit, refreshToken = 0, onNavigateToGroup, onChanged }: Props = $props();

  const BEHAVIOR_LABEL: Record<string, string> = {
    container: 'Контейнер',
    substitute: 'Замещение',
    teardown: 'Разбор',
  };

  function behaviorLabel(code: string): string {
    return BEHAVIOR_LABEL[code] ?? code;
  }

  let showHidden = $state(false);
  let types = $state<GroupTypeDto[]>([]);
  let loaded = $state(false);

  // Служебное состояние загрузки: шаблон его не читает, эффект не пишет читаемого.
  let loadSeq = 0;
  let lastLoadedTypeId: number | null = null;
  let hadType = false;

  const type = $derived(types.find((t) => t.id === typeId) ?? null);

  async function load(includeArchived: boolean): Promise<void> {
    const seq = ++loadSeq;
    try {
      const list = await groupTypes.list(includeArchived);
      if (seq !== loadSeq) return; // пришёл устаревший ответ
      types = list;
      loaded = true;
      const found = list.some((t) => t.id === typeId);
      // «Тип не найден» после удаления: сообщаем странице один раз, не в цикле.
      if (!found && hadType) onChanged?.();
      hadType = found;
    } catch (e) {
      if (seq !== loadSeq) return;
      loaded = true;
      pushToast(
        'error',
        (e as Partial<AppError> | undefined)?.message ?? 'Не удалось загрузить тип.',
      );
    }
  }

  // Перезагрузка по смене типа, токена и «Показать скрытые». Зависимости читаются
  // здесь, сама загрузка — в untrack: иначе запись `types`/`loaded` внутри load
  // замкнула бы эффект на самого себя (effect_update_depth_exceeded).
  $effect(() => {
    const id = typeId;
    void refreshToken;
    const includeArchived = canEdit && showHidden;
    untrack(() => {
      if (lastLoadedTypeId !== id) {
        // Другой тип: показываем спиннер, а не данные прежнего типа.
        lastLoadedTypeId = id;
        loaded = false;
        hadType = false;
      }
      void load(includeArchived);
    });
  });

  async function reloadProperties(): Promise<void> {
    await load(canEdit && showHidden);
    onChanged?.();
  }

  const caption = $derived(
    type
      ? `тип группы · ${behaviorLabel(type.behavior)}${type.is_builtin ? ' · встроенный' : ''}`
      : '',
  );
</script>

{#if !loaded}
  <div class="panel-loading" role="status" aria-label="Загрузка типа">
    <Spinner size="lg" />
  </div>
{:else if type === null}
  <DetailPanel empty emptyTitle="Тип не найден" emptyBody="Возможно, он был удалён." />
{:else}
  <DetailPanel title={type.name}>
    <p class="caption">{caption}</p>

    <DetailSection heading="Тип группы">
      <div class="field-grid">
        <div class="code-field">
          <span class="code-label">Код</span>
          <span class="tr-mono code-value">{type.code}</span>
        </div>
        <DetailField label="Поведение" value={behaviorLabel(type.behavior)} />
        <DetailField label="Встроенный" value={type.is_builtin ? 'Да' : 'Нет'} />
        <DetailField
          label="Быстрое действие"
          value={type.quick_action_enabled && type.quick_action_label
            ? type.quick_action_label
            : null}
        />
      </div>
      <p class="hint">Код и поведение типа изменить нельзя.</p>
    </DetailSection>

    <DetailSection>
      <div class="section-head">
        <h3 class="section-heading">Свойства</h3>
        {#if canEdit}
          <Checkbox bind:checked={showHidden}>Показать скрытые</Checkbox>
        {/if}
      </div>
      <GroupTypePropertiesTable
        typeId={type.id}
        properties={type.properties}
        {canEdit}
        onReload={reloadProperties}
        onOpenGroup={onNavigateToGroup}
      />
    </DetailSection>
  </DetailPanel>
{/if}

<style lang="scss">
  .panel-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    min-height: 0;
    color: var(--tr-text-secondary);
  }

  .caption {
    margin: 0 0 var(--tr-space-lg);
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
  }

  .field-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: var(--tr-space-md);
  }

  // Код — моноширинный, поэтому не DetailField (тот не умеет .tr-mono); вид тот же.
  .code-field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-3xs);
  }
  .code-label {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
  }
  .code-value {
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
  }

  .hint {
    margin: var(--tr-space-sm) 0 0;
    font: var(--tr-text-label);
    color: var(--tr-text-secondary);
  }

  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--tr-space-md);
    margin-bottom: var(--tr-space-md);
  }

  .section-heading {
    margin: 0;
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }
</style>
