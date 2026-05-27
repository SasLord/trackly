//! CSV row parser using the `csv` crate.
//!
//! Supports configurable delimiters (`,` / `;`) and tolerates ragged rows
//! (`flexible(true)`) so per-row validation happens at the commit step.
//!
//! See RESEARCH §Pattern 6 for the design rationale.

/// Parse `text` (already decoded to UTF-8 `str`) as CSV.
///
/// Returns `(headers, rows)` where `headers` is the first row and `rows` are
/// all subsequent data rows (ragged rows are padded/truncated to header length
/// at a higher layer; here we just collect what the csv crate gives us).
///
/// `flexible(true)` means ragged rows are NOT errors — they produce shorter
/// `Vec<String>` slices. Validation at commit step accumulates per-row errors.
pub fn parse_rows(
    text: &str,
    delimiter: u8,
) -> Result<(Vec<String>, Vec<Vec<String>>), csv::Error> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(true)
        .flexible(true) // tolerate ragged rows; per-row errors accumulate at commit
        .trim(csv::Trim::All) // trim whitespace around fields + BOM on first field
        .from_reader(text.as_bytes());

    let headers: Vec<String> = rdr.headers()?.iter().map(|s| s.to_string()).collect();

    let mut rows = Vec::new();
    for rec in rdr.records() {
        let r = rec?;
        rows.push(r.iter().map(|s| s.to_string()).collect());
    }

    Ok((headers, rows))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_comma_delimited() {
        let text = "Имя,Тип\nЛenovo,Принтер\n";
        let (headers, rows) = parse_rows(text, b',').expect("parse ok");
        assert_eq!(headers, vec!["Имя", "Тип"]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][1], "Принтер");
    }

    #[test]
    fn parses_semicolon_delimited() {
        let text = "Наименование;Статус\nНоутбук;На складе\n";
        let (headers, rows) = parse_rows(text, b';').expect("parse ok");
        assert_eq!(headers, vec!["Наименование", "Статус"]);
        assert_eq!(rows[0], vec!["Ноутбук", "На складе"]);
    }

    #[test]
    fn tolerates_ragged_rows() {
        // Row with fewer fields than headers — should NOT panic.
        let text = "A,B,C\n1,2\n4,5,6\n";
        let result = parse_rows(text, b',');
        assert!(result.is_ok(), "ragged rows should not error: {result:?}");
        let (headers, rows) = result.unwrap();
        assert_eq!(headers.len(), 3);
        assert_eq!(rows.len(), 2);
        // Ragged row has only 2 fields.
        assert_eq!(rows[0].len(), 2);
    }

    #[test]
    fn parses_cyrillic_headers_and_values() {
        let text = "Тип,Наименование\nУстройство,Сидоров-Петроградский Иван Александрович (ё) №42\n";
        let (headers, rows) = parse_rows(text, b',').expect("parse ok");
        assert_eq!(headers[0], "Тип");
        assert_eq!(headers[1], "Наименование");
        assert!(rows[0][1].contains("Сидоров-Петроградский"));
    }

    #[test]
    fn strips_bom_from_first_header() {
        // BOM U+FEFF at the start of text — csv::Trim::All should remove it.
        let text_with_bom = "\u{FEFF}Тип,Наименование\nУстройство,Ноутбук\n";
        let (headers, _) = parse_rows(text_with_bom, b',').expect("parse ok");
        // After trim, BOM should be gone from first header.
        assert_eq!(headers[0], "Тип", "BOM should be trimmed from first header");
    }
}
