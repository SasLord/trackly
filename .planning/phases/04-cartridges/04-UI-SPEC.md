---
phase: 4
slug: cartridges
status: approved
shadcn_initialized: false
preset: none
created: 2026-06-07
reviewed_at: 2026-06-07
---

# Phase 4 — UI Design Contract: Картриджи

> Визуальный и интеракционный контракт для Phase 4.
> Сгенерирован gsd-ui-researcher, верифицируется gsd-ui-checker.

---

## Design System

| Property | Value |
|----------|-------|
| Tool | none (custom SCSS tokens) |
| Preset | not applicable |
| Component library | custom — `$lib/components/*` Phase 2 |
| Icon library | inline SVG (паттерн Phase 2/3: иконка поиска 16×16 inline) |
| Font | system stack: `-apple-system, 'Segoe UI', 'Roboto', 'Helvetica Neue', 'Arial', sans-serif` |

**Источник:** `ui/src/styles/_tokens.scss` — подтверждено чтением кодовой базы.

---

## Spacing Scale

Токены auto-prepended через `vitePreprocess prependData`. Использовать только переменные, не хардкоженые значения px.

| Token | Value | Usage |
|-------|-------|-------|
| `--space-xs` | 4px | Зазоры внутри badge, icon gap, разделитель в контекстном меню |
| `--space-sm` | 8px | Отступы между кнопками в toolbar, padding пунктов меню по вертикали |
| `--space-md` | 16px | Padding секций карточки, gap между полями формы, gap master-detail |
| `--space-lg` | 24px | Padding детальной панели, padding модала |
| `--space-xl` | 32px | Отступ между секциями карточки экземпляра |
| `--space-2xl` | 48px | Резерв для будущих разделителей страницы |
| `--space-3xl` | 64px | Не используется в Phase 4 |

Исключения: `--touch-target-min: 36px` — минимальная высота кнопки-действия; `--row-height: 40px` — строка списка; `--row-height-dense: 32px` — строка истории перемещений.

---

## Typography

Все значения берутся из CSS-переменных `_tokens.scss`, не задаются явными px в компонентах Phase 4.

Phase 4 объявляет **2 веса**: `--font-weight-regular: 400` и `--font-weight-semibold: 600`.

Примечание: токен `--font-weight-medium: 500` объявлен в `_tokens.scss` и используется компонентами Phase 2/3 (Sidebar, DeviceFilters, DeviceGroupRow и др.). Phase 4 не вводит этот вес заново — компоненты, унаследованные из Phase 2/3 (напр. паттерн switch-bar из `DeviceFilters.svelte`), опираются на него через существующий токен. Новые компоненты Phase 4 используют только 400 и 600.

| Role | CSS Token | Size | Weight | Line Height |
|------|-----------|------|--------|-------------|
| Body | `--font-size-body` | 14px | 400 (`--font-weight-regular`) | 1.5 (`--line-height-body`) |
| Label | `--font-size-label` | 13px | 400 (`--font-weight-regular`) | 1.4 (`--line-height-label`) |
| Heading | `--font-size-heading` | 20px | 600 (`--font-weight-semibold`) | 1.3 (`--line-height-heading`) |
| Display | `--font-size-display` | 28px | 600 (`--font-weight-semibold`) | 1.2 (`--line-height-display`) |

Активные вкладки switch-bar и labels форм в новых компонентах Phase 4 используют `--font-weight-semibold: 600`; неактивные/прочие — `--font-weight-regular: 400`.

В Phase 4 **не вводятся** новые размеры шрифтов. Код экземпляра `C-000001` отображается с `font-variant-numeric: tabular-nums` (паттерн из `ActDetail.svelte` — `detail-title`).

**Фокальная точка страницы** — поле кода экземпляра в header детальной панели (`--font-size-display`, `tabular-nums`).

---

## Color

Все значения — CSS-переменные из `_tokens.scss`. Светлая и тёмная темы переключаются через `[data-theme]` (паттерн Phase 2).

