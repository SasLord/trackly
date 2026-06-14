---
phase: 6
slug: snmp
status: draft
shadcn_initialized: false
preset: none
created: 2026-06-14
---

# Phase 6 — UI Design Contract: Принтеры (SNMP-мониторинг) и Заявки

> Визуальный и интеракционный контракт для Phase 6.
> Сгенерирован gsd-ui-researcher, верифицируется gsd-ui-checker.
> Два под-домена: (А) Принтеры — SNMP-мониторинг и discovery; (Б) Заявки — портал сотрудника + борд специалиста.

**Критический инвариант фазы:** один Svelte-бандл работает и в Tauri webview, и в браузере LAN (UI-05). Портал заявок (`/requests`) запускается сотрудником в обычном браузере — все компоненты заявок ОБЯЗАНЫ корректно работать без Tauri API (через `apiCall()` dual-transport). WebSocket-уведомления: браузер → axum WS по session-cookie; десктоп → нативные Tauri-события (см. D-Notify-01).

---

## Design System

| Property | Value |
|----------|-------|
| Tool | none (custom SCSS tokens — Svelte 5, не React → shadcn-гейт неприменим) |
| Preset | not applicable |
| Component library | custom — `$lib/components/*` (Phase 2), feature-паттерны Phase 3/4 |
| Icon library | inline SVG (паттерн Phase 2/3/4: 16×16 inline, `stroke="currentColor"`) |
| Font | system stack: `-apple-system, 'Segoe UI', 'Roboto', 'Helvetica Neue', 'Arial', sans-serif` |

**Источник:** `ui/src/styles/_tokens.scss` (auto-prepended через `vite.config.ts prependData`) — подтверждено чтением кодовой базы. Phase 6 НЕ вводит новых токенов, шрифтов или внешних UI-пакетов.

**shadcn-гейт:** неприменим — проект на Vanilla Svelte 5 + SCSS (не React/Next.js). Registry safety: not applicable.

---

## Spacing Scale

Токены auto-prepended. Использовать только CSS-переменные, не хардкоженые px.

| Token | Value | Usage в Phase 6 |
|-------|-------|------------------|
| `--space-xs` | 4px | Зазоры в badge/индикаторе алерта, icon gap, разделитель в контекстном меню |
| `--space-sm` | 8px | Отступы между кнопками toolbar, вертикальный padding пунктов меню, gap в строке toner-gauge |
| `--space-md` | 16px | Padding секций карточки принтера/заявки, gap полей формы заявки, gap master-detail, padding ячеек таблицы discovery |
| `--space-lg` | 24px | Padding детальной панели, padding модала discovery, padding страницы |
| `--space-xl` | 32px | Отступ между секциями карточки принтера (Уровни / Счётчики / История) |
| `--space-2xl` | 48px | Вертикальный отступ пустых состояний |
| `--space-3xl` | 64px | Не используется в Phase 6 |

Исключения: `--touch-target-min: 36px` (кнопки-действия), `--row-height: 40px` (строки списков принтеров/заявок и discovery-таблицы), `--row-height-dense: 32px` (строки истории статусов / истории заявки). Toner-gauge bar: высота 8px (новый локальный размер, не токен — объяснение в §Interaction Contracts).

---

## Typography

Все значения — из CSS-переменных `_tokens.scss`. Phase 6 не вводит новых размеров.

Phase 6 объявляет **2 веса** в новых компонентах: `--font-weight-regular: 400` и `--font-weight-semibold: 600`. Токен `--font-weight-medium: 500` существует в `_tokens.scss` и используется унаследованными компонентами switch-bar/sidebar — Phase 6 его не вводит заново и не использует в новых компонентах.

| Role | CSS Token | Size | Weight | Line Height |
|------|-----------|------|--------|-------------|
| Body | `--font-size-body` | 14px | 400 (`--font-weight-regular`) | 1.5 (`--line-height-body`) |
| Label | `--font-size-label` | 13px | 400 (`--font-weight-regular`) | 1.4 (`--line-height-label`) |
| Heading | `--font-size-heading` | 20px | 600 (`--font-weight-semibold`) | 1.3 (`--line-height-heading`) |
| Display | `--font-size-display` | 28px | 600 (`--font-weight-semibold`) | 1.2 (`--line-height-display`) |

**Фокальная точка карточки принтера** — имя принтера (sysName/Наименование устройства) в header детальной панели: `--font-size-display`, `--font-weight-semibold`.

**Числовые значения** (страничные счётчики, проценты тонера, IP-адреса) — `font-variant-numeric: tabular-nums` (паттерн `ActDetail.svelte detail-title`, `CartridgeDetail` код).

Активные вкладки switch-bar, заголовки секций карточки и labels форм — `--font-weight-semibold`; прочее — `--font-weight-regular`.

