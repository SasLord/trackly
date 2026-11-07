# Уведомления о сторонних компонентах

## libheif и libde265 (LGPL-3.0)

Для конвертации фотографий HEIC в JPEG сборки Trackly для Windows с включённой
Cargo-фичей `heic` статически линкуют:

- **libheif**, лицензия LGPL-3.0 — https://github.com/strukturag/libheif
- **libde265**, лицензия LGPL-3.0 — https://github.com/strukturag/libde265

В сборки без фичи `heic` (по умолчанию, а также macOS и Linux) эти библиотеки
не входят; HEIC в них отклоняется с предложением сохранить файл как JPEG.
Кодировщик HEVC (x265, GPL) не используется.

Получение исходных кодов и пересборка: исходный код Trackly публичен (репозиторий
на GitHub). Шаги сборки с этими библиотеками (vcpkg, триплет
`x64-windows-static-md`, `cargo build -p trackly-app --features heic`)
приведены в `.github/workflows/heic-spike.yml`. Исходные коды библиотек доступны по
ссылкам выше; получатель вправе заменить версию библиотеки и пересобрать
приложение этой же командой.

Привязка к libheif на стороне Rust — крейты `libheif-rs` / `libheif-sys`
(MIT).
