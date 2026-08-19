---
quick_id: 260819-vfg
slug: settings-storage-backups
phase: 260819-vfg
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - ui/src/features/settings/SettingsSubNav.svelte
  - ui/src/pages/SettingsPage.svelte
autonomous: true
requirements: [VFG-01]
must_haves:
  truths:
    - "В подменю раздела «Настройки» вкладки «Бэкапы» больше нет — остаются только Сеть, Организация, Хранилище, Порог остатка, Шаблоны, Active Directory"
    - "На вкладке «Хранилище» пользователь видит блок «Хранилище данных», а сразу под ним, в той же вкладке — блок «Бэкапы» (ручной бэкап, автобэкап: папка/расписание/ретенция) с тем же внешним видом карточки, что и раньше"
    - "Функциональность бэкапов (запуск вручную, выбор папки, расписание, ретенция, сохранение настроек) работает без изменений — бэкенд-команды и данные не тронуты"
    - "Никакой скрытый/устаревший код (URL-хэш, localStorage, тесты, документация в живом коде) не ссылается на удалённый ключ вкладки 'backup' так, чтобы это ломало навигацию — проверено: activeSection — чисто локальное состояние компонента SettingsPage без персистентности и без адресации по URL, deep-link на конкретную вкладку настроек отсутствует как класс"
  artifacts:
    - path: "ui/src/features/settings/SettingsSubNav.svelte"
      provides: "Массив SECTIONS без записи { key: 'backup', ... } — 6 вкладок вместо 7"
      contains: "key: 'storage'"
    - path: "ui/src/pages/SettingsPage.svelte"
      provides: "Ветка activeSection === 'storage' рендерит подряд <StorageSettings /> и <BackupSettings />; отдельной ветки activeSection === 'backup' больше нет"
      contains: "<BackupSettings"
  key_links:
    - from: "ui/src/pages/SettingsPage.svelte {:else if activeSection === 'storage'}"
      to: "StorageSettings + BackupSettings компоненты"
      via: "оба компонента — прямые дети .settings-content (существующий flex-column + gap: var(--tr-space-xl)), без нового wrapper-а — уже даёт вертикальный стек карточек с отступом"
      pattern: "<StorageSettings"
---

<objective>
Объединить разделы «Хранилище» и «Бэкапы» в настройках (Svelte 5, ui/) в одну вкладку
«Хранилище»: карточка «Бэкапы» переезжает под карточку «Хранилище данных» внутри той же
вкладки, вкладка «Бэкапы» убирается из подменю настроек (`SettingsSubNav.svelte`). Чисто
фронтенд-реорганизация — компоненты `StorageSettings.svelte` и `BackupSettings.svelte`,
бэкенд-команды (`settings_get_db_path`, `settings_get_backup_config`,
`settings_save_backup_config`, `backup_run_manual` и т.д.) и структура БД не меняются.

Purpose: два блока относятся к одному смысловому разделу (хранение данных приложения) — не
нужно переключать отдельную вкладку ради настроек резервного копирования.

Output: `SettingsSubNav.svelte` с 6 вкладками вместо 7 (без «Бэкапы»); `SettingsPage.svelte`,
где вкладка «Хранилище» показывает обе карточки друг под другом.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md

<interfaces>
Расследование deep-link/персистентности (сделано на этапе планирования, повторно исполнителю
делать не нужно): во всём `ui/src` ключ секции `'backup'` используется ТОЛЬКО в двух файлах —
`SettingsSubNav.svelte` (массив `SECTIONS`) и `SettingsPage.svelte` (ветка `{:else if}`), оба
правятся этим планом. `activeSection` в `SettingsPage.svelte` — чисто локальный `$state`
(`let activeSection = $state('network')`), не читается из URL/hash (роутер `svelte-spa-router`
маршрутизирует только `'/settings'` целиком, без под-параметра секции, см. `ui/src/routes.ts`)
и никуда не сохраняется (`localStorage`/cookie). Тестов/скриптов, ссылающихся на ключ
`'backup'` вкладки настроек, в кодовой базе нет. Вывод: алиас старого id на 'storage' НЕ
требуется — удаляемая ветка не имеет ни одного живого пути навигации к ней извне.