---

## Color

Все значения — CSS-переменные из `_tokens.scss`. Light/dark переключаются через `[data-theme]`.

| Role | CSS Token (light) | Hex (light) | Usage в Phase 6 |
|------|-------------------|-------------|------------------|
| Dominant (60%) | `--color-bg` | `#ffffff` | Детальная панель принтера/заявки, страница |
| Secondary (30%) | `--color-surface` | `#f5f6f8` | Master-панель списков, hover строк, фон toner-gauge track |
| Surface raised | `--color-surface-raised` | `#ffffff` | Фон модалов (discovery, форма заявки), контекстное меню, alert-баннер фон |
| Surface sunken | `--color-surface-sunken` | `#eaecef` | Badge по умолчанию, заполнение discovery-таблицы, заблокированные поля |
| Accent (10%) | `--color-accent` | `#2563eb` | **Только:** активная вкладка switch-bar + счётчик-badge; активный пункт сайдбара «Принтеры»/«Заявки»; primary CTA; фокус-кольцо; toner-gauge заполнение в норме (≥ порога) |
| Destructive | `--color-destructive` | `#dc2626` | Кнопка «Отклонить» заявки (визуально terminal-negative); кнопка «Удалить»; toner-gauge при критически низком уровне (< 10%); badge статуса принтера «Ошибка»; индикатор алерта (offline/error) |
| Success | `--color-success` | `#16a34a` | Badge статуса принтера «В сети»; badge статуса заявки «Выполнена» |
| Warning | `--color-warning` | `#d97706` | Badge статуса принтера «Предупреждение»; toner-gauge при низком уровне (10–25%); рамка/иконка alert-баннера; badge статуса заявки «В работе» |
| Text primary | `--color-text-primary` | `#111827` | Основное содержимое |
| Text secondary | `--color-text-secondary` | `#4b5563` | Подписи полей, вспомогательный текст, неактивные вкладки, метки счётчиков |
| Text muted | `--color-text-muted` | `#9ca3af` | Placeholder, hint-текст, «нет данных опроса», timestamp last_seen |
| Border | `--color-border` | `#e5e7eb` | Разделители, border карточек, рамки discovery-таблицы |

**Accent reserved for:** активная вкладка switch-bar (статус принтера / тип заявки); активный пункт сайдбара «Принтеры» и «Заявки»; primary-кнопки CTA («Запустить discovery», «Обновить сейчас», «Создать заявку», «Принять в работу», «Выполнить», «Установить картридж», «Завести выбранные»); фокус-ring; заполнение toner-gauge в норме. НЕ для всех интерактивных элементов.

### Badge-цвета статусов принтера (PRN-02, проблемные состояния → DASH-05)

| Статус принтера | Badge variant | Обоснование |
|-----------------|---------------|-------------|
| В сети | `success` | Зелёный = доступен, опрос успешен |
| Предупреждение | `warning` | Оранжевый = низкий тонер / non-critical SNMP-состояние |
| Ошибка | `destructive` | Красный = error-состояние из SNMP-статуса |
| Не в сети | `default` (muted) | Серый = недоступен / опрос не прошёл (offline) |
| Нет данных | `default` (muted) | Серый = ещё ни одного успешного опроса |

### Badge-цвета статусов заявки (REQ-03 lifecycle)

| Статус заявки (CHECK) | Метка RU | Badge variant | Обоснование |
|-----------------------|----------|---------------|-------------|
| `open` | Создана | `accent` | Синий = новая, требует внимания |
| `in_progress` | В работе | `warning` | Оранжевый = в процессе |
| `completed` | Выполнена | `success` | Зелёный = завершена положительно |
| `rejected` | Отклонена | `default` (muted) | Серый = завершена без выполнения |

### Toner-gauge семантика (PRN-02)

Горизонтальный bar (track `--color-surface`, fill цветом по уровню):

| Уровень тонера | Цвет fill | Порог |
|----------------|-----------|-------|
| Норма | `--color-accent` | ≥ 25% |
| Низкий | `--color-warning` | 10–24% |
| Критический | `--color-destructive` | < 10% |
| Неизвестно | `--color-surface-sunken` track, без fill | SNMP вернул `-2`/`-3` (unknown/unrestricted по RFC 3805) |

### Alert-индикатор (PRN-06 alert-каркас, D-Alert-01)

Алерт = проблемное SNMP-состояние (offline/error), один активный на принтер (dedup), только админу. Визуально: красная точка-индикатор (`--color-destructive`, 8×8 dot) в строке списка принтеров + alert-баннер в header детальной панели (рамка/иконка `--color-warning`, паттерн как `LowStockBanner`). Persist до разрешения состояния или acknowledge.

---

## Copywriting Contract

Все тексты на русском (RU-only v1, locked UI-03).

