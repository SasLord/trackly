//! Numbering mask grammar (NUM-02).
//!
//! A mask is a string containing literal text plus a closed set of tokens:
//! `[YYYY]`, `[YY]`, `[MM]`, `[DD]` (date tokens) and `[X…]` (the ordinal
//! number token, written as one or more literal `X` characters between
//! brackets). Tokens are recognised **only** in this exact spelling and
//! case — `[yyyy]`, `[Yyyy]`, `[xX]`, `[Q]` are not tokens and remain
//! literal text (SPEC NUM-02 "Токены распознаются только в верхнем
//! регистре, в точном написании").
//!
//! Exactly one `[X…]` token must be present in a valid mask. Everything
//! else — literal text, unrecognised bracket groups — passes through
//! unchanged.
//!
//! The scanner below is hand-written (no regex, no backtracking) per the
//! threat register (T-40.2-03): it walks the string once, character by
//! character, looking for `[...]` groups and classifying their contents
//! against the closed token set. This rules out the ReDoS class of DoS on
//! attacker-controlled mask input.

use time::OffsetDateTime;

use crate::error::AppError;

// ---------------------------------------------------------------------------
// ParsedMask
// ---------------------------------------------------------------------------

/// A mask with its date tokens already expanded and its single `[X…]` token
/// located, split into the literal text before it (`prefix`) and after it
/// (`suffix`).
///
/// `digit_width: None` means the ordinal token was the unbounded `[X]` (a
/// single `X`) — numbers render without zero-padding and without an upper
/// bound. `digit_width: Some(n)` means the token was `[X` repeated `n`
/// times `]` (`n >= 2`) — numbers render zero-padded to width `n`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMask {
    pub prefix: String,
    pub digit_width: Option<u32>,
    pub suffix: String,
}

// ---------------------------------------------------------------------------
// Scanner internals
// ---------------------------------------------------------------------------

/// A single date token kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DateKind {
    Year4,
    Year2,
    Month,
    Day,
}

/// One recognised or literal chunk of a mask, in source order.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Segment {
    Literal(String),
    Date(DateKind),
    /// The ordinal token, carrying the raw count of `X` characters written
    /// in the mask (`1` for the unbounded `[X]`, `>=2` for a fixed width).
    Number(u32),
}

/// Classify the inner text of a `[...]` group. Returns `None` if the text
/// does not match any recognised token — the caller then treats the whole
/// bracket group (including the brackets) as literal text.
fn classify_token(inner: &str) -> Option<Segment> {
    match inner {
        "YYYY" => Some(Segment::Date(DateKind::Year4)),
        "YY" => Some(Segment::Date(DateKind::Year2)),
        "MM" => Some(Segment::Date(DateKind::Month)),
        "DD" => Some(Segment::Date(DateKind::Day)),
        _ if !inner.is_empty() && inner.chars().all(|c| c == 'X') => {
            Some(Segment::Number(inner.chars().count() as u32))
        }
        _ => None,
    }
}

/// Walk `mask` once, splitting it into literal/date/number segments.
///
/// Infallible: any text that is not a recognised token (including
/// unmatched `[`, or bracket groups with unrecognised contents such as
/// `[Q]`) is folded into the surrounding literal text. Callers decide
/// separately whether the resulting segment sequence is a *valid* mask
/// (exactly one `Number` segment) — see [`validate_mask`].
fn scan_segments(mask: &str) -> Vec<Segment> {
    let chars: Vec<char> = mask.chars().collect();
    let mut segments = Vec::new();
    let mut literal = String::new();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '[' {
            if let Some(rel_close) = chars[i + 1..].iter().position(|&c| c == ']') {
                let close_idx = i + 1 + rel_close;
                let inner: String = chars[i + 1..close_idx].iter().collect();
                match classify_token(&inner) {
                    Some(token) => {
                        if !literal.is_empty() {
                            segments.push(Segment::Literal(std::mem::take(&mut literal)));
                        }
                        segments.push(token);
                    }
                    None => {
                        literal.push('[');
                        literal.push_str(&inner);
                        literal.push(']');
                    }
                }
                i = close_idx + 1;
                continue;
            }
            // No closing bracket anywhere ahead — the '[' is literal text.
            literal.push('[');
            i += 1;
            continue;
        }
        literal.push(chars[i]);
        i += 1;
    }

    if !literal.is_empty() {
        segments.push(Segment::Literal(literal));
    }
    segments
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Syntactic validation only — does not expand date tokens. Ensures the
/// mask contains exactly one `[X…]` ordinal token; everything else
/// (literal text, unrecognised bracket groups) is unrestricted.
///
/// Called before saving a template (Plan 04) and live in the template form
/// (Plan 10) via a debounced round-trip to the server — the check lives
/// here, once, rather than being re-implemented in JS.
pub fn validate_mask(mask: &str) -> Result<(), AppError> {
    let number_token_count = scan_segments(mask)
        .iter()
        .filter(|seg| matches!(seg, Segment::Number(_)))
        .count();

    match number_token_count {
        1 => Ok(()),
        0 => Err(AppError::Validation {
            field: "mask".to_string(),
            message: "В маске нужен порядковый номер: [X], [XX], [XXX] и т. д.".to_string(),
        }),
        _ => Err(AppError::Validation {
            field: "mask".to_string(),
            message: "Порядковый номер в маске может быть только один.".to_string(),
        }),
    }
}

