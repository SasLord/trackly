<script lang="ts">
  // Phase 39 Plan 14 (PLC-01/PLC-06, 39-UI-SPEC.md §7): root component of the
  // "Места" section — PageHeader + primary "Создать место" action (Admin-only,
  // D-20) + PlacesMasterDetail(PlaceTree | placeholder detail panel).
  //
  // Deep link (§7): the selected node is reflected in the hash (`#/places?id=…`)
  // so Plan 20's "Показать содержимое" (D-14 delete-blocked callout) and future
  // cross-feature links can land directly on a node. Read once on mount (this
  // component is only ever (re)created when the router navigates to /places, so
  // "on mount" and "on module init" coincide here); written via
  // history.replaceState (no hashchange fired — the SPA router does not
  // remount, and no extra back-button entry is created per navigation).
  //
  // The right panel is a static "Место не выбрано" placeholder in THIS plan —
  // Plan 20 (Wave 9) replaces the detail snippet with the real PlaceContents
  // component once the tree's selection has somewhere real to render into.
  import { authStore } from '$lib/stores/auth.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import Button from '$lib/components/Button.svelte';
  import DetailPanel from '$lib/components/DetailPanel.svelte';
  import PlacesMasterDetail from './PlacesMasterDetail.svelte';
  import PlaceTree from './PlaceTree.svelte';
  import PlaceFormModal from './PlaceFormModal.svelte';
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

  function handleTreeSelect(place: PlaceDto | null): void {
    const newHash = place ? `#/places?id=${place.id}` : '#/places';
    if (window.location.hash !== newHash) {
      window.history.replaceState(null, '', newHash);
    }
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
        <PlaceTree {initialSelectedId} onSelect={handleTreeSelect} {refreshToken} />
      {/snippet}
      {#snippet detail()}
        <DetailPanel
          empty={true}
          emptyTitle="Место не выбрано"
          emptyBody="Выберите место в дереве слева, чтобы увидеть его содержимое."
        />
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
