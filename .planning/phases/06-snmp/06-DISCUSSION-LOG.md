# Phase 6: Принтеры (SNMP-мониторинг) и Заявки - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-14
**Phase:** 6-snmp
**Areas discussed:** Discovery + опрос SNMP, Схема принтера + история, Pantum-детекция зависания, Портал заявок + жизненный цикл (+ доп. раунд: USB PRN-04, ретенция, детали discovery, scope WebSocket, категории свободной формы)

---

## Discovery + опрос SNMP

### Discovery: как заводить найденные принтеры
| Option | Description | Selected |
|--------|-------------|----------|
| Review перед добавлением | Список найденных, админ галочками выбирает; дубликаты помечаются | ✓ |
| Авто-добавление | Все найденные сразу заводятся | |

### Опрос: как часто
| Option | Description | Selected |
|--------|-------------|----------|
| Фоновый интервал + кнопка | tokio-task по интервалу + «Обновить сейчас» | ✓ |
| Только on-demand | Опрос при открытии карточки/по кнопке | |

### Mock SNMP (PRN-08)
| Option | Description | Selected |
|--------|-------------|----------|
| Trait + две реализации | Порт SnmpClient: real (snmp2) + mock, переключение config/env | ✓ |
| cfg-feature flag | Mock через cargo feature на компиляции | |

### Архитектура опроса относительно single-writer
| Option | Description | Selected |
|--------|-------------|----------|
| Отдельный task, запись через writer | SNMP-I/O вне БД; снимки через single-writer | ✓ |
| Решит планировщик | Детали на этап планирования | |

**Notes:** Все дефолты выбраны как рекомендовано.

---

## Схема принтера + история

### Где хранить принтер-метаданные
| Option | Description | Selected |
|--------|-------------|----------|
| Новая таблица printers (FK) | Чистое разделение device/printer | ✓ |
| Колонки на devices | Nullable SNMP-колонки в devices | |

### История уровней/статусов (PRN-05)
| Option | Description | Selected |
|--------|-------------|----------|
| Отдельная таблица snapshot'ов | printer_readings, одна строка на опрос | ✓ |
| Через audit_log | Замеры в payload_json | |

### OID-профили (Pantum/Kyocera/HP/Canon + RFC3805)
| Option | Description | Selected |
|--------|-------------|----------|
| Hardcoded в Rust | Профили в коде (рекомендовалось) | |
| Data-driven в БД | Таблица OID-профилей, редактируема | ✓ |

### Настройки discovery/опроса
| Option | Description | Selected |
|--------|-------------|----------|
| app_settings (БД) | В существующей таблице, переезжает с portable-БД | ✓ |
| config.toml | В файле рядом с exe | |

**User's choice (OID):** Data-driven в БД — **расхождение с рекомендацией** (предлагался hardcoded). Осознанный выбор ради гибкости добавления моделей.
**Notes:** Остальное — как рекомендовано.

---

## Pantum-детекция зависания

### Подход (spike не проводился, реальный Pantum недостижим из dev)
| Option | Description | Selected |
|--------|-------------|----------|
| Best-effort SNMP-эвристика сейчас | Реализовать на SNMP-сигналах, пометить experimental | |
| Сначала research-spike | Отдельный spike перед реализацией | |
| Отложить детекцию в v2 | Только мониторинг без hang-детекции | ✓ |

### Источник сигнала очереди
| Option | Description | Selected |
|--------|-------------|----------|
| SNMP job-table принтера | prtJobEntry / Job-MIB / hrDeviceStatus | ✓ |
| Host-side Windows spooler | WMI/Get-PrintJob на машине с Trackly | |
| Оба (SNMP + spooler enrich) | SNMP основной + host опционально | |

### Представление alert
| Option | Description | Selected |
|--------|-------------|----------|
| In-app админу, persist до clear | Бэйдж/индикатор, dedup на принтер | ✓ |
| Решит планировщик | UX на усмотрение | |

### Уточнение (что именно откладываем в v2)
| Option | Description | Selected |
|--------|-------------|----------|
| Только Pantum-эвристику | hang-логика → v2; alert-каркас (offline/error) строим сейчас, питает DASH-05 | ✓ |
| Всё PRN-06 целиком | Никаких алертов в Phase 6, обновить ROADMAP | |