### Навигация и заголовки

| Элемент | Текст |
|---------|-------|
| Пункт сайдбара (раздел А) | Принтеры |
| Пункт сайдбара (раздел Б) | Заявки |
| Заголовок страницы «Принтеры» | Принтеры |
| Заголовок страницы «Заявки» | Заявки |

### Принтеры — switch-bar статусов

| ID/ключ | Метка вкладки |
|---------|---------------|
| null | Все |
| online | В сети |
| warning | Предупреждение |
| error | Ошибка |
| offline | Не в сети |

(Счётчики в badge у каждой вкладки. USB-принтеры — отдельный фильтр/метка «USB», без SNMP-статуса.)

### Принтеры — discovery (PRN-01)

| Элемент | Текст |
|---------|-------|
| Primary CTA (toolbar) | Найти принтеры |
| Заголовок модала discovery | Поиск принтеров в сети |
| Поле «Диапазон IP» (label) | Диапазон IP-адресов |
| Поле «Диапазон IP» (placeholder) | Например: 192.168.1.1–192.168.1.254 |
| Поле «Community» (label) | SNMP Community |
| Поле «Community» (placeholder/hint) | По умолчанию: public |
| Кнопка запуска скана | Начать поиск |
| Состояние скана (прогресс) | Поиск принтеров… найдено: {N} |
| Заголовок таблицы результатов | Найденные принтеры |
| Колонки таблицы | IP-адрес / Производитель / Модель / Имя (sysName) / Статус |
| Метка дубликата (badge в строке) | Уже заведён |
| Чекбокс заведения (header) | Завести как «Принтер» |
| Primary CTA подтверждения | Завести выбранные ({N}) |
| Кнопка отмены | Отмена |

### Принтеры — карточка/детальная панель (PRN-02, PRN-05, PRN-07)

| Элемент | Текст |
|---------|-------|
| Кнопка обновления опроса | Обновить сейчас |
| Заголовок секции уровней | Уровни тонера/чернил |
| Заголовок секции счётчиков | Страничные счётчики |
| Метка счётчика страниц | Всего напечатано |
| Заголовок секции связи картриджа (PRN-07) | Установленный картридж |
| Пусто — картридж не установлен | Картридж не закреплён |
| Заголовок секции истории статусов (PRN-05) | История статусов |
| Метка last_seen | Последний опрос: {относительное время} |
| Метка IP | IP-адрес |
| Метка vendor/модель | Производитель / Модель |
| USB-метка (вместо IP/опроса) | Подключён по USB к: {имя рабочей станции} |

### Заявки — типы и форма (REQ-01, REQ-02)

| Элемент | Текст |
|---------|-------|
| Primary CTA (создать) | Создать заявку |
| Заголовок модала создания | Новая заявка |
| Переключатель типа — опция 1 | Замена картриджа |
| Переключатель типа — опция 2 | Свободная форма |
| «Замена картриджа» — поле принтер (label) | Принтер |
| «Замена картриджа» — placeholder принтера | Выберите принтер |
| «Замена картриджа» — поле комментарий (label) | Комментарий |
| «Замена картриджа» — placeholder комментария | Опишите проблему (необязательно) |
| «Свободная форма» — поле категория (label) | Категория |
| «Свободная форма» — поле текст (label) | Описание |
| «Свободная форма» — placeholder текста | Опишите вашу заявку |
| Кнопка отправки заявки | Отправить заявку |

Категории свободной формы (фиксированный набор, D-Req-Categories-01): «Ремонт техники» / «Расходные материалы» / «Программное обеспечение» / «Прочее». Категория опциональна (Select с пустым первым вариантом «Без категории»).

### Заявки — switch-bar / борд статусов (REQ-03)

| Ключ | Метка вкладки |
|------|---------------|
| null | Все |
| open | Созданные |
| in_progress | В работе |
| completed | Выполненные |
| rejected | Отклонённые |

(Счётчики в badge. Для сотрудника — только его заявки; для специалиста/админа — все.)

### Заявки — действия специалиста (REQ-03, REQ-05)

| Контекст / статус | Кнопка | Variant |
|-------------------|--------|---------|
| `open` → принять | Принять в работу | primary |
| `open` → отклонить | Отклонить | destructive |
| `in_progress` → выполнить (свободная форма) | Выполнить | primary |
| `in_progress` → выполнить (замена картриджа, REQ-05) | Установить картридж | primary |
| `in_progress` → отклонить | Отклонить | destructive |
| Поле резолюции при выполнении/отклонении | Комментарий специалиста | — |

«Установить картридж» (REQ-05) открывает существующую `OperationModal` (op=`install`), pre-filled принтером (и моделью если задана в заявке). Успешная установка → заявка переводится в `completed` (см. §Interaction Contracts 9).

### Пустые состояния