SettingsSubNav.svelte — ТЕКУЩЕЕ содержимое (полностью прочитано на этапе планирования):

  const SECTIONS = [
    { key: 'network', label: 'Сеть' },
    { key: 'org', label: 'Организация' },
    { key: 'storage', label: 'Хранилище' },
    { key: 'backup', label: 'Бэкапы' },          // <- эту строку удалить
    { key: 'threshold', label: 'Порог остатка' },
    { key: 'templates', label: 'Шаблоны' },
    { key: 'ad', label: 'Active Directory' },
  ] as const;

  Комментарий над массивом (строка 4-5) содержит фразу «7 sections have no counters» —
  обновить на «6 sections» при удалении строки, чтобы комментарий не врал о количестве.

SettingsPage.svelte — ТЕКУЩАЯ релевантная разметка (полностью прочитана на этапе планирования,
строки могут сместиться, ре-читать перед правкой не обязательно — блок маленький и найдётся по
уникальным строкам ниже):

    {:else if activeSection === 'storage'}
      <!-- Хранилище данных (SET-03) -->
      <StorageSettings />
    {:else if activeSection === 'backup'}
      <!-- Бэкапы (SET-05, SET-06, SET-07) -->
      <BackupSettings />
    {:else if activeSection === 'threshold'}

  Импорт `BackupSettings` (строка 5, `import BackupSettings from
  '../features/settings/BackupSettings.svelte';`) остаётся — компонент по-прежнему
  используется, просто в другой ветке.

  `.settings-content` (контейнер, куда рендерится активная секция) уже
  `display: flex; flex-direction: column; gap: var(--tr-space-xl);` — если в одной ветке
  оказываются два прямых дочерних компонента подряд, они автоматически стекаются вертикально
  с нужным отступом; отдельный wrapper-div не нужен.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Объединить вкладки «Хранилище» и «Бэкапы» в настройках</name>
  <files>ui/src/features/settings/SettingsSubNav.svelte, ui/src/pages/SettingsPage.svelte</files>
  <action>
В `SettingsSubNav.svelte`: удалить из массива `SECTIONS` строку `{ key: 'backup', label:
'Бэкапы' },`. В комментарии над массивом (упоминает «7 sections have no counters») поправить
число на 6, отражая новый размер массива.

В `SettingsPage.svelte`: удалить ветку `{:else if activeSection === 'backup'}` целиком (вместе
с комментарием `<!-- Бэкапы (SET-05, SET-06, SET-07) -->` и строкой `<BackupSettings />` внутри
неё). В ветке `{:else if activeSection === 'storage'}` добавить `<BackupSettings />` сразу
после `<StorageSettings />` (тот же уровень вложенности, без обёртки) — обновить комментарий
над блоком на `<!-- Хранилище данных (SET-03) + Бэкапы (SET-05, SET-06, SET-07) — объединены в
один раздел «Хранилище» -->`. Импорт `BackupSettings` в начале файла оставить без изменений —
компонент по-прежнему используется. Не трогать содержимое `StorageSettings.svelte` и
`BackupSettings.svelte` — только место, откуда они рендерятся.

