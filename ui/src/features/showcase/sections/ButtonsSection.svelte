<script lang="ts">
  import Button from '$lib/components/Button.svelte';
</script>

<section class="buttons-section">
  <h2>Кнопки</h2>

  <div class="variant-row">
    <h3 class="variant-label">Primary</h3>
    <div class="size-group">
      <span class="size-tag">sm</span>
      <Button variant="primary" size="sm">Обычная</Button>
      <Button variant="primary" size="sm" disabled>Отключена</Button>
      <Button variant="primary" size="sm" loading>Загрузка</Button>
    </div>
    <div class="size-group">
      <span class="size-tag">md</span>
      <Button variant="primary" size="md">Обычная</Button>
      <Button variant="primary" size="md" disabled>Отключена</Button>
      <Button variant="primary" size="md" loading>Загрузка</Button>
    </div>
  </div>

  <div class="variant-row">
    <h3 class="variant-label">Secondary</h3>
    <div class="size-group">
      <span class="size-tag">sm</span>
      <Button variant="secondary" size="sm">Обычная</Button>
      <Button variant="secondary" size="sm" disabled>Отключена</Button>
      <Button variant="secondary" size="sm" loading>Загрузка</Button>
    </div>
    <div class="size-group">
      <span class="size-tag">md</span>
      <Button variant="secondary" size="md">Обычная</Button>
      <Button variant="secondary" size="md" disabled>Отключена</Button>
      <Button variant="secondary" size="md" loading>Загрузка</Button>
    </div>
  </div>

  <div class="variant-row">
    <h3 class="variant-label">Destructive</h3>
    <div class="size-group">
      <span class="size-tag">sm</span>
      <Button variant="destructive" size="sm">Обычная</Button>
      <Button variant="destructive" size="sm" disabled>Отключена</Button>
      <Button variant="destructive" size="sm" loading>Загрузка</Button>
    </div>
    <div class="size-group">
      <span class="size-tag">md</span>
      <Button variant="destructive" size="md">Обычная</Button>
      <Button variant="destructive" size="md" disabled>Отключена</Button>
      <Button variant="destructive" size="md" loading>Загрузка</Button>
    </div>
  </div>

  <div class="variant-row">
    <h3 class="variant-label">Ghost</h3>
    <div class="size-group">
      <span class="size-tag">sm</span>
      <Button variant="ghost" size="sm">Обычная</Button>
      <Button variant="ghost" size="sm" disabled>Отключена</Button>
      <Button variant="ghost" size="sm" loading>Загрузка</Button>
    </div>
    <div class="size-group">
      <span class="size-tag">md</span>
      <Button variant="ghost" size="md">Обычная</Button>
      <Button variant="ghost" size="md" disabled>Отключена</Button>
      <Button variant="ghost" size="md" loading>Загрузка</Button>
    </div>
  </div>

  <div class="variant-row">
    <h3 class="variant-label">Link</h3>
    <div class="size-group">
      <span class="size-tag">sm</span>
      <Button variant="link" size="sm">Обычная</Button>
      <Button variant="link" size="sm" disabled>Отключена</Button>
      <Button variant="link" size="sm" loading>Загрузка</Button>
    </div>
    <div class="size-group">
      <span class="size-tag">md</span>
      <Button variant="link" size="md">Обычная</Button>
      <Button variant="link" size="md" disabled>Отключена</Button>
      <Button variant="link" size="md" loading>Загрузка</Button>
    </div>
  </div>
</section>

<style lang="scss">
  .buttons-section {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-lg);
  }

  h2 {
    margin: 0;
    font-size: var(--tr-font-size-h2);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
  }

  .variant-row {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-sm);
  }

  .variant-label {
    margin: 0;
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-secondary);
    text-transform: uppercase;
  }

  .size-group {
    display: flex;
    align-items: center;
    gap: var(--tr-space-md);
  }

  .size-tag {
    width: 28px;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
  }
</style>
