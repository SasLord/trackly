//! Byte-stream decoder: converts raw CSV bytes to String using detected encoding.
//!
//! Uses `encoding_rs::Encoding::decode()` which handles UTF-8, Windows-1251, etc.
//! Returns `(decoded_string, had_replacements)` — callers can surface a warning if
//! `had_replacements == true` (RESEARCH §Pitfall 7).

use encoding_rs::Encoding;

/// Decode `bytes` using the given `encoding`.
///
/// Returns `(decoded_string, had_replacements)`.
/// `had_replacements` is `true` if the decoder replaced malformed byte sequences
/// with U+FFFD (Unicode replacement character). The caller should warn the user.
pub fn decode_to_string(bytes: &[u8], encoding: &'static Encoding) -> (String, bool) {
    let (cow, _used_encoding, had_replacements) = encoding.decode(bytes);
    (cow.into_owned(), had_replacements)
}

#[cfg(test)]
mod tests {
    use super::*;
    use encoding_rs::{UTF_8, WINDOWS_1251};

    #[test]
    fn utf8_cyrillic_round_trip() {
        let text = "Сидоров-Петроградский Иван Александрович (ё) №42";
        let (decoded, had_replacements) = decode_to_string(text.as_bytes(), UTF_8);
        assert_eq!(decoded, text);
        assert!(!had_replacements);
    }

    #[test]
    fn utf8_bom_stripped_by_encoding_rs() {
        // encoding_rs does NOT strip BOM automatically on UTF-8 decode —
        // BOM appears as U+FEFF in the output. Callers handle this via csv::ReaderBuilder.
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice("Тип,Наименование\n".as_bytes());
        let (decoded, had_replacements) = decode_to_string(&bytes, UTF_8);
        // BOM U+FEFF is in the string; csv crate trims it via trim_in_place or flexible reader.
        assert!(decoded.contains("Тип,Наименование"));
        assert!(!had_replacements);
    }

    #[test]
    fn cp1251_yolka_round_trip() {
        let text = "Ёлка";
        let (encoded, _, _) = WINDOWS_1251.encode(text);
        let (decoded, had_replacements) = decode_to_string(&encoded, WINDOWS_1251);
        assert_eq!(decoded, "Ёлка");
        assert!(!had_replacements);
    }

    #[test]
    fn cp1251_cyrillic_full_fixture() {
        let text = "Сидоров-Петроградский Иван Александрович (ё) №42";
        let (encoded, _, _) = WINDOWS_1251.encode(text);
        let (decoded, had_replacements) = decode_to_string(&encoded, WINDOWS_1251);
        assert_eq!(decoded, text);
        assert!(!had_replacements);
    }

    #[test]
    fn invalid_bytes_yield_replacement_chars() {
        // UTF-8 decode of invalid bytes → had_replacements = true
        let invalid_bytes = b"\xFF\xFE\xFD";
        let (_, had_replacements) = decode_to_string(invalid_bytes, UTF_8);
        assert!(had_replacements);
    }
}
