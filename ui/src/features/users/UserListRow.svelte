<script lang="ts">
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

<tr class="user-row">
  <td class="cell">{user.login}</td>
  <td class="cell">{user.full_name}</td>
  <td class="cell">{ROLE_LABELS[user.role] ?? user.role}</td>
  <td class="cell">{user.email ?? '—'}</td>
  <td class="cell">
    {#if user.is_active}
      <span class="badge badge--active">Активен</span>
    {:else}
      <span class="badge badge--blocked">Заблокирован</span>
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
</tr>

<style lang="scss">
  .user-row {
    &:hover {
      background: color-mix(in srgb, var(--color-text-primary) 3%, transparent);
    }
  }

  .cell {
    padding: var(--space-sm) var(--space-md);
    font-size: var(--font-size-body);
    color: var(--color-text-primary);
    border-bottom: 1px solid var(--color-border);
    vertical-align: middle;
  }

  .cell--actions {
    white-space: nowrap;
    display: flex;
    gap: var(--space-xs);
    align-items: center;
  }

  .badge {
    display: inline-block;
    padding: 2px var(--space-sm);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-label);
    font-weight: var(--font-weight-medium);

    &--active {
      background: color-mix(in srgb, #27ae60 15%, transparent);
      color: #1a7a40;
    }

    &--blocked {
      background: color-mix(in srgb, var(--color-text-muted) 15%, transparent);
      color: var(--color-text-muted);
    }
  }

  .btn-action {
    padding: 2px var(--space-sm);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-label);
    background: transparent;
    color: var(--color-text-secondary);
    cursor: pointer;

    &:hover {
      background: color-mix(in srgb, var(--color-text-primary) 8%, transparent);
      color: var(--color-text-primary);
    }

    &--danger {
      color: var(--color-error, #c0392b);
      border-color: color-mix(in srgb, var(--color-error, #c0392b) 40%, transparent);

      &:hover {
        background: color-mix(in srgb, var(--color-error, #c0392b) 10%, transparent);
        color: var(--color-error, #c0392b);
      }
    }
  }

  .confirm-text {
    font-size: var(--font-size-label);
    color: var(--color-text-secondary);
  }
</style>