| Контекст | Заголовок | Тело | Действие |
|----------|-----------|------|----------|
| Список принтеров пуст (вообще нет) | Принтеры ещё не добавлены | Запустите поиск принтеров в сети — система найдёт их по SNMP и заведёт автоматически. | Найти принтеры |
| Список принтеров (фильтр пуст) | Ничего не найдено | Попробуйте изменить фильтр статуса. | — |
| Детальная панель (принтер не выбран) | Выберите принтер | Выберите принтер слева, чтобы увидеть уровни тонера, счётчики и историю. | — |
| История статусов пуста | Опросов ещё не было | Данные появятся после первого успешного опроса принтера. | Обновить сейчас |
| Список заявок пуст (специалист, вообще нет) | Заявок пока нет | Новые заявки от сотрудников появятся здесь. | — |
| Список заявок пуст (сотрудник, вообще нет) | У вас пока нет заявок | Создайте заявку — выберите тип и опишите проблему. | Создать заявку |
| Список заявок (фильтр пуст) | Ничего не найдено | Попробуйте изменить фильтр статуса. | — |
| Детальная панель (заявка не выбрана) | Выберите заявку | Выберите заявку слева, чтобы увидеть детали и историю. | — |
| История заявки пуста | История пуста | Изменения статуса этой заявки появятся здесь. | — |
| Discovery — ничего не найдено | Принтеры не найдены | В указанном диапазоне не обнаружено SNMP-устройств. Проверьте диапазон IP и community. | — |

### Состояния загрузки/прогресса

| Контекст | Текст |
|----------|-------|
| Загрузка списка | (Spinner, без текста — паттерн Phase 4) |
| Discovery в процессе | Поиск принтеров… найдено: {N} |
| Опрос «Обновить сейчас» в процессе | Кнопка `loading` (Spinner внутри), label «Обновить сейчас» |
| WebSocket отключён (браузер) | Тихий reconnect; при длительной потере — toast `warning` «Соединение с сервером потеряно. Переподключение…» |

### Ошибочные состояния

| Контекст | Текст ошибки |
|----------|--------------|
| Discovery — невалидный диапазон IP | Укажите корректный диапазон IP-адресов |
| Discovery — таймаут/сетевая ошибка | Не удалось завершить поиск. Проверьте сеть и повторите. |
| Опрос принтера не прошёл | Принтер не отвечает на SNMP. Проверьте доступность и community. |
| Заявка — обязательное поле пустое | Заполните это поле |
| «Замена картриджа» — принтер не выбран | Выберите принтер |
| «Свободная форма» — описание пустое | Опишите вашу заявку |
| Недопустимый переход статуса заявки | Это действие недоступно для текущего статуса заявки |
| Оптимистичная блокировка | Данные изменились в другом окне. Обновите страницу. |
| Общая ошибка операции | Не удалось выполнить операцию. Повторите попытку. |

### Toast-уведомления (UI-06, и WebSocket push D-Notify-01)

| Событие | Тип | Текст |
|---------|-----|-------|
| Принтеры заведены (discovery) | success | Заведено принтеров: {N} |
| Опрос обновлён | success | Данные принтера обновлены |
| Заявка создана (сотрудник) | success | Заявка отправлена |
| Заявка принята в работу | success | Заявка принята в работу |
| Заявка выполнена | success | Заявка выполнена |
| Заявка отклонена | success | Заявка отклонена |
| **Новая заявка (push специалисту/админу)** | info | Новая заявка: {тип} от {ФИО сотрудника} |
| **Смена статуса заявки (push)** | info | Заявка #{id} — {новый статус} |
| **Алерт принтера (push админу)** | warning | Проблема с принтером: {имя} — {статус} |

### Деструктивные/terminal подтверждения

| Действие | Заголовок модала | Текст | Кнопка подтверждения |
|----------|------------------|-------|----------------------|
| Отклонить заявку | Отклонить заявку? | Заявка будет закрыта без выполнения. Укажите причину в комментарии. | Отклонить |
| Удалить принтер (если применимо) | Удалить принтер? | Принтер «{имя}» будет помечен как удалённый. История опросов сохранится в БД. | Удалить |

«Отклонить заявку» — единственное частое terminal-действие фазы. Подтверждение через модал с обязательным/опциональным полем «Комментарий специалиста» (планировщик уточнит обязательность; copy предполагает рекомендуемое). Удаление принтера — destructive confirm-модал (паттерн Phase 4 delete).

---

## Component Inventory

### Переиспользуемые компоненты (`src/lib/components/`) — без изменений

