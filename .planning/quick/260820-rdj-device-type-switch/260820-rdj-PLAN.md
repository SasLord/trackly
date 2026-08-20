---
phase: 260820-rdj-device-type-switch
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/trackly-infra/src/repos/printers_sqlite.rs
  - crates/trackly-app/src/services/device_service.rs
  - crates/trackly-app/tests/devices_type_conversion.rs
  - ui/src/lib/components/Modal.svelte
  - ui/src/lib/components/ActionMenu.svelte
  - ui/src/features/devices/DeviceFormModal.svelte
  - ui/src/features/devices/DeviceFormBody.svelte
  - ui/src/features/printers/PrinterDetail.svelte
autonomous: false
requirements: [RDJ-01, RDJ-02, RDJ-03, RDJ-04, RDJ-05, RDJ-06]
must_haves:
  truths:
    - "В попапе «Новое устройство»/«Редактирование устройства» сразу после заголовка в той же строке есть ghost-кнопка size=sm с иконкой трёх вертикальных точек (RDJ-01)"
    - "Клик по кнопке открывает контекстное меню с пунктами «Устройство» и «Принтер»; клавиатура (стрелки/Home/End/Escape) и outside-click работают как в остальных ActionMenu (RDJ-01)"
    - "Текущий выбранный тип отмечен галочкой цвета var(--tr-accent) — переиспользован паттерн Dropdown.svelte, без хардкода цвета (RDJ-03)"
    - "Заголовок попапа реактивно меняется между «Новое устройство»/«Новый принтер»/«Редактирование устройства»/«Редактирование принтера» при смене типа (RDJ-02)"
    - "Открытие попапа редактирования из PrinterDetail показывает заголовок «Редактирование принтера» (было — баг: всегда «Редактирование устройства») (RDJ-06)"
    - "Переключение Устройство→Принтер и сохранение атомарно создаёт строку printers с дефолтами (ip_address=NULL, community='public', snmp_version='v2c'); повторное сохранение с тем же типом не создаёт дубликат (RDJ-04)"
    - "Переключение Принтер→Устройство при сохранении показывает предупреждение о потере данных мониторинга (показания тонера, оповещения) ДО записи в БД; после подтверждения строка printers и связанные printer_readings/printer_alerts удаляются атомарно и безвозвратно (RDJ-05)"
    - "После конверсии в любую сторону списки «Устройства» и «Принтеры» корректно отражают перемещение записи между разделами (RDJ-06)"
  artifacts:
    - path: "crates/trackly-infra/src/repos/printers_sqlite.rs"
      provides: "exists_for_device_in_tx / delete_by_device_id_in_tx tx-helpers + unit tests"
      contains: "fn exists_for_device_in_tx"
    - path: "crates/trackly-app/src/services/device_service.rs"
      provides: "printer_repo field + sync_printer_row_in_tx wired into create()/update()/bulk_create()"
      contains: "fn sync_printer_row_in_tx"
    - path: "crates/trackly-app/tests/devices_type_conversion.rs"
      provides: "integration tests: create/update/bulk_create × Устройство↔Принтер, идемпотентность, cascade readings/alerts"
      contains: "sync_printer_row"
    - path: "ui/src/lib/components/Modal.svelte"
      provides: "необязательный titleExtra snippet, рендерится в modal-header рядом с заголовком"
      contains: "titleExtra"
    - path: "ui/src/lib/components/ActionMenu.svelte"
      provides: "вариант триггера variant='ghost-sm' (28px, без бордера, ghost-стиль) поверх старого default-бордер-варианта"
      contains: "ghost-sm"
    - path: "ui/src/features/devices/DeviceFormModal.svelte"
      provides: "typeId $state + реактивный modalTitle + меню выбора типа в titleExtra с галочкой"
      contains: "PRINTER_TYPE_ID"
    - path: "ui/src/features/devices/DeviceFormBody.svelte"
      provides: "typeId в DeviceNew/DevicePatch payload + inline подтверждение потери данных перед downgrade-save"
      contains: "confirmDowngrade"
    - path: "ui/src/features/printers/PrinterDetail.svelte"
      provides: "onSaved вызывает onRefresh() — список принтеров обновляется после downgrade"
      contains: "onRefresh()"
  key_links:
    - from: "ui/src/features/devices/DeviceFormModal.svelte"
      to: "ui/src/lib/components/ActionMenu.svelte"
      via: "titleExtra snippet, variant='ghost-sm'"
      pattern: "variant=\"ghost-sm\""
    - from: "ui/src/features/devices/DeviceFormBody.svelte"
      to: "devices.update / devices.bulkCreate"
      via: "DevicePatch.type_id / DeviceNew.type_id = typeId prop"
      pattern: "type_id: typeId"
    - from: "crates/trackly-app/src/services/device_service.rs"
      to: "crates/trackly-infra/src/repos/printers_sqlite.rs"
      via: "sync_printer_row_in_tx() внутри той же writer-транзакции, что и devices INSERT/UPDATE"
      pattern: "sync_printer_row_in_tx"
    - from: "printers row DELETE"
      to: "printer_readings / printer_alerts"
      via: "ON DELETE CASCADE (V022/V023)"
      pattern: "DELETE FROM printers WHERE device_id"
---

<objective>
Добавить выбор типа устройства (Устройство/Принтер) в попап создания/редактирования устройства (`DeviceFormModal`/`DeviceFormBody`) с полной конверсией записи на бэкенде: переключение типа атомарно синхронизирует строку `printers` (создаёт с дефолтами при переходе в Принтер, удаляет вместе с историей мониторинга — с подтверждением пользователя — при переходе в Устройство). Заодно чинит существующий баг в `PrinterDetail.svelte`, где попап редактирования принтера всегда показывал заголовок «Редактирование устройства».

Purpose: сейчас тип устройства при создании хардкодится (`type_id: 1`), а при редактировании не меняется вовсе (`type_id: null`) — единственный способ завести принтер отдельная форма (`PrinterCreateModal`), а «переклассифицировать» ошибочно заведённую запись (типичный случай — свитч, найденный SNMP-дискавери и заведённый как принтер) невозможно без ручного вмешательства в БД.

