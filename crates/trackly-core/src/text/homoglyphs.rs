//! Cyrillic/Latin script-mix detection and skeleton comparison (NUM-12,
//! D-04).
//!
//! Two responsibilities, both pure (no I/O):
//!
//! - [`has_script_mix`] flags a number that contains **both** a Cyrillic
//!   and a Latin letter — the "Поправлю / Продолжить" warning trigger.
//! - [`to_latin_skeleton`] collapses look-alike letters (О/O, Р/P, …) to a
//!   single canonical form so two differently-typed but visually identical
//!   numbers compare equal — the doppelganger-detection half of NUM-12.
//!
//! The homoglyph table is a fixed, closed set of 12 pairs (CONTEXT
//! "Claude's Discretion" / RESEARCH "Supporting" — the standard set for
//! this phase, not to be extended). It is a linear-scanned static array,
//! not a `HashMap`: 12 pairs make a hash map's allocation more expensive
//! than a linear scan, and this is the only place in the codebase that
//! needs this table.

/// Closed table of Cyrillic → Latin look-alike uppercase letter pairs.
/// Exactly the 12 pairs fixed by CONTEXT/RESEARCH for this phase — do not
/// extend without a corresponding decision update.
const HOMOGLYPHS: &[(char, char)] = &[
    ('А', 'A'),
    ('В', 'B'),
    ('Е', 'E'),
    ('К', 'K'),
    ('М', 'M'),
    ('Н', 'H'),
    ('О', 'O'),
    ('Р', 'P'),
    ('С', 'C'),
    ('Т', 'T'),
    ('Х', 'X'),
    ('У', 'Y'),
];

fn is_cyrillic_letter(c: char) -> bool {
    matches!(c, '\u{0400}'..='\u{04FF}') && c.is_alphabetic()
}

/// `true` if `s` contains at least one Cyrillic letter **and** at least one
/// Latin (ASCII) letter. Digits and punctuation are ignored — mixing is
/// specifically about letters from both alphabets appearing together, not
/// "any non-Russian text" (SPEC NUM-12: "буквы и кириллицы, и латиницы").
pub fn has_script_mix(s: &str) -> bool {
    let mut has_cyrillic = false;
    let mut has_latin = false;

    for c in s.chars() {
        if is_cyrillic_letter(c) {
            has_cyrillic = true;
        } else if c.is_ascii_alphabetic() {
            has_latin = true;
        }
        if has_cyrillic && has_latin {
            return true;
        }
    }

    false
}

/// Collapse `s` to a canonical uppercase "skeleton": every letter that has
/// a homoglyph counterpart in [`HOMOGLYPHS`] is replaced by its Latin
/// equivalent; every other character is uppercased as-is. Two numbers that
/// look identical but were typed with a different mix of alphabets produce
/// the same skeleton, which is exactly the property the doppelganger check
/// in NUM-12 relies on.
pub fn to_latin_skeleton(s: &str) -> String {
    s.chars()
        .map(|c| {
            let upper = c.to_uppercase().next().unwrap_or(c);
            HOMOGLYPHS
                .iter()
                .find(|(cyrillic, _)| *cyrillic == upper)
                .map(|(_, latin)| *latin)
                .unwrap_or(upper)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_script_mix_true_for_latin_and_cyrillic_letters_together() {
        // 'O' and 'P' are Latin, 'Г' is Cyrillic — mixed within one string.
        assert!(has_script_mix("OPГ-00-000001"));
    }

    #[test]
    fn has_script_mix_false_for_pure_cyrillic() {
        assert!(!has_script_mix("ОРГ-00-000001"));
    }

    #[test]
    fn has_script_mix_false_for_pure_latin() {
        assert!(!has_script_mix("ABC-001"));
    }

    #[test]
    fn to_latin_skeleton_collapses_lookalikes_to_the_same_value() {
        let cyrillic = to_latin_skeleton("ОРГ-00-000001");
        let mixed = to_latin_skeleton("OPГ-00-000001");
        assert_eq!(cyrillic, mixed);
    }

    #[test]
    fn to_latin_skeleton_uses_exactly_the_fixed_table() {
        assert_eq!(HOMOGLYPHS.len(), 12);
        // Every mapped pair round-trips to its Latin uppercase target.
        for &(cyrillic, latin) in HOMOGLYPHS {
            assert_eq!(to_latin_skeleton(&cyrillic.to_string()), latin.to_string());
        }
        // A Cyrillic letter with no homoglyph counterpart (e.g. Г) is left
        // as-is (uppercased), not mapped to anything.
        assert_eq!(to_latin_skeleton("г"), "Г");
    }
}