| Компонент | Использование в Phase 6 |
|-----------|--------------------------|
| `Modal` | DiscoveryModal, RequestFormModal, confirm-диалоги (отклонить/удалить) |
| `Input` | Диапазон IP, Community, комментарий заявки, поиск |
| `Select` | Тип заявки, Категория свободной формы, Принтер (dropdown devices type=Принтер) |
| `Textarea` | Описание заявки, комментарий специалиста |
| `Button` | Все кнопки (primary/secondary/destructive/ghost) |
| `Badge` | Статус принтера, статус заявки, тип заявки, счётчики switch-bar, метка «Уже заведён» |
| `Spinner` | Загрузка списков, discovery, опрос |
| `ToastHost` / `Toast` | Уведомления (вкл. WebSocket push) |
| `PersonAutocomplete` | «Кто выдал»/«Кому выдал» внутри переиспользованной OperationModal (REQ-05) |
| `LocationAutocomplete` | «Расположение» внутри переиспользованной OperationModal (REQ-05) |
| `DatePicker` | «Дата» внутри переиспользованной OperationModal (REQ-05) |
| `Placeholder` | Удаляется — заменяется реальными PrintersPage/RequestsPage |

### Переиспользуемые паттерны (`features/`) — как образец

| Паттерн / Образец | Что создаётся в Phase 6 | Отличия |
|-------------------|--------------------------|---------|
| `CartridgesMasterDetail.svelte` (35/65 grid) | `PrintersMasterDetail.svelte`, `RequestsMasterDetail.svelte` | Идентичная структура; разные detail-панели |
| `CartridgesSearchAndTabs.svelte` (search + switch-bar) | `PrintersSearchAndTabs.svelte`, `RequestsSearchAndTabs.svelte` | Статусы принтера / статусы заявки |
| `CartridgeFilters.svelte` (switch-bar + count-badge) | фильтры в обоих разделах | — |
| `DeviceContextMenu.svelte` (portal + mousedown-outside) | при необходимости kebab-меню в строках | Пункты зависят от роли/статуса |
| `CartridgeDetail.svelte` (карточка + история) | `PrinterDetail.svelte`, `RequestDetail.svelte` | Секции уровней/счётчиков (принтер); поля + действия lifecycle (заявка) |
| `CartridgeFormModal.svelte` | `RequestFormModal.svelte` | Тип-переключатель + условные поля |
| `OperationModal.svelte` (Phase 4) | **переиспользуется как есть** для REQ-05 | pre-filled принтером/моделью; запускается из контекста заявки |
| `LowStockBanner.svelte` (warning-баннер) | `PrinterAlertBanner.svelte` | Тот же визуальный паттерн (warning рамка/иконка); содержимое — проблемное состояние принтера |
| `CartridgesList.svelte` (виртуальный список + empty config) | `PrintersList.svelte`, `RequestsList.svelte` | empty-state config-объект (heading/body/actionLabel) — паттерн `emptyConfig` |

### Новые компоненты (`src/features/printers/` и `src/features/requests/`)

**Принтеры:**

| Компонент | Назначение |
|-----------|-----------|
| `PrintersPage.svelte` | Корневой компонент раздела |
| `PrintersSearchAndTabs.svelte` | Поиск + switch-bar статусов + кнопка «Найти принтеры» |
| `PrintersMasterDetail.svelte` | 35/65 layout |
| `PrintersList.svelte` | Список принтеров (empty config) |
| `PrinterListRow.svelte` | Строка: имя, IP/USB, статус-badge, alert-dot, краткий toner |
| `PrinterDetail.svelte` | Карточка: уровни (TonerGauge), счётчики, картридж (PRN-07), история статусов, alert-баннер, «Обновить сейчас» |
| `TonerGauge.svelte` | Горизонтальный bar уровня тонера/чернил (цвет по порогу) |
| `PrinterAlertBanner.svelte` | Warning-баннер проблемного состояния (паттерн LowStockBanner) |
| `DiscoveryModal.svelte` | Поиск по диапазону IP + таблица результатов + review-перед-добавлением |
| `DiscoveryResultsTable.svelte` | Таблица найденных (чекбоксы, дубликаты, vendor/модель) |
| `api.ts` | dual-transport обёртки `printers_*` команд |

**Заявки:**

| Компонент | Назначение |
|-----------|-----------|
| `RequestsPage.svelte` | Корневой компонент раздела (роль-зависимый: сотрудник vs специалист) |
| `RequestsSearchAndTabs.svelte` | Поиск + switch-bar статусов + кнопка «Создать заявку» |
| `RequestsMasterDetail.svelte` | 35/65 layout |
| `RequestsList.svelte` | Список заявок (empty config) |
| `RequestListRow.svelte` | Строка: тип, краткое описание, статус-badge, автор, дата |
| `RequestDetail.svelte` | Карточка: поля заявки + кнопки lifecycle + история (REQ-07) |
| `RequestFormModal.svelte` | Создание заявки (тип-переключатель + условные поля) |
| `api.ts` | dual-transport обёртки `requests_*` команд |

