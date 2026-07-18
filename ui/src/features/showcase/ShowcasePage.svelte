<script lang="ts">
  import ButtonsSection from './sections/ButtonsSection.svelte';
  import FieldsSection from './sections/FieldsSection.svelte';
  import BadgeSection from './sections/BadgeSection.svelte';
  import TabsSection from './sections/TabsSection.svelte';
  import ModalSection from './sections/ModalSection.svelte';
  import TableSection from './sections/TableSection.svelte';
</script>

<div class="showcase-page">
  <h1>Витрина компонентов</h1>
  <p class="intro">
    Живая галерея примитивов дизайн-системы Trackly — отражает референсные `.dc`-макеты (Buttons,
    Fields, Badges, Tabs, Modal) во всех вариантах и состояниях. Страница не содержит боевых данных
    и служит опорной точкой для визуальной проверки при развитии компонентов в фазах 25–30.
  </p>

  <section class="showcase-block">
    <ButtonsSection />
  </section>

  <section class="showcase-block">
    <FieldsSection />
  </section>

  <section class="showcase-block">
    <BadgeSection />
  </section>

  <section class="showcase-block">
    <TabsSection />
  </section>

  <section class="showcase-block">
    <ModalSection />
  </section>

  <section class="showcase-block">
    <TableSection />
  </section>
</div>

<style lang="scss">
  .showcase-page {
    padding: var(--tr-space-xl);
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xl);
  }

  .intro {
    max-width: 720px;
    color: var(--tr-text-secondary);
  }

  .showcase-block {
    padding-bottom: var(--tr-space-xl);
    border-bottom: 1px solid var(--tr-border);

    &:last-child {
      border-bottom: none;
      padding-bottom: 0;
    }
  }
</style>