Output: `ActionMenu`/`Modal` получают переиспользуемые расширения (ghost-sm триггер, titleExtra слот); `DeviceFormModal`/`DeviceFormBody` получают переключатель типа с реактивным заголовком и inline-подтверждением потери данных; `DeviceService` получает атомарную идемпотентную синхронизацию `printers` при create/update/bulk_create с интеграционными тестами.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md

<interfaces>
<!-- ============================================================= -->
<!-- BACKEND — текущее состояние (BEFORE) -->
<!-- ============================================================= -->

<!-- crates/trackly-infra/src/repos/printers_sqlite.rs — tx-helpers уже существуют
     (create_in_tx, upsert_reading_in_tx, upsert_alert_in_tx, update_last_seen_in_tx,
     prune_old_readings_in_tx, set_current_cartridge_in_tx, fetch_in_tx). Добавить
     exists_for_device_in_tx / delete_by_device_id_in_tx в тот же impl-блок
     (секция "Tx-helpers (NOT in trait)", после fetch_in_tx, перед "Private helpers"). -->
```rust
/// Проверить, существует ли строка `printers` для данного device_id.
/// Гейт идемпотентности для конверсии типа устройства (quick 260820-rdj).
pub fn exists_for_device_in_tx(
    &self,
    tx: &Transaction<'_>,
    device_id: i64,
) -> Result<bool, AppError> {
    tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM printers WHERE device_id = ?1)",
        params![device_id],
        |r| r.get(0),
    )
    .map_err(map_rusqlite)
}

/// Удалить строку `printers` для device_id, если она есть. Идемпотентно —
/// 0 затронутых строк не считается ошибкой. Каскадно удаляет
/// `printer_readings`/`printer_alerts` через `ON DELETE CASCADE` (V022/V023).
/// Quick 260820-rdj (конверсия Принтер → Устройство).
pub fn delete_by_device_id_in_tx(
    &self,
    tx: &Transaction<'_>,
    device_id: i64,
) -> Result<(), AppError> {
    tx.execute(
        "DELETE FROM printers WHERE device_id = ?1",
        params![device_id],
    )
    .map_err(map_rusqlite)?;
    Ok(())
}
```

<!-- crates/trackly-app/src/services/device_service.rs — текущий struct (BEFORE, строки 45-53) -->
```rust
#[derive(Clone)]
pub struct DeviceService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
    pub(crate) repo: Arc<SqliteDeviceRepository>,
    #[allow(dead_code)]
    pub(crate) csv_sessions: Arc<ImportSessionStore>,
}
```
Целевой struct (добавить поле `printer_repo`, порядок полей — после `repo`):
```rust
#[derive(Clone)]
pub struct DeviceService {
    pub writer: Arc<WriterHandle>,
    pub readers: Arc<ReaderPool>,
    pub(crate) clock: Arc<dyn Clock + Send + Sync>,
    pub(crate) repo: Arc<SqliteDeviceRepository>,
    pub(crate) printer_repo: Arc<SqlitePrinterRepository>,
    #[allow(dead_code)]
    pub(crate) csv_sessions: Arc<ImportSessionStore>,
}
```
`new()` (строки 59-71) — добавить инициализацию `printer_repo: Arc::new(SqlitePrinterRepository)` в `Self { ... }`. Сигнатура `new(writer, readers, clock)` НЕ меняется — все существующие вызовы (`DeviceService::new(writer, readers, clock)` в `devices_crud.rs` и др.) остаются рабочими без изменений.

Импорты (добавить рядом с существующими `use` в начале файла):
```rust
use trackly_core::domain::printers::PrinterNew;
use trackly_infra::repos::SqlitePrinterRepository;
```

Новая private-функция (добавить в `impl DeviceService`, например сразу после `validate_new`):
```rust
/// device_types seed ids (V001): устройство=1, принтер=2.
const DEVICE_TYPE_ID: i64 = 1;
const PRINTER_TYPE_ID: i64 = 2;

/// Синхронизировать строку `printers` с `type_id` (quick 260820-rdj: полная
/// конверсия Устройство ⇄ Принтер). Идемпотентна — безопасно вызывать при
/// каждом create/update/bulk_create независимо от того, менялся ли type_id
/// в этом конкретном вызове; выполняется ВНУТРИ той же транзакции, что и
/// INSERT/UPDATE устройства, поэтому конверсия атомарна (никогда не оставляет
/// devices.type_id=2 без строки printers, и наоборот).
fn sync_printer_row_in_tx(
    printer_repo: &SqlitePrinterRepository,
    tx: &rusqlite::Transaction<'_>,
    device_id: i64,
    type_id: i64,
    now_utc: i64,
) -> Result<(), AppError> {
    match type_id {
        PRINTER_TYPE_ID => {
            if !printer_repo.exists_for_device_in_tx(tx, device_id)? {
                printer_repo.create_in_tx(
                    tx,
                    &PrinterNew {
                        device_id,
                        ip_address: None,
                        community_raw: "public".to_string(),
                        snmp_version: "v2c".to_string(),
                        oid_profile_id: None,
                        usb_host_device_id: None,
                    },
                    now_utc,
                )?;
            }
        }
        DEVICE_TYPE_ID => {
            printer_repo.delete_by_device_id_in_tx(tx, device_id)?;
        }
        _ => {} // неизвестный/будущий тип — printers не трогаем (вне области decision)
    }
    Ok(())
}
```

Три места вызова (каждое требует `let printer_repo = self.printer_repo.clone();` перед `move`-closure, аналогично уже существующему `let repo = self.repo.clone();`):

1. `create()` (строка ~118-149) — сразу после `let id = repo.create_in_tx(&tx, &domain_new, now)?;`:
```rust
let id = repo.create_in_tx(&tx, &domain_new, now)?;
Self::sync_printer_row_in_tx(&printer_repo, &tx, id, domain_new.type_id, now)?;
let after = repo.get_in_tx(&tx, id)?;
```

