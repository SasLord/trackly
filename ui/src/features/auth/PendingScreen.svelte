<script lang="ts">
  // Phase 9 Plan 05 — Screen 2 (UI-SPEC). Informational terminal state shown
  // after `auth_login` returns AppError code REGISTRATION_PENDING (pending
  // registration mode, D-REG-01). No primary CTA — purely informational.
  interface Props {
    onBackToLogin: () => void;
  }

  const { onBackToLogin }: Props = $props();
</script>

<div class="login-container">
  <div class="login-card">
    <h1 class="login-title">Заявка отправлена</h1>
    <p class="screen-body">
      Ваша заявка на регистрацию отправлена администратору. Доступ появится после
      подтверждения. Попробуйте войти позже.
    </p>
    <button class="btn-link" type="button" onclick={onBackToLogin}>
      Войти под другим пользователем
    </button>
  </div>
</div>

<style lang="scss">
  .login-container {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    background: var(--color-bg);
  }

  .login-card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: var(--space-xl) var(--space-2xl, 2rem);
    width: 100%;
    max-width: 360px;
    box-shadow: 0 2px 12px rgba(0, 0, 0, 0.08);
    text-align: center;
  }

  .login-title {
    margin: 0 0 var(--space-lg);
    font-size: var(--font-size-heading);
    font-weight: var(--font-weight-semibold);
    color: var(--color-text-primary);
    text-align: center;
  }

  .screen-body {
    margin: 0 0 var(--space-lg);
    font-size: var(--font-size-body);
    line-height: var(--line-height-body);
    color: var(--color-text-secondary);
  }

  .btn-link {
    background: transparent;
    border: none;
    padding: 0;
    color: var(--color-accent);
    font-size: var(--font-size-body);
    cursor: pointer;

    &:hover {
      text-decoration: underline;
    }
    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--color-accent-focus);
    }
  }
</style>