| Role | CSS Token (light) | Hex (light) | Usage |
|------|-------------------|-------------|-------|
| Dominant (60%) | `--color-bg` | `#ffffff` | Детальная панель, страница |
| Secondary (30%) | `--color-surface` | `#f5f6f8` | Master-панель, hover строк, LowStockBanner фон |
| Surface raised | `--color-surface-raised` | `#ffffff` | Фон модалов, контекстное меню |
| Surface sunken | `--color-surface-sunken` | `#eaecef` | Badge по умолчанию, заблокированные поля |
| Accent (10%) | `--color-accent` | `#2563eb` | **Только:** активная вкладка switch-bar + её счётчик-badge; активный элемент сайдбара; primary CTA-кнопки; фокус-кольцо (`--color-accent-focus`) |
| Destructive | `--color-destructive` | `#dc2626` | Кнопка «Удалить» в модале подтверждения; пункт «Удалить» в контекстном меню; кнопка «Списать» (визуально деструктивная операция) |
| Success | `--color-success` | `#16a34a` | Badge статуса «На складе» (variant=`success`) |
| Warning | `--color-warning` | `#d97706` | Badge статуса «На заправке»; LowStockBanner иконка/рамка |
| Text primary | `--color-text-primary` | `#111827` | Всё основное содержимое |
| Text secondary | `--color-text-secondary` | `#4b5563` | Подписи полей, вспомогательный текст, неактивные вкладки |
| Text muted | `--color-text-muted` | `#9ca3af` | Placeholder, hint-текст в форме операции |
| Border | `--color-border` | `#e5e7eb` | Все разделители, border карточек |

**Accent reserved for:** активная вкладка switch-bar (статус + тип + модель), активный пункт сайдбара «Картриджи», primary-кнопки CTA («Добавить картридж», «Добавить модель», «Сохранить изменения»), фокус-ring на всех интерактивных элементах.

### Badge-цвета статусов

| Статус | Badge variant | Обоснование |
|--------|---------------|-------------|
| На складе | `success` | Зелёный = в наличии, норма |
| В работе | `accent` | Синий = активно используется |
| На заправке | `warning` | Оранжевый = временно недоступен |
| Списано | `default` (muted) | Серый = неактивный/завершённый |

### LowStockBanner семантика

Рамка и иконка — `--color-warning`. Фон — `color-mix(in srgb, var(--color-warning) 10%, transparent)`. Паттерн аналогичен Badge `warning`.

---

## Copywriting Contract

Все тексты на русском языке (RU-only v1, locked UI-03).

### Навигация и заголовки

| Элемент | Текст |
|---------|-------|
| Пункт сайдбара | Картриджи |
| Вкладка 1 (по умолчанию) | Картриджи |
| Вкладка 2 | Модели |
| Заголовок страницы | Картриджи |

### Switch-bar статусов (CART-05)

| ID | Метка вкладки |
|----|---------------|
| null | Все |
| 1 | На складе |
| 2 | В работе |
| 3 | На заправке |
| 4 | Списано |

### Дополнительные фильтры

| Фильтр | Placeholder / метка |
|--------|---------------------|
| По типу расходника | Тип: Все / Картридж / Фотобарабан |
| По модели | Модель: все |
| Поиск | Поиск по коду, модели, расположению |

### Пустые состояния

| Контекст | Заголовок | Тело | Действие |
|---------|-----------|------|----------|
| Список пуст (нет картриджей вообще) | Картриджей пока нет | Добавьте первый картридж, чтобы начать отслеживать расходники. | + Добавить картридж |
| Список пуст (фильтр не дал результатов) | Ничего не найдено | Попробуйте изменить фильтры или поисковый запрос. | — |
| Детальная панель (не выбран экземпляр) | Выберите картридж | Выберите картридж слева, чтобы увидеть историю и выполнить действие, или добавьте новый. | + Добавить картридж |
| Список моделей пуст | Моделей пока нет | Добавьте модель картриджа — укажите бренд, тип и совместимые принтеры. | + Добавить модель |
| История перемещений пуста | История пуста | Операции с этим картриджем появятся здесь. | — |

### Primary CTA