2. `update()` (строка ~228-273) — сразу после `let after = repo.update_in_tx(&tx, id, version, &domain_patch, now)?;`. Синхронизация ключуется от ФИНАЛЬНОГО `after.type_id` (не от `patch.type_id`, который чаще всего `None`) — так вызов остаётся идемпотентным при КАЖДОМ update, а не только когда тип явно меняется:
```rust
let after = repo.update_in_tx(&tx, id, version, &domain_patch, now)?;
Self::sync_printer_row_in_tx(&printer_repo, &tx, id, after.type_id, now)?;
let after_json = ...
```

3. `bulk_create()` (строка ~1018-1039) — внутри `for _ in 0..count { ... }`, сразу после `let id = repo.create_in_tx(&tx, &domain_new, now)?;`:
```rust
let id = repo.create_in_tx(&tx, &domain_new, now)?;
Self::sync_printer_row_in_tx(&printer_repo, &tx, id, domain_new.type_id, now)?;
```

<!-- ============================================================= -->
<!-- FRONTEND — текущее состояние (BEFORE) -->
<!-- ============================================================= -->

<!-- ui/src/lib/components/Modal.svelte — Props (строки 4-13) -->
```ts
interface Props {
  open: boolean;
  title: string;
  size?: 'md' | 'wide' | 'xwide' | 'pdf-preview';
  onClose: () => void;
  children?: Snippet;
  footer?: Snippet;
}
const { open, title, size = 'md', onClose, children, footer }: Props = $props();
```
Целевое: добавить `titleExtra?: Snippet;` в Props и в деструктуризацию.

`modal-header` markup (строки 143-146, BEFORE):
```svelte
<header class="modal-header">
  <h2 id={titleId} class="modal-title">{title}</h2>
  <button type="button" class="modal-close" onclick={onClose} aria-label="Закрыть">×</button>
</header>
```
Целевое (оборачиваем title + titleExtra в общую группу — `justify-content: space-between` на `.modal-header` остаётся, но теперь распределяет 2 элемента: title-group и close-button, а не 3, что сохраняет «сразу после заголовка в той же строке»):
```svelte
<header class="modal-header">
  <div class="modal-title-group">
    <h2 id={titleId} class="modal-title">{title}</h2>
    {#if titleExtra}
      {@render titleExtra()}
    {/if}
  </div>
  <button type="button" class="modal-close" onclick={onClose} aria-label="Закрыть">×</button>
</header>
```
Новый CSS-класс (добавить в `<style lang="scss">` рядом с `.modal-title`):
```scss
.modal-title-group {
  display: flex;
  align-items: center;
  gap: var(--tr-space-xs);
  min-width: 0;
  flex: 1 1 auto;
}
```
Ни один существующий вызов `<Modal>` не передаёт `titleExtra` — поведение для них не меняется (снипет не рендерится).

<!-- ui/src/lib/components/ActionMenu.svelte — Props (строки 4-9, BEFORE) -->
```ts
interface Props {
  label?: string;
  children: Snippet;
}
const { label = 'Действия', children }: Props = $props();
```
Целевое:
```ts
interface Props {
  label?: string;
  /** 'default' — текущий бордер-триггер (36×36, используется в DevicesPage
   *  «Импорт и экспорт» — НЕ трогать). 'ghost-sm' — без бордера, 28px,
   *  ghost-стиль как Button variant="ghost" size="sm" (quick 260820-rdj). */
  variant?: 'default' | 'ghost-sm';
  children: Snippet;
}
const { label = 'Действия', variant = 'default', children }: Props = $props();
```
Триггер-кнопка (строки 78-93, BEFORE) — добавить `class:action-menu-trigger--ghost-sm`:
```svelte
<button
  type="button"
  class="action-menu-trigger"
  class:action-menu-trigger--ghost-sm={variant === 'ghost-sm'}
  aria-haspopup="menu"
  aria-expanded={open}
  aria-label={label}
  bind:this={triggerEl}
  onclick={() => (open = !open)}
  onkeydown={onTriggerKeydown}
>
  <svg width="18" height="18" viewBox="0 0 18 18" aria-hidden="true">
    <circle cx="9" cy="3.5" r="1.5" fill="currentColor" />
    <circle cx="9" cy="9" r="1.5" fill="currentColor" />
    <circle cx="9" cy="14.5" r="1.5" fill="currentColor" />
  </svg>
</button>
```
Новый CSS-модификатор (добавить в `<style lang="scss">` сразу после `.action-menu-trigger { ... }`, наследует border-radius/flex/focus-visible от базового класса, переопределяет только размер/бордер/фон/цвет):
```scss
.action-menu-trigger--ghost-sm {
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  color: var(--tr-text-primary);

  &:hover {
    background: var(--tr-surface-sunken);
  }
}
```

<!-- ui/src/features/devices/DeviceFormModal.svelte — ПОЛНЫЙ текущий файл прочитан
     (92 строки) при планировании. Ключевые точки изменения: -->
