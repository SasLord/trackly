---
created: 2026-09-23T00:00:00.000Z
title: Редактирование сетевых параметров принтера (IP / SNMP community)
area: feature
resolves_phase: null
milestone: v1.4
source: UAT чекпоинта плана 40.3-07 (живая проверка пользователем 2026-09-23)
files:
  - crates/trackly-core/src/ports/printers.rs
  - crates/trackly-infra/src/repos/printers_sqlite.rs
  - crates/trackly-app/src/services/printer_service.rs
  - crates/trackly-app/src/dto/printer.rs
  - crates/trackly-app/src/tauri_cmds/
  - crates/trackly-app/src/http/
  - ui/src/features/devices/DeviceFormBody.svelte
---

## Problem

Во время живой UAT-проверки чекпоинта плана 40.3-07 пользователь сформулировал
требование:

> «Блок IP/SNMP всегда должен отображаться: при создании, редактировании.
> Может у него сменится IP, его надо будет поправить.»

Сегодня блок IP/SNMP в `DeviceFormBody.svelte` намеренно виден только в режиме
создания (`{#if typeId === PRINTER_TYPE_ID && !isEdit}`) — так его спроектировал
план 40.3-05. Это не косметическое ограничение: **бэкенд не умеет менять сетевые
параметры принтера вообще.**

Установленные факты (проверено 2026-09-23):

- Порт `PrinterRepo` (`crates/trackly-core/src/ports/printers.rs`) содержит только
  `get` / `get_by_device_id` / `list` / `get_last_reading` / `list_active_alerts` /
  `list_oid_profiles` / `get_oid_profile_by_prefix` / `current_cartridge_for_printer`.
  Метода обновления нет.
- Единственный `UPDATE printers` во всём инфра-слое — `printers_sqlite.rs:150`,
  и он пишет только `last_seen_utc` / `updated_at_utc` из поллера.
- `PrinterService` имеет `create_from_device`, поллинг, `discover`,
  `acknowledge_alert` — но не `update`.
- В UI IP показывается только на чтение (`PrinterDetail.svelte:427`).

То есть «просто показать блок в edit-режиме» приведёт к форме, которая
молча ничего не сохраняет — ровно тот класс дефекта (молчаливая потеря
введённых данных), который закрывал этот же раунд 40.3 (BLOCKER-2 / CR-02).

## Scope

Сквозная фича на 6 слоёв:

1. `PrinterRepo` — метод обновления сетевых параметров.
2. `printers_sqlite` — реализация `UPDATE printers SET ip_address = ?, community = ?`.
3. `PrinterService` — сервисный метод + валидация IP.
4. DTO-патч в `dto/printer.rs`.
5. Оба транспорта: Tauri-команда + HTTP-хендлер (одинаковый контракт).
6. UI: снять `!isEdit` с блока, подключить сохранение, обработать ошибки полей.

## Edge cases (обязательно разобрать на этапе spec)

- **Устройство стало принтером после смены типа** — строки в `printers` ещё нет,
  нужен upsert / create-on-edit, иначе правка IP молча ничего не найдёт.
- **Очистка IP в NULL** — снова `Option<Option<T>>` на serde-границе. Это ровно
  тот класс бага, который чинил план 40.3-06 (`DevicePatch.place_id`,
  `UserPatch.email`): без `#[serde(default, with = "serde_with::rust::double_option")]`
  явный `null` схлопнется в «поле не передано». Не повторить.
- **USB-принтеры** (`usb_host_device_id`, без IP) — блок не должен требовать IP.
- **Смена IP у опрашиваемого принтера** — что происходит с накопленными
  readings / активными алертами, привязанными к тому же `printer.id`;
  нужно ли сбрасывать `last_seen_utc` / гасить алерты старого адреса.
- **RBAC** — кто имеет право менять сетевые параметры (вероятно, не employee).

## Next step

`/gsd-spec-phase` на отдельную фазу после закрытия 40.3. Пользователь явно
выбрал вариант «отдельной фазой после 40.3», чтобы фаза получила собственную
верификацию и тесты, а не проехала зайцем в раунде закрытия пробелов аудита.
