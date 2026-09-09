//! Settings schema, defaults, and validation.
//!
//! The schema is data rather than prose so one definition serves three
//! callers: `put_settings` enforces it, the API publishes it, and the UI
//! renders its editor from it. A UI that hardcoded the field list would drift
//! from what the store actually accepts.

use ctxpect_schema::{array, object, string, Value};

use crate::StoreError;

/// What a settings field may hold.
pub enum FieldSpec {
    /// One of a fixed set of strings.
    Enum(&'static [&'static str]),
    Bool,
    /// A boolean that must keep one value. Used for invariants that are not
    /// preferences: they are recorded here so they cannot be edited away.
    ConstBool(bool),
    Int {
        min: i64,
        max: i64,
    },
    Object(&'static [(&'static str, FieldSpec)]),
}

/// Every settings field. A document with unknown or missing fields is
/// refused rather than merged, so a typo cannot silently do nothing and a
/// dropped field cannot silently take a default.
pub const SETTINGS_SCHEMA: &[(&str, FieldSpec)] = &[
    ("privacy_mode", FieldSpec::Enum(&["default", "screenshot"])),
    ("screenshot_privacy", FieldSpec::Bool),
    // R07: revealing masked text is not an egress grant. That is an
    // invariant of the product, not a user preference, so it cannot be
    // switched off through settings.
    ("unmask_does_not_grant_egress", FieldSpec::ConstBool(true)),
    ("copy_confirm", FieldSpec::Bool),
    ("locale", FieldSpec::Enum(&["zh-CN", "en"])),
    (
        "retention_days",
        FieldSpec::Int {
            min: 1,
            max: 3650,
        },
    ),
    ("vault", FieldSpec::Enum(&["metadata-only", "required"])),
    ("notifications", FieldSpec::Bool),
    (
        "resource_limits",
        FieldSpec::Object(&[
            // Recorded and validated, but this process does not bound its
            // own resident memory: doing so portably needs cgroups or
            // setrlimit, which this slice does not use. The published schema
            // says so, rather than letting the field read as a working knob.
            (
                "daemon_rss_mb",
                FieldSpec::Int {
                    min: 64,
                    max: 65_536,
                },
            ),
            (
                "scan_files",
                FieldSpec::Int {
                    min: 1,
                    max: 10_000_000,
                },
            ),
        ]),
    ),
    // Only the local heuristic exists in this slice. Naming another adapter
    // would make the advisor claim an analysis path that is not implemented.
    ("analysis_adapter", FieldSpec::Enum(&["none"])),
];

impl FieldSpec {
    /// Machine-readable description, published so the UI renders the editor
    /// from the same definition the store enforces.
    fn to_value(&self) -> Value {
        match self {
            FieldSpec::Enum(values) => object([
                ("kind", string("enum")),
                (
                    "values",
                    array(values.iter().map(|item| string(*item)).collect::<Vec<_>>()),
                ),
            ]),
            FieldSpec::Bool => object([("kind", string("bool"))]),
            FieldSpec::ConstBool(value) => object([
                ("kind", string("const_bool")),
                ("value", Value::Bool(*value)),
                (
                    "reason",
                    string("product invariant; not a user preference"),
                ),
            ]),
            FieldSpec::Int { min, max } => object([
                ("kind", string("int")),
                ("min", Value::Int(*min)),
                ("max", Value::Int(*max)),
            ]),
            FieldSpec::Object(fields) => object([
                ("kind", string("object")),
                (
                    "fields",
                    object(
                        fields
                            .iter()
                            .map(|(name, spec)| (*name, spec.to_value()))
                            .collect::<Vec<_>>(),
                    ),
                ),
            ]),
        }
    }

