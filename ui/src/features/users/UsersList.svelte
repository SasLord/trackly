<script lang="ts">
  // Plan 28-09 (D-03): rebuilt on shared Table primitive per ActsList.svelte
  // precedent — bespoke <table>/.th/.empty-state removed, Table now owns the
  // frame/empty-state. UsersList has no loading state (unlike ActsList) and
  // no pagination — simplest of the D-03 consumers.
  import Table from '$lib/components/Table.svelte';
  import type { UserDto } from '../../bindings';
  import UserListRow from './UserListRow.svelte';

  interface Props {
    items: UserDto[];
    onEdit: (user: UserDto) => void;
    onDelete: (id: number, version: number) => void;
  }

  const { items, onEdit, onDelete }: Props = $props();
</script>

{#snippet tableHead()}
  <th>Логин</th>
  <th>ФИО</th>
  <th>Роль</th>
  <th>Email</th>
  <th>Статус</th>
  <th class="th-actions">Действия</th>
{/snippet}

<Table columns={6} empty={items.length === 0} emptyTitle="Пользователи не найдены" head={tableHead}>
  {#each items as user (user.id)}
    <UserListRow {user} {onEdit} {onDelete} />
  {/each}
</Table>