```svelte
<!-- BEFORE (строки 28-30) -->
const isEdit = $derived(target !== null);
const modalTitle = $derived(isEdit ? 'Редактирование устройства' : 'Новое устройство');
const submitLabel = $derived(isEdit ? 'Сохранить' : 'Создать');
```
```svelte
<!-- AFTER -->
const DEVICE_TYPE_ID = 1;
const PRINTER_TYPE_ID = 2;

const isEdit = $derived(target !== null);
let typeId = $state(DEVICE_TYPE_ID);
const modalTitle = $derived.by(() => {
  const isPrinter = typeId === PRINTER_TYPE_ID;
  if (isEdit) return isPrinter ? 'Редактирование принтера' : 'Редактирование устройства';
  return isPrinter ? 'Новый принтер' : 'Новое устройство';
});
const submitLabel = $derived(isEdit ? 'Сохранить' : 'Создать');
```
BEFORE (строки 40-46, remount-effect) — добавить сброс `typeId` рядом со сбросом `openInstanceCounter`, тем же условием (`isOpen && !_wasOpen`), чтобы каждое открытие попапа брало актуальный `target?.type_id`:
```svelte
<!-- BEFORE -->
$effect(() => {
  const isOpen = open;
  if (isOpen && !_wasOpen) {
    openInstanceCounter += 1;
  }
  _wasOpen = isOpen;
});
```
```svelte
<!-- AFTER -->
$effect(() => {
  const isOpen = open;
  if (isOpen && !_wasOpen) {
    openInstanceCounter += 1;
    typeId = target?.type_id ?? DEVICE_TYPE_ID;
  }
  _wasOpen = isOpen;
});
```
Импорт `ActionMenu` (добавить рядом с существующими импортами):
```ts
import ActionMenu from '$lib/components/ActionMenu.svelte';
```
`<Modal>`-вызов (строки 69-92, BEFORE) — добавить `titleExtra` снипет и передать `typeId` вниз в `DeviceFormBody`:
```svelte
<!-- BEFORE -->
<Modal {open} title={modalTitle} size="md" {onClose}>
  {#key openInstanceCounter}
    <DeviceFormBody
      {target}
      {stateHints}
      {onSaved}
      onLoading={(l) => (formLoading = l)}
      onCanSubmitChange={(can) => (formCanSubmit = can)}
      onRegisterSubmit={(fn) => (bodySubmitFn = fn)}
    />
  {/key}
  {#snippet footer()}
    ...
  {/snippet}
</Modal>
```
```svelte
<!-- AFTER -->
<Modal {open} title={modalTitle} size="md" {onClose} {titleExtra}>
  {#key openInstanceCounter}
    <DeviceFormBody
      {target}
      {stateHints}
      {typeId}
      {onSaved}
      onLoading={(l) => (formLoading = l)}
      onCanSubmitChange={(can) => (formCanSubmit = can)}
      onRegisterSubmit={(fn) => (bodySubmitFn = fn)}
    />
  {/key}
  {#snippet titleExtra()}
    <ActionMenu label="Тип устройства" variant="ghost-sm">
      <button type="button" role="menuitem" onclick={() => (typeId = DEVICE_TYPE_ID)}>
        <span class="type-menu-row">
          <span>Устройство</span>
          {#if typeId === DEVICE_TYPE_ID}
            <span class="type-menu-check" aria-hidden="true">✓</span>
          {/if}
        </span>
      </button>
      <button type="button" role="menuitem" onclick={() => (typeId = PRINTER_TYPE_ID)}>
        <span class="type-menu-row">
          <span>Принтер</span>
          {#if typeId === PRINTER_TYPE_ID}
            <span class="type-menu-check" aria-hidden="true">✓</span>
          {/if}
        </span>
      </button>
    </ActionMenu>
  {/snippet}
  {#snippet footer()}
    ...  <!-- без изменений -->
  {/snippet}
</Modal>
```
Добавить `<style lang="scss">` блок в конец файла (сейчас его нет — файл заканчивается на `</Modal>`, строка 92):
```scss
<style lang="scss">
  .type-menu-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    gap: var(--tr-space-xs);
  }
  .type-menu-check {
    color: var(--tr-accent);
    font-weight: var(--tr-font-weight-semibold);
  }
</style>
```

<!-- ui/src/features/devices/DeviceFormBody.svelte — ПОЛНЫЙ текущий файл прочитан
     (487 строк) при планировании. Ключевые точки изменения: -->
