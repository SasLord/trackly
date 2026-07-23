<script lang="ts">
  // Phase 9 Plan 05 — Screen 2 (UI-SPEC). Informational terminal state shown
  // after `auth_login` returns AppError code REGISTRATION_PENDING (pending
  // registration mode, D-REG-01). No primary CTA — purely informational.
  import AuthShell from '$lib/components/AuthShell.svelte';
  import Button from '$lib/components/Button.svelte';

  interface Props {
    onBackToLogin: () => void;
  }

  const { onBackToLogin }: Props = $props();
</script>

<AuthShell>
  <div class="pending-card">
    <h1 class="login-title">Заявка отправлена</h1>
    <p class="screen-body">
      Ваша заявка на регистрацию отправлена администратору. Доступ появится после подтверждения.
      Попробуйте войти позже.
    </p>
    <Button variant="link" onclick={onBackToLogin}>Войти под другим пользователем</Button>
  </div>
</AuthShell>

<style lang="scss">
  .pending-card {
    text-align: center;
  }

  .login-title {
    margin: 0 0 var(--tr-space-xl);
    font-size: var(--tr-font-size-h3);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
    text-align: center;
  }

  .screen-body {
    margin: 0 0 var(--tr-space-xl);
    font-size: var(--tr-font-size-body);
    line-height: var(--tr-line-height-body);
    color: var(--tr-text-secondary);
  }
</style>
