---
status: complete
phase: 24-base-components
source:
  - 24-01-SUMMARY.md
  - 24-02-SUMMARY.md
  - 24-03-SUMMARY.md
  - 24-04-SUMMARY.md
  - 24-05-SUMMARY.md
  - 24-06-SUMMARY.md
  - 24-07-SUMMARY.md
  - 24-08-SUMMARY.md
  - 24-09-SUMMARY.md
  - 24-10-SUMMARY.md
  - 24-11-SUMMARY.md
  - 24-12-SUMMARY.md
  - 24-13-SUMMARY.md
started: 2026-07-18T15:40:00Z
updated: 2026-07-18T16:20:00Z
---

## Current Test

[session complete]

## Tests

### 1. Витрина компонентов против референсов
expected: Под admin открыть #/showcase. Пять секций (Кнопки, Поля, Бейджи, Вкладки, Модалка) визуально совпадают с .dc.html-эталонами из .planning/reference/design-system-v2/. Это единственный пункт фазы, который человек ни разу не смотрел вживую — чекпоинт 24-07 был авто-одобрен под workflow.auto_advance.
result: pass

### 2. Кнопки — варианты и состояния
expected: 5 вариантов (primary/secondary/destructive/ghost/link) в двух размерах. Каждый визуально различим при наведении, фокусе с клавиатуры, нажатии, в disabled и в состоянии загрузки. Переходы плавные (~0.12s), без рывков.
result: pass

### 3. Поля ввода — состояния и двусторонний биндинг
expected: Input/Select/Textarea/Checkbox/Radio различимы в обычном состоянии, фокусе, ошибке и disabled. При вводе в поле demo-значение рядом обновляется на лету (это чинилось в раунде 1 — раньше молча не обновлялось).
result: pass

### 4. Числовое поле не ломает значение
expected: В поле с type="number" ввести цифры, затем полностью очистить его. Поле не подставляет null/NaN, не сбрасывается само и не блокирует дальнейший ввод. Связанное значение остаётся строкой.
result: pass

### 5. Бейджи — 5 тонов во всех 4 вариантах
expected: Мягкая подложка, сплошной, с точкой и счётчик-пилюля — в 5 тонах (нейтральный/акцент/успех/предупреждение/опасность). Ключевое: у варианта «счётчик» success/warning/danger теперь цветные, а не серые, и все 5 тонов одной высоты. Размер sm у счётчика реально меньше обычного.
result: pass

### 6. Вкладки
expected: Switch-bar показывает счётчики рядом с названиями, активная вкладка подчёркнута акцентным цветом. Segmented-вариант отрисован отдельно, с тенью.
result: issue
reported: "pass если это нормально, что у segmented-варианта подложка растягивается на всю ширину" (со скриншотом)
severity: cosmetic
diagnosis: |
  Не нормально. Эталон Tabs.dc.html:64 задаёт сегментированному контейнеру
  display:inline-flex — подложка обжимает содержимое. Tabs.svelte:132-137 это
  воспроизводит верно, компонент не при чём.
  Растягивает обёртка витрины: TabsSection.svelte:48-51 задаёт
  .variant-block { display:flex; flex-direction:column } без align-items,
  а дефолтный align-items:stretch перебивает inline-flex по поперечной оси.
  Underline-вариант не страдает — он и в эталоне полноширинный (Tabs.dc.html:43).
  Дефект локализован в витрине и не влияет на использование Tabs в приложении.
fix: ".variant-block { align-items: flex-start; } в TabsSection.svelte"

### 7. Модалка — внешний вид
expected: Затемняющий оверлей, шапка с заголовком, тело и футер с действиями. Скругление 12px, заметная тень (уровень 3).
result: pass

### 8. Модалка — клавиатура и фокус
expected: Escape закрывает модалку ровно один раз; в PdfPreviewModal Tab заходит в iframe и выходит из него в футер; выпадашки автодополнения внутри модалки ловятся Tab-ловушкой.
result: pass
note: Подтверждено пользователем вживую до старта UAT (2026-07-18).