**Общее (transport):**

| Компонент/модуль | Назначение |
|------------------|-----------|
| `ui/src/lib/api/ws.ts` (или stores) | WS-клиент (браузер) + Tauri-event listener (десктоп) + reconnect; питает toast + инвалидацию списков |

---

## Interaction Contracts

### 1. PrintersPage / RequestsPage — master-detail

- 35/65 grid (паттрн `CartridgesMasterDetail`). Master: `--color-surface` + border + `--radius-md`. Detail: `--color-bg` + border.
- Breakpoint < 1100px: `380px 1fr`, `min-width: 900px` (горизонтальный scroll у родителя — паттерн Phase 2/3/4).
- Переключение switch-bar / поиск сбрасывает выделение detail.
- `RequestsPage` роль-зависим (D-RBAC-02): роль «Сотрудник» видит только кнопку «Создать заявку» + свои заявки read-only (без кнопок lifecycle); «Специалист»/«Администратор» видят все заявки + кнопки переходов. Enforcement — на сервис-слое (UI лишь скрывает; обход через HTTP → 403, паттерн Phase 5).

### 2. PrintersSearchAndTabs — switch-bar + поиск

- Поиск: debounce 250ms. Placeholder «Поиск по имени, IP, модели».
- Switch-bar статусов принтера: `role="tablist"/"tab"`, `aria-selected`, count-badge (активная — `accent`, прочие — `default`).
- Кнопка «Найти принтеры» (primary, справа в toolbar) — видна только админу — открывает DiscoveryModal.

### 3. DiscoveryModal — поиск с review-перед-добавлением (PRN-01, D-Discovery-01)

- `size="wide"` (960px) — из-за таблицы результатов.
- Поля: «Диапазон IP-адресов» (Input), «SNMP Community» (Input, placeholder «public»). Кнопка «Начать поиск» (primary).
- Во время скана: Spinner + «Поиск принтеров… найдено: {N}»; таблица наполняется инкрементально по мере находок.
- DiscoveryResultsTable: колонки IP / Производитель / Модель / Имя / Статус + чекбокс «Завести как Принтер» (header — выбрать все ненайденные-дубликаты).
- Уже заведённые (по IP/serial) — строка с badge «Уже заведён» (`default`), чекбокс disabled, не дублируются (D-Discovery-01).
- CTA подтверждения: «Завести выбранные ({N})» (primary, disabled при 0 выбранных). Успех → toast `success` «Заведено принтеров: {N}» + модал закрывается + список обновляется.
- Закрытие/отмена не сохраняет результаты скана.

### 4. PrinterDetail — карточка принтера (PRN-02/05/07)

- Header: имя принтера (`--font-size-display`, semibold), статус-badge, alert-баннер (если активный алерт), кнопка «Обновить сейчас» (`size="sm"`, primary; `loading` во время опроса).
- Секция «Уровни тонера/чернил»: по одному TonerGauge на supply (CMYK/чёрный). Каждая строка: метка цвета + TonerGauge (8px height bar) + процент (tabular-nums). Уровень `-2`/`-3` (unknown) → серый track без fill + текст «—».
- Секция «Страничные счётчики»: «Всего напечатано: {N}» (tabular-nums). Дополнительные счётчики из профиля — построчно.
- Секция «Установленный картридж» (PRN-07): код + модель картриджа (FK от CART-07 установки) или «Картридж не закреплён».
- Секция «История статусов» (PRN-05): хронологический список (новые сверху), строка высотой `--row-height-dense`. Формат: «{дата/время} — {статус}; тонер {значения}». Пусто → «Опросов ещё не было».
- Метаданные (IP, vendor/модель, last_seen относительным временем `--color-text-muted`).
- USB-принтер: вместо IP/опроса — «Подключён по USB к: {рабочая станция}»; секции SNMP-уровней скрыты.

### 5. TonerGauge — индикатор уровня

- Горизонтальный bar: track `--color-surface` (или `--color-surface-sunken` для unknown), border-radius `--radius-sm`, высота 8px (локальный размер — не токен: gauge тоньше row-height).
- Fill width = процент; цвет по порогу (см. §Color Toner-gauge семантика): ≥25% accent, 10–24% warning, <10% destructive.
- `role="progressbar"`, `aria-valuenow`, `aria-valuemin=0`, `aria-valuemax=100`, `aria-label="Уровень {цвет}: {N}%"`.
- Unknown (`-2`/`-3`): пустой track + «—», `aria-label="Уровень {цвет}: неизвестно"`.

### 6. PrinterAlertBanner — alert-каркас (PRN-06, D-Alert-01)