Расследование deep-link/персистентности уже сделано на этапе планирования (см. `<interfaces>`
выше) — активный alias/редирект со старого ключа `'backup'` на `'storage'` не требуется, так
как ни один живой путь (URL, localStorage, тесты) на него не ссылается.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && grep -q "key: 'backup'" ui/src/features/settings/SettingsSubNav.svelte && echo FAIL_BACKUP_TAB_STILL_LISTED || echo OK_BACKUP_TAB_REMOVED; grep -q "activeSection === 'backup'" ui/src/pages/SettingsPage.svelte && echo FAIL_BACKUP_BRANCH_STILL_PRESENT || echo OK_BACKUP_BRANCH_REMOVED; grep -q "import BackupSettings from '../features/settings/BackupSettings.svelte';" ui/src/pages/SettingsPage.svelte && echo OK_IMPORT_KEPT || echo FAIL_IMPORT_REMOVED; awk "/activeSection === 'storage'/,/activeSection === 'threshold'/" ui/src/pages/SettingsPage.svelte | grep -q "<StorageSettings" && echo OK_STORAGE_PRESENT || echo FAIL_STORAGE_MISSING; awk "/activeSection === 'storage'/,/activeSection === 'threshold'/" ui/src/pages/SettingsPage.svelte | grep -q "<BackupSettings" && echo OK_BACKUP_UNDER_STORAGE || echo FAIL_BACKUP_NOT_MOVED; pnpm --dir ui run svelte-check 2>&1 | tail -30 && pnpm --dir ui build 2>&1 | tail -20</automated>
  </verify>
  <done>SettingsSubNav.svelte больше не содержит вкладку 'backup' (6 вкладок в SECTIONS); SettingsPage.svelte больше не имеет отдельной ветки activeSection === 'backup'; ветка activeSection === 'storage' рендерит подряд StorageSettings и BackupSettings; импорт BackupSettings сохранён; pnpm --dir ui run svelte-check и pnpm --dir ui build проходят без ошибок.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|--------------|
| Пользователь (admin) → раздел «Настройки» | Изменения этого плана — чисто фронтендовая реорганизация навигации (перенос уже существующей карточки между вкладками одной и той же страницы). Ни одна задача не меняет бэкенд-команды, не добавляет новый источник ввода и не расширяет сетевую поверхность. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|------------------|
| T-vfg-01 | Tampering (supply chain) | N/A | accept | Задача не добавляет новых npm-зависимостей и не запускает установку пакетов — только правки существующих .svelte-файлов, переиспользующие уже присутствующие в кодовой базе компоненты (StorageSettings, BackupSettings). Package Legitimacy Gate не применим. |
| T-vfg-02 | Elevation of Privilege / Information Disclosure | Вкладка «Хранилище» в SettingsPage.svelte | accept | Оба объединяемых блока и раньше были доступны из одного и того же раздела «Настройки» тому же кругу пользователей (admin); перенос карточки между вкладками одной страницы не меняет ролевую видимость и не открывает новых данных/действий, которые не были доступны раньше. |
</threat_model>

<verification>
1. `pnpm --dir ui run svelte-check` — 0 ошибок.
2. `pnpm --dir ui build` — успешная сборка.
3. Визуальная проверка выполняется пользователем в живом приложении (UAT) — синтетические
   харнессы не считаются верификацией для Svelte/WKWebView-приложения (см. проектный урок
   «Synthetic harness not verification»). Проверить вручную: подменю «Настройки» показывает
   6 вкладок без «Бэкапы»; вкладка «Хранилище» показывает карточку «Хранилище данных», а под
   ней — карточку «Бэкапы» с рабочими кнопками (открыть папку, сменить расположение, ручной
   бэкап, выбор папки бэкапов, расписание, ретенция, сохранить).
</verification>

<success_criteria>
- Подменю раздела «Настройки» больше не содержит вкладку «Бэкапы» — 6 вкладок вместо 7.
- Вкладка «Хранилище» показывает карточку «Хранилище данных», а сразу под ней — карточку
  «Бэкапы», с тем же внешним видом и функциональностью, что и раньше.
- Бэкенд-команды и данные бэкапов/хранилища не изменены.
- `pnpm --dir ui run svelte-check` и `pnpm --dir ui build` проходят чисто.
</success_criteria>

<output>
Create `.planning/quick/260819-vfg-settings-storage-backups/260819-vfg-SUMMARY.md` when done
</output>
