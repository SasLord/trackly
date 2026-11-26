<script lang="ts">
  // Вкладка «Чат» (D-07, D-10, D-14, D-15, D-18, D-23; UI-SPEC §6.4, §8, §10.6, §17).
  // Лента прокручивается внутри себя (корень height:100% + min-height:0), поле
  // ввода стоит внизу. Один AttachmentViewer — соседним элементом, не в пузыре.
  import { tick, untrack } from 'svelte';
  import Button from '@trackly/kit/components/Button.svelte';
  import Spinner from '@trackly/kit/components/Spinner.svelte';
  import { onWsEvent, onWsResync } from '@trackly/transport/ws';
  import { parseAppError } from '@trackly/transport/errors';
  import { pushToast } from '@trackly/kit/stores/toast.svelte';
  import { saveFile } from '@trackly/kit/utils/saveFile';
  import AttachmentViewer from '@trackly/attachments/AttachmentViewer.svelte';
  import { attachments, fetchAttachmentBytes } from '@trackly/attachments/api';
  import type { AttachmentDto, ChatFeedDto, ChatItemDto, ChatMessageDto } from '../../../bindings';
  import ChatAttachmentEvent from './ChatAttachmentEvent.svelte';
  import ChatComposer from './ChatComposer.svelte';
  import ChatDayDivider from './ChatDayDivider.svelte';
  import ChatEvent from './ChatEvent.svelte';
  import ChatMessage from './ChatMessage.svelte';
  import ChatPendingMessage from './ChatPendingMessage.svelte';
  import { dayKey, dayLabel, sameGroup } from './chatDates';
  import { chat } from './api';
  import { clearDraft, draft, useDraftFor } from './draftStore.svelte';
  import { pendingFor, retryMessage } from './pendingStore.svelte';
  import { rejectText } from './sendOutcomeText';
  import { dismissChatToast } from './chatNotifications.svelte';
  import { setActiveChat } from './unreadStore.svelte';

  interface Props {
    requestId: number;
    isAdmin: boolean;
  }

  const { requestId, isAdmin }: Props = $props();

  type Row =
    | { type: 'day'; key: string; label: string }
    | { type: 'new'; key: string }
    | { type: 'item'; key: string; item: ChatItemDto; groupStart: boolean };

  let feed = $state<ChatFeedDto | null>(null);
  let requestFiles = $state<AttachmentDto[]>([]);
  let loading = $state(true);
  let loadError = $state<'network' | 'forbidden' | null>(null);
  let firstUnreadId = $state<string | null>(null);
  let liveReady = $state(false);
  let showJump = $state(false);
  let dragging = $state(false);
  let dragDepth = 0;

  let feedEl = $state<globalThis.HTMLOListElement | null>(null);
  let composer = $state<ReturnType<typeof ChatComposer> | null>(null);

  let loadToken = 0;
  // «Следовать за низом» (R3-2): не реактивно — меняется только в обработчиках событий и в
  // scrollToBottom/revealNewDivider; в эффектах не участвует, поэтому петли рун нет.
  let followBottom = true;
  let lastScrollTop = 0;
  let generation = 0;

  const canPost = $derived(feed?.canPost ?? false);
  const maxBytes = $derived(feed?.maxSizeBytes ?? 0);
  const items = $derived(feed?.items ?? []);
  const hasMessages = $derived(items.some((i) => i.kind === 'message'));

  const rows = $derived.by<Row[]>(() => {
    const out: Row[] = [];
    const nowSeconds = Math.floor(Date.now() / 1000);
    let prevDay: string | null = null;
    let prev: ChatItemDto | null = null;
    let sinceBreak = true;
    for (const item of items) {
      const day = dayKey(item.createdAtUtc);
      if (day !== prevDay) {
        out.push({ type: 'day', key: `d:${day}`, label: dayLabel(item.createdAtUtc, nowSeconds) });
        prevDay = day;
        sinceBreak = true;
      }
      if (firstUnreadId !== null && item.key === firstUnreadId) {
        out.push({ type: 'new', key: 'new' });
        sinceBreak = true;
      }
      let groupStart = true;
      if (item.kind === 'message' && prev !== null && prev.kind === 'message' && !sinceBreak) {
        groupStart = !(
          prev.actorUserId === item.actorUserId &&
          prev.isOwn === item.isOwn &&
          sameGroup(prev.createdAtUtc, item.createdAtUtc)
        );
      }
      out.push({ type: 'item', key: item.key, item, groupStart });
      sinceBreak = false;
      prev = item;
    }
    return out;
  });

  function validFeed(res: unknown): res is ChatFeedDto {
    return !!res && Array.isArray((res as ChatFeedDto).items);
  }

  function nearBottom(): boolean {
    const el = feedEl;
    if (!el) return true;
    return el.scrollHeight - el.scrollTop - el.clientHeight <= 80;
  }

  function reducedMotion(): boolean {
    return (
      typeof window !== 'undefined' &&
      typeof window.matchMedia === 'function' &&
      window.matchMedia('(prefers-reduced-motion: reduce)').matches
    );
  }

  function scrollToBottom(smooth: boolean) {
    const el = feedEl;
    if (!el) return;
    if (smooth && !reducedMotion()) el.scrollTo({ top: el.scrollHeight, behavior: 'smooth' });
    else el.scrollTop = el.scrollHeight;
    followBottom = true;
    showJump = false;
  }

  function onFeedScroll() {
    const el = feedEl;
    if (el) {
      // Прокрутка пользователя вверх снимает слежение; возврат к низу — возвращает.
      if (el.scrollTop < lastScrollTop - 2 && !nearBottom()) followBottom = false;
      else if (nearBottom()) followBottom = true;
      lastScrollTop = el.scrollTop;
    }
    if (showJump && nearBottom()) showJump = false;
  }

  // Высота картинок неизвестна до загрузки превью (load) или замены на иконку (error): после
  // догрузки лента остаётся прижатой к низу, если пользователь следует за низом (R3-2).
  // Прокрутка — после tick(): обработчик фазы захвата срабатывает раньше обновления DOM.
  function onFeedMediaSettled(e: Event) {
    if (!(e.target instanceof globalThis.HTMLImageElement)) return;
    void tick().then(() => {
      if (followBottom) scrollToBottom(false);
    });
  }

  async function markRead() {
    try {
      await chat.markRead(requestId);
    } catch {
      // Отметка прочтения не критична: повторится при следующем поступлении.
    }
  }

  async function loadFiles(gen: number) {
    try {
      const res = await attachments.list('request', requestId);
      if (gen === generation && res && Array.isArray(res.items)) {
        requestFiles = [...res.items, ...(Array.isArray(res.deleted) ? res.deleted : [])];
      }
    } catch {
      // Список файлов нужен только для просмотра; лента от него не зависит.
    }
  }

  async function load(initial: boolean) {
    const gen = generation;
    const token = ++loadToken;
    if (initial) {
      loading = true;
      loadError = null;
    }
    try {
      const res = await chat.feed(requestId);
      if (token !== loadToken || gen !== generation) return;
      if (!validFeed(res)) throw new Error('bad feed shape');
      const prevKeys = new Set(feed?.items.map((i) => i.key) ?? []);
      const wasNear = nearBottom();
      // firstUnreadId берётся из ПЕРВОГО ответа, до отметки прочтения (D-18).
      if (initial) firstUnreadId = res.firstUnreadId ?? null;
      feed = res;
      loadError = null;
      loading = false;
      void loadFiles(gen);
      void markRead();
      await tick();
      if (token !== loadToken || gen !== generation) return;
      if (initial) {
        scrollToBottom(false);
        revealNewDivider();
        liveReady = true;
      } else {
        const fresh = res.items.filter((i) => !prevKeys.has(i.key));
        if (fresh.length > 0) {
          const ownOnly = fresh.every((i) => i.kind === 'message' && i.isOwn);
          if (wasNear || ownOnly) scrollToBottom(true);
          else showJump = true;
        }
      }
    } catch (e) {
      if (token !== loadToken || gen !== generation) return;
      loading = false;
      if (initial || feed === null) {
        const code = parseAppError(e).code;
        loadError = code === 'FORBIDDEN' || code === 'Forbidden' ? 'forbidden' : 'network';
      }
    }
  }

  // Если первое непрочитанное выше видимой области — «Новые» у верхнего края.
  function revealNewDivider() {
    const el = feedEl;
    if (!el || firstUnreadId === null) return;
    const mark = el.querySelector<HTMLElement>('[data-new-divider]');
    if (!mark) return;
    const top = mark.getBoundingClientRect().top - el.getBoundingClientRect().top;
    if (top < 0) {
      followBottom = false;
      el.scrollTop += top - 8;
    }
  }

  // Смена заявки: сбросить состояние, загрузить ленту, отметить активный чат.
  $effect(() => {
    const id = requestId;
    untrack(() => {
      generation += 1;
      feed = null;
      requestFiles = [];
      firstUnreadId = null;
      liveReady = false;
      showJump = false;
      dragging = false;
      dragDepth = 0;
      useDraftFor(id);
      setActiveChat(id);
      dismissChatToast(id);
      void load(true);
    });
    return () => {
      setActiveChat(null);
    };
  });

  // WS: перечитываем ленту целиком, ключи уникальны — дубликатов нет (D-23).
  $effect(() => {
    const off = onWsEvent((ev) => {
      if (
        (ev.type === 'chat_message' && ev.requestId === requestId) ||
        (ev.type === 'request_status_changed' && ev.requestId === requestId) ||
        (ev.type === 'attachments_changed' &&
          ev.parentKind === 'request' &&
          ev.parentId === requestId)
      ) {
        void load(false);
      }
    });
    // D-08/D-10: ресинк перечитывает ленту; черновик и компонент не трогаются.
    const offResync = onWsResync(() => {
      void load(false);
    });
    return () => {
      off();
      offResync();
    };
  });

  // Закрытая заявка: черновик пропадает без отправки (D-07).
  $effect(() => {
    // Единственная зависимость — feed/canPost; clearDraft читает и пишет draft.chips,
    // без untrack эффект подписался бы на них и перезапускался бесконечно.
    if (feed !== null && !feed.canPost) untrack(() => clearDraft());
  });

  // Фокус в поле при открытии вкладки, если писать можно.
  $effect(() => {
    if (canPost && composer) untrack(() => composer?.focus());
  });

  function onSent(message: ChatMessageDto) {
    if (feed && !feed.items.some((i) => i.key === message.key)) {
      feed = { ...feed, items: [...feed.items, { kind: 'message', ...message }] };
    }
    void tick().then(() => scrollToBottom(true));
  }

  const pendingList = $derived(pendingFor(requestId));

  // Прокрутка вниз при появлении/смене неотправленных (как для своих сообщений).
  $effect(() => {
    void pendingList.length;
    untrack(() => void tick().then(() => scrollToBottom(true)));
  });

  async function onRetry(clientMsgId: string) {
    const res = await retryMessage(clientMsgId);
    if (res === null) return;
    if (res.kind === 'sent') {
      onSent(res.message);
    } else if (res.kind === 'rejected') {
      draft.errors = [...draft.errors, rejectText(res.code)];
      if (res.code === 'closed') onClosed();
    } else if (res.kind === 'error') {
      const code = parseAppError(res.error).code;
      const forbidden = code === 'FORBIDDEN' || code === 'Forbidden' || code === 'UNAUTHORIZED';
      draft.errors = [...draft.errors, rejectText(forbidden ? 'forbidden' : null)];
    }
  }

  function onClosed() {
    clearDraft();
    void load(false);
  }

  // --- перетаскивание файлов -------------------------------------------

  function hasFiles(e: globalThis.DragEvent): boolean {
    return Array.from(e.dataTransfer?.types ?? []).includes('Files');
  }

  function onDragEnter(e: globalThis.DragEvent) {
    if (!canPost || !hasFiles(e)) return;
    e.preventDefault();
    dragDepth += 1;
    dragging = true;
  }

  function onDragOver(e: globalThis.DragEvent) {
    if (!canPost || !hasFiles(e)) return;
    e.preventDefault();
    dragging = true;
  }

  function onDragLeave(e: globalThis.DragEvent) {
    if (!canPost || !hasFiles(e)) return;
    dragDepth = Math.max(0, dragDepth - 1);
    if (dragDepth === 0) dragging = false;
  }

  function onDrop(e: globalThis.DragEvent) {
    if (!hasFiles(e)) return;
    e.preventDefault();
    dragDepth = 0;
    dragging = false;
    if (!canPost) return;
    const files: globalThis.File[] = [];
    const folders: string[] = [];
    for (const it of Array.from(e.dataTransfer?.items ?? [])) {
      if (it.kind !== 'file') continue;
      const entry = typeof it.webkitGetAsEntry === 'function' ? it.webkitGetAsEntry() : null;
      if (entry?.isDirectory) {
        folders.push(entry.name);
        continue;
      }
      const f = it.getAsFile();
      if (f) files.push(f);
    }
    composer?.addFiles(files, folders);
  }

  // --- просмотр и скачивание -------------------------------------------

  let viewerOpen = $state(false);
  let viewerStartId = $state<number | null>(null);
  // Источник набора: файлы одного сообщения или один файл из события вложения.
  let viewerSource = $state<{ messageKey: string } | { single: number } | null>(null);

  function visibleFile(a: AttachmentDto): boolean {
    return a.viewable && !a.missing && (a.deletedAtUtc === null || isAdmin);
  }

  const viewerItems = $derived.by<AttachmentDto[]>(() => {
    const src = viewerSource;
    if (src === null) return [];
    if ('single' in src) {
      return requestFiles.filter((a) => a.id === src.single && visibleFile(a));
    }
    const msg = items.find((i) => i.key === src.messageKey);
    if (!msg || msg.kind !== 'message') return [];
    return msg.attachments.filter(visibleFile);
  });

  async function download(a: AttachmentDto) {
    try {
      const buf = await fetchAttachmentBytes(a.id);
      await saveFile(new Uint8Array(buf), a.originalName, a.mime);
    } catch {
      pushToast('error', 'Не удалось скачать файл. Попробуйте ещё раз.');
    }
  }

  function openFromMessage(messageKey: string, a: AttachmentDto) {
    if (a.missing) {
      pushToast('error', 'Файл не найден на диске.');
      return;
    }
    viewerSource = { messageKey };
    viewerStartId = a.id;
    viewerOpen = true;
  }

  function openFromEvent(a: AttachmentDto) {
    if (a.missing) {
      pushToast('error', 'Файл не найден на диске.');
      return;
    }
    if (a.viewable) {
      viewerSource = { single: a.id };
      viewerStartId = a.id;
      viewerOpen = true;
    } else {
      void download(a);
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="chat"
  ondragenter={onDragEnter}
  ondragover={onDragOver}
  ondragleave={onDragLeave}
  ondrop={onDrop}
>
  <div class="feed-wrap">
    {#if loadError !== null}
      <div class="state">
        {#if loadError === 'forbidden'}
          <p class="state-text">Нет доступа к переписке этой заявки.</p>
        {:else}
          <p class="state-text">Не удалось загрузить переписку.</p>
          <Button variant="secondary" size="sm" onclick={() => void load(true)}>
            Загрузить снова
          </Button>
        {/if}
      </div>
    {:else if loading}
      <div class="state">
        <Spinner size="md" />
        <p class="state-text">Загрузка переписки…</p>
      </div>
    {:else}
      <ol
        class="chat-feed"
        role="log"
        aria-live={liveReady ? 'polite' : 'off'}
        aria-relevant="additions"
        bind:this={feedEl}
        onscroll={onFeedScroll}
        onloadcapture={onFeedMediaSettled}
        onerrorcapture={onFeedMediaSettled}
      >
        {#each rows as row (row.key)}
          {#if row.type === 'day'}
            <ChatDayDivider label={row.label} />
          {:else if row.type === 'new'}
            <li class="new-divider" role="separator" data-new-divider>
              <span class="new-label">Новые</span>
            </li>
          {:else if row.item.kind === 'message'}
            {@const msgKey = row.item.key}
            <ChatMessage
              item={row.item}
              groupStart={row.groupStart}
              onOpenAttachment={(a) => openFromMessage(msgKey, a)}
              onDownloadAttachment={(a) => void download(a)}
            />
          {:else if row.item.kind === 'event'}
            <ChatEvent item={row.item} resolutionNotes={feed?.resolutionNotes ?? null} />
          {:else}
            {@const attId = row.item.event.attachmentId}
            <ChatAttachmentEvent
              item={row.item}
              attachment={requestFiles.find((a) => a.id === attId) ?? null}
              onOpen={openFromEvent}
            />
          {/if}
        {/each}
        {#each pendingList as p (p.clientMsgId)}
          <ChatPendingMessage item={p} {onRetry} />
        {/each}
        {#if !hasMessages && pendingList.length === 0}
          <li class="empty">
            <p class="empty-title">Сообщений пока нет</p>
            {#if canPost}
              <p class="empty-hint">
                Напишите, если нужно что-то уточнить по заявке. Можно приложить фото или файл.
              </p>
            {/if}
          </li>
        {/if}
      </ol>
      {#if showJump}
        <div class="jump">
          <Button variant="secondary" size="sm" onclick={() => scrollToBottom(true)}>
            <svg
              width="14"
              height="14"
              viewBox="0 0 16 16"
              fill="none"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
              stroke-linejoin="round"
              aria-hidden="true"
            >
              <path d="M8 3v10M3.5 8.5L8 13l4.5-4.5" />
            </svg>
            К новым
          </Button>
        </div>
      {/if}
    {/if}
  </div>

  <ChatComposer
    bind:this={composer}
    {requestId}
    {canPost}
    {maxBytes}
    loaded={feed !== null}
    {onSent}
    {onClosed}
  />

  {#if dragging && canPost}
    <div class="drop-overlay" aria-hidden="true">
      <span class="drop-text">Отпустите, чтобы прикрепить к сообщению</span>
    </div>
  {/if}
</div>

<AttachmentViewer
  items={viewerItems}
  startId={viewerStartId}
  open={viewerOpen}
  onClose={() => (viewerOpen = false)}
/>

<style lang="scss">
  .chat {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .feed-wrap {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--tr-bg);
  }

  .chat-feed {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    margin: 0;
    padding: var(--tr-space-sm) var(--tr-space-xl);
    list-style: none;
    display: flex;
    flex-direction: column;
    // Зазор строк ленты; ChatMessage гасит его внутри группы (margin-top continuation).
    --chat-row-gap: var(--tr-space-md);
    gap: var(--chat-row-gap);
    white-space: normal;
    text-align: left;
  }

  .state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--tr-space-sm);
    color: var(--tr-text-secondary);
  }

  .state-text {
    margin: 0;
    color: var(--tr-text-secondary);
    font-size: var(--tr-font-size-body);
  }

  .new-divider {
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    height: 16px;
    list-style: none;

    &::before {
      content: '';
      position: absolute;
      left: 0;
      right: 0;
      top: 50%;
      height: 1px;
      background: var(--tr-accent);
    }
  }

  .new-label {
    position: relative;
    padding: 0 var(--tr-space-sm);
    background: var(--tr-bg);
    color: var(--tr-accent-text);
    font-size: var(--tr-font-size-caption);
    font-weight: var(--tr-font-weight-medium);
  }

  .empty {
    list-style: none;
    margin-top: var(--tr-space-xl);
    text-align: center;
  }

  .empty-title {
    margin: 0 0 var(--tr-space-2xs);
    color: var(--tr-text-primary);
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-semibold);
  }

  .empty-hint {
    margin: 0;
    color: var(--tr-text-secondary);
    font-size: var(--tr-font-size-label);
  }

  .jump {
    position: absolute;
    right: var(--tr-space-xl);
    bottom: var(--tr-space-sm);
    box-shadow: var(--tr-elev-2);
    border-radius: var(--tr-radius-sm);
  }

  .drop-overlay {
    position: absolute;
    inset: 0;
    z-index: 5;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 2px dashed var(--tr-accent);
    background: var(--tr-accent-soft);
    pointer-events: none;
  }

  .drop-text {
    color: var(--tr-accent-text);
    font-size: var(--tr-font-size-body);
    font-weight: var(--tr-font-weight-semibold);
  }
</style>