**Notes:** Изначальный ответ «отложить в v2» противоречил выбору источника/алерта — разрешено уточняющим вопросом: откладывается ТОЛЬКО Pantum-специфичная эвристика; генеричный alert-каркас строится в Phase 6. Частичное изменение трактовки ROADMAP success criterion #3 зафиксировано в CONTEXT (D-Pantum-01).

---

## Портал заявок + жизненный цикл

### Замена картриджа: выбор принтера/модели
| Option | Description | Selected |
|--------|-------------|----------|
| Принтер → модель по совместимости | Dropdown принтеров, модель фильтруется по CART-02 | |
| Принтер + любая модель | Без фильтрации | |
| Только принтер, модель опц. | Модель определяет специалист при выполнении | ✓ |

### Категории «Свободной формы»
| Option | Description | Selected |
|--------|-------------|----------|
| Без категорий | Только текст | |
| Фиксированный набор | Dropdown категорий | ✓ |

### In-app уведомление (REQ-04)
| Option | Description | Selected |
|--------|-------------|----------|
| Polling (счётчик непрочит.) | Периодический опрос (рекомендовалось) | |
| WebSocket push | Пуш через axum WebSocket | ✓ |

### Связь с CART-07 (REQ-05)
| Option | Description | Selected |
|--------|-------------|----------|
| Кнопка → pre-filled CART-07 | OperationModal pre-filled принтером/моделью | ✓ |
| Раздельные шаги | Установка и закрытие заявки отдельно | |

**User's choice (уведомления):** WebSocket push — **расхождение с рекомендацией** (предлагался polling). Осознанный выбор ради реал-тайма.
**Notes:** Замена картриджа — только принтер обязателен (изменяет ранее предполагавшийся в Phase 4 вариант). Категории свободной формы уточнены отдельным вопросом → «Ремонт техники / Расходные материалы / ПО / Прочее».

---

## Доп. раунд (USB, ретенция, discovery-детали, WS-scope)

### PRN-04 (USB-принтеры) — противоречие требований (механизм → Phase 8 spike)
| Option | Description | Selected |
|--------|-------------|----------|
| Только учёт (без опроса) | Пометить USB-подключение к станции, без SNMP | ✓ |
| Полностью в Phase 8 | Убрать PRN-04 из Phase 6 | |

### Ретенция snapshot-истории
| Option | Description | Selected |
|--------|-------------|----------|
| Прореживание + retention | downsample/удаление старше retention, настройка в app_settings | ✓ |
| Хранить всё (v1) | Без прореживания | |

### Детали discovery
| Option | Description | Selected |
|--------|-------------|----------|
| v2c + community, sysObjectID/sysDescr | Идентификация vendor/модели, маппинг на OID-профиль | ✓ |
| Решит planner/researcher | Детали на research | |

### WebSocket: транспорт и scope
| Option | Description | Selected |
|--------|-------------|----------|
| WS в веб, события в Tauri | Браузер→axum WS (auth cookie), десктоп→Tauri-события; пушим заявки+алерты | ✓ |
| Только заявки, единый WS | WS к localhost везде, только события заявок | |

**Notes:** Все доп. вопросы — как рекомендовано.

---

## Claude's Discretion

- Concurrency/timeout discovery; интервал опроса по умолчанию; форма хранения toner_levels (JSON).
- Стратегия/числа retention и downsample.
- Форма USB-учёта (флаг + FK на host-device).
- Хранение категорий (lookup vs CHECK) и алертов (таблица vs статус-поле).
- Сигнатура WS-протокола/событий и payload.
- Hexagonal-слои/feature-папки (паттерн devices/cartridges).
- Состав миграций V020+ (printers, printer_readings, oid_profiles+seed, request_categories/alerts по необходимости, FK установки картриджа на принтер).
- Нужен ли человекочитаемый номер заявки (counter) — планировщику.

## Deferred Ideas

- Pantum hang-эвристика (prtMarkerLifeCount + SNMP job-table) + авто-restart → v2 (PNT).
- Host-side Windows spooler как источник очереди → не v1.
- USB-механизм опроса (агент/WMI/RPC) → Phase 8 spike.
- Email/Telegram/Webhook уведомления (NTF-02..05) → v2.
- Заявка на регистрацию AD (REQ-06) → Phase 8.
- Виджет «Принтеры» (DASH-05) + отчёты по принтерам → Phase 7.
- UI-редактор OID-профилей / настроек discovery → Phase 7 при необходимости.
- Доп. вендоры принтеров сверх 4 целевых → ADV (v2).
</content>
