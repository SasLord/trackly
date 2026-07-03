//! `DocSpec` — typed intermediate representation for PDF rendering.
//!
//! Sits between the MiniJinja template render stage (`String`) and the krilla
//! draw stage (`Vec<u8>` PDF bytes). The serde tag `type` is a frontend-friendly
//! snake_case discriminator that lines up with the upstream template contract
//! described in `03-CONTEXT.md` §D-PDF-Render-Path-01.
//!
//! All variants are fully typed — no `raw_pdf_op: Vec<u8>` (RESEARCH
//! §Anti-Patterns). This guarantees that user-supplied templates cannot inject
//! arbitrary PDF operators.

use serde::{Deserialize, Serialize};

/// Top-level document spec — the deserialized JSON tree that the template
/// renderer produces and that krilla consumes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocSpec {
    pub title: String,
    pub header: HeaderBlock,
    pub sections: Vec<Section>,
}

/// Organization header — shown at the top of every printed document.
///
/// `logo_path` is an absolute path the renderer resolves; an `Option<String>`
/// so templates can render an org without a logo. Phase 3 plan 04 fills this
/// from `OrganizationService` + `Paths::root()`.
///
/// Phase 7 plan 02: `logo_bytes` + `logo_mime` added for BLOB logo from org_settings.
/// Priority: if `logo_bytes` is Some, use them directly; else fall back to `logo_path`.
/// Both fields have `#[serde(default)]` for backward compat — existing templates that
/// don't include these fields deserialize correctly with `None` (RESEARCH Pitfall 7).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct HeaderBlock {
    pub org_name: String,
    pub org_inn: String,
    pub org_kpp: String,
    pub org_address: String,
    /// Filesystem path to logo (backward compat from Phase 3 plan 04).
    /// Used only when `logo_bytes` is None.
    pub logo_path: Option<String>,
    /// Logo raw bytes from org_settings BLOB (Phase 7 plan 02).
    /// Takes priority over `logo_path` when present.
    #[serde(default)]
    pub logo_bytes: Option<Vec<u8>>,
    /// MIME type of the BLOB logo ("image/png" | "image/jpeg" | "image/svg+xml").
    #[serde(default)]
    pub logo_mime: Option<String>,
    /// Extended requisites (PDFA-03, Phase 14): phone/fax/email/OKPO/OGRN.
    /// `#[serde(default)]` keeps old templates/JSON that omit these keys
    /// deserializing correctly with an empty string (RESEARCH Pitfall 7).
    #[serde(default)]
    pub org_phone: String,
    #[serde(default)]
    pub org_fax: String,
    #[serde(default)]
    pub org_email: String,
    #[serde(default)]
    pub org_okpo: String,
    #[serde(default)]
    pub org_ogrn: String,
    /// Russian display label for the act, e.g. «Акт приёма-передачи №42».
    pub act_label: String,
    /// Localized human-readable date, e.g. «28 мая 2026 г.».
    pub date_label: String,
}

