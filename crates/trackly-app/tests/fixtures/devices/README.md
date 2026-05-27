# CSV fixtures для тестов devices_csv_import

Эта директория содержит тестовые CSV-файлы для integration-тестов импорта устройств.

## Файлы

| Файл | Кодировка | Разделитель | Описание |
|------|-----------|-------------|----------|
| `utf8.csv` | UTF-8 (без BOM) | `,` | Базовый файл — источник для остальных |
| `utf8_bom.csv` | UTF-8 + 3-byte BOM (`EF BB BF`) | `,` | Excel на Windows часто сохраняет с BOM |
| `cp1251_comma.csv` | Windows-1251 (binary) | `,` | Старые русские Excel / 1C-выгрузки |
| `cp1251_semicolon.csv` | Windows-1251 (binary) | `;` | Та же выгрузка но с `;` (RU-locale Excel default) |
| `malformed_mixed_rows.csv` | UTF-8 (без BOM) | `,` | 2 строки с пустым полем «Наименование» (обязательное) |

## Fixture string

Все файлы содержат строку:
```
Сидоров-Петроградский Иван Александрович (ё) №42
```
Это тестовая фикстура Phase 1, проверяющая корректность обработки кириллицы.

## Регенерация бинарных файлов

**ВНИМАНИЕ:** `cp1251_comma.csv` и `cp1251_semicolon.csv` — бинарные блобы, не редактировать вручную.
Для регенерации используйте Python:

```bash
# cp1251_comma.csv (те же данные что utf8.csv, но в CP1251)
python3 -c "
content = open('utf8.csv', encoding='utf-8').read()
open('cp1251_comma.csv', 'wb').write(content.encode('cp1251'))
"

# cp1251_semicolon.csv (данные с ';' разделителем, в CP1251)
python3 -c "
content = open('utf8.csv', encoding='utf-8').read().replace(',', ';')
open('cp1251_semicolon.csv', 'wb').write(content.encode('cp1251'))
"

# utf8_bom.csv (utf8.csv с 3-byte BOM в начале)
python3 -c "
content = open('utf8.csv', 'rb').read()
open('utf8_bom.csv', 'wb').write(b'\xef\xbb\xbf' + content)
"
```

## Через iconv (альтернативно)

```bash
# UTF-8 → CP1251 (comma)
iconv -f UTF-8 -t WINDOWS-1251 utf8.csv > cp1251_comma.csv

# UTF-8 → CP1251 (semicolon — сначала заменить разделитель)
sed 's/,/;/g' utf8.csv | iconv -f UTF-8 -t WINDOWS-1251 > cp1251_semicolon.csv
```
