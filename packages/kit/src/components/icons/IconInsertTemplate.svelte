<script lang="ts">
  // Первый переиспользуемый компонент-иконка проекта (UI-SPEC Discretion #1,
  // план 40.2-09): до этого все иконки — inline SVG в месте использования
  // (PageHeader.svelte, Select.svelte). Эта иконка нужна минимум в двух
  // местах (кнопка «Вставка» поля номера + витрина), поэтому вынесена в
  // отдельный компонент по образцу их же стиля: fill="none", stroke
  // currentColor 1.5, round caps/joins, aria-hidden.
  interface Props {
    /** Сторона квадратного SVG в px. По умолчанию 20 (UI-SPEC §9). */
    size?: number;
  }

  const { size = 20 }: Props = $props();
</script>

<svg width={size} height={size} viewBox="0 0 20 20" fill="none" aria-hidden="true">
  <!-- Папка -->
  <path
    d="M2.5 7.5A1.5 1.5 0 0 1 4 6h2.5l1.5 2h8A1.5 1.5 0 0 1 17.5 9.5v6A1.5 1.5 0 0 1 16 17H4a1.5 1.5 0 0 1-1.5-1.5z"
    stroke="currentColor"
    stroke-width="1.5"
    stroke-linecap="round"
    stroke-linejoin="round"
  />
  <!-- Лист, выглядывающий сверху -->
  <path
    d="M6.5 8V3.5h5l2 2V8"
    stroke="currentColor"
    stroke-width="1.5"
    stroke-linecap="round"
    stroke-linejoin="round"
  />
  <!-- Линия текста на листе -->
  <path
    d="M8.5 6h3"
    stroke="currentColor"
    stroke-width="1.5"
    stroke-linecap="round"
    stroke-linejoin="round"
  />
</svg>
