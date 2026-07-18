<script lang="ts">
  import Badge from '$lib/components/Badge.svelte';
</script>

<section class="badge-section">
  <h2>Бейджи</h2>

  <div class="tone-row">
    <h3 class="tone-label">Default (neutral)</h3>
    <div class="appearance-group">
      <Badge variant="default" appearance="soft">Метка</Badge>
      <Badge variant="default" appearance="solid">Метка</Badge>
      <Badge variant="default" appearance="dot">Метка</Badge>
      <Badge variant="default" appearance="count">12</Badge>
    </div>
  </div>

  <div class="tone-row">
    <h3 class="tone-label">Accent</h3>
    <div class="appearance-group">
      <Badge variant="accent" appearance="soft">Метка</Badge>
      <Badge variant="accent" appearance="solid">Метка</Badge>
      <Badge variant="accent" appearance="dot">Метка</Badge>
      <Badge variant="accent" appearance="count">12</Badge>
    </div>
  </div>

  <div class="tone-row">
    <h3 class="tone-label">Success</h3>
    <div class="appearance-group">
      <Badge variant="success" appearance="soft">Метка</Badge>
      <Badge variant="success" appearance="solid">Метка</Badge>
      <Badge variant="success" appearance="dot">Метка</Badge>
      <Badge variant="success" appearance="count">12</Badge>
    </div>
  </div>

  <div class="tone-row">
    <h3 class="tone-label">Warning</h3>
    <div class="appearance-group">
      <Badge variant="warning" appearance="soft">Метка</Badge>
      <Badge variant="warning" appearance="solid">Метка</Badge>
      <Badge variant="warning" appearance="dot">Метка</Badge>
      <Badge variant="warning" appearance="count">12</Badge>
    </div>
  </div>

  <div class="tone-row">
    <h3 class="tone-label">Destructive (danger)</h3>
    <div class="appearance-group">
      <Badge variant="destructive" appearance="soft">Метка</Badge>
      <Badge variant="destructive" appearance="solid">Метка</Badge>
      <Badge variant="destructive" appearance="dot">Метка</Badge>
      <Badge variant="destructive" appearance="count">12</Badge>
    </div>
  </div>
</section>

<style lang="scss">
  .badge-section {
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

  .tone-row {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-sm);
  }

  .tone-label {
    margin: 0;
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-secondary);
    text-transform: uppercase;
  }

  .appearance-group {
    display: flex;
    align-items: center;
    gap: var(--tr-space-md);
  }
</style>
