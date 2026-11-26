<script lang="ts">
  interface Props {
    size?: 'sm' | 'md' | 'lg';
  }
  const { size = 'md' }: Props = $props();

  const px: Record<string, number> = { sm: 12, md: 16, lg: 24 };
  const dim = $derived(px[size] ?? 16);
</script>

<svg
  class="spinner"
  width={dim}
  height={dim}
  viewBox="0 0 24 24"
  fill="none"
  aria-hidden="true"
  style="width:{dim}px;height:{dim}px"
>
  <circle
    cx="12"
    cy="12"
    r="10"
    stroke="currentColor"
    stroke-width="3"
    stroke-linecap="round"
    stroke-dasharray="31.4 31.4"
  />
</svg>

<style lang="scss">
  .spinner {
    display: inline-block;
    animation: spin 0.8s linear infinite;
    color: currentColor;
    flex-shrink: 0;
    vertical-align: middle;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }
</style>
