<script lang="ts">
  // Plan 04-05: LowStockBanner — предупреждение о низком остатке картриджей.
  // Показывается только при непустом массиве items (UI-SPEC §LowStockBanner, CART-12).
  import type { LowStockItemDto } from '../../bindings';

  interface Props {
    items: LowStockItemDto[];
  }

  const { items }: Props = $props();
</script>

{#if items.length > 0}
  <div class="low-stock-banner" role="alert" aria-live="polite">
    <span class="low-stock-icon" aria-hidden="true">
      <!-- Иконка предупреждения: треугольник с восклицательным знаком, --tr-warning -->
      <svg
        width="16"
        height="16"
        viewBox="0 0 16 16"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        aria-hidden="true"
      >
        <path
          d="M8 1.5L14.5 13H1.5L8 1.5Z"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linejoin="round"
        />
        <path d="M8 6V9" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
        <circle cx="8" cy="11" r="0.75" fill="currentColor" />
      </svg>
    </span>
    <div class="low-stock-content">
      <h4 class="low-stock-title">Низкий остаток картриджей</h4>
      <ul class="low-stock-list">
        {#each items as item (`${item.basis}:${item.model_id ?? item.label}`)}
          <li>
            {#if item.basis === 'cartridge_model'}
              {item.brand}
              {item.model} — {item.count} шт. на складе (порог: {item.threshold})
            {:else}
              {item.label} — {item.count} шт. совместимых картриджей на складе (порог: {item.threshold})
            {/if}
          </li>
        {/each}
      </ul>
    </div>
  </div>
{/if}

<style lang="scss">
  .low-stock-banner {
    display: flex;
    align-items: flex-start;
    gap: var(--tr-space-xs);
    padding: var(--tr-space-md);
    margin-bottom: var(--tr-space-md);
    background: color-mix(in srgb, var(--tr-warning) 10%, transparent);
    border: 1px solid var(--tr-warning);
    border-radius: var(--tr-radius-md);
    color: var(--tr-text-primary);
  }

  .low-stock-icon {
    color: var(--tr-warning);
    flex-shrink: 0;
    margin-top: 2px;
    display: flex;
    align-items: center;
    width: 16px;
    height: 16px;
  }

  .low-stock-content {
    flex: 1;
  }

  .low-stock-title {
    display: block;
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-semibold);
    color: var(--tr-text-primary);
    margin: 0 0 var(--tr-space-2xs);
  }

  .low-stock-list {
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);

    li {
      line-height: 1.6;
    }
  }
</style>
