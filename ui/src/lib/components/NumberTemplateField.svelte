<script lang="ts">
  // Phase 40.2 Plan 12 (NUM-06/07/08/09/10/11/12 — field-level slice):
  // единая точка автоподстановки инвентарного/акт/картриджного номера —
  // заменяет прежний специализированный компонент поля номера акта
  // (удалён планом 14) и обычные Input в
  // трёх других формах создания (планы 13-15 встраивают этот компонент).
  //
  // Anti-pattern guard (js_rust_mirror_needs_fixture_gate, RESEARCH.md):
  // этот компонент НИКОГДА не вычисляет следующий номер или соответствие
  // маске сам. Все числа приходят с сервера (number_templates_peek_next/
  // list_by_context); клиент делает только ЧИСТОЕ строковое сравнение
  // (trim + toLowerCase) для видимости стрелки ↑/↓ и для решения «правили
  // ли значение вручную» (D-14/D-19/NUM-07).
  import { onMount, untrack } from 'svelte';
  import Input from './Input.svelte';
  import ActionMenu from './ActionMenu.svelte';
  import IconInsertTemplate from './icons/IconInsertTemplate.svelte';
  import { apiCall } from '$lib/api/client';
  import { pushToast } from '$lib/stores/toast.svelte';
  import { connectWs, onWsEvent } from '$lib/api/ws';
  import type { WsEvent } from '../../bindings-phase6';
  import type {
    NextNumberDto,
    NumberTemplateDto,
    OccupyingRecordDto,
    TemplateContextDto,
  } from '../../bindings';

  interface Props {
    /** Какой из 5 попапов создания встраивает поле — определяет и RBAC-гейт
     *  серверных команд (`action_for_context`, план 05), и запомненный
     *  шаблон по умолчанию. */
    context: TemplateContextDto;
    value?: string;
    placeholder?: string;
    invalid?: boolean;
    errorMessage?: string | null;
    disabled?: boolean;
    /** Форма правки может передать id редактируемой записи, чтобы живая
     *  подсказка «занят» не находила саму себя (план 12 SPEC-заметка при
     *  Task 2 — используется только формами правки, план 12 сам поле не
     *  встраивает никуда, это готовность интерфейса для планов 13-15). */
    excludeId?: number | null;
    /** UI-SPEC §3 D-13: пустое меню «Вставка» показывает ссылку на
     *  «Настройки / Организация» только пользователям с этим правом —
     *  компонент не имеет доступа к Identity напрямую. */
    canManageSettings?: boolean;
    /** Task 2: автоподстановка запомненного шаблона при монтировании.
     *  По умолчанию true; форма-хост может отключить, если явно передаёт
     *  предзаполненное значение (например, дублирование записи) и не хочет,
     *  чтобы оно было перетёрто. */
    autofillOnMount?: boolean;
    /** Родитель должен знать активный шаблон для D-01/D-11 mismatch-проверки
     *  при сохранении — компонент вызывает это при каждом изменении
     *  выбранного шаблона (включая null — «Без шаблона»). Fix 40.2-13
     *  (NUM-11): второй аргумент — маска выбранного шаблона (или `null`)
     *  — родитель строит текст попапа «Не соответствует шаблону»
     *  (mask+contextLabel), не имея собственного доступа к списку шаблонов
     *  этого поля. */
    onSelectedTemplateChange?: (_templateId: number | null, _mask: string | null) => void;
  }

  let {
    context,
    value = $bindable(''),
    placeholder,
    invalid = false,
    errorMessage = null,
    disabled = false,
    excludeId = null,
    canManageSettings = false,
    autofillOnMount = true,
    onSelectedTemplateChange,
  }: Props = $props();

  // ---------------------------------------------------------------------
  // Список шаблонов своего контекста (для меню «Вставка»)
  // ---------------------------------------------------------------------
  let templates = $state<NumberTemplateDto[]>([]);
  let templatesLoading = $state(false);
  let templatesError = $state<string | null>(null);

  const sortedTemplates = $derived(
    [...templates].sort((a, b) => a.mask.localeCompare(b.mask, 'ru')),
  );

  // FE-CR-05: номер последнего запроса списка — ответ, пришедший после более
  // нового запроса (быстрая смена контекста, LAN-HTTP), отбрасывается, иначе
  // меню «Вставка» показало бы шаблоны чужого вида.
  let templatesSeq = 0;

  async function loadTemplates(
    ctx: TemplateContextDto,
    opts: { dropMissingSelection?: boolean } = {},
  ) {
    const seq = ++templatesSeq;
    templatesLoading = true;
    templatesError = null;
    try {
      const list = await apiCall<NumberTemplateDto[]>('number_templates_list_by_context', {
        context: ctx,
      });
      if (seq !== templatesSeq) return;
      templates = list;
      // FE-WR-06: выбранный шаблон удалён в другой сессии — не отправлять
      // несуществующий templateId при сохранении. Как «Без шаблона»:
      // непоправленное предложение уходит, ручной ввод остаётся.
      if (
        opts.dropMissingSelection &&
        selectedTemplateId !== null &&
        !list.some((t) => t.id === selectedTemplateId)
      ) {
        nextGeneration();
        clearSelection({ preserveManualEdits: true });
      }
    } catch {
      if (seq !== templatesSeq) return;
      templatesError = 'Не удалось загрузить шаблоны. Закройте меню и попробуйте ещё раз.';
      templates = [];
    } finally {
      if (seq === templatesSeq) templatesLoading = false;
    }
  }

  // ---------------------------------------------------------------------
  // Активный шаблон + два предложенных числа (первый свободный / max+1)
  // ---------------------------------------------------------------------
  let selectedTemplateId = $state<number | null>(null);
  let selectedOverflowed = $state(false);
  let lastSuggested = $state<string | null>(null);
  let altSuggested = $state<string | null>(null);
  let loadingNumber = $state(false);

  const selectedMask = $derived(templates.find((t) => t.id === selectedTemplateId)?.mask ?? null);

  // FE-CR-05 / FE-WR-04: «поколение» выбора шаблона. Растёт при смене
  // контекста и при каждом явном действии пользователя в меню «Вставка».
  // Каждая асинхронная подстановка запоминает поколение и контекст ДО
  // запроса и после ответа применяется, только если оба не изменились —
  // поздний ответ старого контекста (или автоподстановки при монтировании,
  // обогнанной явным выбором) не перезапишет ни значение, ни templateId.
  let selectionGen = 0;

  function nextGeneration(): number {
    // Устаревшая операция больше не трогает loadingNumber — сбрасываем его
    // здесь, новая операция выставит заново, когда начнёт свой запрос.
    loadingNumber = false;
    return ++selectionGen;
  }

  function isCurrent(gen: number, ctx: TemplateContextDto): boolean {
    return gen === selectionGen && ctx === context;
  }

  function normalize(s: string): string {
    return s.trim().toLowerCase();
  }

  /** FE-CR-04: пользователь сам стёр значение (последний ввод дал пустую
   *  строку). Стёртое поле — это ПРАВКА (D-14): ни WS-инвалидация, ни смена
   *  контекста в режиме D-14 не заполняют его заново. Любая программная
   *  подстановка/очистка сбрасывает флаг. */
  let clearedByUser = false;

  function setValueProgrammatically(next: string) {
    clearedByUser = false;
    value = next;
  }

  /** Значение НЕ правили вручную — пустое (и не стёртое пользователем) или
   *  совпадает с одним из двух последних серверных предложений (чистое
   *  string-equality, NUM-07). */
  function isValueUnedited(): boolean {
    const norm = normalize(value);
    if (norm === '') return !clearedByUser;
    if (lastSuggested !== null && norm === normalize(lastSuggested)) return true;
    if (altSuggested !== null && norm === normalize(altSuggested)) return true;
    return false;
  }

  function applyPeekResult(
    templateId: number,
    dto: NextNumberDto,
    opts: { preserveManualEdits: boolean },
  ) {
    selectedTemplateId = templateId;
    onSelectedTemplateChange?.(
      templateId,
      templates.find((t) => t.id === templateId)?.mask ?? null,
    );
    selectedOverflowed = dto.overflowed;
    if (dto.overflowed) {
      // Свободных номеров нет — ничего не подставляем, строка под полем
      // покажет «переполнен» (приоритет 2, UI-SPEC §2).
      lastSuggested = null;
      altSuggested = null;
      // FE-WR-02 (б): в режиме «заменять всегда» (картридж ↔ фотобарабан,
      // явный выбор) значение старого контекста/шаблона оставлять нельзя —
      // например, `C-00NN` в «Новом фотобарабане».
      if (!opts.preserveManualEdits) setValueProgrammatically('');
      return;
    }
    const shouldReplace = !opts.preserveManualEdits || isValueUnedited();
    // FE-WR-03: пользователь стрелкой выбрал «после максимального» (max+1) —
    // тихая замена D-14 сохраняет этот выбор и подставляет новое max+1, а не
    // возвращает первый свободный. Явный выбор в меню (preserveManualEdits =
    // false) по-прежнему начинает с первого свободного.
    const norm = normalize(value);
    const wasShowingAlt =
      altSuggested !== null &&
      norm === normalize(altSuggested) &&
      !(lastSuggested !== null && norm === normalize(lastSuggested));
    lastSuggested = dto.rendered;
    altSuggested = dto.altRendered;
    if (shouldReplace) {
      const keepAlt = opts.preserveManualEdits && wasShowingAlt && dto.altRendered !== null;
      setValueProgrammatically(
        keepAlt && dto.altRendered !== null ? dto.altRendered : dto.rendered,
      );
    }
  }

  function clearSelection(opts: { preserveManualEdits: boolean }) {
    const shouldClear = !opts.preserveManualEdits || isValueUnedited();
    selectedTemplateId = null;
    selectedOverflowed = false;
    onSelectedTemplateChange?.(null, null);
    lastSuggested = null;
    altSuggested = null;
    if (shouldClear) setValueProgrammatically('');
  }

  /** Читает запомненный контексту шаблон и подставляет его первое
   *  свободное число (mount + смена `context`, D-14/NUM-08). */
  async function applyContextDefault(
    ctx: TemplateContextDto,
    preserveManualEdits: boolean,
    gen: number,
  ) {
    let templateId: number | null;
    try {
      templateId = await apiCall<number | null>('number_template_contexts_get', {
        context: ctx,
      });
    } catch {
      if (!isCurrent(gen, ctx)) return;
      // FE-WR-02 (а): сбой чтения запомненного шаблона — сбрасываем выбор
      // ПРЕДЫДУЩЕГО контекста, иначе родитель продолжил бы отправлять чужой
      // templateId. Значение — по правилам текущего режима (D-14 / «заменять
      // всегда»). Строки под полем не будет (D-13).
      clearSelection({ preserveManualEdits });
      return;
    }
    if (!isCurrent(gen, ctx)) return;
    if (templateId === null) {
      clearSelection({ preserveManualEdits });
      return;
    }
    loadingNumber = true;
    try {
      const dto = await apiCall<NextNumberDto>('number_templates_peek_next', {
        templateId,
        context: ctx,
      });
      if (!isCurrent(gen, ctx)) return;
      applyPeekResult(templateId, dto, { preserveManualEdits });
    } catch {
      if (!isCurrent(gen, ctx)) return;
      pushToast(
        'error',
        'Не удалось получить следующий номер. Введите номер вручную или попробуйте ещё раз.',
      );
    } finally {
      if (isCurrent(gen, ctx)) loadingNumber = false;
    }
  }

  /** Только повторный запрос по уже выбранному шаблону — используется по
   *  WS-инвалидации (D-14), где выбор шаблона не меняется, меняется только
   *  число. Полностью тихо: без toast, без анимации. */
  // FE-CR-05: два WS-события подряд — ответ на первое не должен прийти
  // последним и откатить значение.
  let refreshSeq = 0;

  async function refreshCurrentSuggestion() {
    if (selectedTemplateId === null) return;
    // FE-CR-04: заблокированное поле (например, «Количество» > 1 в «Новом
    // устройстве») никогда не меняется тихо — иначе номер появился бы в
    // поле, которое пользователь не может даже отредактировать.
    if (disabled) return;
    const gen = selectionGen;
    const ctx = context;
    const templateId = selectedTemplateId;
    const seq = ++refreshSeq;
    try {
      const dto = await apiCall<NextNumberDto>('number_templates_peek_next', {
        templateId,
        context: ctx,
      });
      if (!isCurrent(gen, ctx) || seq !== refreshSeq || templateId !== selectedTemplateId) return;
      applyPeekResult(templateId, dto, { preserveManualEdits: true });
    } catch {
      // Тихая замена — при сбое просто оставляем текущее значение как есть.
    }
  }

  // Discretion #8 (UI-SPEC): смена контекста устройство↔принтер применяет
  // D-14 (заменяет, только если не правили вручную); картридж↔фотобарабан —
  // заменяет ВСЕГДА (коды двух видов живут в разных шаблонах, SPEC п.8).
  // Определяется чисто по строковому значению context — без второй копии
  // серверной связи "контекст → шаблон-тип" (js_rust_mirror_needs_fixture_gate
  // — здесь речь не о вычислении номера, а только о выборе стратегии
  // подстановки, поэтому это не то же самое запрещённое зеркалирование).
  function contextAlwaysReplaces(ctx: TemplateContextDto): boolean {
    return ctx === 'cartridge_create' || ctx === 'drum_create';
  }

  let didInit = false;
  let previousContext: TemplateContextDto | null = null;

  $effect(() => {
    const ctx = context;
    // Единственная зависимость эффекта — `context`; всё остальное (запуск
    // запросов, счётчики поколений, loadingNumber) — вне отслеживания.
    untrack(() => {
      void loadTemplates(ctx);

      if (!didInit) {
        didInit = true;
        previousContext = ctx;
        const gen = nextGeneration();
        if (autofillOnMount) void applyContextDefault(ctx, /* preserveManualEdits */ true, gen);
        return;
      }
      if (previousContext === ctx) return;
      previousContext = ctx;
      const gen = nextGeneration();
      void applyContextDefault(ctx, /* preserveManualEdits */ !contextAlwaysReplaces(ctx), gen);
    });
  });

  // ---------------------------------------------------------------------
  // WS-инвалидация (D-14, W6): чистое членство в event.contexts — никакого
  // сопоставления "тип шаблона → контекст" на клиенте.
  // ---------------------------------------------------------------------
  function handleWsEvent(event: WsEvent) {
    if (event.type === 'number_space_changed' && event.contexts.includes(context)) {
      void refreshCurrentSuggestion();
      occupiedRecheck += 1;
      // FE-WR-06: «→ следующий номер», флаги «переполнен» и сам список
      // шаблонов в меню «Вставка» тоже устаревают (сервер шлёт это событие
      // и на CRUD шаблонов, BE-CR-04).
      void loadTemplates(context, { dropMissingSelection: true });
    }
  }

  onMount(() => {
    let wsRelease: (() => void) | null = null;
    connectWs()
      .then((release) => {
        wsRelease = release;
      })
      .catch(() => {
        // WS необязателен — без него просто не будет тихой замены (D-14).
      });
    const unsubscribeWs = onWsEvent(handleWsEvent);

    return () => {
      unsubscribeWs();
      wsRelease?.();
    };
  });

  // ---------------------------------------------------------------------
  // Меню «Вставка» — выбор шаблона / «Без шаблона»
  // ---------------------------------------------------------------------
  const inputId = `ntf-input-${Math.random().toString(36).slice(2)}`;

  function focusInputAtEnd() {
    const el = document.getElementById(inputId) as HTMLInputElement | null;
    if (!el) return;
    el.focus();
    const len = el.value.length;
    try {
      el.setSelectionRange(len, len);
    } catch {
      // input[type=number] не поддерживает setSelectionRange в некоторых
      // браузерах — не критично, фокус уже установлен.
    }
  }

  // Phase 40.2 Plan 13 (minimal addition per its own <action> text): вызывающая
  // форма (`DeviceFormBody.svelte` и т.д.) должна вернуть фокус в поле номера
  // после закрытия попапа цепочки D-01 («Поправлю»/«Закрыть») — план 12 не
  // предусматривал явного метода фокуса, поэтому он экспортируется здесь,
  // переиспользуя тот же `focusInputAtEnd()`, что и меню «Вставка».
  export function focus() {
    focusInputAtEnd();
  }

  async function selectTemplate(t: NumberTemplateDto) {
    if (t.overflowed) return;
    // FE-WR-04: явный выбор делает недействительной любую ещё не пришедшую
    // автоподстановку (монтирование/смена контекста/WS).
    const gen = nextGeneration();
    const ctx = context;
    loadingNumber = true;
    try {
      const dto = await apiCall<NextNumberDto>('number_templates_peek_next', {
        templateId: t.id,
        context: ctx,
      });
      if (!isCurrent(gen, ctx)) return;
      // Явный выбор пользователя в меню — всегда подставляем (не D-14).
      applyPeekResult(t.id, dto, { preserveManualEdits: false });
    } catch {
      if (!isCurrent(gen, ctx)) return;
      pushToast(
        'error',
        'Не удалось получить следующий номер. Введите номер вручную или попробуйте ещё раз.',
      );
    } finally {
      if (isCurrent(gen, ctx)) loadingNumber = false;
    }
    if (isCurrent(gen, ctx)) focusInputAtEnd();
  }

  function selectNoTemplate() {
    nextGeneration();
    clearSelection({ preserveManualEdits: true });
    focusInputAtEnd();
  }

  function goToSettings() {
    // Discretion #6 (UI-SPEC): переход на блок шаблонов в «Настройки /
    // Организация». Закрытие текущего попапа создания и разбор `section` из
    // hash в SettingsPage — ответственность вызывающей формы/маршрутизации
    // (планы 13-15) и вне файловой области этого плана (files_modified —
    // только NumberTemplateField.svelte); здесь — только сама навигация.
    window.location.hash = '#/settings?section=org';
  }

  // ---------------------------------------------------------------------
  // Стрелка ↑/↓ (NUM-07, D-19) — чистое string-equality, никакого пересчёта
  // ---------------------------------------------------------------------
  const normalizedValue = $derived(normalize(value));
  const matchesLast = $derived(
    lastSuggested !== null && normalizedValue === normalize(lastSuggested),
  );
  const matchesAlt = $derived(altSuggested !== null && normalizedValue === normalize(altSuggested));
  const arrowVisible = $derived(altSuggested !== null && (matchesLast || matchesAlt));
  // Стрелка вверх — сейчас показан первый свободный (lastSuggested);
  // стрелка вниз — сейчас показан max+1 (altSuggested).
  const showingAlt = $derived(matchesAlt && !matchesLast);

  function toggleSuggested() {
    if (!arrowVisible) return;
    if (showingAlt) {
      if (lastSuggested !== null) setValueProgrammatically(lastSuggested);
    } else {
      if (altSuggested !== null) setValueProgrammatically(altSuggested);
    }
  }

  const arrowTitle = $derived(
    showingAlt
      ? `Первый свободный: ${lastSuggested ?? ''}`
      : `Следующий после максимального: ${altSuggested ?? ''}`,
  );

  // ---------------------------------------------------------------------
  // Живая подсказка «занят» (D-02/D-03) — 400 мс debounce, устаревшие ответы
  // отбрасываются по номеру запроса.
  // ---------------------------------------------------------------------
  // FE-WR-05: результат проверки хранится вместе с проверенным значением —
  // подсказка видна, только пока поле всё ещё содержит именно его (после
  // очистки или правки строка о прежнем значении сразу пропадает).
  let occupied = $state<{ candidate: string; record: OccupyingRecordDto } | null>(null);
  const occupiedRecord = $derived(
    occupied !== null && occupied.candidate === value.trim() ? occupied.record : null,
  );
  let occupiedRequestId = 0;
  // FE-WR-05: WS-инвалидация (D-14) перепроверяет «занят» для того же значения.
  let occupiedRecheck = $state(0);

  const KIND_LABEL_LOWER: Record<string, string> = {
    device: 'устройство',
    printer: 'принтер',
    cartridge: 'картридж',
    drum: 'фотобарабан',
    act: 'акт',
  };

  function occupiedLineText(record: OccupyingRecordDto): string {
    if (record.kind === 'act') {
      // OccupyingRecordDto не несёт даты для акта (плоская структура,
      // намеренно — см. doc-комментарий OccupyingRecordDto в
      // dto/number_template.rs). Точный текст Copywriting Contract «Занят:
      // акт от {дата}» нельзя собрать без даты — используем record.title
      // («Акт передачи»/«Акт возврата»), тот же тип адаптации, что и в
      // плане 11 (NumberTakenPopup) для того же плоского DTO.
      return `Занят: ${record.title.toLowerCase()}`;
    }
    const kindLabel = KIND_LABEL_LOWER[record.kind] ?? record.kind;
    return `Занят: ${kindLabel} «${record.title}»`;
  }

  $effect(() => {
    const candidate = value.trim();
    const ctx = context;
    const exclude = excludeId ?? null;
    void occupiedRecheck;
    // Любое изменение (включая очистку поля) делает недействительным уже
    // отправленный запрос — его ответ не запишется.
    const requestId = ++occupiedRequestId;
    if (candidate === '') return;
    const timer = setTimeout(() => {
      apiCall<OccupyingRecordDto | null>('number_templates_is_occupied', {
        context: ctx,
        candidate,
        excludeId: exclude,
      })
        .then((rec) => {
          if (requestId !== occupiedRequestId) return; // устаревший ответ
          occupied = rec ? { candidate, record: rec } : null;
        })
        .catch(() => {
          if (requestId !== occupiedRequestId) return;
          occupied = null;
        });
    }, 400);
    return () => clearTimeout(timer);
  });

  // ---------------------------------------------------------------------
  // Строка под полем — приоритет по убыванию (UI-SPEC §2)
  // ---------------------------------------------------------------------
  interface StatusLine {
    text: string;
    tone: 'danger' | 'warning' | 'tertiary';
    mono?: boolean;
  }

  const statusLine = $derived.by((): StatusLine | null => {
    if (errorMessage) return { text: errorMessage, tone: 'danger' };
    if (selectedOverflowed && selectedMask !== null) {
      return {
        text: `Шаблон ${selectedMask} переполнен — свободных номеров нет. Выберите другой шаблон или введите номер вручную.`,
        tone: 'warning',
      };
    }
    if (occupiedRecord) {
      return { text: occupiedLineText(occupiedRecord), tone: 'warning' };
    }
    if (selectedTemplateId !== null && selectedMask !== null) {
      return { text: `Шаблон: ${selectedMask}`, tone: 'tertiary', mono: true };
    }
    return null;
  });

  const describedById = `${inputId}-status`;