```ts
// BEFORE (Props, строки 29-43)
interface Props {
  target: DeviceDto | null;
  stateHints: string[];
  onSaved: () => void;
  onLoading: (_loading: boolean) => void;
  onCanSubmitChange: (_can: boolean) => void;
  onRegisterSubmit: (_fn: () => void) => void;
}
const { target, stateHints, onSaved, onLoading, onCanSubmitChange, onRegisterSubmit }: Props =
  $props();
```
```ts
// AFTER
interface Props {
  target: DeviceDto | null;
  stateHints: string[];
  /** Выбранный тип устройства (1=Устройство, 2=Принтер) — управляется
   *  ActionMenu в заголовке DeviceFormModal, не этим компонентом. */
  typeId: number;
  onSaved: () => void;
  onLoading: (_loading: boolean) => void;
  onCanSubmitChange: (_can: boolean) => void;
  onRegisterSubmit: (_fn: () => void) => void;
}
const {
  target,
  stateHints,
  typeId,
  onSaved,
  onLoading,
  onCanSubmitChange,
  onRegisterSubmit,
}: Props = $props();

const DEVICE_TYPE_ID = 1;
const PRINTER_TYPE_ID = 2;
```
Импорт (добавить рядом с существующими):
```ts
import Button from '$lib/components/Button.svelte';
```
`canSubmit` (BEFORE, строка 76-78) — добавить `confirmDowngrade`-гейт:
```ts
// BEFORE
const canSubmit = $derived(
  name.trim() !== '' && location.trim() !== '' && statusId !== '' && !submitting,
);
```
```ts
// AFTER
let confirmDowngrade = $state(false);

// Сбросить inline-подтверждение при любой смене типа (пользователь мог снова
// переключить меню в заголовке, пока подтверждение было открыто) — иначе
// подтверждение может «зависнуть» для уже неактуального перехода типа.
$effect(() => {
  typeId;
  confirmDowngrade = false;
});

const canSubmit = $derived(
  name.trim() !== '' &&
    location.trim() !== '' &&
    statusId !== '' &&
    !submitting &&
    !confirmDowngrade,
);
```
`handleSubmit` (BEFORE, строки 108-182) — разбить на гейт + `performSave`. `type_id: null`/`type_id: 1` заменяются на `typeId`:
```ts
// BEFORE (сигнатуры/фрагменты, полный текст см. файл)
async function handleSubmit() {
  if (!canSubmit) return;
  if (submitting) return;
  submitting = true;
  loading = true;
  fieldErrors = {};
  try {
    if (isEdit && target) {
      const patch: DevicePatch = {
        type_id: null,
        name: name.trim() || null,
        ...
      };
      const updated = await devices.update(target.id, currentVersion, patch);
      currentVersion = updated.version;
      pushToast('success', 'Устройство сохранено');
    } else {
      const newDevice: DeviceNew = {
        type_id: 1,
        name: name.trim(),
        ...
      };
      const qty = quantityDisabled ? 1 : Math.max(1, Math.min(100, quantity || 1));
      await devices.bulkCreate(newDevice, qty);
      ...
    }
    onSaved();
  } catch (e: unknown) {
    ... // без изменений
  } finally {
    loading = false;
    submitting = false;
  }
}
```
```ts
// AFTER
async function handleSubmit() {
  if (!canSubmit) return;
  if (submitting) return;

  // RDJ-05: перед сохранением конверсии Принтер→Устройство — подтверждение
  // потери данных мониторинга (показания тонера, активные оповещения).
  // confirmDowngrade сам исключён из canSubmit выше, так что повторный клик
  // по «Сохранить» сюда уже не попадёт — реальное сохранение запускает
  // отдельная кнопка «Да, сохранить» в inline-предупреждении (onclick={performSave}).
  const isDowngrade =
    isEdit && target?.type_id === PRINTER_TYPE_ID && typeId === DEVICE_TYPE_ID;
  if (isDowngrade && !confirmDowngrade) {
    confirmDowngrade = true;
    return;
  }

  await performSave();
}

async function performSave() {
  submitting = true;
  loading = true;
  fieldErrors = {};
  try {
    if (isEdit && target) {
      const patch: DevicePatch = {
        type_id: typeId,
        name: name.trim() || null,
        inventory_no: inventoryNo.trim() || null,
        serial_no: serialNo.trim() || null,
        model: model.trim() || null,
        specs: specs.trim() || null,
        kit: kit.trim() || null,
        state: stateField.trim() || null,
        location: location.trim() || null,
        location_id: null,
        status_id: parseInt(statusId, 10) || null,
      };
      const updated = await devices.update(target.id, currentVersion, patch);
      currentVersion = updated.version;
      pushToast('success', 'Устройство сохранено');
    } else {
      const newDevice: DeviceNew = {
        type_id: typeId,
        name: name.trim(),
        inventory_no: inventoryNo.trim() || null,
        serial_no: serialNo.trim() || null,
        model: model.trim() || null,
        specs: specs.trim() || null,
        kit: kit.trim() || null,
        state: stateField.trim() || null,
        location: location.trim() || null,
        location_id: null,
        status_id: parseInt(statusId, 10),
      };
      const qty = quantityDisabled ? 1 : Math.max(1, Math.min(100, quantity || 1));
      await devices.bulkCreate(newDevice, qty);
      if (qty === 1) {
        pushToast('success', typeId === PRINTER_TYPE_ID ? 'Принтер создан' : 'Устройство создано');
      } else {
        pushToast('success', `Создано ${qty} устройств`);
      }
    }
    onSaved();
  } catch (e: unknown) {
    // БЕЗ ИЗМЕНЕНИЙ — тот же catch-блок, что и в текущем файле (строки 161-177).
  } finally {
    loading = false;
    submitting = false;
    confirmDowngrade = false;
  }
}
```
Template (BEFORE, строка 185: `<form class="device-form" ...> ... </form>` — единственный корневой блок разметки) — обернуть в `{#if confirmDowngrade}...{:else}...{/if}`, форма целиком уходит в `{:else}` БЕЗ изменений содержимого полей:
```svelte
{#if confirmDowngrade}
  <div class="downgrade-confirm" role="alertdialog" aria-live="polite">
    <p>
      Тип устройства меняется с «Принтер» на «Устройство». История показаний тонера и активные
      оповещения по этому принтеру будут удалены безвозвратно.
    </p>
    <div class="downgrade-confirm-actions">
      <Button variant="secondary" onclick={() => (confirmDowngrade = false)}>Отмена</Button>
      <Button variant="destructive" loading={submitting} onclick={performSave}>
        Да, сохранить
      </Button>
    </div>
  </div>
{:else}
  <form class="device-form" onsubmit={(e) => { e.preventDefault(); e.stopPropagation(); }}>
    <!-- ВЕСЬ существующий контент формы (поля 1–10, строки 195-365) БЕЗ ИЗМЕНЕНИЙ -->
  </form>
{/if}
```
Добавить в конец `<style lang="scss">` (рядом с `.hint-chip`/`.state-hints*`):
```scss
.downgrade-confirm {
  display: flex;
  flex-direction: column;
  gap: var(--tr-space-md);
}

.downgrade-confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--tr-space-xs);
}
```
Footer-кнопка «Сохранить» в `DeviceFormModal.svelte` (`disabled={!formCanSubmit}`) автоматически дизейблится, пока `confirmDowngrade === true` (т.к. `canSubmit` уже включает `!confirmDowngrade`) — никаких изменений в `DeviceFormModal.svelte`'s footer не требуется, inline-кнопки берут действие на себя.

<!-- ui/src/features/printers/PrinterDetail.svelte — ПОЛНЫЙ файл прочитан при
     планировании. Единственное изменение — секция "данные устройства",
     DeviceFormModal onSaved (BEFORE): -->
```svelte
<DeviceFormModal
  open={deviceEditOpen}
  target={deviceData}
  onClose={() => (deviceEditOpen = false)}
  onSaved={() => {
    deviceEditOpen = false;
    if (printer) {
      devices.get(printer.deviceId).then((d) => (deviceData = d));
    }
  }}
/>
```
```svelte
<!-- AFTER: onRefresh() добавлен ПЕРВЫМ действием — после конверсии
     Принтер→Устройство эта запись должна пропасть из списка «Принтеры»
     (`onRefresh` уже приходит как prop, `PrinterDetail`'s Props interface,
     см. строку ~28: `onRefresh: () => void`). -->
<DeviceFormModal
  open={deviceEditOpen}
  target={deviceData}
  onClose={() => (deviceEditOpen = false)}
  onSaved={() => {
    deviceEditOpen = false;
    onRefresh();
    if (printer) {
      devices.get(printer.deviceId).then((d) => (deviceData = d));
    }
  }}
/>
```
Заголовок «Редактирование принтера» появляется автоматически (без правок разметки PrinterDetail) — DeviceFormModal теперь сам вычисляет заголовок из `target.type_id` (принтер здесь всегда `type_id=2`, `deviceData` уже загружается через `devices.get(p.deviceId)`).