| Контекст | Кнопка |
|---------|--------|
| Создать экземпляр (toolbar) | + Добавить картридж |
| Создать модель (toolbar) | + Добавить модель |
| Сохранить форму экземпляра (новый) в модале | Добавить картридж |
| Сохранить форму экземпляра (редактирование) в модале | Сохранить изменения |
| Сохранить форму модели (новая) в модале | Добавить модель |
| Сохранить форму модели (редактирование) в модале | Сохранить изменения |
| Подтвердить «Установить в принтер» в OperationModal | Установить |
| Подтвердить «Вернуть на склад» в OperationModal | Вернуть на склад |
| Подтвердить «Отправить на заправку» в OperationModal | Отправить на заправку |
| Подтвердить «Забрать с заправки» в OperationModal | Вернуть с заправки |
| Подтвердить «Списать» в OperationModal | Списать |
| Отмена в любом модале | Отмена |

### Контекстное меню (status-dependent, CART-06)

| Статус | Пункты меню (порядок) |
|--------|----------------------|
| На складе | Установить в принтер / Отправить на заправку / Редактировать / — / Списать / Удалить |
| В работе | Вернуть на склад / Редактировать / — / Удалить |
| На заправке | Забрать с заправки / Редактировать / — / Удалить |
| Списано | Редактировать / — / Удалить |

Разделитель `<hr class="ctx-menu-sep">` отделяет деструктивные действия («Удалить», «Списать») от остальных.

### Accessibility — aria-labels для kebab-триггеров

| Компонент | `aria-label` |
|-----------|-------------|
| `CartridgeContextMenu` (kebab ⋮ в CartridgeListRow) | `aria-label="Действия с картриджем {code}"` |
| `ModelListRow` (kebab ⋮) | `aria-label="Действия с моделью {brand} {model}"` |

### Заголовки OperationModal

| Операция | Заголовок модала |
|---------|-----------------|
| Установить в принтер | Установка в принтер |
| Вернуть на склад | Возврат на склад |
| Отправить на заправку | Отправка на заправку |
| Забрать с заправки | Получение с заправки |
| Списать | Списание картриджа |

### Поля OperationModal (по типу операции)

#### Установить в принтер

| Поле | Компонент | Default | Обязательное |
|------|-----------|---------|--------------|
| Дата | DatePicker | сегодня | да |
| Кто выдал | PersonAutocomplete | — | да |
| Кому выдал | PersonAutocomplete | — | да |
| Расположение | LocationAutocomplete | — | да |

Hint под полем «Расположение»: «Укажите рабочее место или кабинет (не склад)»

#### Вернуть на склад

| Поле | Компонент | Default | Обязательное |
|------|-----------|---------|--------------|
| Состояние заряда | Select | Пустой (state_id=3) | да |
| Расположение | LocationAutocomplete | — | да |
| Примечание | Textarea | — | нет |

Hint под полем «Расположение»: «Укажите склад или место хранения»

#### Отправить на заправку

| Поле | Компонент | Default | Обязательное |
|------|-----------|---------|--------------|
| Дата | DatePicker | сегодня | да |
| Кто выдал | PersonAutocomplete | — | да |
| Кому выдал | PersonAutocomplete | — | да |
| Расположение | LocationAutocomplete | — | да |

#### Забрать с заправки

| Поле | Компонент | Default | Обязательное |
|------|-----------|---------|--------------|
| Состояние заряда | Select | Полный (state_id=1) | да |
| Расположение | LocationAutocomplete | — | да |
| Примечание | Textarea | — | нет |

#### Списать

| Поле | Компонент | Default | Обязательное |
|------|-----------|---------|--------------|
| Дата | DatePicker | сегодня | да |
| Причина / Примечание | Textarea | — | нет |

### Поля CartridgeFormModal (CRUD экземпляра)

| Поле | Компонент | Default | Обязательное |
|------|-----------|---------|--------------|
| Код | Input (text) | auto: `C-XXXXXX` (placeholder) | да |
| Модель | Select (из cartridge_models) | — | да |
| Состояние заряда | Select | Полный | да |
| Расположение | LocationAutocomplete | — | нет |
| Примечания | Textarea | — | нет |

Hint под полем «Код»: «Будет присвоен автоматически. Введите свой код (например, штрих-код) при необходимости.»

### Поля ModelFormModal (CRUD модели, CART-01/CART-02)

