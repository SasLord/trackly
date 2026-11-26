# Architecture Decision Records

Решения вехи v1.5 (Trackly Link). Формат: Статус / Контекст / Решение / Последствия / Отвергнутые варианты / Источники. Язык русский. Реальные данные организации в ADR не пишутся.

| № | Заголовок | Статус |
|---|-----------|--------|
| [0001](0001-monorepo-packages-versioning.md) | Монорепо, общие пакеты, версионирование | Accepted |
| [0002](0002-link-network-pinning-secrets.md) | Сеть Link, закрепление сертификата, секреты | Accepted |
| [0003](0003-chat-model-fts-delivery.md) | Модель чата, FTS, доставка | Accepted |
| [0004](0004-updates-distribution-rds.md) | Обновления, распространение, RDS | Accepted (provisional) |
| [0005](0005-install-path-and-user-data.md) | Путь установки и данные пользователя | Accepted (provisional) |

Provisional: решение зависит от итога пилота (AppLocker/SRP, режим WebView2, фактический путь NSIS).

## Покрытие D-22

| Решение или расхождение | Файл |
|-------------------------|------|
| Сеть Link только в Rust, webview без сети | [0002](0002-link-network-pinning-secrets.md) |
| Закрепление SPKI-отпечатка | [0002](0002-link-network-pinning-secrets.md) |
| Токен в хранилище ОС | [0002](0002-link-network-pinning-secrets.md) |
| Модель чата `im_*` вместо `chat_*` | [0003](0003-chat-model-fts-delivery.md) |
| Доставка «сигнал + догрузка по курсору» | [0003](0003-chat-model-fts-delivery.md) |
| Откат = выпуск с большим номером, запасной `version_comparator` | [0001](0001-monorepo-packages-versioning.md), [0004](0004-updates-distribution-rds.md) |
| Каналы обновлений только на сервере | [0004](0004-updates-distribution-rds.md) |
| Вариант A распространения | [0004](0004-updates-distribution-rds.md) |
| Поддержка RDS (single-instance на сеанс) | [0004](0004-updates-distribution-rds.md) |
| Расхождение: `keyring-core` вместо `keyring` | [0002](0002-link-network-pinning-secrets.md) |
| Расхождение: имя крейта `link-app` (bin `trackly-link`) | [0001](0001-monorepo-packages-versioning.md) |
| Расхождение: FTS trigram + нормализация `ё->е` | [0003](0003-chat-model-fts-delivery.md) |
