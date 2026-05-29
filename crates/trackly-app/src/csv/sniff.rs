//! Encoding and delimiter detection for CSV import.
//!
//! Detects: UTF-8 BOM, UTF-8 plain, Windows-1251 via chardetng.
//! Delimiter sniff: counts `,` and `;` in first non-empty decoded line.
//!
//! See RESEARCH §Pattern 6 for the full design rationale.

use chardetng::EncodingDetector;
use encoding_rs::{Encoding, UTF_8};

/// Detected CSV profile: encoding + delimiter.
pub struct CsvProfile {
    pub encoding: &'static Encoding,
    pub delimiter: u8,
}

/// Detect the encoding and delimiter of `bytes`.
///
/// Detection order:
/// 1. BOM check (fast path): UTF-8 BOM `\xEF\xBB\xBF` → UTF_8.
/// 2. `chardetng` byte-stream analysis (falls back to ASCII for pure ASCII).
/// 3. Delimiter sniff: count `,` vs `;` in first non-empty decoded line.
pub fn detect(bytes: &[u8]) -> CsvProfile {
    // 1. BOM check (fast path).
    let encoding = if bytes.starts_with(b"\xEF\xBB\xBF") {
        UTF_8
    } else {
        let mut det = EncodingDetector::new();
        det.feed(bytes, true);
        det.guess(None, true) // allow_utf8=true
    };

    // 2. Delimiter sniff: decode the first non-empty line and count `,` vs `;`.
    let (decoded, _, _) = encoding.decode(bytes);
    let first_line = decoded.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    let comma = first_line.bytes().filter(|b| *b == b',').count();
    let semi = first_line.bytes().filter(|b| *b == b';').count();
    let delimiter = if semi > comma { b';' } else { b',' };

    CsvProfile {
        encoding,
        delimiter,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use encoding_rs::WINDOWS_1251;

    #[test]
    fn detects_utf8_bom() {
        // UTF-8 BOM + some cyrillic
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice("Тип,Наименование\n".as_bytes());
        let profile = detect(&bytes);
        assert_eq!(profile.encoding, UTF_8);
    }

    #[test]
    fn detects_plain_utf8() {
        let bytes = "Тип,Наименование\nУстройство,Ноутбук\n".as_bytes();
        let profile = detect(bytes);
        assert_eq!(profile.encoding, UTF_8);
    }

    #[test]
    fn detects_cp1251() {
        // Encode "Тип,Наименование" in CP1251
        let text = "Тип,Наименование\nУстройство,Ноутбук\n";
        let (encoded, _, _) = WINDOWS_1251.encode(text);
        let profile = detect(&encoded);
        assert_eq!(profile.encoding, WINDOWS_1251);
    }

    #[test]
    fn sniffs_comma_delimiter() {
        let bytes = "Тип,Наименование,Модель\n".as_bytes();
        let profile = detect(bytes);
        assert_eq!(profile.delimiter, b',');
    }

    #[test]
    fn sniffs_semicolon_delimiter() {
        let bytes = "Тип;Наименование;Модель\n".as_bytes();
        let profile = detect(bytes);
        assert_eq!(profile.delimiter, b';');
    }

    #[test]
    fn semicolon_wins_when_more_semis() {
        // 3 semicolons vs 1 comma
        let bytes = "Тип;Наименование;Модель;Статус,x\n".as_bytes();
        let profile = detect(bytes);
        assert_eq!(profile.delimiter, b';');
    }
}
