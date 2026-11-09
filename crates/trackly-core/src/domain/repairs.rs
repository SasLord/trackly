//! Domain model of repair requests (Phase 41.4): статусы, заходы в сервис, разбор стоимости.
//!
//! Только значения и чистые функции; DTO (serde/specta) живут в trackly-app.

/// Значение `requests.request_type` для ремонтной заявки.
pub const REPAIR_REQUEST_TYPE: &str = "repair";

/// Статусы заявки, при которых ремонт считается открытым (D-19).
pub const OPEN_REPAIR_STATUSES: [&str; 3] = ["open", "in_progress", "in_repair"];

/// Коды статусов устройства (`device_statuses.code`, V014/V047).
pub const DEVICE_STATUS_CODE_AWAITING_REPAIR: &str = "на_ремонте";
pub const DEVICE_STATUS_CODE_IN_REPAIR: &str = "в_ремонте";
pub const DEVICE_STATUS_CODE_TO_WRITE_OFF: &str = "на_списание";
pub const DEVICE_STATUS_CODE_IN_STORAGE: &str = "на_складе";
pub const DEVICE_STATUS_CODE_IN_USE: &str = "в_работе";

/// Верхняя граница стоимости: 10 000 000 ₽ в копейках.
pub const MAX_COST_KOPECKS: i64 = 1_000_000_000;

/// D-14: статус исключён из обычных проверок по КОДУ (не по подписи и не по id).
pub fn is_repair_exempt_status_code(code: &str) -> bool {
    matches!(
        code,
        DEVICE_STATUS_CODE_AWAITING_REPAIR
            | DEVICE_STATUS_CODE_IN_REPAIR
            | DEVICE_STATUS_CODE_TO_WRITE_OFF
    )
}

/// Строка `repair_visits` (D-13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairVisitRow {
    pub id: i64,
    pub request_id: i64,
    pub device_id: i64,
    pub seq: i64,
    pub service_place_id: i64,
    pub from_place_id: Option<i64>,
    pub sent_at_utc: i64,
    pub returned_at_utc: Option<i64>,
    pub giver_name: String,
    pub receiver_name: String,
    pub fault_description: String,
    pub returned_to_place_id: Option<i64>,
    pub received_by_name: Option<String>,
    pub works_done: Option<String>,
    pub cost_kopecks: Option<i64>,
    pub act_number: String,
    pub created_by_user_id: Option<i64>,
    pub created_at_utc: i64,
    pub updated_at_utc: i64,
    pub version: i64,
}

/// Данные для вставки нового захода (`seq` назначает репозиторий).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairVisitNew {
    pub request_id: i64,
    pub device_id: i64,
    pub service_place_id: i64,
    pub from_place_id: Option<i64>,
    pub sent_at_utc: i64,
    pub giver_name: String,
    pub receiver_name: String,
    pub fault_description: String,
    pub act_number: String,
    pub created_by_user_id: Option<i64>,
}

const COST_ERROR: &str = "Введите сумму числом, например 1500 или 1500,50.";

/// Разбор стоимости (D-29): `1500`, `1500,50`, `1 500,5`, `1500.5`. Пусто -> `Ok(None)`.
/// Хранит копейки; отрицательное, нечисловое, более двух знаков и > 10 000 000 ₽ — ошибка.
pub fn parse_cost_kopecks(input: &str) -> Result<Option<i64>, String> {
    let cleaned: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '\u{a0}' && *c != '\u{202f}')
        .collect();
    if cleaned.is_empty() {
        return Ok(None);
    }
    let err = || COST_ERROR.to_string();
    let mut parts = cleaned.split([',', '.']);
    let whole = parts.next().ok_or_else(err)?;
    let frac = parts.next();
    if parts.next().is_some() {
        return Err(err());
    }
    if whole.is_empty() || !whole.chars().all(|c| c.is_ascii_digit()) {
        return Err(err());
    }
    let frac_kop = match frac {
        None => 0,
        Some(f) => {
            if f.is_empty() || f.len() > 2 || !f.chars().all(|c| c.is_ascii_digit()) {
                return Err(err());
            }
            let n: i64 = f.parse().map_err(|_| err())?;
            if f.len() == 1 {
                n * 10
            } else {
                n
            }
        }
    };
    let whole_n: i64 = whole.parse().map_err(|_| err())?;
    let total = whole_n
        .checked_mul(100)
        .and_then(|v| v.checked_add(frac_kop))
        .ok_or_else(err)?;
    if total > MAX_COST_KOPECKS {
        return Err(err());
    }
    Ok(Some(total))
}

/// `150050` -> `1 500,50`; `150000` -> `1 500` (разделитель тысяч — неразрывный пробел).
pub fn format_kopecks(kopecks: i64) -> String {
    let sign = if kopecks < 0 { "-" } else { "" };
    let abs = kopecks.unsigned_abs();
    let whole = (abs / 100).to_string();
    let frac = abs % 100;
    let mut grouped = String::new();
    for (i, ch) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i) % 3 == 0 {
            grouped.push('\u{a0}');
        }
        grouped.push(ch);
    }
    if frac == 0 {
        format!("{sign}{grouped}")
    } else {
        format!("{sign}{grouped},{frac:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_whole() {
        assert_eq!(parse_cost_kopecks("1500"), Ok(Some(150_000)));
    }
    #[test]
    fn parses_comma_fraction() {
        assert_eq!(parse_cost_kopecks("1500,50"), Ok(Some(150_050)));
    }
    #[test]
    fn parses_spaces_and_single_digit_fraction() {
        assert_eq!(parse_cost_kopecks("1 500,5"), Ok(Some(150_050)));
        assert_eq!(parse_cost_kopecks("1\u{a0}500,5"), Ok(Some(150_050)));
    }
    #[test]
    fn parses_dot() {
        assert_eq!(parse_cost_kopecks("1500.5"), Ok(Some(150_050)));
    }
    #[test]
    fn empty_is_none() {
        assert_eq!(parse_cost_kopecks(""), Ok(None));
        assert_eq!(parse_cost_kopecks("   "), Ok(None));
    }
    #[test]
    fn rejects_bad_input() {
        for bad in ["-1", "abc", "1,234", "1500,50,1", ",5", "1,"] {
            assert!(parse_cost_kopecks(bad).is_err(), "{bad}");
        }
    }
    #[test]
    fn upper_bound() {
        assert_eq!(parse_cost_kopecks("10000000"), Ok(Some(MAX_COST_KOPECKS)));
        assert!(parse_cost_kopecks("10000000,01").is_err());
        assert!(parse_cost_kopecks("99999999999999999999").is_err());
    }
    #[test]
    fn formats() {
        assert_eq!(format_kopecks(150_050), "1\u{a0}500,50");
        assert_eq!(format_kopecks(150_000), "1\u{a0}500");
        assert_eq!(format_kopecks(5), "0,05");
    }
    #[test]
    fn exempt_codes() {
        assert!(is_repair_exempt_status_code("на_ремонте"));
        assert!(is_repair_exempt_status_code("в_ремонте"));
        assert!(is_repair_exempt_status_code("на_списание"));
        assert!(!is_repair_exempt_status_code("на_складе"));
        assert!(!is_repair_exempt_status_code("в_работе"));
    }
}
