<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '$lib/components/Button.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { apiCall } from '$lib/api/client';
  import UsersList from './UsersList.svelte';
  import UserFormModal from './UserFormModal.svelte';
  import type { UserDto, UserListResponse, UserPatch } from '../../bindings';

  let items = $state<UserDto[]>([]);
  let loading = $state(false);
  let modalOpen = $state(false);
  let editTarget = $state<UserDto | null>(null);

  async function refresh() {
    loading = true;
    try {
      const resp = await apiCall<UserListResponse>('users_list', {
        filter: { search: null },
        pagination: { offset: 0, limit: 100 },
      });
      items = resp.items;
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось загрузить список пользователей';
      pushToast('error', msg);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    refresh();
  });

  function openCreate() {
    editTarget = null;
    modalOpen = true;
  }

  function openEdit(user: UserDto) {
    editTarget = user;
    modalOpen = true;
  }

  function closeModal() {
    modalOpen = false;
    editTarget = null;
  }

  async function handleSave(data: {
    login: string;
    full_name: string;
    password: string;
    role: string;
    email: string;
    is_active: boolean;
  }) {
    if (editTarget) {
      // Edit mode — build UserPatch (only non-empty fields)
      // WR-01: forward the new password only when the admin typed one; an
      // empty field means «не менять» (the backend treats null/empty as no-op).
      const patch: UserPatch = {
        full_name: data.full_name || null,
        role: data.role || null,
        email: data.email ? data.email : null,
        is_active: data.is_active,
        password: data.password ? data.password : null,
      };
      await apiCall<UserDto>('users_update', {
        id: editTarget.id,
        version: editTarget.version,
        patch,
      });
      pushToast('success', 'Пользователь обновлён');
    } else {
      // Create mode
      await apiCall<UserDto>('users_create', {
        userNew: {
          login: data.login.trim(),
          full_name: data.full_name.trim(),
          password: data.password,
          role: data.role,
          email: data.email || null,
        },
      });
      pushToast('success', 'Пользователь создан');
    }
    closeModal();
    await refresh();
  }

  async function handleDelete(id: number, version: number) {
    try {
      await apiCall<void>('users_delete', { id, version });
      pushToast('success', 'Пользователь удалён');
      await refresh();
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось удалить пользователя';
      pushToast('error', msg);
    }
  }
</script>

<div class="users-page">
  <PageHeader title="Пользователи">
    {#snippet actions()}
      <Button variant="primary" onclick={openCreate}>+ Добавить пользователя</Button>
    {/snippet}
  </PageHeader>

  <div class="page-content">
    {#if loading}
      <div class="loading-state">Загрузка...</div>
    {:else}
      <UsersList {items} onEdit={openEdit} onDelete={handleDelete} />
    {/if}
  </div>
</div>

<UserFormModal
  open={modalOpen}
  mode={editTarget ? 'edit' : 'create'}
  user={editTarget}
  onSave={handleSave}
  onCancel={closeModal}
/>

<style lang="scss">
  .users-page {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .page-content {
    flex: 1;
    overflow: auto;
    padding: var(--tr-space-xl) var(--tr-space-2xl);
  }

  .loading-state {
    color: var(--tr-text-tertiary);
    font-size: var(--tr-font-size-body);
  }
</style>