    fn check(&self, path: &str, value: &Value) -> Result<(), StoreError> {
        match self {
            FieldSpec::Enum(allowed) => {
                let Some(text) = value.as_str() else {
                    return Err(type_error(path, "string"));
                };
                if !allowed.contains(&text) {
                    return Err(StoreError::new(
                        "settings.value_not_allowed",
                        format!("`{path}` must be one of {allowed:?}, found `{text}`"),
                    ));
                }
                Ok(())
            }
            FieldSpec::Bool => value
                .as_bool()
                .map(|_| ())
                .ok_or_else(|| type_error(path, "boolean")),
            FieldSpec::ConstBool(expected) => {
                let Some(actual) = value.as_bool() else {
                    return Err(type_error(path, "boolean"));
                };
                if actual != *expected {
                    return Err(StoreError::new(
                        "settings.invariant_not_editable",
                        format!("`{path}` is a product invariant fixed at {expected}"),
                    ));
                }
                Ok(())
            }
            FieldSpec::Int { min, max } => {
                let Some(number) = value.as_i64() else {
                    return Err(type_error(path, "integer"));
                };
                if number < *min || number > *max {
                    return Err(StoreError::new(
                        "settings.out_of_range",
                        format!("`{path}` must be between {min} and {max}, found {number}"),
                    ));
                }
                Ok(())
            }
            FieldSpec::Object(fields) => check_fields(path, fields, value),
        }
    }
}

fn type_error(path: &str, expected: &str) -> StoreError {
    StoreError::new(
        "settings.type_mismatch",
        format!("`{path}` must be a {expected}"),
    )
}

fn check_fields(
    prefix: &str,
    fields: &[(&str, FieldSpec)],
    value: &Value,
) -> Result<(), StoreError> {
    let Value::Object(map) = value else {
        return Err(type_error(if prefix.is_empty() { "settings" } else { prefix }, "object"));
    };
    for (name, spec) in fields {
        let path = if prefix.is_empty() {
            (*name).to_string()
        } else {
            format!("{prefix}.{name}")
        };
        let Some(child) = map.get(*name) else {
            return Err(StoreError::new(
                "settings.field_missing",
                format!("`{path}` is required; settings are replaced whole, not merged"),
            ));
        };
        spec.check(&path, child)?;
    }
    for key in map.keys() {
        if !fields.iter().any(|(name, _)| name == key) {
            let path = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };
            return Err(StoreError::new(
                "settings.field_unknown",
                format!("`{path}` is not a settings field; a misspelled key must not be stored silently"),
            ));
        }
    }
    Ok(())
}

/// Validate a complete settings document.
///
/// Settings are replaced whole, so this checks the whole document: every
/// declared field present and well-typed, and nothing else.
pub fn validate_settings(value: &Value) -> Result<(), StoreError> {
    check_fields("", SETTINGS_SCHEMA, value)
}

/// The schema as a document, for the API and the UI editor.
/// Fields the product stores and validates but does not act on, with the
/// reason. Publishing this stops a setting from reading as a working knob.
pub const UNENFORCED_FIELDS: &[(&str, &str)] = &[
    (
        "resource_limits.daemon_rss_mb",
        "this process does not bound its own resident memory; doing so portably needs cgroups or setrlimit, which this slice does not use",
    ),
    (
        "retention_days",
        "no retention sweep exists in this slice; Receipts are removed only by explicit redact/delete, so this value is stored but never acted on",
    ),
    (
        "copy_confirm",
        "the UI always confirms before copying revealed text; it does not read this field, so it cannot be switched off from here",
    ),
    (
        "privacy_mode",
        "the UI's privacy mode is a session toggle in the coordinate bar and is not read from settings in this slice",
    ),
    (
        "screenshot_privacy",
        "screenshot privacy is the UI's session toggle (`privacy_mode = screenshot`); this stored flag is not read by any surface in this slice",
    ),
];

