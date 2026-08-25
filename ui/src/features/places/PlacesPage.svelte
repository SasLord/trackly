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
  // `handleShowBlockedContents`. `activeTab` (UAT gap 10) is lifted out the
  // same way and for the same reason, but is NEVER reset by
  // `handleShowBlockedContents` — unlike `onlyHere`, nothing about the D-14
  // "Показать содержимое" path implies the previously active tab is wrong.
  import { authStore } from '$lib/stores/auth.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import Button from '$lib/components/Button.svelte';
  import DetailPanel from '$lib/components/DetailPanel.svelte';
  import PlacesMasterDetail from './PlacesMasterDetail.svelte';
  import PlaceTree from './PlaceTree.svelte';
  import PlaceFormModal from './PlaceFormModal.svelte';
  import PlaceContents from './PlaceContents.svelte';
  import type { ContentTab } from './PlaceContents.svelte';
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

  // UAT gap 5 (2026-08-25): navigating away from "Места" and back used to lose
  // the selected node and the "Только здесь" toggle (the tree's own expanded
  // nodes are persisted separately, in PlaceTree.svelte). Follows the app's
  // existing localStorage convention (see $lib/stores/theme.svelte.ts:
  // `trackly:`-prefixed key, plain localStorage.getItem/setItem, no shared
  // store abstraction) rather than inventing a second one. The URL hash
  // (`#/places?id=…`) still WINS when present — it is a more specific,
  // explicit deep link (breadcrumb click, D-14 "Показать содержимое", a
  // shared/bookmarked link) than "whatever was last open".
  const SELECTED_STORAGE_KEY = 'trackly:places:selectedId';
  const ONLY_HERE_STORAGE_KEY = 'trackly:places:onlyHere';
  // GAP-10 (39-UAT.md, Прогон 5): the content-panel tab (Все/Устройства/
  // Принтеры/Картриджи) used to live as local state INSIDE PlaceContents,
  // which is remounted on every node selection AND fully destroyed whenever
  // the user leaves "Места" — same class of bug as GAP-1/GAP-5 before them.
  // Same fix, same convention: lifted here, persisted as a plain string.
  const ACTIVE_TAB_STORAGE_KEY = 'trackly:places:activeTab';
  const VALID_ACTIVE_TABS: ContentTab[] = ['all', 'device', 'printer', 'cartridge'];

  function readPersistedSelectedId(): number | null {
    if (typeof window === 'undefined') return null;
    try {
      const raw = localStorage.getItem(SELECTED_STORAGE_KEY);
      if (!raw) return null;
      const n = Number(raw);
      return Number.isInteger(n) ? n : null;
    } catch {
      return null;
    }
  }

  function persistSelectedId(id: number | null): void {
    if (typeof window === 'undefined') return;
    try {
      if (id === null) {
        localStorage.removeItem(SELECTED_STORAGE_KEY);
      } else {
        localStorage.setItem(SELECTED_STORAGE_KEY, String(id));
      }
    } catch {
      // Storage unavailable/quota exceeded — selection simply won't persist
      // across navigation; the page itself still works.
    }
  }

  function readPersistedOnlyHere(): boolean {
    if (typeof window === 'undefined') return false;
    try {
      return localStorage.getItem(ONLY_HERE_STORAGE_KEY) === '1';
    } catch {
      return false;
    }
  }

  function persistOnlyHere(v: boolean): void {
    if (typeof window === 'undefined') return;
    try {
      localStorage.setItem(ONLY_HERE_STORAGE_KEY, v ? '1' : '0');
    } catch {
      // Storage unavailable/quota exceeded — the toggle simply won't persist
      // across navigation.
    }
  }

  // GAP-10: only ever returns one of the four known tab keys — a stray/stale
  // localStorage value (older app version, manual edit, future tab renamed)
  // degrades to 'all' rather than being passed straight through to
  // PlaceContents, which would otherwise render a permanently-empty filtered
  // table with no way for the user to tell why (§`columnCount`/`showKindColumn`
  // in PlaceContents.svelte are only proven correct for these four values).
  function readPersistedActiveTab(): ContentTab {
    if (typeof window === 'undefined') return 'all';
    try {
      const raw = localStorage.getItem(ACTIVE_TAB_STORAGE_KEY);
      return (VALID_ACTIVE_TABS as string[]).includes(raw ?? '') ? (raw as ContentTab) : 'all';
    } catch {
      return 'all';
    }
  }

  function persistActiveTab(v: ContentTab): void {
    if (typeof window === 'undefined') return;
    try {
      localStorage.setItem(ACTIVE_TAB_STORAGE_KEY, v);
    } catch {
      // Storage unavailable/quota exceeded — the tab simply won't persist
      // across navigation.
    }
  }

  // Read exactly once — this component instance lives for the duration of the
  // /places route; re-reading on every re-render would fight the tree's own
  // internal selection state. A restored id that no longer resolves (place
  // deleted, or archived while "Показывать архивные" defaults back to off)
  // degrades gracefully further down: PlaceTree's own loadTree() only applies
  // `initialSelectedId` when it actually finds a matching place, otherwise it
  // falls back to no selection — never a broken/empty detail panel.
  const initialSelectedId = parseIdFromHash() ?? readPersistedSelectedId();

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
  // `handleShowBlockedContents` (the D-14 same-node edge case) — that reset
  // is ALSO persisted (see below), so navigating away and back never
  // resurrects the pre-reset value (UAT gap 5).
  let onlyHere = $state(readPersistedOnlyHere());
  // GAP-10: same lifting as `onlyHere` above — survives PlaceContents'
  // {#key} remount on node selection AND a full route change away and back.
  let activeTab = $state<ContentTab>(readPersistedActiveTab());
  // An out-of-band selection request for PlaceTree — see PlaceTree.svelte's
  // `externalSelect` prop doc-comment. A fresh object per breadcrumb click.
  let externalSelect = $state<{ id: number; token: number } | null>(null);

  function handleTreeSelect(place: PlaceDto | null): void {
    selectedPlace = place;
    const newHash = place ? `#/places?id=${place.id}` : '#/places';
    if (window.location.hash !== newHash) {
      window.history.replaceState(null, '', newHash);
    }
    // UAT gap 5: also fires on PlaceTree's own freshness-sync re-selects
    // (rename/archive/move/reload) and on `onSelect(null)` when the selected
    // place was just deleted — in both cases persisting the current value is
    // correct (deleted → clear, so a future visit doesn't keep retrying a
    // dead id).
    persistSelectedId(place ? place.id : null);
  }

  function handleSelectAncestor(id: number): void {
    externalSelect = { id, token: Date.now() };
  }

  function handleShowBlockedContents(): void {
    contentsResetToken += 1;
    onlyHere = false;
    persistOnlyHere(false);
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
              onOnlyHereChange={(v) => {
                onlyHere = v;
                persistOnlyHere(v);
              }}
              {activeTab}
              onActiveTabChange={(v) => {
                activeTab = v;
                persistActiveTab(v);
              }}
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
