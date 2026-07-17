<script lang="ts">
  import type { UserDto } from '../../bindings';
  import UserListRow from './UserListRow.svelte';

  interface Props {
    items: UserDto[];
    onEdit: (user: UserDto) => void;
    onDelete: (id: number, version: number) => void;
  }

  const { items, onEdit, onDelete }: Props = $props();
</script>

<div class="users-list-container">
  {#if items.length === 0}
    <div class="empty-state">Пользователи не найдены</div>
  {:else}
    <table class="users-table">
      <thead>
        <tr>
          <th class="th">Логин</th>
          <th class="th">ФИО</th>
          <th class="th">Роль</th>
          <th class="th">Email</th>
          <th class="th">Статус</th>
          <th class="th">Действия</th>
        </tr>
      </thead>
      <tbody>
        {#each items as user (user.id)}
          <UserListRow {user} {onEdit} {onDelete} />
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style lang="scss">
  .users-list-container {
    overflow-x: auto;
  }

  .empty-state {
    padding: var(--space-xl);
    text-align: center;
    color: var(--tr-text-tertiary);
    font-size: var(--font-size-body);
  }

  .users-table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--font-size-body);
  }

  .th {
    padding: var(--space-sm) var(--space-md);
    text-align: left;
    font-size: var(--font-size-label);
    font-weight: var(--font-weight-medium);
    color: var(--tr-text-tertiary);
    border-bottom: 1px solid var(--tr-border);
    white-space: nowrap;
  }
</style>