#[must_use]
pub fn settings_schema() -> Value {
    object([
        ("schema", string("ctxpect-settings-schema-v1")),
        (
            "unenforced",
            object(
                UNENFORCED_FIELDS
                    .iter()
                    .map(|(path, reason)| (*path, string(*reason)))
                    .collect::<Vec<_>>(),
            ),
        ),
        (
            "fields",
            object(
                SETTINGS_SCHEMA
                    .iter()
                    .map(|(name, spec)| (*name, spec.to_value()))
                    .collect::<Vec<_>>(),
            ),
        ),
    ])
}

#[must_use]
pub fn default_settings() -> Value {
    object([
        ("privacy_mode", string("default")),
        ("screenshot_privacy", Value::Bool(false)),
        ("unmask_does_not_grant_egress", Value::Bool(true)),
        ("copy_confirm", Value::Bool(true)),
        ("locale", string("zh-CN")),
        ("retention_days", Value::Int(30)),
        ("vault", string("metadata-only")),
        ("notifications", Value::Bool(true)),
        (
            "resource_limits",
            object([
                ("daemon_rss_mb", Value::Int(512)),
                ("scan_files", Value::Int(100_000)),
            ]),
        ),
        ("analysis_adapter", string("none")),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctxpect_schema::parse;

    fn with(field: &str, json: &str) -> Value {
        let mut doc = match default_settings() {
            Value::Object(map) => map,
            _ => unreachable!(),
        };
        doc.insert(field.to_string(), parse(json).unwrap());
        Value::Object(doc)
    }

    #[test]
    fn the_defaults_satisfy_their_own_schema() {
        validate_settings(&default_settings()).expect("defaults must validate");
    }

    #[test]
    fn an_invariant_cannot_be_edited_away() {
        // R07: revealing masked text never grants egress. Settings must not
        // be able to assert otherwise.
        let err = validate_settings(&with("unmask_does_not_grant_egress", "false"))
            .expect_err("invariant");
        assert_eq!(err.code, "settings.invariant_not_editable");
    }

    #[test]
    fn values_outside_the_declared_set_are_refused() {
        let err = validate_settings(&with("privacy_mode", r#""pwned""#)).expect_err("enum");
        assert_eq!(err.code, "settings.value_not_allowed");

        // Naming an adapter that does not exist would make the advisor claim
        // an analysis path this slice does not implement.
        let err =
            validate_settings(&with("analysis_adapter", r#""gpt-5""#)).expect_err("adapter");
        assert_eq!(err.code, "settings.value_not_allowed");

        let err = validate_settings(&with("retention_days", "0")).expect_err("range");
        assert_eq!(err.code, "settings.out_of_range");

        let err = validate_settings(&with("retention_days", r#""thirty""#)).expect_err("type");
        assert_eq!(err.code, "settings.type_mismatch");
    }

    #[test]
    fn unknown_and_missing_fields_are_refused_rather_than_merged() {
        let err = validate_settings(&with("surprise", "true")).expect_err("unknown");
        assert_eq!(err.code, "settings.field_unknown");

        let mut doc = match default_settings() {
            Value::Object(map) => map,
            _ => unreachable!(),
        };
        doc.remove("vault");
        let err = validate_settings(&Value::Object(doc)).expect_err("missing");
        assert_eq!(err.code, "settings.field_missing");
    }

    #[test]
    fn nested_limits_are_checked_too() {
        let err = validate_settings(&with("resource_limits", r#"{"daemon_rss_mb":1,"scan_files":10}"#))
            .expect_err("nested range");
        assert_eq!(err.code, "settings.out_of_range");

        let err = validate_settings(&with(
            "resource_limits",
            r#"{"daemon_rss_mb":512,"scan_files":100000,"extra":1}"#,
        ))
        .expect_err("nested unknown");
        assert_eq!(err.code, "settings.field_unknown");
    }

    #[test]
    fn the_published_schema_names_every_field_it_enforces() {
        let published = settings_schema();
        let fields = published.get("fields").expect("fields");
        for (name, _) in SETTINGS_SCHEMA {
            assert!(fields.get(name).is_some(), "{name} missing from schema doc");
        }
    }
}
