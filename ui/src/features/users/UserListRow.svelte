<script lang="ts">
  // Plan 28-09 (D-03): rebuilt on shared TableRow primitive per ActListRow.svelte
  // precedent — bespoke <tr>/.badge replaced with <TableRow>/<Badge>. Inline
  // delete-confirmation ("Удалить?"/"Да"/"Нет") kept verbatim (UI-SPEC §7.4
  // forbids replacing it with a modal). The 4 small text buttons (Изменить/
  // Удалить/Да/Нет) keep the bespoke .btn-action class — Button primitive
  // targets larger CTAs, not inline table-row actions (Claude's Discretion).
  import Badge from '$lib/components/Badge.svelte';
  import TableRow from '$lib/components/TableRow.svelte';
  import type { UserDto } from '../../bindings';

  const ROLE_LABELS: Record<string, string> = {
    admin: 'Администратор',
    manager: 'Специалист',
    employee: 'Сотрудник',
  };

  interface Props {
    user: UserDto;
    onEdit: (user: UserDto) => void;
    onDelete: (id: number, version: number) => void;
  }

  const { user, onEdit, onDelete }: Props = $props();

  let confirmDelete = $state(false);

  function handleDeleteClick() {
    confirmDelete = true;
  }

  function handleConfirmDelete() {
    confirmDelete = false;
    onDelete(user.id, user.version);
  }

  function handleCancelDelete() {
    confirmDelete = false;
  }
</script>

<TableRow class="user-row">
  <td class="cell">{user.login}</td>
  <td class="cell">{user.full_name}</td>
  <td class="cell">{ROLE_LABELS[user.role] ?? user.role}</td>
  <td class="cell">{user.email ?? '—'}</td>
  <td class="cell">
    {#if user.is_active}
      <Badge variant="success">Активен</Badge>
    {:else}
      <Badge variant="default">Заблокирован</Badge>
    {/if}
  </td>
  <td class="cell cell--actions">
    {#if confirmDelete}
      <span class="confirm-text">Удалить?</span>
      <button class="btn-action btn-action--danger" onclick={handleConfirmDelete}>Да</button>
      <button class="btn-action" onclick={handleCancelDelete}>Нет</button>
    {:else}
      <button class="btn-action" onclick={() => onEdit(user)} title="Изменить">Изменить</button>
      <button class="btn-action btn-action--danger" onclick={handleDeleteClick} title="Удалить">
        Удалить
      </button>
    {/if}
  </td>
</TableRow>

<style lang="scss">
  // TableRow renders its own <tr> (a DIFFERENT Svelte scope-hash than this
  // file) — caller-supplied class needs :global(), ancestor part stays in
  // THIS file's scope: `.user-row :global(> td)`, never
  // `:global(.user-row > td)` (specificity trap, see TableRow.svelte contract).
  .cell {
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-primary);
  }

  .cell--actions {
    white-space: nowrap;
    display: flex;
    gap: var(--tr-space-2xs);
    align-items: center;
  }

  .btn-action {
    padding: 2px var(--tr-space-xs);
    border: 1px solid var(--tr-border);
    border-radius: var(--tr-radius-xs);
    font-size: var(--tr-font-size-label);
    background: transparent;
    color: var(--tr-text-secondary);
    cursor: pointer;

    &:hover {
      background: color-mix(in srgb, var(--tr-text-primary) 8%, transparent);
      color: var(--tr-text-primary);
    }

    &--danger {
      color: var(--tr-danger);
      border-color: color-mix(in srgb, var(--tr-danger) 40%, transparent);

      &:hover {
        background: color-mix(in srgb, var(--tr-danger) 10%, transparent);
        color: var(--tr-danger);
      }
    }
  }

  .confirm-text {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
  }
</style>