### 9. Переключение темы без «размазывания» (D-09)
expected: Быстро переключить light→dark→system несколько раз подряд, глядя на кнопки/вкладки/бейджи. Цвет меняется мгновенно, без видимого перетекания. Правило подавления transition чинилось в раунде 1 — до этого оно не работало вовсе.
result: pass

### 10. Доступ к витрине не-админом (D-02)
expected: Войти как manager, вбить #/showcase прямо в адрес. По D-02 доступ должен быть admin-only. Известно из верификации: пункт сайдбара скрыт корректно, но сам маршрут не гейтится — manager, вероятно, страницу получит. Нужно подтвердить фактическое поведение.
result: issue
reported: "По адресу #/showcase у Специалиста открывается Витрина и в сайдбаре самого пункта на Витрину нет."
severity: major
diagnosis: |
  D-02 не выполнен для manager. App.svelte:59-69 раздваивает роутинг по роли:
  employee уходит в employeeRoutes, где /showcase отсутствует и срабатывает
  catch-all '*': AccessDenied (routes.ts:39) — поэтому сотрудник получает отказ.
  Manager попадает в else-ветку с общей картой routes, где
  '/showcase': ComponentShowcasePage (routes.ts:28) не гейтится ролью вовсе.
  Ролевая проверка стоит только на пункте сайдбара
  (sidebar-config.ts:31, roles: ['admin']) — меню скрыто, страница доступна.
  Первая попытка проверки прошла под employee и D-02 не затрагивала;
  подтверждено повторно под manager.
not_a_phase_24_regression: |
  Тот же незагейченный паттерн у /users и /settings — маршруты в общей карте
  ролями не защищены нигде. Это архитектурный пробел ролевого гейта маршрутов,
  а не дефект базовых компонентов. Чинить отдельной задачей, а не в фазе 24.

## Summary

total: 10
passed: 8
issues: 2
pending: 0
skipped: 0

## Gaps

- truth: "Segmented-вариант вкладок обжимает подложку по содержимому, как в эталоне Tabs.dc.html:64 (display:inline-flex)"
  status: failed
  reason: "User reported: pass если это нормально, что у segmented-варианта подложка растягивается на всю ширину. Диагностика: .variant-block в TabsSection.svelte:48-51 — flex-column без align-items, дефолтный stretch перебивает inline-flex компонента."
  severity: cosmetic
  test: 6
  artifacts:
    - path: "ui/src/features/showcase/sections/TabsSection.svelte"
      issue: "строки 48-51: .variant-block { display:flex; flex-direction:column } растягивает inline-flex потомка на всю ширину"
  missing:
    - "Добавить align-items: flex-start в .variant-block"
    - "Пересобрать витрину и убедиться, что подложка segmented обжимает содержимое, а underline остаётся полноширинным"

- truth: "Доступ к /showcase admin-only (D-02) — не-админ не получает страницу даже по прямому хэшу"
  status: failed
  reason: "User confirmed: по адресу #/showcase у Специалиста (manager) открывается Витрина, пункта в сайдбаре при этом нет. Ролевой гейт стоит только на пункте меню, не на маршруте."
  severity: major
  test: 10
  scope: cross-cutting
  artifacts:
    - path: "ui/src/routes.ts"
      issue: "строка 28: '/showcase': ComponentShowcasePage в общей карте routes без ролевой проверки"
    - path: "ui/src/App.svelte"
      issue: "строки 59-69: роутинг ветвится только employee vs остальные; admin и manager делят одну карту routes"
  missing:
    - "Ролевой гейт на уровне маршрута, а не только пункта сайдбара"
  defer_reason: |
    Не регрессия фазы 24: тот же паттерн у /users и /settings. Это общий пробел
    ролевой защиты маршрутов, выходящий за рамки базовых компонентов.
    Рекомендуется отдельная задача/фаза, а не gap-closure раунд 3 в фазе 24.
