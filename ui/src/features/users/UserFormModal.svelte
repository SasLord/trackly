<script lang="ts">
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import type { UserDto } from '../../bindings';

  interface UserFormData {
    login: string;
    full_name: string;
    password: string;
    role: string;
    email: string;
    is_active: boolean;
  }

  interface Props {
    open: boolean;
    mode: 'create' | 'edit';
    user?: UserDto | null;
    onSave: (data: UserFormData) => Promise<void>;
    onCancel: () => void;
  }

  const { open, mode, user = null, onSave, onCancel }: Props = $props();

  const roleOptions = [
    { value: 'admin', label: 'Администратор' },
    { value: 'manager', label: 'Специалист' },
    { value: 'employee', label: 'Сотрудник' },
  ];

  let form = $state<UserFormData>({
    login: '',
    full_name: '',
    password: '',
    role: 'employee',
    email: '',
    is_active: true,
  });

  let saving = $state(false);
  let error = $state<string | null>(null);

  // Per-field validation errors
  let loginErr = $state<string | null>(null);
  let passwordErr = $state<string | null>(null);
  let roleErr = $state<string | null>(null);

  // Reinitialize form when modal opens or user prop changes.
  $effect(() => {
    if (open) {
      loginErr = null;
      passwordErr = null;
      roleErr = null;
      error = null;
      if (mode === 'edit' && user) {
        form = {
          login: user.login,
          full_name: user.full_name,
          password: '',
          role: user.role,
          email: user.email ?? '',
          is_active: user.is_active,
        };
      } else {
        form = {
          login: '',
          full_name: '',
          password: '',
          role: 'employee',
          email: '',
          is_active: true,
        };
      }
    }
  });

  function validate(): boolean {
    loginErr = null;
    passwordErr = null;
    roleErr = null;
    let ok = true;

    if (mode === 'create' && form.login.trim().length < 3) {
      loginErr = 'Логин должен быть не менее 3 символов';
      ok = false;
    }
    if (form.password && form.password.length < 8) {
      passwordErr = 'Пароль должен быть не менее 8 символов';
      ok = false;
    }
    if (mode === 'create' && !form.password) {
      passwordErr = 'Введите пароль';
      ok = false;
    }
    if (!form.role) {
      roleErr = 'Выберите роль';
      ok = false;
    }
    return ok;
  }

  async function handleSave() {
    if (!validate()) return;
    saving = true;
    error = null;
    try {
      await onSave({ ...form });
    } catch (e: unknown) {
      const msg =
        e && typeof e === 'object' && 'message' in e
          ? String((e as { message: unknown }).message)
          : 'Не удалось сохранить';
      error = msg;
    } finally {
      saving = false;
    }
  }

  const modalTitle = $derived(mode === 'edit' ? 'Редактирование пользователя' : 'Новый пользователь');
  const submitLabel = $derived(mode === 'edit' ? 'Сохранить' : 'Создать');
</script>

<Modal {open} title={modalTitle} size="md" onClose={onCancel}>
  <div class="user-form">
    <div class="form-field">
      <label class="form-label" for="uf-login">Логин</label>
      <input
        id="uf-login"
        class="form-input"
        class:is-error={loginErr !== null}
        type="text"
        bind:value={form.login}
        disabled={saving || mode === 'edit'}
        readonly={mode === 'edit'}
        placeholder="Логин пользователя"
      />
      {#if loginErr}
        <span class="field-error">{loginErr}</span>
      {/if}
    </div>

    <div class="form-field">
      <label class="form-label" for="uf-fullname">ФИО</label>
      <input
        id="uf-fullname"
        class="form-input"
        type="text"
        bind:value={form.full_name}
        disabled={saving}
        placeholder="Полное имя"
      />
    </div>

    <div class="form-field">
      <label class="form-label" for="uf-password">
        {mode === 'create' ? 'Пароль' : 'Новый пароль (оставьте пустым, чтобы не менять)'}
      </label>
      <input
        id="uf-password"
        class="form-input"
        class:is-error={passwordErr !== null}
        type="password"
        bind:value={form.password}
        disabled={saving}
        placeholder={mode === 'create' ? 'Не менее 8 символов' : 'Оставьте пустым'}
        autocomplete="new-password"
      />
      {#if passwordErr}
        <span class="field-error">{passwordErr}</span>
      {/if}
    </div>

    <div class="form-field">
      <label class="form-label" for="uf-role">Роль</label>
      <select
        id="uf-role"
        class="form-select"
        class:is-error={roleErr !== null}
        bind:value={form.role}
        disabled={saving}
      >
        {#each roleOptions as opt}
          <option value={opt.value}>{opt.label}</option>
        {/each}
      </select>
      {#if roleErr}
        <span class="field-error">{roleErr}</span>
      {/if}
    </div>

    <div class="form-field">
      <label class="form-label" for="uf-email">Email (необязательно)</label>
      <input
        id="uf-email"
        class="form-input"
        type="email"
        bind:value={form.email}
        disabled={saving}
        placeholder="user@example.com"
      />
    </div>

    <div class="form-field form-field--checkbox">
      <label class="checkbox-label">
        <input type="checkbox" bind:checked={form.is_active} disabled={saving} />
        <span>Активен</span>
      </label>
    </div>

    {#if error}
      <div class="server-error">{error}</div>
    {/if}
  </div>

  {#snippet footer()}
    <Button variant="secondary" onclick={onCancel} disabled={saving}>Отмена</Button>
    <Button variant="primary" loading={saving} onclick={handleSave}>
      {#if saving}Сохранение…{:else}{submitLabel}{/if}
    </Button>
  {/snippet}
</Modal>

<style lang="scss">
  .user-form {
    display: flex;
    flex-direction: column;
    gap: var(--space-md);
    padding: var(--space-md) 0;
  }

  .form-field {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
  }

  .form-field--checkbox {
    flex-direction: row;
    align-items: center;
  }

  .form-label {
    font-size: var(--font-size-label);
    font-weight: var(--font-weight-medium);
    color: var(--color-text-secondary);
  }

  .form-input,
  .form-select {
    padding: var(--space-sm) var(--space-md);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-body);
    background: var(--color-bg);
    color: var(--color-text-primary);

    &:focus {
      outline: none;
      border-color: var(--color-accent);
      box-shadow: 0 0 0 2px color-mix(in srgb, var(--color-accent) 20%, transparent);
    }

    &.is-error {
      border-color: var(--color-error, #c0392b);
    }

    &:disabled {
      opacity: 0.6;
      cursor: not-allowed;
    }
  }

  .field-error {
    font-size: var(--font-size-label);
    color: var(--color-error, #c0392b);
  }

  .server-error {
    padding: var(--space-sm) var(--space-md);
    background: color-mix(in srgb, var(--color-error, #c0392b) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--color-error, #c0392b) 30%, transparent);
    border-radius: var(--radius-sm);
    font-size: var(--font-size-body);
    color: var(--color-error, #c0392b);
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: var(--space-sm);
    font-size: var(--font-size-body);
    color: var(--color-text-primary);
    cursor: pointer;

    input[type='checkbox'] {
      width: 16px;
      height: 16px;
      accent-color: var(--color-accent);
    }
  }
</style>