| Поле | Компонент | Условие видимости | Default | Обязательное |
|------|-----------|-------------------|---------|--------------|
| Тип расходника | Select (Картридж / Фотобарабан) | всегда | Картридж | да |
| Бренд | Input + focus-open autocomplete | всегда | — | да |
| Модель | Input + focus-open autocomplete | всегда | — | да |
| Цвет | Select (фиксированный набор) | только если Тип = Картридж | Чёрный | нет |
| Примечание | Textarea | всегда | — | нет |
| Совместимые принтеры | CompatibilityEditor | всегда | — | нет |

Цвета (фиксированный набор, D-Model-Color-01): Чёрный / Голубой / Пурпурный / Жёлтый / Светло-голубой / Светло-пурпурный.

CompatibilityEditor — добавляемый список пар (Бренд принтера + Модель принтера), каждая пара с focus-open autocomplete из ранее введённых значений. Кнопка «+ Добавить принтер» под списком.

### Ошибочные состояния

| Контекст | Текст ошибки |
|---------|-------------|
| Код уже занят (custom conflict) | Картридж с кодом «{code}» уже существует. Введите другой код. |
| Обязательное поле пустое | Заполните это поле |
| Модель не выбрана | Выберите модель картриджа |
| Модель используется (удаление) | Нельзя удалить модель: она используется {N} картриджами |
| Бренд + Модель уже существует | Модель «{brand} {model}» уже создана |
| Оптимистичная блокировка | Данные изменились в другом окне. Обновите страницу. |
| Общая ошибка операции | Не удалось выполнить операцию. Повторите попытку. |

### Деструктивные подтверждения

| Действие | Заголовок модала | Текст | Кнопка подтверждения |
|---------|-----------------|-------|---------------------|
| Удалить картридж | Удалить картридж? | Картридж «{code}» будет помечен как удалённый. Отменить можно только восстановлением из резервной копии БД. | Удалить |
| Удалить модель | Удалить модель? | Модель «{brand} {model}» будет помечена как удалённая. | Удалить |
| Списать картридж | Списать картридж? | Картридж «{code}» будет переведён в статус «Списано». Укажите причину. | Списать |

### LowStockBanner (CART-12)

Показывается сверху раздела «Картриджи» (вкладка «Картриджи»), если есть хотя бы одна модель ниже порога.

Заголовок: «Низкий остаток картриджей»

Строка на модель: «{brand} {model} — {count} шт. на складе (порог: {threshold})»

Если LowStockBanner пуст (нет моделей ниже порога) — компонент не рендерится (display: none).

---

## Component Inventory

Все компоненты ниже — **переиспользуются без изменений**. Новые компоненты в Phase 4 строятся из этих блоков.

### Переиспользуемые компоненты (src/lib/components/)

| Компонент | Путь | Использование в Phase 4 |
|-----------|------|------------------------|
| `Modal` | `$lib/components/Modal.svelte` | CartridgeFormModal, ModelFormModal, OperationModal, confirm-delete диалоги |
| `Input` | `$lib/components/Input.svelte` | Поля Бренд, Модель, Код, Бренд принтера, Модель принтера в CompatibilityEditor |
| `Select` | `$lib/components/Select.svelte` | Тип расходника, Цвет, Состояние заряда |
| `Textarea` | `$lib/components/Textarea.svelte` | Примечание в формах и операциях, Причина списания |
| `Button` | `$lib/components/Button.svelte` | Все кнопки (primary/secondary/destructive) |
| `Badge` | `$lib/components/Badge.svelte` | Статус-badge в строке списка и детали, счётчик switch-bar |
| `DatePicker` | `$lib/components/DatePicker.svelte` | Дата в OperationModal (установка, заправка, списание) |
| `PersonAutocomplete` | `$lib/components/PersonAutocomplete.svelte` | «Кто выдал», «Кому выдал» |
| `LocationAutocomplete` | `$lib/components/LocationAutocomplete.svelte` | «Расположение» во всех формах и операциях |
| `Spinner` | `$lib/components/Spinner.svelte` | Загрузка списка, детали |
| `ToastHost` | `$lib/components/ToastHost.svelte` | Уведомления об успехе/ошибке операций |

### Переиспользуемые паттерны (features/)

