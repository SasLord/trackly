<script lang="ts">
  // Phase 41 Plan 23 (GRP-04, GRP-06, UI-SPEC 10.1/10.2/10.6): правая панель
  // выбранной ГРУППЫ — шапка и три вкладки «Состав» / «Свойства» / «История».
  // «Подключённые принтеры» — рендер свойства внутри «Свойств», не вкладка.
  //
  // Выбранная вкладка поднята в страницу (activeTab + onTabChange): она
  // переживает ремаунт панели при смене группы и уход из раздела.
  //
  // Загрузка карточки — первая показывает Spinner, повторные (после правки
  // свойств / изменения состава / токена страницы) тихие: иначе вкладка
  // «Состав» размонтировалась бы посреди работы пользователя.
  //
  // Все $effect читают только входные пропы, а запись $state, которое эффект сам
  // читает, идёт в untrack (компиляционные гейты слепы к effect_update_depth_exceeded).
  import { untrack } from 'svelte';
  import { push } from 'svelte-spa-router';
  import Tabs from '$lib/components/Tabs.svelte';
  import Badge from '$lib/components/Badge.svelte';
  import Button from '$lib/components/Button.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import MovementTimeline from '$lib/components/MovementTimeline.svelte';
  import GroupContentsTable from './GroupContentsTable.svelte';
  import GroupPropertiesForm from './GroupPropertiesForm.svelte';
  import { groups } from '$lib/api/groups';
  import { apiCall } from '$lib/api/client';
  import { authStore } from '$lib/stores/auth.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import type { AppError } from '$lib/api/errors';
  import type { GroupCardDto, MovementEntryDto } from '../../bindings';

  interface Props {
    groupId: number;
    /** admin | manager: правка состава и свойств, «Перенести…». */
    canEdit: boolean;
    /** Смена токена — тихая перезагрузка карточки, состава и истории. */
    refreshToken?: number;
    activeTab: string;
    onTabChange: (_key: string) => void;
    /** Переход-фокус на группу в дереве (корень вложенной группы, «Открыть группу»). */
    onNavigateToGroup: (_groupId: number) => void;
    /** «Настроить свойства типа»: фокус на типе в дереве. */
    onOpenType: (_typeId: number) => void;
    /** Вторая точка входа D-18: тот же обработчик страницы, что у меню узла. */
    onMoveRequest: (_groupId: number) => void;
    onRenameRequest: (_groupId: number) => void;
    onOpenDevice?: (_deviceId: number) => void;
    /** Карточка/состав/свойства изменились — страница обновляет дерево. */
    onChanged?: () => void;
  }

  const {
    groupId,
    canEdit,
    refreshToken = 0,
    activeTab,
    onTabChange,
    onNavigateToGroup,
    onOpenType,
    onMoveRequest,
    onRenameRequest,
    onOpenDevice,
    onChanged,
  }: Props = $props();

  const BEHAVIOR_LABEL: Record<string, string> = {
    container: 'Контейнер',
    substitute: 'Замещение',
    teardown: 'Разбор',
  };

  const isAdmin = $derived(authStore.user?.role === 'admin');

  // --- Карточка ----------------------------------------------------------------
  let card = $state<GroupCardDto | null>(null);
  let cardLoading = $state(true);
  // Не $state: счётчик устаревших ответов; шаблон его не читает.
  let cardSeq = 0;

  async function loadCard(): Promise<void> {
    const seq = ++cardSeq;
    try {
      const next = await groups.card(groupId);
      if (seq !== cardSeq) return;
      card = next;
    } catch (e) {
      if (seq !== cardSeq) return;
      pushToast(
        'error',
        (e as Partial<AppError> | undefined)?.message ?? 'Не удалось загрузить группу.',
      );
    } finally {
      if (seq === cardSeq) cardLoading = false;
    }
  }

  $effect(() => {
    void groupId;
    void refreshToken;
    untrack(() => {
      // Другая группа — прежняя карточка не показывается (спиннер вместо неё).
      if (card !== null && card.group.id !== groupId) {
        card = null;
        cardLoading = true;
      }
      void loadCard();
    });
  });

  const group = $derived(card?.group ?? null);
  const behaviorLabel = $derived(
    group ? (BEHAVIOR_LABEL[group.type_behavior] ?? group.type_behavior) : '',
  );
  const isRoot = $derived(group !== null && group.parent_group_id === null);

  const tabs = $derived([
    { key: 'contents', label: 'Состав', count: group?.device_count },
    { key: 'properties', label: 'Свойства' },
    { key: 'history', label: 'История' },
  ]);

  // --- История (D-29) ----------------------------------------------------------
  let history = $state<MovementEntryDto[]>([]);
  let historyLoading = $state(false);
  let historyError = $state(false);

  $effect(() => {
    if (activeTab !== 'history') return;
    const id = groupId;
    void refreshToken;
    let cancelled = false;
    untrack(() => {
      historyLoading = true;
      historyError = false;
    });
    apiCall<MovementEntryDto[]>('place_movements_get_timeline', {
      entityType: 'group',
      entityId: id,
    })
      .then((rows) => {
        if (!cancelled) history = rows;
      })
      .catch(() => {
        if (!cancelled) {
          history = [];
          historyError = true;
        }
      })
      .finally(() => {
        if (!cancelled) historyLoading = false;
      });
    return () => {
      cancelled = true;
    };
  });

  function handleSaved(next: GroupCardDto): void {
    card = next;
    onChanged?.();
  }

  function handleContentsChanged(): void {
    void loadCard();
    onChanged?.();
  }