- Рендерится в header PrinterDetail только при активном алерте принтера.
- Визуал: паттерн `LowStockBanner` — warning рамка/иконка (inline SVG 16×16), фон `color-mix(--color-warning 10%, transparent)`. Для error/offline — допустимо усиление до `--color-destructive` рамки.
- Текст: «{статус-описание}. Последний успешный опрос: {время}.»
- `role="alert"`, `aria-live="polite"`.
- В строке списка (`PrinterListRow`) — компактный indicator-dot (8×8, `--color-destructive`), `aria-label="Есть проблема с принтером"`.
- Только админ видит алерты (роль-фильтр). Persist до разрешения/acknowledge (один активный на принтер, dedup — D-Alert-01).

### 7. RequestFormModal — создание заявки (REQ-01/02, D-Req-Form-01)

- `size="md"` (640px). Доступно сотруднику в браузере (без Tauri API).
- Переключатель типа (radio/segmented): «Замена картриджа» / «Свободная форма». Переключение меняет видимые поля (Svelte `{#if}`, без анимации).
- **Замена картриджа:** «Принтер» (Select из devices type=Принтер) — **обязательно**; «Комментарий» (Input/Textarea) — опционально. Модель картриджа НЕ выбирается сотрудником (D-Req-Form-01).
- **Свободная форма:** «Категория» (Select, опционально, первый вариант «Без категории»); «Описание» (Textarea) — обязательно.
- CTA «Отправить заявку» (primary, `loading`). Успех → toast `success` «Заявка отправлена» + модал закрывается + список обновляется + WS push специалисту/админу.
- Валидация на frontend перед submit (принтер выбран / описание непусто).

### 8. RequestDetail — карточка заявки + lifecycle (REQ-03/07)

- Header: тип заявки, статус-badge, автор (ФИО сотрудника), дата создания.
- Поля по типу: «Замена картриджа» → принтер + комментарий + (модель если определена специалистом); «Свободная форма» → категория + описание.
- Кнопки lifecycle (только специалист/админ, visible по статусу):
  - `open`: «Принять в работу» (primary) + «Отклонить» (destructive).
  - `in_progress` (свободная форма): «Выполнить» (primary) + «Отклонить» (destructive).
  - `in_progress` (замена картриджа): «Установить картридж» (primary, → REQ-05) + «Отклонить» (destructive).
  - `completed`/`rejected`: без кнопок (terminal), показывается резолюция.
- «Отклонить» → confirm-модал с полем «Комментарий специалиста». «Выполнить» (свободная форма) → опциональный комментарий резолюции.
- Секция «История» (REQ-07): хронология смен статуса из audit_log (новые сверху), строка `--row-height-dense`. Формат: «{дата/время} — {статус}; {кто}; {комментарий}». Пусто → «История пуста».

### 9. REQ-05 — «Установить картридж» из контекста заявки (D-Req-CART07-01)

- Кнопка «Установить картридж» в RequestDetail (заявка `cartridge_replace`, статус `in_progress`) открывает существующую `OperationModal` (Phase 4) с `op='install'`.
- Pre-fill: принтер из заявки (как целевое устройство/расположение-контекст) + модель картриджа, если задана. OperationModal сигнатура не меняется — передаётся выбранный экземпляр картриджа специалистом.
- Успешная установка (CART-07) → заявка автоматически переводится в `completed`; toast `success` «Заявка выполнена»; список обновляется; WS push.
- Установка связывает картридж с конкретным устройством-принтером (FK, PRN-07) — отражается в PrinterDetail «Установленный картридж».

### 10. WebSocket уведомления (REQ-04, D-Notify-01)

- **Браузер:** WS-соединение к axum-эндпоинту, auth по session-cookie (как `/api/*`). Reconnect с backoff при разрыве; при длительной потере — toast `warning` «Соединение с сервером потеряно. Переподключение…».
- **Десктоп (Tauri):** нативные Tauri-события от бэкенда (не требует включённого сервера) — тот же набор payload'ов.
- События: новая заявка (специалисту/админу), смена статуса заявки, алерт принтера (админу). Каждое → toast (см. §Toast) + инвалидация соответствующего списка (re-fetch).
- Роль-фильтрация push — на сервере (сотрудник не получает чужих заявок/алертов).
- WS — единственный realtime-канал; остальной UI остаётся request/response (UI-05).

### 11. Доступность (паттерн Phase 4)

- Все switch-bar: `role="tablist"/"tab"`, `aria-selected`.
- TonerGauge: `role="progressbar"` + aria-value*.
- Alert-баннер/dot: `aria-label`, `role="alert"`/`aria-live`.
- Kebab-триггеры (если используются): `aria-label="Действия с принтером {имя}"` / `aria-label="Действия с заявкой #{id}"`.
- Фокус-ring на всех интерактивных — `box-shadow: 0 0 0 3px var(--color-accent-focus)` (паттерн Button/tab).

---

