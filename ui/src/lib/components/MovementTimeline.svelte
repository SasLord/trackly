<script lang="ts">
  // Фаза 41.10 (HST-05): единая лента «История» для устройства, принтера, картриджа
  // и группы. Здесь — ЕДИНСТВЕННОЕ место анатомии строки (UI-SPEC R-09).
  //
  // D-02: клиент ничего не склеивает и не пересчитывает — записи приходят готовыми
  // (`HistoryEntryDto[]`), пути мест — server-computed `*_path_short`, ключ {#each} —
  // `entry.key`. Тексты событий и оснований — единый словарь `historyEvents.ts`.
  // Имена групп/устройств выводятся только текстовой интерполяцией (без {@html}).
  //
  // Состояние загрузки: компонент спиннер не рисует — это делает родитель.
  import type { HistoryEntryDto, HistorySegmentDto, MovementEntryDto } from '../../bindings';
  import { reasonText } from '../movementReason';
  import {
    entryIcon,
    kindText,
    reasonLabel,
    UNKNOWN_HISTORY_EVENT,
    UNKNOWN_HISTORY_REASON,
    type HistoryTextCtx,
  } from '../historyEvents';
  import { statusByCode } from '../stores/deviceStatuses.svelte';
  import HistoryKindChips from './HistoryKindChips.svelte';
  import HistoryKindIcon from './icons/HistoryKindIcon.svelte';

  type Variant = 'device' | 'cartridge' | 'group';

  interface Props {
    entries: HistoryEntryDto[];
    loading: boolean;
    loadError: boolean;
    /** Текст при loadError; по умолчанию — про карточку. */
    errorText?: string;
    /** D-13/D-03: выбирает пустые состояния, сноски и набор чипов. */
    variant?: Variant;
    /** Открытая группа (вариант group): на неё ссылка не рисуется, тексты иные. */
    groupId?: number;
    onNavigateToPlace?: (placeId: number) => Promise<void> | void;
    onNavigateToAct?: (actId: number) => Promise<void> | void;
    onNavigateToGroup?: (groupId: number) => Promise<void> | void;
    onNavigateToRequest?: (requestId: number) => Promise<void> | void;
    /** Ссылка на устройство в ленте группы; без него имя — обычный текст. */
    onNavigateToDevice?: (deviceId: number) => Promise<void> | void;
  }

  const {
    entries,
    loading,
    loadError,
    errorText = 'Не удалось загрузить историю. Проверьте подключение и откройте карточку снова.',
    variant = 'device',
    groupId,
    onNavigateToPlace,
    onNavigateToAct,
    onNavigateToGroup,
    onNavigateToRequest,
    onNavigateToDevice,
  }: Props = $props();

  function formatDate(utcSeconds: number): string {
    const d = new Date(utcSeconds * 1000);
    const day = String(d.getUTCDate()).padStart(2, '0');
    const month = String(d.getUTCMonth() + 1).padStart(2, '0');
    const year = d.getUTCFullYear();
    return `${day}.${month}.${year}`;
  }

  // ---- Чипы (D-21): локальное состояние, нигде не сохраняется ----
  let chipState = $state<Record<string, boolean>>({});

  const KNOWN_KINDS = ['move', 'membership', 'status'];

  function entryKinds(entry: HistoryEntryDto): string[] {
    return entry.segments.map((s) => (KNOWN_KINDS.includes(s.kind) ? s.kind : 'other'));
  }

  const presentKinds = $derived.by(() => {
    if (variant === 'cartridge') return [] as string[];
    const set = new Set<string>();
    for (const e of entries) {
      for (const k of entryKinds(e)) {
        if (k === 'other') continue;
        if (variant === 'group' && k === 'status') continue;
        set.add(k);
      }
    }
    return KNOWN_KINDS.filter((k) => set.has(k));
  });

  const showChips = $derived(presentKinds.length >= 2);

  function isVisible(entry: HistoryEntryDto): boolean {
    if (!showChips) return true;
    const kinds = entryKinds(entry);
    if (kinds.length === 0 || kinds.includes('other')) return true;
    return kinds.some((k) => chipState[k] !== false);
  }

  const visibleEntries = $derived(entries.filter(isVisible));

  // ---- Тексты ----
  const textCtx = $derived<HistoryTextCtx>({
    variant: variant === 'group' ? 'group' : 'device',
    currentGroupId: groupId,
    statusLabel: (code) => statusByCode(code)?.name,
  });

  function orderedSegments(entry: HistoryEntryDto): HistorySegmentDto[] {
    const rank = (s: HistorySegmentDto) =>
      s.kind === 'membership' ? 0 : s.kind === 'move' ? 1 : s.kind === 'status' ? 2 : 3;
    return entry.segments
      .map((s, i) => ({ s, i }))
      .sort((a, b) => rank(a.s) - rank(b.s) || a.i - b.i)
      .map((x) => x.s);
  }

  /** Текст сегмента не из перемещения: «Статус:» после первого сегмента — строчная. */
  function segmentText(seg: HistorySegmentDto, idx: number): string {
    const t = kindText(seg, textCtx);
    if (idx > 0 && seg.kind === 'status' && t.startsWith('Статус:')) {
      return `статус:${t.slice('Статус:'.length)}`;
    }
    return t;
  }

  interface Cand {
    label: string;
    type: 'group' | 'device';
    id: number;
  }

  function candidates(seg: HistorySegmentDto): Cand[] {
    const out: Cand[] = [];
    const push = (label: string | null, type: 'group' | 'device', id: number | null) => {
      const l = label?.trim() ?? '';
      if (l.length > 0 && id !== null) out.push({ label: l, type, id });
    };
    push(seg.from_group_label, 'group', seg.from_group_id);
    push(seg.to_group_label, 'group', seg.to_group_id);
    push(seg.subject_label, seg.subject_type === 'group' ? 'group' : 'device', seg.subject_id);
    return out;
  }

  interface Part {
    text: string;
    link?: Cand;
  }

  /** Режет текст по «…»; имя, совпавшее с кандидатом и имеющее обработчик, становится ссылкой. */
  function splitParts(text: string, cands: Cand[]): Part[] {
    const parts: Part[] = [];
    const re = /«([^»]*)»/g;
    let last = 0;
    let m: RegExpExecArray | null;
    while ((m = re.exec(text)) !== null) {
      const inner = m[1];
      // Неоднозначная подпись (несколько кандидатов) ссылкой не становится.
      const matches = cands.filter((c) => c.label === inner);
      const cand = matches.length === 1 ? matches[0] : undefined;
      const clickable =
        cand !== undefined &&
        (cand.type === 'group' ? !!onNavigateToGroup && cand.id !== groupId : !!onNavigateToDevice);
      parts.push({ text: text.slice(last, m.index) + '«' });
      parts.push({ text: inner, link: clickable ? cand : undefined });
      last = m.index + m[0].length - 1;
    }
    parts.push({ text: text.slice(last) });
    return parts;
  }

  function handleNavigateToAct(actId: number) {
    void onNavigateToAct?.(actId);
  }

  function handleNavigateToGroup(groupId: number) {
    void onNavigateToGroup?.(groupId);
  }

  function handleNavigateToRequest(requestId: number) {
    void onNavigateToRequest?.(requestId);
  }

  function followLink(c: Cand) {
    if (c.type === 'group') void onNavigateToGroup?.(c.id);
    else void onNavigateToDevice?.(c.id);
  }

  function moveOf(entry: HistoryEntryDto): MovementEntryDto | null {
    return entry.segments.find((s) => s.kind === 'move')?.movement ?? null;
  }

  // Подпись для основания из события: ссылочные части рисует шаблон.
  function eventReasonKnown(entry: HistoryEntryDto): boolean {
    return reasonLabel(entry) !== UNKNOWN_HISTORY_REASON;
  }

  // Хвост основания «актом №N» у строк пакета группового акта (как раньше).
  function actTail(entry: MovementEntryDto): 'group' | 'member' | null {
    if (entry.entity_type === 'group') return 'group';
    if (entry.group_id !== null && onNavigateToGroup && (entry.group_label?.trim() ?? '') !== '') {
      return 'member';
    }
    return null;
  }

  const footnotes = $derived.by((): string[] => {
    const initial =
      'Первичное размещение при поступлении в историю не попадает — она начинается с первого перемещения между двумя известными местами.';
    if (variant === 'cartridge') return [initial];
    if (variant === 'group') {
      return [
        'Переносы группы показаны за всё время. Вход и выход устройств, образование группы и вложение в другие группы программа записывает с обновления, в котором появилась эта лента; более ранних таких событий здесь нет.',
      ];
    }
    return [
      initial,
      'Перемещения показаны за всё время. Вход в группу, выход из неё и смену статуса программа записывает с обновления, в котором появилась эта лента; более ранних таких событий здесь нет.',
    ];
  });

  const emptyHeading = $derived(
    variant === 'cartridge' ? 'Перемещений ещё не было' : 'Событий ещё не было',
  );
  const emptyBody = $derived(
    variant === 'cartridge'
      ? 'Место изменится — здесь появится запись: откуда, куда, когда, кем и почему.'
      : variant === 'group'
        ? 'Здесь появятся переносы группы, вход и выход устройств, вложение в другие группы.'
        : 'Здесь появятся перемещения, вход в группу и выход из неё, смена статуса — с датой, автором и причиной.',
  );
