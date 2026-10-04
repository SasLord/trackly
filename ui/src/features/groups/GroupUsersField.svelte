<script lang="ts">
  // Phase 41 Plan 22 (GRP-08, D-11): поле типа `users` — список чипсов уже
  // добавленных людей и плоский Dropdown добавления. Основной пользователь —
  // ровно один либо ни одного; сервер всё равно перепроверяет («не более одного
  // основного»), здесь только UX-слой.
  //
  // Источник кандидатов — ТОЛЬКО groups.userOptions (таблица `users`: локальные
  // и ранее входившие AD-пользователи; id/full_name/login). Поиска по каталогу
  // AD нет, и обещать его нельзя.
  import { onDestroy } from 'svelte';
  import Dropdown from '$lib/components/Dropdown.svelte';
  import { groups } from '$lib/api/groups';
  import type { UserOptionDto } from '../../bindings';

  export interface SelectedUser {
    user_id: number;
    full_name: string;
    is_primary: boolean;
  }

  interface Props {
    /** Id контрола для связи с внешним <label for>. */
    id?: string;
    label?: string;
    selected: SelectedUser[];
    disabled?: boolean;
    errorText?: string | null;
    onChange: (_next: SelectedUser[]) => void;
  }

  const {
    id,
    label = 'Пользователи',
    selected,
    disabled = false,
    errorText = null,
    onChange,
  }: Props = $props();

  let options = $state<UserOptionDto[]>([]);
  let loading = $state(false);
  // Счётчик ответов: при быстром наборе поздний ответ старого запроса не должен
  // перетирать свежий.
  let searchSeq = 0;
  let destroyed = false;
  onDestroy(() => {
    destroyed = true;
  });

  const available = $derived(options.filter((o) => !selected.some((s) => s.user_id === o.id)));

  async function search(query: string): Promise<void> {
    const seq = ++searchSeq;
    loading = true;
    try {
      const list = await groups.userOptions(query.trim());
      if (seq === searchSeq && !destroyed) options = list;
    } catch {
      if (seq === searchSeq && !destroyed) options = [];
    } finally {
      if (seq === searchSeq && !destroyed) loading = false;
    }
  }

  function togglePrimary(userId: number): void {
    const target = selected.find((s) => s.user_id === userId);
    if (!target) return;
    const makePrimary = !target.is_primary;
    onChange(
      selected.map((s) => ({ ...s, is_primary: s.user_id === userId ? makePrimary : false })),
    );
  }

  function remove(userId: number): void {
    onChange(selected.filter((s) => s.user_id !== userId));
  }

  function add(option: UserOptionDto): void {
    if (selected.some((s) => s.user_id === option.id)) return;
    onChange([...selected, { user_id: option.id, full_name: option.full_name, is_primary: false }]);
  }

  // Плоский список без drill-in.
  function noMembers(): UserOptionDto[] {
    return [];
  }
</script>

<div class="users-field">
  {#if selected.length === 0}
    <p class="empty">Пользователи не указаны</p>
  {:else}
    <ul class="chips" aria-label={label}>
      {#each selected as u (u.user_id)}
        <li class="chip">
          <button
            type="button"
            class="chip-btn star"
            class:on={u.is_primary}
            aria-pressed={u.is_primary}
            aria-label={u.is_primary ? 'Основной пользователь' : 'Сделать основным пользователем'}
            title={u.is_primary ? 'Основной пользователь' : 'Сделать основным пользователем'}
            {disabled}
            onclick={() => togglePrimary(u.user_id)}
          >
            <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
              <path
                d="M12 3.5l2.6 5.3 5.9.9-4.25 4.15 1 5.85L12 16.9l-5.25 2.8 1-5.85L3.5 9.7l5.9-.9L12 3.5z"
                fill={u.is_primary ? 'currentColor' : 'none'}
                stroke="currentColor"
                stroke-width="1.6"
                stroke-linejoin="round"
              />
            </svg>
          </button>
          <span class="chip-name">{u.full_name}</span>
          <button
            type="button"
            class="chip-btn remove"
            aria-label={`Убрать ${u.full_name}`}
            {disabled}
            onclick={() => remove(u.user_id)}
          >
            <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true">
              <path
                d="M6 6l12 12M18 6L6 18"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
              />
            </svg>
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  <div class="add">
    <Dropdown
      {id}
      variant="select"
      flat={true}
      value=""
      placeholder="Добавить пользователя"
      searchPlaceholder="Поиск по ФИО или логину"
      invalid={!!errorText}
      {disabled}
      {loading}
      groups={available}
      getGroupId={(o: UserOptionDto) => o.id}
      getGroupName={(o: UserOptionDto) => o.full_name}
      getGroupMeta={(o: UserOptionDto) => o.login}
      getGroupCount={() => 0}
      isGroupExpandable={() => false}
      onExpandGroup={noMembers}
      getMemberId={(m: UserOptionDto) => m.id}
      getMemberName={(m: UserOptionDto) => m.full_name}
      onSearch={(q) => void search(q)}
      onPickGroup={add}
      onPickMember={() => {}}
    />
  </div>

  {#if errorText}
    <span class="field-error" role="alert">{errorText}</span>
  {/if}
</div>

<style lang="scss">
  .users-field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-xs);
  }

  .empty {
    margin: 0;
    color: var(--tr-text-tertiary);
    font-size: var(--tr-font-size-body);
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--tr-space-xs);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .chip {
    display: inline-flex;
    align-items: center;
    height: 28px;
    padding: 0 var(--tr-space-xs);
    background: var(--tr-surface-sunken);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-full);
    max-width: 100%;
  }

  .chip-name {
    color: var(--tr-text-body);
    font-size: var(--tr-font-size-body);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chip-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    padding: 0;
    border: 0;
    background: transparent;
    border-radius: var(--tr-radius-full);
    cursor: pointer;
    flex: 0 0 auto;

    &:disabled {
      cursor: default;
      opacity: 0.5;
    }
  }

  .star {
    color: var(--tr-text-tertiary);

    &.on {
      color: var(--tr-accent);
    }
  }

  .remove {
    color: var(--tr-text-secondary);
  }

  .field-error {
    font-size: var(--tr-font-size-label);
    color: var(--tr-danger-text);
  }
</style>