## Layout Contract

### PrintersPage

```
<main>                              (page-content, padding: --space-lg)
  <PrintersSearchAndTabs />         (поиск + switch-bar статусов + «Найти принтеры»[admin])
  <PrintersMasterDetail>
    {#snippet master}
      <PrintersList />              (строки PrinterListRow)
    {/snippet}
    {#snippet detail}
      <PrinterDetail />            (alert-баннер + уровни + счётчики + картридж + история)
    {/snippet}
  </PrintersMasterDetail>
</main>
<DiscoveryModal />                  (по «Найти принтеры»)
```

### RequestsPage

```
<main>                              (page-content, padding: --space-lg)
  <RequestsSearchAndTabs />         (поиск + switch-bar статусов + «Создать заявку»)
  <RequestsMasterDetail>
    {#snippet master}
      <RequestsList />             (строки RequestListRow)
    {/snippet}
    {#snippet detail}
      <RequestDetail />           (поля + lifecycle-кнопки[specialist] + история)
    {/snippet}
  </RequestsMasterDetail>
</main>
<RequestFormModal />                (по «Создать заявку»)
<OperationModal op="install" />     (по «Установить картридж», REQ-05)
```

### Master-detail (оба раздела)

- Grid: `35% 65%`, `gap: --space-md`. Master `--color-surface` + border + `--radius-md`; Detail `--color-bg` + border.
- Breakpoint < 1100px: `380px 1fr`, `min-width: 900px`.

### DiscoveryModal layout

- `size="wide"` (960px). Поля скана (Диапазон IP / Community / кнопка) — верхняя строка, grid `1fr 1fr auto`.
- DiscoveryResultsTable — под полями, полная ширина, вертикальный scroll при многих результатах. Строки `--row-height` (40px).

### RequestFormModal layout

- `size="md"` (640px). Поля — одна колонка, gap `--space-md`. Hint под полем: `--font-size-label`, `--color-text-muted`.

### PrinterDetail layout

- Секции вертикально, разделены `--space-xl`. Заголовок секции semibold `--font-size-body`.
- TonerGauge-строки: grid `auto 1fr auto` (метка цвета / bar / процент), gap `--space-sm`.

---

## Registry Safety

| Registry | Blocks Used | Safety Gate |
|----------|-------------|-------------|
| shadcn official | none (проект на Svelte 5 — shadcn неприменим) | not applicable |
| Сторонние реестры | none | not applicable |

Phase 6 не добавляет новых внешних UI-пакетов. WebSocket-клиент — нативный браузерный `WebSocket` API + Tauri events; новых npm-зависимостей UI-слой не вводит. Все компоненты — из `$lib/components/` (Phase 2) и feature-паттернов Phase 3/4 как образцы. Registry vetting gate: не требуется (нет third-party блоков).

---

## Pre-Population Sources

| Источник | Решений использовано |
|----------|----------------------|
| `CONTEXT.md` (06-CONTEXT.md) | 24 locked decisions (D-Discovery-01..02, D-Poll-01, D-Arch-01, D-Mock-01, D-Schema-01, D-History-01, D-Retention-01, D-OID-01, D-Settings-01, D-Pantum-01, D-Alert-01, D-Req-Form-01, D-Req-Categories-01, D-Notify-01, D-Req-Lifecycle-01, D-Req-CART07-01, D-PRN07-01 + scope/specifics) |
| `ROADMAP.md` §Phase 6 | Goal + 5 success criteria (⚠️ #3 alert-каркас, hang-детекция deferred → v2) |
| `REQUIREMENTS.md` §PRN-01..08, REQ-01..05, REQ-07 | 14 требований (формулировки полей, lifecycle, типы заявок) |
| `ui/src/styles/_tokens.scss` | Полный design token set (цвета, spacing, типографика) — без изменений |
| `04-UI-SPEC.md` (Phase 4) | Формат контракта, switch-bar/master-detail/OperationModal паттерны, empty-config паттерн |
| `Badge.svelte`, `Button.svelte`, `LowStockBanner.svelte`, `CartridgesSearchAndTabs.svelte`, `OperationModal.svelte` | Реальные сигнатуры/варианты компонентов |
| `05-CONTEXT.md` (Phase 5) | D-RBAC-02 (роль-зависимый портал заявок), D-Session-01 (WS auth по cookie) |
| Пользователь (input) | 0 (все вопросы закрыты upstream-артефактами и кодовой базой) |

---

## Checker Sign-Off

- [ ] Dimension 1 Copywriting: PASS
- [ ] Dimension 2 Visuals: PASS
- [ ] Dimension 3 Color: PASS
- [ ] Dimension 4 Typography: PASS
- [ ] Dimension 5 Spacing: PASS
- [ ] Dimension 6 Registry Safety: PASS

**Approval:** pending