/// Expand `[YYYY]`/`[YY]`/`[MM]`/`[DD]` date tokens in `mask` into their
/// concrete digit values for `today_utc` (a Unix timestamp). The `[X…]`
/// ordinal token and any unrecognised bracket groups pass through
/// unchanged. `trackly-core` never reads system time itself — the caller
/// (a service in `trackly-app`/`trackly-infra`) decides what "today" means,
/// which is the seam D-07 (server "today") builds on in a later plan.
pub fn expand_date_tokens(mask: &str, today_utc: i64) -> String {
    let today =
        OffsetDateTime::from_unix_timestamp(today_utc).unwrap_or(OffsetDateTime::UNIX_EPOCH);
    let mut out = String::new();

    for segment in scan_segments(mask) {
        match segment {
            Segment::Literal(text) => out.push_str(&text),
            Segment::Number(width) => {
                out.push('[');
                out.push_str(&"X".repeat(width as usize));
                out.push(']');
            }
            Segment::Date(kind) => {
                let rendered = match kind {
                    DateKind::Year4 => format!("{:04}", today.year()),
                    DateKind::Year2 => format!("{:02}", today.year().rem_euclid(100)),
                    DateKind::Month => format!("{:02}", u8::from(today.month())),
                    DateKind::Day => format!("{:02}", today.day()),
                };
                out.push_str(&rendered);
            }
        }
    }

    out
}

/// Parse `mask`, expanding date tokens using `today_utc` and locating the
/// single `[X…]` token, splitting the mask into `prefix`/`digit_width`/
/// `suffix`. Fails with the same errors as [`validate_mask`] if the mask
/// does not contain exactly one ordinal token.
pub fn parse_and_expand(mask: &str, today_utc: i64) -> Result<ParsedMask, AppError> {
    validate_mask(mask)?;
    let expanded = expand_date_tokens(mask, today_utc);

    let mut prefix = String::new();
    let mut suffix = String::new();
    let mut digit_width: Option<u32> = None;
    let mut seen_number = false;

    for segment in scan_segments(&expanded) {
        match segment {
            Segment::Literal(text) => {
                if seen_number {
                    suffix.push_str(&text);
                } else {
                    prefix.push_str(&text);
                }
            }
            Segment::Number(width) => {
                seen_number = true;
                digit_width = if width == 1 { None } else { Some(width) };
            }
            Segment::Date(_) => {
                // expand_date_tokens already replaced every date token with
                // plain digits — re-scanning the expanded string cannot
                // rediscover a `[YYYY]`-shaped bracket group.
                unreachable!("date tokens must already be expanded before this scan")
            }
        }
    }

    Ok(ParsedMask {
        prefix,
        digit_width,
        suffix,
    })
}

/// Render `n` through `parsed`: `prefix + digits + suffix`, where `digits`
/// is zero-padded to `digit_width` when set, or plain (no padding) when
/// unbounded.
pub fn render_number(parsed: &ParsedMask, n: u64) -> String {
    let digits = match parsed.digit_width {
        Some(width) => format!("{:0width$}", n, width = width as usize),
        None => n.to_string(),
    };
    format!("{}{}{}", parsed.prefix, digits, parsed.suffix)
}

