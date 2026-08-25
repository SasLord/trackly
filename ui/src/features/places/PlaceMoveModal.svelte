<script lang="ts">
  // Phase 39 Plan 19 (PLC-01, D-21): move modal per UI-SPEC §11.3. Invoked by
  // Plan 14's tree ActionMenu ("Переместить в…") and by drag-drop (§8.4, which
  // always opens this same dialog rather than moving directly).
  //
  // Cycle detection (D-21): PlacePicker has no "exclude this subtree" option
  // (its Props contract is value/onChange/id/disabled/invalid/fetch*/createPlace
  // only — Plan 13), so this component does not attempt a client-side ancestor
  // pre-check. Per this plan's own <action> text, the cycle check is the
  // server's (`places_move`'s `AppError::Validation{field:"parent_id"}`,
  // Plan 04/05) — the response is mapped inline verbatim rather than
  // duplicated client-side, so the copy can never drift from the backend.
  import Modal from '$lib/components/Modal.svelte';
  import Button from '$lib/components/Button.svelte';
  import Spinner from '$lib/components/Spinner.svelte';
  import PlacePicker from '$lib/components/PlacePicker.svelte';
  import { apiCall } from '$lib/api/client';
  import { pushToast } from '$lib/stores/toast.svelte';
  import type { AppError } from '$lib/api/errors';
  import type { PlaceDto, SubtreeStatsDto } from '../../bindings';

  interface Props {
    place: PlaceDto;
    onClose: () => void;
    onMoved: (_place: PlaceDto) => void;
    /**
     * Plan 39-14 addition (Rule 1 — bug fix, not in Plan 19's original contract):
     * when the caller already knows the destination (drag-drop dropping a row onto
     * a target, or onto the "В корень дерева" zone — D-03), pass it here to
     * pre-fill the picker AND immediately enable "Переместить" without requiring
     * a redundant re-pick. `undefined` (the default, and what the ActionMenu
     * "Переместить в…" path passes implicitly by omitting the prop) preserves
     * Plan 19's original behavior: nothing pre-selected, submit disabled until
     * the user actively chooses a target via PlacePicker.
     *
     * NOTE the distinction this enables: `null` here means "root — explicitly
     * chosen", not "nothing chosen yet". Plan 19's original disabled-check
     * (`selectedParentId === null`) could not tell those apart, which made
     * moving a place to the tree root via this modal structurally unreachable
     * (D-03's move-to-root drop zone would open a modal whose submit button
     * could never enable). `targetChosen` (derived from whether this prop was
     * passed at all, not from the value) fixes that.
     */
    defaultParentId?: number | null;
  }

  const { place, onClose, onMoved, defaultParentId }: Props = $props();

  const hasDefaultTarget = defaultParentId !== undefined;

  let stats = $state<SubtreeStatsDto | null>(null);
  let statsLoading = $state(true);
  let statsError = $state(false);

  let selectedParentId = $state<number | null>(hasDefaultTarget ? (defaultParentId ?? null) : null);
  // Tracks whether a destination has been EXPLICITLY chosen (by prop or by the
  // user), independent of the value itself — `null` is a legitimate chosen
  // value (root, D-03), not an "unfilled" sentinel.
  let targetChosen = $state(hasDefaultTarget);
  let moveErr = $state<string | null>(null);
  let serverErr = $state<string | null>(null);
  let saving = $state(false);

  $effect(() => {
    const rootId = place.id;
    let cancelled = false;
    statsLoading = true;
    statsError = false;
    apiCall<SubtreeStatsDto>('places_subtree_stats', { rootId })
      .then((s) => {
        if (cancelled) return;
        stats = s;
      })
      .catch(() => {
        if (cancelled) return;
        statsError = true;
      })
      .finally(() => {
        if (cancelled) return;
        statsLoading = false;
      });
    return () => {
      cancelled = true;
    };
  });

  // §14.3-style Russian pluralization (one/2-4/many-or-11-14), matching the
  // backend's own `ru_plural` helper (place_service.rs, D-14's delete-blocked
  // message) so identical counts read identically everywhere in the app.
  function ruPlural(n: number, one: string, few: string, many: string): string {
    const nAbs = Math.abs(n);
    const mod100 = nAbs % 100;
    const mod10 = nAbs % 10;
    if (mod100 >= 11 && mod100 <= 14) return many;
    if (mod10 === 1) return one;
    if (mod10 >= 2 && mod10 <= 4) return few;
    return many;
  }

  function joinWithAnd(parts: string[]): string {
    if (parts.length === 0) return '';
    if (parts.length === 1) return parts[0];
    return `${parts.slice(0, -1).join(', ')} и ${parts[parts.length - 1]}`;
  }

  // §11.3's literal example orders "вложенных мест" before "устройств"
  // (opposite of the D-14 delete-blocked message's device-first order) —
  // followed literally here. `cartridge_count` is not in the §11.3 example
  // but is included as a third clause when non-zero, mirroring the backend's
  // own Rule 2 rationale for `build_delete_blocked_message` (a place holding
  // only cartridges must not produce an empty sentence).
  function buildConsequencesText(s: SubtreeStatsDto | null): string | null {
    if (!s) return null;
    const parts: string[] = [];
    if (s.nested_places > 0) {
      parts.push(
        `${s.nested_places} ${ruPlural(s.nested_places, 'вложенное место', 'вложенных места', 'вложенных мест')}`,
      );
    }
    if (s.device_count > 0) {
      parts.push(`${s.device_count} ${ruPlural(s.device_count, 'устройство', 'устройства', 'устройств')}`);
    }
    if (s.cartridge_count > 0) {
      parts.push(
        `${s.cartridge_count} ${ruPlural(s.cartridge_count, 'картридж', 'картриджа', 'картриджей')}`,
      );
    }
    if (parts.length === 0) return null;
    return `Вместе с местом переедет ${joinWithAnd(parts)}.`;
  }

  const consequencesText = $derived(buildConsequencesText(stats));

  function mapServerError(e: unknown): void {
    const err = e as Partial<AppError> | undefined;
    const details = err?.details;
    const field =
      details && typeof details === 'object' && !Array.isArray(details) && 'field' in details
        ? (details as { field?: unknown }).field
        : undefined;
    if (err?.code === 'VALIDATION' && field === 'parent_id') {
      moveErr = err.message ?? 'Ошибка валидации';
      return;
    }
    serverErr = err?.message ?? 'Не удалось переместить место.';
  }

  async function handleSubmit() {
    if (!targetChosen || saving) return;
    moveErr = null;
    serverErr = null;
    saving = true;
    try {
      const moved = await apiCall<PlaceDto>('places_move', {
        id: place.id,
        newParentId: selectedParentId,
        version: place.version,
      });
      pushToast('success', 'Место перемещено');
      onMoved(moved);
    } catch (e) {
      mapServerError(e);
    } finally {
      saving = false;
    }
  }