| Паттерн / Компонент-образец | Что создаётся в Phase 4 | Отличия от оригинала |
|----------------------------|-------------------------|----------------------|
| `ActsMasterDetail.svelte` (35/65 grid) | `CartridgesMasterDetail.svelte` | Идентичная структура; детальная панель — CartridgeDetail, а не ActDetail |
| `DeviceContextMenu.svelte` (portal + mousedown-outside) | `CartridgeContextMenu.svelte` | Пункты меню зависят от `status_id` экземпляра; «Установить в принтер» / «Вернуть на склад» и т.д. вместо фиксированных пунктов |
| `DeviceFilters.svelte` (switch-bar + search) | `CartridgeFilters.svelte` | Статусы CART (`cartridge_statuses`); дополнительные фильтры «Тип» + «Модель»; группировка отсутствует |
| `ActsSearchAndTabs.svelte` | `CartridgesSearchAndTabs.svelte` | Search input; два таба «Картриджи» / «Модели» (не 3 таба актов) |
| `ActDetail.svelte` | `CartridgeDetail.svelte` | Поля экземпляра + OperationHistory; кнопки — lifecycle-действия из CartridgeContextMenu |
| `ActFormModal.svelte` | `CartridgeFormModal.svelte` | CRUD экземпляра |
| `ReturnModal.svelte` | `OperationModal.svelte` | Единая параметризованная модалка — заголовок и поля зависят от `op` prop |

### Новые компоненты (src/features/cartridges/)

| Компонент | Назначение |
|-----------|-----------|
| `CartridgesPage.svelte` | Корневой компонент раздела, два таба |
| `CartridgesSearchAndTabs.svelte` | Search + таб «Картриджи» / «Модели» + LowStockBanner |
| `LowStockBanner.svelte` | Предупреждение о низком остатке |
| `CartridgesMasterDetail.svelte` | 35/65 layout (по образцу ActsMasterDetail) |
| `CartridgesList.svelte` | Список экземпляров с виртуальным scroll |
| `CartridgeListRow.svelte` | Строка списка: код, модель, статус-badge, расположение, kebab |
| `CartridgeDetail.svelte` | Детальная панель: поля + OperationHistory |
| `CartridgeContextMenu.svelte` | Status-dependent kebab меню |
| `CartridgeFilters.svelte` | Switch-bar статусов + фильтр типа + фильтр модели |
| `CartridgeFormModal.svelte` | CRUD экземпляра |
| `OperationModal.svelte` | Единая модалка lifecycle-операций |
| `ModelsList.svelte` | Список моделей |
| `ModelListRow.svelte` | Строка модели: бренд+модель, тип, цвет-badge, кол-во экземпляров, kebab |
| `ModelFormModal.svelte` | CRUD модели + CompatibilityEditor |
| `CompatibilityEditor.svelte` | Добавляемый список пар бренд+модель принтера |
| `api.ts` | Tauri invoke / HTTP fetch обёртки для cartridges_* команд |

---

## Interaction Contracts

### 1. CartridgesPage — два таба

- По умолчанию открыт таб «Картриджи» (экземпляры).
- Таб «Модели» открывает полноширинный CRUD-список (без master-detail — таб заменяет всю рабочую область).
- Переключение таба сбрасывает выделение в master-detail.
- LowStockBanner показывается только на вкладке «Картриджи», сверху перед master-detail.

### 2. CartridgesSearchAndTabs — search + switch-bar

- Search input: debounce 250ms (паттерн DeviceFilters).
- Switch-bar статусов: роль `tablist`/`tab`, `aria-selected`.
- Доп. фильтр «Тип»: Select `Все / Картридж / Фотобарабан`. Изменение немедленно перезапрашивает список.
- Доп. фильтр «Модель»: Select из `cartridge_models` (только активные). Изменение немедленно перезапрашивает список.
- Все фильтры независимы, применяются совместно (AND-логика).

### 3. CartridgeContextMenu — status-dependent

- Kebab кнопка (⋮): `width: 28px, height: 28px`, `border-radius: --radius-sm`, `aria-label="Действия с картриджем {code}"`.
- Меню рендерится в portal (`use:portal`), `position: fixed`, `z-index: 2000`.
- Закрытие: mousedown вне меню, Escape, scroll/resize окна.
- Пункты меню генерируются из `status_id` экземпляра (не показываются недопустимые операции — нет серых disabled пунктов).
- «Установить в принтер» / «Отправить на заправку» / «Забрать с заправки» — открывают `OperationModal` с соответствующим `op`.
- «Вернуть на склад» — открывает `OperationModal` с `op='return_to_stock'`.
- «Списать» — открывает `OperationModal` с `op='write_off'`.
- «Редактировать» — открывает `CartridgeFormModal` в режиме редактирования.
- «Удалить» — открывает confirm-Modal (destructive).