<!-- Не требуется: специальных Tauri-команд / axum-роутов / бинды НЕ добавляется —
     DeviceNew.type_id / DevicePatch.type_id уже существуют в DTO и уже
     передаются через существующие devices_create/devices_update/
     devices_bulk_create команды. ui/src/bindings.ts НЕ регенерируется. -->
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Backend — атомарная синхронизация printers при конверсии типа устройства</name>
  <files>crates/trackly-infra/src/repos/printers_sqlite.rs, crates/trackly-app/src/services/device_service.rs, crates/trackly-app/tests/devices_type_conversion.rs</files>
  <behavior>
    - `create()` с `type_id=2` → возвращённый device.id имеет строку `printers` (ip_address=NULL, community='public', snmp_version='v2c').
    - `create()` с `type_id=1` (по умолчанию) → строки `printers` НЕТ.
    - `update()` меняет `type_id: 1→2` → строка `printers` появляется.
    - `update()` меняет `type_id: 2→1` для устройства с существующими `printer_readings`/`printer_alerts` → строка `printers` и обе дочерние таблицы пусты (cascade).
    - `update()` БЕЗ смены типа (`type_id: None`, тип остаётся 2) вызванный дважды подряд → ровно одна строка `printers`, ни ошибки, ни дубликата (идемпотентность).
    - `bulk_create()` с `type_id=2`, `count=3` → 3 строки `printers`, по одной на каждый созданный device.id.
  </behavior>
  <action>
Реализовать полную конверсию Устройство ⇄ Принтер согласно `<interfaces>` (снипеты BEFORE/AFTER — используй их как основной источник истины; при расхождении с текущим состоянием файла — сначала перечитай точные номера строк, т.к. они зафиксированы на момент планирования 2026-08-20).

1. `crates/trackly-infra/src/repos/printers_sqlite.rs`: добавить `exists_for_device_in_tx` и `delete_by_device_id_in_tx` в существующий impl-блок tx-helpers (секция "NOT in trait", после `fetch_in_tx`, перед `// Private helpers`). Добавить 2 unit-теста в существующий `#[cfg(test)] mod tests`: `exists_for_device_in_tx_reflects_printer_presence` (создать printer через `create_in_tx`, assert true; для другого device_id без printer — assert false) и `delete_by_device_id_in_tx_is_idempotent_and_cascades` (создать printer, вручную вставить строку в `printer_readings` и `printer_alerts` через `tx.execute`/`conn.execute` для этого `printer_id`, вызвать `delete_by_device_id_in_tx`, assert обе дочерние таблицы пусты; вызвать `delete_by_device_id_in_tx` ВТОРОЙ раз на тот же device_id — assert `Ok(())`, не паникует и не возвращает ошибку).

2. `crates/trackly-app/src/services/device_service.rs`: добавить импорты, поле `printer_repo`, константы `DEVICE_TYPE_ID`/`PRINTER_TYPE_ID`, функцию `sync_printer_row_in_tx` и три точки вызова (в `create()`, `update()`, `bulk_create()`) — точно как в `<interfaces>`. Важно: в `update()` синхронизация ключуется от `after.type_id` (финальное состояние ПОСЛЕ `COALESCE`), а не от `patch.type_id` — это гарантирует, что синхронизация остаётся корректной и идемпотентной при любом вызове `update()`, включая те, где `type_id` вообще не передан.

3. Новый файл `crates/trackly-app/tests/devices_type_conversion.rs` — интеграционные тесты, паттерн харнесса взять из `crates/trackly-app/tests/devices_crud.rs` (`make_service()` через `test_writer_and_readers()`, `DeviceService::new(writer, readers, clock)`, `minimal_new(name)` для `DeviceNew{type_id:1,...}`). Для проверки состояния `printers`/`printer_readings`/`printer_alerts` использовать `svc.readers.acquire()` (поле `pub readers`) и `svc.writer.execute(move |conn| {...}).await` (поле `pub writer`) для прямых SQL-запросов/сидинга — оба поля публичны на `DeviceService`, см. пример `crates/trackly-app/tests/acts_clone_handover.rs:297-299` (`let readers = svc.readers.clone(); let conn = readers.acquire();`) и `crates/trackly-app/tests/acts_date_source.rs:34-49` (`seed_device` через `writer.execute`). Тест-кейсы — покрыть все пункты из `<behavior>` выше (минимум 5 тестов: create-printer-type, create-device-type-no-row, update-upgrade, update-downgrade-with-cascade-seed, update-idempotent-double-call; опционально bulk_create). Фиктивные имена устройств (приватность — CLAUDE.md), например "Test Printer 1", "Test Device 1".

4. `cargo fmt` только на изменённых файлах, если форматирование разъехалось.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly && cargo test -p trackly-infra --lib printers_sqlite:: -- --test-threads=1 2>&1 | tail -60 && cargo test -p trackly-app --test devices_type_conversion -- --test-threads=1 2>&1 | tail -60</automated>
  </verify>
  <done>`cargo check -p trackly-core -p trackly-infra -p trackly-app` компилируется чисто; новые unit- и integration-тесты проходят; конверсия в обе стороны атомарна (одна writer-транзакция) и идемпотентна (повторный вызов с тем же type_id не создаёт дубликат и не падает).</done>
</task>

<task type="auto">
  <name>Task 2: Frontend — переключатель типа устройства в попапе + фикс заголовка PrinterDetail</name>
  <files>ui/src/lib/components/Modal.svelte, ui/src/lib/components/ActionMenu.svelte, ui/src/features/devices/DeviceFormModal.svelte, ui/src/features/devices/DeviceFormBody.svelte, ui/src/features/printers/PrinterDetail.svelte</files>
  <action>
