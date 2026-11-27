---
phase: 44-link
plan: 07
subsystem: link-client
tags: [link-app, tls, spki-pin, reqwest, websocket]
requires: [44-05]
provides:
  - pin.rs (PinVerifier, build_client_config, PinProbe, classify_tls_error, pin_matches)
  - http_client.rs (LinkHttp, DeviceCreds, HttpError)
  - ws_client.rs (connect, WsConn, WsFrame, WsError, сторож D-23)
  - link_proto::dto::REVOKED_CLOSE_CODE = 4401
affects: [44-08, 44-09, 44-12, 44-14, 44-15]
key-files:
  created:
    - crates/link-app/tests/pin_tls.rs
  modified:
    - crates/link-app/src/pin.rs
    - crates/link-app/src/http_client.rs
    - crates/link-app/src/ws_client.rs
    - crates/link-app/Cargo.toml
    - Cargo.lock
    - crates/link-proto/src/dto.rs
decisions:
  - "Один rustls ClientConfig (ring, ALPN http/1.1) на HTTP и WS; LinkHttp::tls_config() отдаёт его WS-клиенту"
  - "Различение подмены и сети: PinProbe.seen очищается перед каждой попыткой, чужой seen после ошибки = PinMismatch"
requirements-completed: [LNK-08, LNK-09]
duration: 40min
completed: 2026-11-28
---

# Phase 44 Plan 07: Сетевой слой Link Summary

SPKI-пин (constant-time, имя хоста не сверяется), reqwest-клиент и WS-клиент на одном TLS-конфиге; при чужом пине рукопожатие падает до первого прикладного байта, мёртвое WS-соединение обнаруживает сторож.

## Коммиты

- 2524875b: pin.rs + 5 тестов на локальном TLS (rcgen)
- 12b2916b: http_client.rs + 6 тестов
- 467526dd: ws_client.rs, REVOKED_CLOSE_CODE, 6 тестов

## Проверки

- `cargo test -p link-app --test pin_tls`: 17/17 зелёные (pin 5, http 6, ws 6)
- `cargo test -p link-app`, `cargo test -p link-proto`: зелёные
- `cargo clippy -p link-app -p link-proto --all-targets -- -D warnings`: чисто; `cargo fmt --check`: чисто
- `cargo tree -p link-app -i aws-lc-rs`: пусто
- grep-ворота: `danger_accept_invalid_certs` отсутствует; в http_client.rs нет tracing с Authorization/Cookie/secret; `tls_backend_preconfigured`, `http1_only`, `Policy::none` на месте; `4401` определён один раз (link-proto)

## Отклонения от плана

1. **[Rule 3] Нет `Connector::Rustls`** (key_link и acceptance-grep плана): у tokio-tungstenite в link-app нет TLS-фич (решение плана 05, aws-lc-rs). Вместо этого TLS вручную: `tokio_rustls::TlsConnector` с ТЕМ ЖЕ `Arc<ClientConfig>` из `LinkHttp::tls_config()`, затем `tokio_tungstenite::client_async`. Суть LNK-08 (один конфиг) сохранена; grep по `Connector::Rustls` ничего не найдёт.
2. **[Rule 3] Добавлена зависимость `futures-util = "0.3"`** в link-app (StreamExt/SinkExt для WebSocketStream). Крейт уже в графе (tokio-tungstenite, trackly-app); из списка плана 01 не входил, прошу учесть.
3. `LinkHttp::seed_cookie` (`#[doc(hidden)]`): положить cookie в jar без реального входа; нужен тесту WS и пригодится при восстановлении сессии.
4. Test 4 сторожа (D-23) сделан на `tokio::io::duplex` + `start_paused`, а не на сетевом TLS: автопродвижение виртуального времени при реальном сокете нестабильно. Узел `WsConn<S>` обобщён по потоку.
5. `classify_tls_error` принимает ошибку, но решает по `PinProbe` (seen != pin); для корректности вызывающий делает `probe.clear()` перед попыткой (LinkHttp и connect это делают). При параллельных запросах на одном LinkHttp окно гонки теоретически есть (сброс чужим запросом) и даёт `Network` вместо `PinMismatch`, но не ложное принятие.
6. Мок-HTTP-сервер в тесте написан вручную поверх tokio-rustls (без hyper/axum), чтобы не расширять dev-зависимости.

## Не проверено

- Работа на Windows (SChannel не используется, только rustls/ring; платформенных веток в плане 07 нет).
- Реальный сервер Trackly (ws.rs плана 14 ещё не отправляет Close 4401); сторож и разбор кадров проверены на моках по формату из ws.rs.
- IPv6-литерал хоста: код обрабатывает скобки, тестов нет.

## Авто-одобренные чекпоинты

Нет (план автономный).

## Known Stubs

Нет в файлах этого плана.

## Threat Flags

Новых поверхностей вне threat_model нет.

## Self-Check: PASSED

Файлы pin.rs, http_client.rs, ws_client.rs, tests/pin_tls.rs существуют; коммиты 2524875b, 12b2916b, 467526dd в истории.