</script>

{#snippet linked(text: string, cands: Cand[])}
  {#each splitParts(text, cands) as part, i (i)}
    {#if part.link}<button
        type="button"
        class="timeline-link"
        onclick={() => followLink(part.link as Cand)}>{part.text}</button
      >{:else}{part.text}{/if}
  {/each}
{/snippet}

{#snippet notes()}
  {#each footnotes as note, i (i)}
    <p class="timeline-empty-body timeline-footnote">{note}</p>
  {/each}
{/snippet}

{#if loading}
  <!-- Родитель рисует спиннер загрузки — здесь ничего. -->
{:else if loadError}
  <p class="timeline-error">
    {errorText}
  </p>
{:else if entries.length === 0}
  <div class="timeline-empty">
    <p class="timeline-empty-heading">{emptyHeading}</p>
    <p class="timeline-empty-body">{emptyBody}</p>
    {@render notes()}
  </div>
{:else}
  {#if showChips}
    <HistoryKindChips kinds={presentKinds} bind:selected={chipState} />
  {/if}
  {#if visibleEntries.length === 0}
    <div class="timeline-empty">
      <p class="timeline-empty-heading">Нет событий выбранных видов</p>
      <p class="timeline-empty-body">Включите хотя бы один фильтр выше.</p>
    </div>
  {:else}
    <ul class="timeline-list">
      {#each visibleEntries as row (row.key)}
        {@const entry = moveOf(row)}
        {@const segs = orderedSegments(row)}
        <li class="timeline-row">
          <HistoryKindIcon kind={entryIcon(row)} />
          <span class="timeline-date">{formatDate(row.created_at_utc)}</span>
          <span class="timeline-dash">—</span>
          <span class="timeline-event">
            {#if segs.length === 0}
              {UNKNOWN_HISTORY_EVENT}
            {:else}
              {#each segs as seg, idx (idx)}
                <span class="timeline-seg"
                  >{#if seg.kind === 'move' && seg.movement}{#if idx > 0}место:
                    {/if}<button
                      type="button"
                      class="timeline-link"
                      title={seg.movement.from_place_path}
                      onclick={() => void onNavigateToPlace?.(seg.movement!.from_place_id)}
                    >
                      {seg.movement.from_place_path_short ?? seg.movement.from_place_path}
                    </button>
                    <span class="timeline-arrow">→</span>
                    <button
                      type="button"
                      class="timeline-link"
                      title={seg.movement.to_place_path}
                      onclick={() => void onNavigateToPlace?.(seg.movement!.to_place_id)}
                    >
                      {seg.movement.to_place_path_short ?? seg.movement.to_place_path}
                    </button>{:else}{@render linked(
                      segmentText(seg, idx),
                      candidates(seg),
                    )}{/if}{#if idx < segs.length - 1};{/if}</span
                >
              {/each}
            {/if}
          </span>
          <span class="timeline-sep">·</span>
          <span class="timeline-actor">{row.actor_display}</span>
          <span class="timeline-sep">·</span>
          {#if row.reason_is_event}
            {#if row.reason_source === 'act' && row.act_id !== null && row.act_number}
              <span class="timeline-reason">
                актом №<button
                  type="button"
                  class="timeline-link"
                  onclick={() => handleNavigateToAct(row.act_id as number)}
                >
                  {row.act_number}
                </button>
              </span>
            {:else if row.reason_source === 'repair' && row.request_id !== null && onNavigateToRequest}
              <span class="timeline-reason">
                по ремонтной заявке №<button
                  type="button"
                  class="timeline-link"
                  onclick={() => handleNavigateToRequest(row.request_id as number)}
                >
                  {row.request_id}
                </button>
              </span>
            {:else if eventReasonKnown(row)}
              <span class="timeline-reason">
                {@render linked(
                  reasonLabel(row),
                  row.ref_group_id !== null && row.ref_group_label
                    ? [
                        {
                          label: row.ref_group_label.trim(),
                          type: 'group',
                          id: row.ref_group_id,
                        },
                      ]
                    : [],
                )}
              </span>
            {:else}
              <span class="timeline-reason">{UNKNOWN_HISTORY_REASON}</span>
            {/if}
          {:else if entry === null}
            <span class="timeline-reason">{UNKNOWN_HISTORY_REASON}</span>
          {:else if entry.source === 'act' && entry.act_id !== null && entry.act_number}
            <span class="timeline-reason">
              актом №<button
                type="button"
                class="timeline-link"
                onclick={() => handleNavigateToAct(entry.act_id as number)}
              >
                {entry.act_number}
              </button>
            </span>
            {#if actTail(entry) === 'group'}
              <span class="timeline-sep">·</span>
              <span class="timeline-reason">перенос группы</span>
            {:else if actTail(entry) === 'member'}
              <span class="timeline-sep">·</span>
              <span class="timeline-reason">
                в составе группы «<button
                  type="button"
                  class="timeline-link"
                  onclick={() => handleNavigateToGroup(entry.group_id as number)}
                >
                  {entry.group_label?.trim()}
                </button>»
              </span>
            {/if}
          {:else if entry.source === 'group' && entry.entity_type !== 'group' && entry.group_id !== null && onNavigateToGroup && (entry.group_label?.trim() ?? '').length > 0}
            <span class="timeline-reason">
              в составе группы «<button
                type="button"
                class="timeline-link"
                onclick={() => handleNavigateToGroup(entry.group_id as number)}
              >
                {entry.group_label?.trim()}
              </button>»
            </span>
          {:else if entry.source === 'repair' && entry.request_id !== null && onNavigateToRequest}
            <span class="timeline-reason">
              по ремонтной заявке №<button
                type="button"
                class="timeline-link"
                onclick={() => handleNavigateToRequest(entry.request_id as number)}
              >
                {entry.request_id}
              </button>
            </span>
            {#if entry.note && entry.note.trim().length > 0}
              <span class="timeline-sep">·</span>
              <span class="timeline-reason">{entry.note}</span>
            {/if}
          {:else}
            <span class="timeline-reason">{reasonText(entry)}</span>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
  {@render notes()}
{/if}

<style lang="scss">
  .timeline-error {
    margin: 0;
    padding: var(--tr-space-md);
    color: var(--tr-danger-text);
  }

  .timeline-empty {
    padding: var(--tr-space-md) 0;
  }

  .timeline-empty-heading {
    margin: 0 0 var(--tr-space-2xs) 0;
    font-size: var(--tr-font-size-body);
    color: var(--tr-text-tertiary);
    font-style: italic;
  }

  .timeline-footnote {
    margin-top: var(--tr-space-xs);
  }

  .timeline-empty-body {
    margin: 0;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-tertiary);
    font-style: italic;
  }

  .timeline-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .timeline-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--tr-space-2xs);
    min-height: var(--row-height-dense, 32px);
    padding: var(--tr-space-2xs) 0;
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-primary);
    border-bottom: 1px solid var(--tr-border);

    &:last-child {
      border-bottom: none;
    }
  }

  .timeline-row {
    overflow-wrap: anywhere;
  }

  .timeline-dash,
  .timeline-arrow,
  .timeline-sep {
    color: var(--tr-text-secondary);
  }

  .timeline-actor,
  .timeline-reason {
    color: var(--tr-text-primary);
  }

  // Clickable inline-text link contract (UI-SPEC "Clickable-link CSS
  // contract") — `PlaceContents.svelte`'s `.crumb` rule set with
  // `color: inherit` replaced by `var(--tr-accent-text)`, nothing else
  // changed. Reused for both place segments and the act-number segment
  // (exactly the two accent-colored elements per row, per UI-SPEC Color §).
  .timeline-link {
    padding: 0;
    background: none;
    border: none;
    font: inherit;
    color: var(--tr-accent-text);
    cursor: pointer;

    &:hover {
      text-decoration: underline;
    }

    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
      border-radius: var(--tr-radius-xs);
    }
  }
</style>