Реализовать UI согласно `<interfaces>` (снипеты BEFORE/AFTER — основной источник истины). Порядок правок важен (interface-first): сначала общие компоненты (используются другими фичами — НЕ ломать существующее поведение для их текущих вызывающих), затем компонент-потребитель `DeviceFormModal`/`DeviceFormBody`, затем однострочный фикс в `PrinterDetail`.

1. `ui/src/lib/components/Modal.svelte`: добавить `titleExtra?: Snippet` в Props, обернуть `<h2>` в `.modal-title-group`, отрендерить `{@render titleExtra?.()}` внутри группы, добавить CSS-класс `.modal-title-group`. Все существующие вызовы `<Modal>` (PrinterCreateModal, RequestDetail и т.д.) не передают `titleExtra` — их вёрстка не меняется (снипет условно рендерится только если передан).

2. `ui/src/lib/components/ActionMenu.svelte`: добавить `variant?: 'default' | 'ghost-sm'` в Props (default = `'default'`, сохраняет текущий бордер-триггер для существующего вызова в `DevicesPage.svelte` «Импорт и экспорт» — НЕ трогать его поведение), добавить `class:action-menu-trigger--ghost-sm` на триггер-кнопку, добавить CSS-модификатор `.action-menu-trigger--ghost-sm`.

3. `ui/src/features/devices/DeviceFormModal.svelte`: добавить константы `DEVICE_TYPE_ID`/`PRINTER_TYPE_ID`, `let typeId = $state(...)`, реактивный `modalTitle` (4 варианта: Новое устройство / Новый принтер / Редактирование устройства / Редактирование принтера), сброс `typeId` в существующем remount-эффекте, импорт `ActionMenu`, передачу `typeId` в `DeviceFormBody`, `titleExtra` снипет с двумя `role="menuitem"` пунктами и галочкой на выбранном (цвет `var(--tr-accent)`, НЕ хардкодить hex), добавить `<style>` блок (файл сейчас его не имеет).

4. `ui/src/features/devices/DeviceFormBody.svelte`: принять `typeId: number` пропом, заменить хардкод `type_id: null`/`type_id: 1` на `type_id: typeId` в обоих payload (`DevicePatch`, `DeviceNew`), добавить `confirmDowngrade` state + сброс-эффект при смене `typeId`, включить `!confirmDowngrade` в `canSubmit`, разбить `handleSubmit` на гейт-проверку (`isEdit && target?.type_id === PRINTER_TYPE_ID && typeId === DEVICE_TYPE_ID`) + `performSave()`, обернуть форму в `{#if confirmDowngrade}...{:else}<form>...</form>{/if}` с inline-предупреждением (текст про потерю показаний тонера и оповещений) и двумя кнопками («Отмена» → `confirmDowngrade=false`, «Да, сохранить» → `performSave()`), импортировать `Button`, добавить CSS для `.downgrade-confirm`/`.downgrade-confirm-actions`.

5. `ui/src/features/printers/PrinterDetail.svelte`: добавить вызов `onRefresh()` первым действием в `onSaved` callback у `DeviceFormModal` (после `deviceEditOpen = false`, до `devices.get(...)`) — иначе после конверсии Принтер→Устройство список «Принтеры» останется со stale-записью.

6. Пересобрать Svelte SPA для LAN-браузер режима (per project-конвенция "Dev browser testing needs ui build"): `pnpm --dir ui build`.
  </action>
  <verify>
    <automated>cd /Users/madsas/Projects/trackly/ui && pnpm run svelte-check 2>&1 | tail -60 && pnpm run build 2>&1 | tail -40</automated>
  </verify>
  <done>`pnpm run svelte-check` проходит без новых ошибок типов; `pnpm run build` собирается чисто; попап показывает ghost-sm кебаб рядом с заголовком с меню «Устройство»/«Принтер», галочкой на выбранном и реактивным заголовком; `PrinterDetail` открывает форму редактирования с заголовком «Редактирование принтера» и обновляет список после сохранения.</done>
</task>

<task type="checkpoint:human-verify" gate="blocking">
  <name>Task 3: Human UAT — полный флоу конверсии типа устройства (Tauri desktop)</name>
  <action>Блокирующая ручная проверка полного флоу переключения типа устройства в живом cargo tauri dev — Task 1/2 автоматизировали всё, что можно; визуальные и интерактивные аспекты (меню, галочка, реактивный заголовок, inline-подтверждение) требуют человеческого подтверждения перед закрытием задачи.</action>
  <what-built>
    Полный флоу конверсии типа устройства: ghost-sm кебаб-меню в заголовке попапа (Устройство/Принтер) с галочкой на выбранном, реактивный заголовок попапа, атомарная бэкенд-синхронизация строки `printers` в обе стороны, inline-подтверждение потери данных мониторинга при переходе Принтер→Устройство, фикс заголовка в `PrinterDetail`.
  </what-built>
  <how-to-verify>
    1. Запустить `cargo tauri dev` (или уже запущенный dev-инстанс — учти "Worktree fix not in running app": фиксы должны быть в ветке, из которой реально запущен dev-процесс).
    2. Устройства → «+ Добавить устройство»: убедиться, что заголовок «Новое устройство», сразу справа от него — ghost-кнопка с тремя точками. Открыть меню — два пункта «Устройство»/«Принтер», на «Устройство» синяя галочка. Выбрать «Принтер» — заголовок меняется на «Новый принтер», галочка переезжает. Заполнить обязательные поля (Наименование/Статус/Расположение), сохранить.
    3. Перейти на страницу «Принтеры» — убедиться, что только что созданная запись там появилась (а НЕ в «Устройства»).
    4. Открыть карточку этого принтера → «Редактировать» в секции «Данные устройства»: заголовок попапа должен быть «Редактирование принтера» (не «Редактирование устройства» — это был баг).
    5. В этом попапе через кебаб-меню переключить тип на «Устройство», нажать «Сохранить»: должно появиться inline-предупреждение о потере данных мониторинга (показания тонера, оповещения) с кнопками «Отмена»/«Да, сохранить». Нажать «Да, сохранить».
    6. Убедиться, что запись пропала со страницы «Принтеры» и появилась на странице «Устройства» с тем же наименованием.
    7. Клавиатура: открыть кебаб-меню Tab'ом/Enter, проверить ArrowDown/ArrowUp/Home/End/Escape между пунктами «Устройство»/«Принтер».
    8. (Опционально) Проверить LAN-браузер режим: убедиться, что `ui/dist` пересобран (Task 2, шаг 6) и то же поведение воспроизводится в браузере.
  </how-to-verify>
  <resume-signal>Напиши "approved" или опиши, что не так.</resume-signal>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| UI (выбор типа + inline-подтверждение) → Tauri invoke / axum HTTP → `DeviceService.create/update/bulk_create` | Untrusted `type_id: i64` пересекает границу в составе `DeviceNew`/`DevicePatch` — уже существующее поле, конверсия лишь добавляет серверную реакцию на его значение. |