/// The reverse of [`render_number`]: does `candidate` (after `.trim()`,
/// literal parts compared case-insensitively) belong to the sequence
/// described by `parsed`? Returns the parsed number if so.
///
/// This is the single place where a stored text value is turned back into
/// "does this record belong to this template's sequence" — Plan 03 calls
/// it for every live row when computing next-number candidates.
pub fn extract_digits(parsed: &ParsedMask, candidate: &str) -> Option<u64> {
    let trimmed = candidate.trim();
    let trimmed_chars: Vec<char> = trimmed.chars().collect();
    let trimmed_lower: Vec<char> = trimmed.to_lowercase().chars().collect();
    let prefix_lower: Vec<char> = parsed.prefix.to_lowercase().chars().collect();
    let suffix_lower: Vec<char> = parsed.suffix.to_lowercase().chars().collect();

    if trimmed_lower.len() < prefix_lower.len() + suffix_lower.len() {
        return None;
    }
    if trimmed_chars.len() != trimmed_lower.len() {
        // Case-folding changed the character count (e.g. German ß) — our
        // closed alphabet (Cyrillic/Latin/digits/punctuation) never hits
        // this, but bail out safely rather than mis-slice.
        return None;
    }
    if trimmed_lower[..prefix_lower.len()] != prefix_lower[..] {
        return None;
    }
    let suffix_start = trimmed_lower.len() - suffix_lower.len();
    if trimmed_lower[suffix_start..] != suffix_lower[..] {
        return None;
    }

    let digits_chars = &trimmed_chars[prefix_lower.len()..suffix_start];
    if digits_chars.is_empty() || !digits_chars.iter().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if let Some(width) = parsed.digit_width {
        if digits_chars.len() as u32 != width {
            return None;
        }
    }

    let digits_str: String = digits_chars.iter().collect();
    digits_str.parse::<u64>().ok()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use time::{Date, Month};

    fn unix_ts(year: i32, month: Month, day: u8) -> i64 {
        Date::from_calendar_date(year, month, day)
            .expect("valid calendar date")
            .midnight()
            .assume_utc()
            .unix_timestamp()
    }

    #[test]
    fn org_prefix_fixed_width_empty_base_gives_000001() {
        let today = unix_ts(2026, Month::September, 18);
        let parsed = parse_and_expand("ОРГ-00-[XXXXXX]", today).expect("valid mask");
        assert_eq!(parsed.prefix, "ОРГ-00-");
        assert_eq!(parsed.digit_width, Some(6));
        assert_eq!(parsed.suffix, "");

        let next = crate::text::sequence::compute_next(&[], parsed.digit_width);
        assert_eq!(render_number(&parsed, next.first_free), "ОРГ-00-000001");
    }

    #[test]
    fn date_prefix_unbounded_x_expands_literally() {
        let today = unix_ts(2026, Month::September, 18);
        let parsed = parse_and_expand("[YYYY]/[MM]-[X]", today).expect("valid mask");
        assert_eq!(parsed.prefix, "2026/09-");
        assert_eq!(parsed.digit_width, None);
        assert_eq!(parsed.suffix, "");

        let next = crate::text::sequence::compute_next(&[], parsed.digit_width);
        assert_eq!(render_number(&parsed, next.first_free), "2026/09-1");
    }

    #[test]
    fn unrecognised_bracket_group_stays_literal() {
        let today = unix_ts(2026, Month::September, 18);
        let parsed = parse_and_expand("A[Q]-[XX]", today).expect("valid mask");
        assert_eq!(parsed.prefix, "A[Q]-");
        assert_eq!(parsed.digit_width, Some(2));
        assert_eq!(parsed.suffix, "");
    }

    #[test]
    fn validate_mask_rejects_missing_number_token() {
        let err = validate_mask("ОРГ-[YYYY]").expect_err("must reject");
        match err {
            AppError::Validation { field, message } => {
                assert_eq!(field, "mask");
                assert_eq!(
                    message,
                    "В маске нужен порядковый номер: [X], [XX], [XXX] и т. д."
                );
            }
            other => panic!("expected Validation, got {other:?}"),
        }
    }

    #[test]
    fn validate_mask_rejects_two_number_tokens() {
        let err = validate_mask("[X]-[XX]").expect_err("must reject");
        match err {
            AppError::Validation { field, message } => {
                assert_eq!(field, "mask");
                assert_eq!(message, "Порядковый номер в маске может быть только один.");
            }
            other => panic!("expected Validation, got {other:?}"),
        }
    }

    #[test]
    fn tokens_recognised_only_in_exact_uppercase_spelling() {
        let today = unix_ts(2026, Month::September, 18);

        // [yyyy] — all lowercase — not a date token, stays literal.
        let parsed = parse_and_expand("[yyyy]-[XX]", today).expect("valid mask");
        assert_eq!(parsed.prefix, "[yyyy]-");
        assert_eq!(parsed.digit_width, Some(2));

        // [Yyyy] — mixed case — not a date token, stays literal.
        let parsed = parse_and_expand("[Yyyy]-[XX]", today).expect("valid mask");
        assert_eq!(parsed.prefix, "[Yyyy]-");
        assert_eq!(parsed.digit_width, Some(2));

        // [xX] — mixed case — not the ordinal token, stays literal; the
        // real ordinal token is [XX].
        let parsed = parse_and_expand("A-[xX]-[XX]", today).expect("valid mask");
        assert_eq!(parsed.prefix, "A-[xX]-");
        assert_eq!(parsed.digit_width, Some(2));
        assert_eq!(parsed.suffix, "");
    }

    #[test]
    fn date_tokens_expand_with_correct_ranges_and_padding() {
        // 2026-01-05: exercises zero-padding for single-digit month/day and
        // the two-digit year form separately from the four-digit form.
        let today = unix_ts(2026, Month::January, 5);
        let parsed = parse_and_expand("[YYYY]-[YY]-[MM]-[DD]-[X]", today).expect("valid mask");
        assert_eq!(parsed.prefix, "2026-26-01-05-");
        assert_eq!(parsed.digit_width, None);
    }

    #[test]
    fn date_tokens_handle_end_of_year_boundary() {
        // 2026-12-31: exercises the upper end of month/day ranges.
        let today = unix_ts(2026, Month::December, 31);
        let parsed = parse_and_expand("[YYYY][MM][DD]-[XXX]", today).expect("valid mask");
        assert_eq!(parsed.prefix, "20261231-");
        assert_eq!(parsed.digit_width, Some(3));
    }

    #[test]
    fn render_number_pads_fixed_width_and_leaves_unbounded_plain() {
        let parsed = ParsedMask {
            prefix: "C-".to_string(),
            digit_width: Some(4),
            suffix: String::new(),
        };
        assert_eq!(render_number(&parsed, 6), "C-0006");

        let parsed_unbounded = ParsedMask {
            prefix: "2026/09-".to_string(),
            digit_width: None,
            suffix: String::new(),
        };
        assert_eq!(render_number(&parsed_unbounded, 1), "2026/09-1");
    }

    #[test]
    fn extract_digits_round_trips_render_number() {
        let parsed = ParsedMask {
            prefix: "ОРГ-00-".to_string(),
            digit_width: Some(6),
            suffix: String::new(),
        };
        let rendered = render_number(&parsed, 17);
        assert_eq!(extract_digits(&parsed, &rendered), Some(17));

        // Case-insensitive on literals, trims whitespace.
        assert_eq!(extract_digits(&parsed, "  орг-00-000017  "), Some(17));

        // Wrong digit count for a fixed-width mask is rejected.
        assert_eq!(extract_digits(&parsed, "ОРГ-00-17"), None);

        // Doesn't match prefix/suffix at all.
        assert_eq!(extract_digits(&parsed, "ИНВ-000017"), None);
    }

    #[test]
    fn extract_digits_unbounded_accepts_any_digit_count() {
        let parsed = ParsedMask {
            prefix: "2026/09-".to_string(),
            digit_width: None,
            suffix: String::new(),
        };
        assert_eq!(extract_digits(&parsed, "2026/09-1"), Some(1));
        assert_eq!(extract_digits(&parsed, "2026/09-123456"), Some(123456));
        assert_eq!(extract_digits(&parsed, "2026/09-"), None);
    }
}