### 4. OperationModal — параметризованная модалка

- `size="md"` (640px max-width). Исключение: если форма высокая (все поля «Установить в принтер») — `size="md"` с вертикальным скроллом.
- `op` prop: `'install' | 'return_to_stock' | 'to_refill' | 'from_refill' | 'write_off'`.
- Заголовок, поля, дефолты — строго по Copywriting Contract §Поля OperationModal.
- Состояние заряда — `Select` из `cartridge_states` (1 Полный / 2 Частичный / 3 Пустой). Всегда редактируемо.
- Кнопка подтверждения — `variant="primary"`, `loading` во время запроса, блокируется при валидационных ошибках. Текст — operation-specific: «Установить» / «Вернуть на склад» / «Отправить на заправку» / «Вернуть с заправки» / «Списать».
- Кнопка «Отмена» — `variant="secondary"`, закрывает модал без действия.
- Успех: Toast `success` + модал закрывается + список обновляется.
- Ошибка: Toast `error` + модал остаётся открытым.

### 5. ModelFormModal + CompatibilityEditor

- `size="wide"` (960px) — из-за CompatibilityEditor.
- Поле «Цвет» скрывается через Svelte `{#if kindId !== 2}` (kind=2 = Фотобарабан). Анимация не нужна — простой if.
- CompatibilityEditor: каждая пара Бренд + Модель принтера в отдельной строке. Кнопка «+ Добавить принтер» добавляет пустую строку. Кнопка ✕ справа от строки удаляет пару. Focus-open autocomplete на каждом поле (источник: DISTINCT из `cartridge_model_compatibility`).
- Сохранение модели: валидация «Бренд не пустой», «Модель не пустая», уникальность пары brand+model — на frontend перед submit.

### 6. CartridgeDetail — карточка + история

- Header: код (tabular-nums, `--font-size-display`, `--font-weight-semibold`), название модели (brand+model), статус-badge, состояние заряда.
- Секция «Расположение / Держатель»: текущее расположение + holder_name (если В работе).
- Секция «История перемещений»: хронологический список из audit_log (новые сверху). Каждая запись — одна строка высотой `--row-height-dense` (32px).
- Формат строки истории: `«{дата}» — {операция_label}; {доп_поля}`. Например: `«12.06.2026 — Установлен в принтер; выдал Иванов И.И., получил Петров П.П.; Каб. 305»`.
- Пустая история: inline-текст «История пуста» (не пустой экран с кнопкой).
- Кнопки действий («Установить», «Вернуть», «Отправить на заправку») — в header детальной панели, visible в зависимости от статуса. Паттерн как в `ActDetail.svelte` — кнопки `size="sm"`.

### 7. LowStockBanner

- Рендерится только если `cartridges_low_stock()` вернула непустой массив.
- Расположение: между CartridgesSearchAndTabs и CartridgesMasterDetail.
- Структура: иконка предупреждения (inline SVG 16×16, `--color-warning`) + заголовок «Низкий остаток картриджей» + список моделей ниже порога.
- Каждая строка модели: «{brand} {model} — {count} шт. на складе (порог: {threshold})».
- Не содержит кнопок действий в Phase 4 (ссылка на Phase 7 дашборд).

### 8. ModelListRow — строка модели

- Поля: бренд, модель, тип-badge (`Картридж` / `Фотобарабан`), цвет-badge (только для Картриджа), кол-во активных экземпляров.
- Kebab ⋮ — `aria-label="Действия с моделью {brand} {model}"` — контекстное меню только с «Редактировать» / «Удалить».
- Удаление при наличии живых экземпляров: показывает Toast `error` «Нельзя удалить модель: она используется {N} картриджами» — без открытия confirm-модала.

### 9. Автокомплит — focus-open паттерн (DEF-1)