| `DeviceService` write-path → `SqlitePrinterRepository` (создание/удаление `printers`) | Внутренняя граница, внешний ввод не пересекает — но операция деструктивна (безвозвратное удаление `printer_readings`/`printer_alerts`). |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-rdj-01 | Tampering | `sync_printer_row_in_tx` (`type_id` из клиента) | mitigate | `devices.type_id` имеет `REFERENCES device_types(id)` (V003) + `PRAGMA foreign_keys=ON` — недопустимый `type_id` отклоняется на уровне SQLite ДО того, как `sync_printer_row_in_tx` вообще увидит новую строку; сама функция дополнительно no-op'ает на любом значении кроме 1/2 (`_ => {}`), не паникует и не создаёт частичное состояние. |
| T-rdj-02 | Repudiation / Data Loss | Конверсия Принтер→Устройство (удаление `printer_readings`/`printer_alerts`) | mitigate | Обязательный inline-чекпоинт подтверждения в UI (`confirmDowngrade`) перед отправкой PATCH — пользователь явно видит текст о потере истории показаний/оповещений и должен нажать «Да, сохранить». Остаточный риск: прямой вызов API в обход UI (Tauri invoke/HTTP) пропускает подтверждение — принято (accept), т.к. `DeviceService` в целом ещё не имеет authorize()-гейта (Phase 2 scope, см. T-rdj-03) — это предсуществующее свойство приложения, не вводится этой фичей. |
| T-rdj-03 | Elevation of Privilege | `DeviceService.create/update/bulk_create` | accept | Ни один из этих методов сейчас не вызывает `authorize()` (в отличие от `PrinterService`, ср. `Action::MutatePrinters`) — `user_id_opt: Option<i64> = None` зафиксирован с Phase 2 («no auth yet»). Эта фича не меняет и не расширяет данный пробел (то же самое верно уже сегодня для любого другого поля `DevicePatch`) — ретрофит авторизации на уровне `DeviceService` вне области этой quick-задачи. |
| T-rdj-04 | Integrity (atomicity) | `sync_printer_row_in_tx` внутри writer-транзакции | mitigate | Синхронизация `printers` выполняется той же `rusqlite::Transaction`, что и INSERT/UPDATE `devices` (единый `BEGIN`/`COMMIT` через `conn.transaction()`), а не отдельным write-job'ом — крах процесса до `commit()` откатывает обе стороны разом, никогда не оставляя `devices.type_id=2` без строки `printers` (или наоборот); идемпотентность (`exists_for_device_in_tx` гейт) делает повторный ретрай безопасным. |
</threat_model>

<verification>
1. `cargo test -p trackly-infra --lib printers_sqlite::` — новые unit-тесты `exists_for_device_in_tx`/`delete_by_device_id_in_tx` проходят.
2. `cargo test -p trackly-app --test devices_type_conversion -- --test-threads=1` — конверсия в обе стороны, идемпотентность, cascade подтверждены на реальной SQLite (tempfile).
3. `cargo check -p trackly-core -p trackly-infra -p trackly-app` — компилируется чисто (сигнатура `DeviceService::new()` не менялась, существующие вызовы не сломаны).
4. `pnpm run svelte-check` (в `ui/`) — новые пропы `Modal.titleExtra`/`ActionMenu.variant`/`DeviceFormBody.typeId` типобезопасны, существующие вызовы (`PrinterCreateModal`, `RequestDetail`, `DevicesPage`'s «Импорт и экспорт») не сломаны.
5. `pnpm run build` (в `ui/`) — SPA собирается, `ui/dist` актуален для LAN-браузер режима.
6. Human-checkpoint (Task 3) — визуальное/интерактивное подтверждение полного флоу в `cargo tauri dev`.
</verification>

<success_criteria>
- Ghost-sm кебаб-меню в заголовке попапа «Новое устройство»/«Редактирование устройства» переключает тип между «Устройство» и «Принтер» с галочкой на выбранном (accent-токен, не хардкод) и реактивным заголовком (4 варианта).
- Устройство→Принтер атомарно и идемпотентно создаёт строку `printers` с дефолтами (`ip_address=NULL`, `community='public'`, `snmp_version='v2c'`) — работает и при создании (включая `bulk_create`), и при редактировании.
- Принтер→Устройство требует явного подтверждения потери данных мониторинга ДО записи, затем атомарно и идемпотентно удаляет `printers` + каскадно `printer_readings`/`printer_alerts`.
- `PrinterDetail.svelte` больше не показывает неверный заголовок «Редактирование устройства» для принтера; списки «Устройства»/«Принтеры» корректно отражают перемещение записи после конверсии в любую сторону.
- Ни один существующий вызов `Modal`/`ActionMenu`/`DeviceFormModal` не меняет поведение (все новые пропы опциональны с обратно-совместимыми дефолтами).
- Никаких реальных данных организации/людей в новых тестах/фикстурах (только вымышленные имена, per CLAUDE.md).
</success_criteria>

<output>
Create `.planning/quick/260820-rdj-device-type-switch/260820-rdj-SUMMARY.md` when done
</output>
