<script lang="ts">
  // Phase 29, plan 01 (D-01/D-02): label/control/error/hint field wrapper with
  // aria wiring. Formalizes the app-wide .field/.label/.field-error convention
  // (see DeviceFormBody.svelte) for auth screens, and — unlike the pre-existing
  // LoginPage.svelte markup — actually computes `aria-describedby` so error/hint
  // text is wired to the control, not just visually adjacent.
  // Snippet-with-parameters lets each call site receive a ready-made
  // describedBy/invalid pair and forward it straight to <Input>.
  import type { Snippet } from 'svelte';

  interface Props {
    label: string;
    id: string;
    error?: string | null;
    hint?: string;
    children: Snippet<[{ describedBy: string | undefined; invalid: boolean }]>;
  }

  const { label, id, error, hint, children }: Props = $props();

  const describedBy = $derived(error ? `${id}-error` : hint ? `${id}-hint` : undefined);
</script>

<div class="form-field">
  <label class="form-label" for={id}>{label}</label>
  {@render children({ describedBy, invalid: !!error })}
  {#if error}
    <span class="field-error" id="{id}-error">{error}</span>
  {:else if hint}
    <span class="format-hint" id="{id}-hint">{hint}</span>
  {/if}
</div>

<style lang="scss">
  .form-field {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  .form-label {
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-secondary);
  }

  .field-error {
    font-size: var(--tr-font-size-label);
    color: var(--tr-danger-text);
  }

  .format-hint {
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
    line-height: var(--tr-line-height-label);
  }
</style>