</script>

<div class="ntf">
  <div class="ntf-row">
    <div class="ntf-input-wrap" class:has-arrow={arrowVisible}>
      <Input
        id={inputId}
        {value}
        placeholder={loadingNumber ? 'Загрузка…' : placeholder}
        {disabled}
        {invalid}
        aria-describedby={statusLine ? describedById : undefined}
        oninput={(v) => {
          clearedByUser = v.trim() === '';
          value = v;
        }}
      />
      {#if arrowVisible}
        <button
          type="button"
          class="ntf-arrow"
          {disabled}
          aria-label={arrowTitle}
          title={arrowTitle}
          onclick={toggleSuggested}
        >
          {#if showingAlt}
            <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
              <path
                d="M4 6l4 4 4-4"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          {:else}
            <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
              <path
                d="M4 10l4-4 4 4"
                fill="none"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          {/if}
        </button>
      {/if}
    </div>

    <ActionMenu
      variant="ghost-md"
      portal
      panelMinWidth="280px"
      label="Вставить номер по шаблону"
      {disabled}
    >
      {#snippet icon()}
        <IconInsertTemplate size={20} />
      {/snippet}
      {#if templatesLoading && templates.length === 0}
        <div class="ntf-menu-loading">…</div>
      {:else if templatesError}
        <div class="ntf-menu-empty">{templatesError}</div>
      {:else if sortedTemplates.length === 0}
        <div class="ntf-menu-empty">
          {#if canManageSettings}
            Шаблонов нет — создайте в
            <button type="button" role="menuitem" class="ntf-menu-link" onclick={goToSettings}>
              Настройки / Организация
            </button>
          {:else}
            Шаблонов нет — создайте в Настройки / Организация
          {/if}
        </div>
      {:else}
        {#each sortedTemplates as t (t.id)}
          <button
            type="button"
            role="menuitem"
            class="ntf-menu-item"
            class:overflowed={t.overflowed}
            aria-disabled={t.overflowed || undefined}
            aria-label={`${t.mask}, следующий номер ${t.overflowed ? 'переполнен' : t.nextFirstFree}${
              t.id === selectedTemplateId ? ', выбран' : ''
            }`}
            onclick={(e) => {
              if (t.overflowed) {
                e.stopPropagation();
                return;
              }
              void selectTemplate(t);
            }}
          >
            <span class="ntf-check" aria-hidden="true"
              >{t.id === selectedTemplateId ? '✓' : ''}</span
            >
            <span class="ntf-mask tr-mono" title={t.mask}>{t.mask}</span>
            <span class="ntf-next tr-mono">
              {t.overflowed ? 'переполнен' : `→ ${t.nextFirstFree}`}
            </span>
          </button>
        {/each}
      {/if}
      <div role="separator" class="ntf-menu-separator"></div>
      <button
        type="button"
        role="menuitem"
        class="ntf-menu-item"
        aria-label={selectedTemplateId === null ? 'Без шаблона, выбран' : 'Без шаблона'}
        onclick={selectNoTemplate}
      >
        <span class="ntf-check" aria-hidden="true">{selectedTemplateId === null ? '✓' : ''}</span>
        <span class="ntf-mask">Без шаблона</span>
      </button>
    </ActionMenu>
  </div>

  {#if statusLine}
    <p
      id={describedById}
      class="ntf-status"
      class:danger={statusLine.tone === 'danger'}
      class:warning={statusLine.tone === 'warning'}
      class:tertiary={statusLine.tone === 'tertiary'}
      class:mono={statusLine.mono}
      title={statusLine.text}
      aria-live={statusLine.tone === 'warning' ? 'polite' : undefined}
    >
      {statusLine.text}
    </p>
  {/if}
</div>

<style lang="scss">
  .ntf {
    display: flex;
    flex-direction: column;
    gap: var(--tr-space-2xs);
  }

  .ntf-row {
    display: flex;
    align-items: center;
    gap: var(--tr-space-xs);
  }

  .ntf-input-wrap {
    position: relative;
    flex: 1 1 auto;
    min-width: 0;

    // Override Input.svelte's own padding-right when the ↑/↓ arrow is
    // shown — `:global()` reaches past Svelte's per-component style scoping
    // to the literal `.input` class rendered by the child component
    // (established pattern: PlaceTree.svelte `:global(.input-wrap)`).
    &.has-arrow :global(.input) {
      padding-right: 36px;
    }
  }

  .ntf-arrow {
    position: absolute;
    right: 4px;
    top: 50%;
    transform: translateY(-50%);
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: var(--tr-radius-xs);
    color: var(--tr-text-secondary);
    cursor: pointer;

    &:hover:not(:disabled) {
      color: var(--tr-text-primary);
    }
    &:focus-visible {
      outline: none;
      box-shadow: 0 0 0 3px var(--tr-focus-ring);
    }
    &:disabled {
      opacity: 0.45;
      cursor: not-allowed;
    }
  }

  .ntf-menu-loading,
  .ntf-menu-empty {
    padding: var(--tr-space-xs) var(--tr-space-sm);
    font-size: var(--tr-font-size-label);
    color: var(--tr-text-secondary);
    white-space: normal;
  }

  .ntf-menu-link {
    display: inline;
    padding: 0;
    background: transparent;
    border: none;
    color: var(--tr-accent-text);
    text-decoration: underline;
    cursor: pointer;
    font: inherit;
  }

  .ntf-menu-item {
    display: grid;
    grid-template-columns: 16px 1fr auto;
    align-items: center;
    gap: var(--tr-space-xs);
    padding: var(--tr-space-xs) var(--tr-space-sm);
    white-space: nowrap;

    &.overflowed {
      color: var(--tr-text-disabled);
      cursor: not-allowed;

      &:hover {
        background: transparent;
      }
    }
  }

  .ntf-check {
    color: var(--tr-accent-text);
    font-size: 13px;
    line-height: 1;
  }

  .ntf-mask {
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--tr-text-primary);
  }

  .overflowed .ntf-mask {
    color: var(--tr-text-disabled);
  }

  .ntf-next {
    color: var(--tr-text-tertiary);
  }

  .ntf-menu-separator {
    height: 1px;
    margin: var(--tr-space-2xs) 0;
    background: var(--tr-border);
  }

  .ntf-status {
    margin: 0;
    font-size: var(--tr-font-size-label);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;

    &.danger {
      color: var(--tr-danger-text);
    }
    &.warning {
      color: var(--tr-warning-text);
    }
    &.tertiary {
      color: var(--tr-text-tertiary);
    }
    &.mono {
      font-family: var(--tr-font-mono);
    }
  }
</style>