- Применяется к: PersonAutocomplete, LocationAutocomplete, полям Бренд/Модель в ModelFormModal, полям Бренд/Модель принтера в CompatibilityEditor.
- Открывается при focus (без ввода), закрывается при blur/Escape/выборе.
- Источник данных: DISTINCT значения из соответствующих таблиц.
- Переиспользуются существующие компоненты без изменений.

### 10. Badge-цвета типа расходника

| Тип | CSS |
|-----|-----|
| Картридж | `Badge variant="accent"` (синий) |
| Фотобарабан | `Badge variant="default"` (серый) |

---

## Layout Contract

### CartridgesPage

```
<main>                              (page-content, padding: --space-lg)
  <CartridgesSearchAndTabs />       (search + 2 таба: Картриджи / Модели)
  {#if tab === 'cartridges'}
    <LowStockBanner />              (если есть low-stock; warning-цвет)
    <CartridgesMasterDetail>
      {#snippet master}
        <CartridgeFilters />        (switch-bar + тип + модель фильтры)
        <CartridgesList />          (строки CartridgeListRow)
      {/snippet}
      {#snippet detail}
        <CartridgeDetail />         (карточка + history)
      {/snippet}
    </CartridgesMasterDetail>
  {:else}
    <ModelsList />                  (полноширинный список ModelListRow)
  {/if}
</main>
```

### CartridgesMasterDetail

- Grid: `grid-template-columns: 35% 65%`, `gap: --space-md`.
- Master panel: `background: --color-surface`, `border: 1px solid --color-border`, `border-radius: --radius-md`.
- Detail panel: `background: --color-bg`, `border: 1px solid --color-border`, `border-radius: --radius-md`.
- Breakpoint < 1100px: `grid-template-columns: 380px 1fr`, `min-width: 900px` (горизонтальный scroll у родителя, как Phase 2/3).

### OperationModal layout

- Поля формы: grid `1fr` (одна колонка). Gap: `--space-md`.
- Label → Input/Select/Autocomplete/Textarea вертикально.
- Hint-текст под полем: `font-size: --font-size-label`, `color: --color-text-muted`.

### ModelFormModal layout

- `size="wide"` (960px).
- Основные поля (тип, бренд, модель, цвет, примечание): grid `1fr 1fr` (две колонки).
- CompatibilityEditor — под основными полями, полная ширина.
- Каждая пара в CompatibilityEditor: `grid-template-columns: 1fr 1fr 28px` (Бренд принтера / Модель принтера / кнопка удаления).

---

## Registry Safety

| Registry | Blocks Used | Safety Gate |
|----------|-------------|-------------|
| shadcn official | none (не используется в этом проекте) | not applicable |
| Сторонние реестры | none | not applicable |

Phase 4 не добавляет новых внешних UI-пакетов. Используются исключительно компоненты из `$lib/components/` Phase 2 и feature-компоненты Phase 2/3 как образцы.

---

## Pre-Population Sources

| Источник | Решений использовано |
|---------|---------------------|
| `CONTEXT.md` (04-CONTEXT.md) | 16 locked decisions (D-Scope-01 → D-Search-01) |
| `RESEARCH.md` (04-RESEARCH.md) | Архитектурные паттерны, component inventory |
| `REQUIREMENTS.md` §CART-01..12 | 12 требований (формулировки полей, поведение) |
| `ui/src/styles/_tokens.scss` | Полный design token set (цвета, spacing, типографика) |
| `DeviceContextMenu.svelte` | Portal + mousedown-outside паттерн |
| `DeviceFilters.svelte` | Switch-bar + count-badge паттерн |
| `ActsMasterDetail.svelte` | 35/65 grid layout |
| `ActDetail.svelte` | Detail panel structure |
| `Modal.svelte` | G-1 backdrop fix, размеры |
| `Badge.svelte` | Варианты badge |
| Пользователь (input) | 0 (все вопросы закрыты upstream-артефактами) |

---

## Checker Sign-Off

- [ ] Dimension 1 Copywriting: PASS
- [ ] Dimension 2 Visuals: PASS
- [ ] Dimension 3 Color: PASS
- [ ] Dimension 4 Typography: PASS
- [ ] Dimension 5 Spacing: PASS
- [ ] Dimension 6 Registry Safety: PASS

**Approval:** pending
