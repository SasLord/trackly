---
quick_id: 261109-tc6
slug: ci-ubuntu-disk-space
date: 2026-11-09
status: complete
---

# Quick 261109-tc6: ci-fast и ci-full на ubuntu-latest

ubuntu-джобы падали на `cargo test` из-за нехватки места при линковке ~214 тестовых бинарников
(`No space left on device`, `ld ... Bus error`). Добавлен шаг до rust-cache: удаление Android/
.NET/GHC/CodeQL с раннера и `CARGO_PROFILE_DEV_DEBUG=0`; в ci-full он только для Linux.

На коммите 7231a4a8 зелёные ci-fast (42631633603) и ci-full (42631633529), включая macOS,
Windows и procmon. После очистки на корневом разделе свободно 108 ГБ из 145 ГБ.
