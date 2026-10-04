//! Общие хелперы русского склонения (выделены из place_service.rs, чтобы не дублировать).

/// Russian noun pluralization by count: `one` (1, 21, 31, …), `few` (2-4, 22-24, …),
/// `many` (0, 5-20, 25-30, …). The 11-14 exception (which would otherwise match
/// `few` via `n % 10 == 1..4`) always resolves to `many`.
pub(crate) fn ru_plural(
    n: i64,
    one: &'static str,
    few: &'static str,
    many: &'static str,
) -> &'static str {
    let n_abs = n.unsigned_abs();
    let mod100 = n_abs % 100;
    let mod10 = n_abs % 10;
    if (11..=14).contains(&mod100) {
        many
    } else {
        match mod10 {
            1 => one,
            2..=4 => few,
            _ => many,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ru_plural_device_word_matches_ui_spec_example() {
        assert_eq!(
            ru_plural(12, "устройство", "устройства", "устройств"),
            "устройств"
        );
        assert_eq!(
            ru_plural(1, "устройство", "устройства", "устройств"),
            "устройство"
        );
        assert_eq!(
            ru_plural(2, "устройство", "устройства", "устройств"),
            "устройства"
        );
    }

    #[test]
    fn ru_plural_handles_teens_and_twenty_one() {
        assert_eq!(
            ru_plural(11, "устройство", "устройства", "устройств"),
            "устройств"
        );
        assert_eq!(
            ru_plural(21, "устройство", "устройства", "устройств"),
            "устройство"
        );
        assert_eq!(
            ru_plural(0, "устройство", "устройства", "устройств"),
            "устройств"
        );
    }
}