</script>

<Modal open={true} title="Переместить место" {onClose}>
  <div class="move-form">
    <div class="form-field" class:has-error={moveErr !== null}>
      <label class="form-label" for="pm-parent">Новое родительское место</label>
      <PlacePicker
        id="pm-parent"
        value={selectedParentId}
        invalid={moveErr !== null}
        disabled={saving}
        onChange={(id) => {
          selectedParentId = id;
          targetChosen = true;
          moveErr = null;
        }}
      />
      {#if moveErr}
        <span class="field-error">{moveErr}</span>
      {/if}
    </div>

    {#if statsLoading}
      <div class="stats-loading">
        <Spinner size="sm" />
        <span>Загрузка…</span>
      </div>
    {:else if statsError}
      <div class="server-error">Не удалось загрузить места. Проверьте подключение и повторите.</div>
    {:else if consequencesText}
      <div class="consequences-callout">{consequencesText}</div>
    {/if}

    {#if serverErr}
      <div class="server-error">{serverErr}</div>
    {/if}
  </div>

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose} disabled={saving}>Отмена</Button>
    <Button
      variant="primary"
      loading={saving}
      disabled={!targetChosen}
      onclick={handleSubmit}
    >
      {#if saving}Перемещение…{:else}Переместить{/if}
    </Button>
  {/snippet}
</Modal>

<style lang="scss">
  .move-form {
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

  .form-label {
    font-size: var(--tr-font-size-label);
    font-weight: var(--tr-font-weight-medium);
    color: var(--tr-text-secondary);
  }

  .field-error {
    font-size: var(--tr-font-size-label);
    color: var(--tr-danger);
  }

  .stats-loading {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
    color: var(--tr-text-secondary);
    font-size: var(--tr-font-size-body);
  }

  // §11.3's literal CSS block for the move-consequences warning callout.
  .consequences-callout {
    background: var(--tr-warning-soft);
    border-left: 3px solid var(--tr-warning);
    border-radius: var(--tr-radius-sm);
    padding: var(--tr-space-sm) var(--tr-space-md);
    color: var(--tr-warning-text);
    font: var(--tr-text-body);
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