</script>

<div class="group-panel">
  {#if cardLoading && card === null}
    <div class="panel-loading"><Spinner /></div>
  {:else if card === null || group === null}
    <div class="panel-loading muted">Группа недоступна.</div>
  {:else}
    <header class="panel-header">
      <div class="caption">{group.type_name} · {behaviorLabel}</div>
      <div class="title-row">
        <h3 class="group-name">{group.name}</h3>
        {#if group.place_id === null}<Badge variant="default">Без места</Badge>{/if}
        {#if canEdit && isRoot}
          <div class="title-actions">
            <Button variant="secondary" onclick={() => onMoveRequest(group.id)}>Перенести…</Button>
          </div>
        {/if}
      </div>
      <div class="place-row" class:unset={group.place_path === null}>
        {#if group.parent_group_id !== null}
          <span class="place-label">Место:</span>
          <span class="place-value" title={group.place_path ?? undefined}>
            {group.place_path ?? 'не задано'}
          </span>
          <span class="place-owner">
            ({group.place_path === null ? 'корневая группа' : 'задано группой'}
            «<Button variant="link" onclick={() => onNavigateToGroup(group.root_group_id)}>
              {group.root_group_name}
            </Button>»)
          </span>
        {:else if group.place_path !== null}
          <span class="place-label">Место:</span>
          <span class="place-value" title={group.place_path}>{group.place_path}</span>
        {:else}
          <span class="place-label">Место:</span>
          <span class="place-value">не задано</span>
        {/if}
      </div>
    </header>

    <div class="tabs-row">
      <Tabs
        variant="underline"
        {tabs}
        active={activeTab}
        onchange={onTabChange}
        ariaLabel="Разделы группы"
      />
    </div>

    <div class="tab-body">
      {#if activeTab === 'properties'}
        <GroupPropertiesForm
          {card}
          {canEdit}
          canOpenTypeSettings={isAdmin}
          onSaved={handleSaved}
          onOpenType={() => onOpenType(group.type_id)}
        />
      {:else if activeTab === 'history'}
        <div class="history-region">
          {#if historyLoading}
            <div class="panel-loading"><Spinner /></div>
          {:else if !historyError && history.length === 0}
            <div class="history-empty">
              <p class="history-empty-title">Перемещений нет</p>
              <p class="history-empty-body">Группу ещё не переносили.</p>
            </div>
          {:else}
            <MovementTimeline
              entries={history}
              loading={false}
              loadError={historyError}
              showInitialPlacementNote={false}
              onNavigateToPlace={(placeId) => {
                void push(`#/places?id=${placeId}`);
              }}
            />
          {/if}
        </div>
      {:else}
        <GroupContentsTable
          {group}
          {canEdit}
          {refreshToken}
          onChanged={handleContentsChanged}
          onOpenGroup={onNavigateToGroup}
          onRenameGroup={onRenameRequest}
          {onOpenDevice}
        />
      {/if}
    </div>
  {/if}
</div>

<style lang="scss">
  .group-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .panel-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--tr-space-xl);
    color: var(--tr-text-secondary);
  }

  .muted {
    color: var(--tr-text-tertiary);
  }

  .panel-header {
    flex: none;
    padding: var(--tr-space-sm) var(--tr-space-md);
    border-bottom: 1px solid var(--tr-border);
  }

  .caption {
    font-size: var(--tr-font-size-caption);
    color: var(--tr-text-secondary);
    margin-bottom: var(--tr-space-2xs);
  }

  .title-row {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
    flex-wrap: wrap;
  }

  .group-name {
    margin: 0;
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .title-actions {
    margin-left: auto;
  }

  .place-row {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: var(--tr-space-2xs);
    margin-top: var(--tr-space-2xs);
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);

    &.unset {
      color: var(--tr-text-tertiary);
    }
  }

  .place-label {
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-secondary);
  }

  .place-owner {
    color: var(--tr-text-secondary);
  }

  .tabs-row {
    flex: none;
    padding: var(--tr-space-sm) var(--tr-space-md) 0;
    border-bottom: 1px solid var(--tr-border);
  }

  // Тело вкладки — единственный fill/scroll-регион панели.
  .tab-body {
    flex: 1 1 auto;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .history-region {
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
    padding: var(--tr-space-md);
  }

  .history-empty {
    padding: var(--tr-space-md) 0;
  }

  .history-empty-title {
    margin: 0 0 var(--tr-space-2xs) 0;
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-secondary);
  }

  .history-empty-body {
    margin: 0;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
  }
</style>
