<script lang="ts">
  // Plan 28-09 (D-04): rebuilt on Input/Select/Checkbox primitives per
  // DeviceFormBody.svelte precedent (.form-field/.form-label + primitive +
  // {#if fieldErr}<span class="field-error">). Пароль — обязательное raw
  // password-input исключение: Input.svelte's `type` contract is
  // 'text' | 'number' | 'search' only, no 'password' — rendering it via
  // `Input type="text"` would strip masking and show password characters in
  // plaintext (T-28-09-01). Email uses Input type="text" — Input.svelte has
  // no 'email' type either; native HTML5 email validation is lost here, the
  // server-side validation remains authoritative (documented in SUMMARY).
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Input from '$lib/components/Input.svelte';
  // Plan 28-13 (GAP-1): Select (нативный <select>) заменён на кастомный
  // Dropdown (flat + variant="select") для поля «Роль», по прецеденту
  // CartridgeFormBody.svelte (Plan 27-G1).
  import Dropdown from '$lib/components/Dropdown.svelte';
  import Checkbox from '$lib/components/Checkbox.svelte';
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

  // Плоские опции без drill-in — onExpandGroup никогда реально не вызывается
  // (isGroupExpandable всегда false), но Dropdown требует типизированную
  // функцию для вывода TMember (иначе `() => []` выводит `never[]`).
  function noExpandRole(): { value: string; label: string }[] {
    return [];
  }

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

  const selectedRoleLabel = $derived(roleOptions.find((o) => o.value === form.role)?.label ?? '');

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

  const modalTitle = $derived(
    mode === 'edit' ? 'Редактирование пользователя' : 'Новый пользователь',
  );
  const submitLabel = $derived(mode === 'edit' ? 'Сохранить' : 'Создать');
</script>

<Modal {open} title={modalTitle} size="md" onClose={onCancel}>
  <div class="user-form">
    <div class="form-field" class:has-error={loginErr !== null}>
      <label class="form-label" for="uf-login">Логин</label>
      <Input
        id="uf-login"
        value={form.login}
        invalid={loginErr !== null}
        disabled={saving || mode === 'edit'}
        placeholder="Логин пользователя"
        oninput={(v) => (form.login = v)}
      />
      {#if loginErr}
        <span class="field-error">{loginErr}</span>
      {/if}
    </div>

    <div class="form-field">
      <label class="form-label" for="uf-fullname">ФИО</label>
      <Input
        id="uf-fullname"
        value={form.full_name}
        disabled={saving}
        placeholder="Полное имя"
        oninput={(v) => (form.full_name = v)}
      />
    </div>

    <div class="form-field" class:has-error={passwordErr !== null}>
      <label class="form-label" for="uf-password">
        {mode === 'create' ? 'Пароль' : 'Новый пароль (оставьте пустым, чтобы не менять)'}
      </label>
      <!--
        T-28-09-01 — обязательное raw-исключение из D-04: Input.svelte не
        поддерживает password-тип (контракт ограничен 'text'|'number'|
        'search'). Рендер через Input с текстовым типом ЗАПРЕЩЁН — это сняло
        бы маскировку пароля и показало бы вводимые символы открытым текстом.
      -->
      <input
        id="uf-password"
        class="input"
        class:invalid={passwordErr !== null}
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

    <div class="form-field" class:has-error={roleErr !== null}>
      <label class="form-label dropdown-label">
        <span>Роль</span>
        <Dropdown
          variant="select"
          flat={true}
          value={selectedRoleLabel}
          placeholder="Выберите роль"
          searchPlaceholder="Поиск"
          invalid={roleErr !== null}
          disabled={saving}
          loading={false}
          groups={roleOptions}
          getGroupId={(o) => o.value}
          getGroupName={(o) => o.label}
          getGroupCount={() => 0}
          isGroupExpandable={() => false}
          isGroupSelected={(o) => o.value === form.role}
          onExpandGroup={noExpandRole}
          getMemberId={(o) => o.value}
          getMemberName={(o) => o.label}
          onSearch={() => {}}
          onPickGroup={(o) => (form.role = o.value)}
          onPickMember={() => {}}
        />
      </label>
      {#if roleErr}
        <span class="field-error">{roleErr}</span>
      {/if}
    </div>

    <div class="form-field">
      <label class="form-label" for="uf-email">Email (необязательно)</label>
      <Input
        id="uf-email"
        type="text"
        value={form.email}
        disabled={saving}
        placeholder="user@example.com"
        oninput={(v) => (form.email = v)}
      />
    </div>

    <div class="form-field form-field--checkbox">
      <Checkbox
        id="uf-active"
        checked={form.is_active}
        disabled={saving}
        onchange={(checked) => (form.is_active = checked)}
      >
        Активен
      </Checkbox>
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
    gap: var(--tr-space-md);
    padding: var(--tr-space-md) 0;
  }

  .form-field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  .form-field--checkbox {
    flex-direction: row;
    align-items: center;
  }

  .form-label {
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-secondary);
  }

  // Plan 28-13 (GAP-1): Dropdown не принимает `id`, поэтому подпись
  // оборачивает поле (implicit label) вместо `for`/`id` association —
  // сохраняет вертикальный макет «подпись сверху, поле снизу» (см.
  // CartridgeFormBody.svelte's .dropdown-label precedent).
  .dropdown-label {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  // Пароль — raw password-input (T-28-09-01), не Input-примитив.
  // Стиль дублирует Input.svelte's .input/.invalid токены 1:1 для визуальной
  // консистентности с остальными полями формы, не создавая параллельного
  // конкурирующего оформления.
  .input {
    display: block;
    width: 100%;
    height: 36px;
    padding: 0 var(--tr-space-md);
    background: var(--tr-surface-raised);
    color: var(--tr-text-primary);
    border: 1px solid var(--tr-border-strong);
    border-radius: var(--tr-radius-sm);
    font-family: var(--tr-font-family);
    font-size: var(--tr-font-size-body);
    line-height: var(--tr-line-height-body);

    &::placeholder {
      color: var(--tr-text-tertiary);
    }

    &:focus-visible {
      outline: none;
      border-color: var(--tr-accent);
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }

    &.invalid {
      border-color: var(--tr-danger);
      box-shadow: 0 0 0 3px var(--tr-danger-ring);
    }

    &:disabled {
      background: var(--tr-surface-sunken);
      color: var(--tr-text-tertiary);
      cursor: not-allowed;
    }
  }

  .field-error {
    font-size: var(--tr-font-size-label);
    color: var(--tr-danger);
  }

  .server-error {
    padding: var(--tr-space-xs) var(--tr-space-md);
    background: color-mix(in srgb, var(--tr-danger) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--tr-danger) 30%, transparent);
    border-radius: var(--tr-radius-xs);
    font-size: var(--tr-font-size-body);
    color: var(--tr-danger);
  }
</style>
