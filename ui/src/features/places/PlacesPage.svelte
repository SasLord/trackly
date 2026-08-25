<script lang="ts">
  // Phase 39 Plan 14 (PLC-01/PLC-06, 39-UI-SPEC.md §7): root component of the
  // "Места" section — PageHeader + primary "Создать место" action (Admin-only,
  // D-20) + PlacesMasterDetail(PlaceTree | PlaceContents/placeholder).
  //
  // Deep link (§7): the selected node is reflected in the hash (`#/places?id=…`)
  // so the D-14 delete-blocked callout's "Показать содержимое" action and future
  // cross-feature links can land directly on a node. Read once on mount (this
  // component is only ever (re)created when the router navigates to /places, so
  // "on mount" and "on module init" coincide here); written via
  // history.replaceState (no hashchange fired — the SPA router does not
  // remount, and no extra back-button entry is created per navigation).
  //
  // Plan 20: the right panel renders `PlaceContents` (breadcrumbs/tabs/table,
  // §9) for the selected node, keyed on `${id}:${contentsResetToken}` so a
  // fresh instance mounts both on genuine selection changes AND on the D-14
  // "Показать содержимое" same-node edge case (see `handleShowBlockedContents`
  // below) — falls back to the static "Место не выбрано" placeholder (§14.2)
  // when nothing is selected. `onlyHere` is lifted OUT of PlaceContents (UAT
  // gap 4.3) so this remount does not reset it on ordinary place-to-place
  // selection; it is reset to false explicitly only by
  // `handleShowBlockedContents`.
  import { authStore } from '$lib/stores/auth.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import Button from '$lib/components/Button.svelte';
  import DetailPanel from '$lib/components/DetailPanel.svelte';
  import PlacesMasterDetail from './PlacesMasterDetail.svelte';
  import PlaceTree from './PlaceTree.svelte';
  import PlaceFormModal from './PlaceFormModal.svelte';
  import PlaceContents from './PlaceContents.svelte';
  import type { PlaceDto } from '../../bindings';

  function parseIdFromHash(): number | null {
    if (typeof window === 'undefined') return null;
    const hash = window.location.hash;
    const qIdx = hash.indexOf('?');
    if (qIdx === -1) return null;
    const qs = new URLSearchParams(hash.slice(qIdx + 1));
    const raw = qs.get('id');
    if (!raw) return null;
    const n = Number(raw);
    return Number.isInteger(n) ? n : null;
  }

  // Read exactly once — this component instance lives for the duration of the
  // /places route; re-reading on every re-render would fight the tree's own
  // internal selection state.
  const initialSelectedId = parseIdFromHash();

  const isAdmin = $derived(authStore.user?.role === 'admin');

  let createModalOpen = $state(false);
  // Bumped whenever an external mutation (the header "Создать место" button)
  // needs PlaceTree to reload — PlaceTree's own ActionMenu-driven mutations
  // (rename/create-child/move/archive/delete) reload themselves internally and
  // don't need this.
  let refreshToken = $state(0);

  // The currently selected node's full data, now that the detail slot renders
  // real content (Plan 20) instead of a static placeholder — PlaceTree keeps
  // this fresh across reloads (rename/archive/move), see PlaceTree.svelte's
  // own `loadTree()` freshness-sync comment.
  let selectedPlace = $state<PlaceDto | null>(null);
  // Bumped by the D-14 delete-blocked "Показать содержимое" action to force a
  // PlaceContents remount in the edge case where the blocked node is ALREADY
  // the selected node (no id change for the {#key} below to react to on its
  // own). `onlyHere` itself is lifted to this component (below) so it
  // otherwise SURVIVES normal place-to-place remounts (UAT gap 4.3) — it is
  // reset to false explicitly, only here, alongside the token bump.
  let contentsResetToken = $state(0);
  // Lifted out of PlaceContents so the "Только здесь" toggle persists across
  // selection changes (which still remount PlaceContents via the {#key}
  // below for its other per-node state). Reset ONLY by
  // `handleShowBlockedContents` (the D-14 same-node edge case).
  let onlyHere = $state(false);
  // An out-of-band selection request for PlaceTree — see PlaceTree.svelte's
  // `externalSelect` prop doc-comment. A fresh object per breadcrumb click.
  let externalSelect = $state<{ id: number; token: number } | null>(null);

  function handleTreeSelect(place: PlaceDto | null): void {
    selectedPlace = place;
    const newHash = place ? `#/places?id=${place.id}` : '#/places';
    if (window.location.hash !== newHash) {
      window.history.replaceState(null, '', newHash);
    }
  }

  function handleSelectAncestor(id: number): void {
    externalSelect = { id, token: Date.now() };
  }

  function handleShowBlockedContents(): void {
    contentsResetToken += 1;
    onlyHere = false;
  }
</script>

<div class="places-page">
  <PageHeader title="Места">
    {#snippet actions()}
      {#if isAdmin}
        <Button variant="primary" onclick={() => (createModalOpen = true)}>Создать место</Button>
      {/if}
    {/snippet}
  </PageHeader>

  <div class="page-content">
    <PlacesMasterDetail>
      {#snippet master()}
        <PlaceTree
          {initialSelectedId}
          onSelect={handleTreeSelect}
          {refreshToken}
          {externalSelect}
          onShowBlockedContents={handleShowBlockedContents}
        />
      {/snippet}
      {#snippet detail()}
        {#if selectedPlace}
          {#key `${selectedPlace.id}:${contentsResetToken}`}
            <PlaceContents
              place={selectedPlace}
              onSelectAncestor={handleSelectAncestor}
              {onlyHere}
              onOnlyHereChange={(v) => (onlyHere = v)}
            />
          {/key}
        {:else}
          <DetailPanel
            empty={true}
            emptyTitle="Место не выбрано"
            emptyBody="Выберите место в дереве слева, чтобы увидеть его содержимое."
          />
        {/if}
      {/snippet}
    </PlacesMasterDetail>
  </div>
</div>

{#if createModalOpen}
  <PlaceFormModal
    mode="create"
    place={null}
    defaultParentId={null}
    onClose={() => (createModalOpen = false)}
    onSaved={() => {
      createModalOpen = false;
      refreshToken += 1;
    }}
  />
{/if}

<style lang="scss">
  .places-page {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .page-content {
    flex: 1 1 auto;
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
    padding: var(--tr-space-lg) var(--tr-space-xl);
  }
</style>