/// Renderable section — a tagged enum so the JSON shape is
/// `{ "type": "heading", "level": 1, "text": "..." }`.
///
/// The tag `type` and `rename_all = "snake_case"` discriminator must NOT be
/// changed without updating both downstream templates and the frontend
/// preview renderer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Section {
    /// Paragraph of body text.
    Paragraph {
        text: String,
        #[serde(default)]
        style: TextStyle,
    },
    /// Heading — `level` is 1..=3.
    Heading { level: u8, text: String },
    /// Two-column key/value table — used for «Сдал/Принял/Дата» blocks.
    KeyValueTable { rows: Vec<KvRow> },
    /// Multi-column tabular data — used for the act items list.
    ItemsTable {
        columns: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    /// Two side-by-side signature lines.
    Signature {
        left_label: String,
        right_label: String,
        #[serde(default = "default_spacer_pt")]
        spacer_pt: f32,
    },
    /// Vertical spacer.
    Spacer { height_pt: f32 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KvRow {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TextStyle {
    #[default]
    Regular,
    Bold,
}

fn default_spacer_pt() -> f32 {
    24.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_docspec() -> DocSpec {
        DocSpec {
            title: "Акт приёма-передачи №42".into(),
            header: HeaderBlock {
                org_name: "ООО Ромашка".into(),
                org_inn: "7700000000".into(),
                org_kpp: "770001001".into(),
                org_address: "г. Москва, ул. Ленина, 1".into(),
                logo_path: None,
                logo_bytes: None,
                logo_mime: None,
                act_label: "Акт приёма-передачи №42".into(),
                date_label: "28 мая 2026 г.".into(),
                ..Default::default()
            },
            sections: vec![
                Section::Heading {
                    level: 1,
                    text: "Акт приёма-передачи №42".into(),
                },
                Section::Spacer { height_pt: 12.0 },
                Section::KeyValueTable {
                    rows: vec![
                        KvRow {
                            key: "Сдал".into(),
                            value: "Сидоров-Петроградский Иван Александрович (ё)".into(),
                        },
                        KvRow {
                            key: "Принял".into(),
                            value: "Петров Пётр Петрович".into(),
                        },
                    ],
                },
                Section::ItemsTable {
                    columns: vec!["№".into(), "Наименование".into(), "Кол-во".into()],
                    rows: vec![vec!["1".into(), "Ноутбук Lenovo".into(), "1".into()]],
                },
                Section::Signature {
                    left_label: "Сдал: ____________".into(),
                    right_label: "Принял: ____________".into(),
                    spacer_pt: 30.0,
                },
            ],
        }
    }

    #[test]
    fn round_trip_full_doc() {
        let spec = sample_docspec();
        let json = serde_json::to_string(&spec).expect("serialize");
        let back: DocSpec = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, spec);
    }

    #[test]
    fn section_enum_tagged_serde() {
        let heading = Section::Heading {
            level: 1,
            text: "Hello".into(),
        };
        let json = serde_json::to_value(&heading).expect("serialize heading");
        assert_eq!(json["type"], "heading");
        assert_eq!(json["level"], 1);
        assert_eq!(json["text"], "Hello");

        let spacer = Section::Spacer { height_pt: 12.0 };
        let json = serde_json::to_value(&spacer).expect("serialize spacer");
        assert_eq!(json["type"], "spacer");
        assert_eq!(json["height_pt"], 12.0);

        let kv = Section::KeyValueTable {
            rows: vec![KvRow {
                key: "k".into(),
                value: "v".into(),
            }],
        };
        let json = serde_json::to_value(&kv).expect("serialize kv");
        assert_eq!(json["type"], "key_value_table");

        let items = Section::ItemsTable {
            columns: vec!["a".into()],
            rows: vec![vec!["1".into()]],
        };
        let json = serde_json::to_value(&items).expect("serialize items");
        assert_eq!(json["type"], "items_table");

        let sig = Section::Signature {
            left_label: "L".into(),
            right_label: "R".into(),
            spacer_pt: 24.0,
        };
        let json = serde_json::to_value(&sig).expect("serialize signature");
        assert_eq!(json["type"], "signature");
    }

    #[test]
    fn text_style_default_is_regular() {
        let style: TextStyle = serde_json::from_value(serde_json::json!("regular")).expect("parse");
        assert_eq!(style, TextStyle::Regular);
        // Default derive
        assert_eq!(TextStyle::default(), TextStyle::Regular);
    }

    #[test]
    fn signature_spacer_pt_defaults_when_absent() {
        let json = serde_json::json!({
            "type": "signature",
            "left_label": "L",
            "right_label": "R"
        });
        let section: Section = serde_json::from_value(json).expect("parse");
        match section {
            Section::Signature { spacer_pt, .. } => assert_eq!(spacer_pt, 24.0),
            _ => panic!("wrong variant"),
        }
    }
}
