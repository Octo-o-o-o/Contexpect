//! A small JSON Schema validator for the frozen product schemas.
//!
//! Supports the subset the schemas under `docs/schemas/` use: `type` (a name
//! or a list of names), `const`, `enum`, `properties`, `required`,
//! `additionalProperties` (boolean), `items`, `minItems`, `$defs` with local
//! `$ref` (`#/$defs/<name>`), plus the annotation `x-display-only` (a list of
//! field paths the Receipt digest rule reads; it constrains nothing here).
//! Anything else in a schema is a validation error, so an unsupported keyword
//! cannot silently pass: every node of the schema (including `$defs` and
//! branches the instance never reaches) is swept once before validation,
//! and `additionalProperties` / `minItems` apply on their own, not only when
//! `properties` / `items` happen to sit beside them.

use crate::json::Value;

const SUPPORTED_KEYWORDS: &[&str] = &[
    "$schema",
    "$id",
    "$defs",
    "$ref",
    "title",
    "description",
    "type",
    "const",
    "enum",
    "properties",
    "required",
    "additionalProperties",
    "items",
    "minItems",
    "x-display-only",
];

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Int(_) => "integer",
        Value::Str(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn type_matches(expected: &str, value: &Value) -> bool {
    match expected {
        "number" => matches!(value, Value::Int(_)),
        other => type_name(value) == other,
    }
}

fn resolve<'a>(root: &'a Value, schema: &'a Value) -> Result<&'a Value, String> {
    let Some(reference) = schema.get("$ref").and_then(Value::as_str) else {
        return Ok(schema);
    };
    // A `$ref` node carries nothing else: a sibling keyword would be dropped
    // on the floor, which is exactly the silent pass this validator refuses.
    if let Some(map) = schema.as_object()
        && let Some(extra) = map.keys().find(|k| *k != "$ref" && *k != "description")
    {
        return Err(format!("`$ref` node also carries `{extra}`; sibling keywords are not supported"));
    }
    let Some(name) = reference.strip_prefix("#/$defs/") else {
        return Err(format!("unsupported $ref `{reference}`; only #/$defs/<name> is supported"));
    };
    root.pointer(&["$defs", name])
        .ok_or_else(|| format!("$ref `{reference}` has no definition"))
}

/// Walk every schema node once, independent of any instance, and report
/// unsupported keywords and unsupported keyword *forms* (a schema-valued
/// `additionalProperties`, a non-integer `minItems`).
fn sweep(node: &Value, path: &str, errors: &mut Vec<String>) {
    let Some(map) = node.as_object() else {
        errors.push(format!("{path}: schema node is not an object"));
        return;
    };
    for (keyword, value) in map {
        match keyword.as_str() {
            "properties" | "$defs" => {
                if let Some(children) = value.as_object() {
                    for (name, child) in children {
                        sweep(child, &format!("{path}/{keyword}/{name}"), errors);
                    }
                } else {
                    errors.push(format!("{path}: `{keyword}` must be an object"));
                }
            }
            "items" => sweep(value, &format!("{path}/items"), errors),
            "additionalProperties" => {
                if !matches!(value, Value::Bool(_)) {
                    errors.push(format!("{path}: only boolean `additionalProperties` is supported"));
                }
            }
            "minItems" => {
                if !matches!(value, Value::Int(n) if *n >= 0) {
                    errors.push(format!("{path}: `minItems` must be a non-negative integer"));
                }
            }
            other if SUPPORTED_KEYWORDS.contains(&other) => {}
            other => errors.push(format!("{path}: unsupported schema keyword `{other}`")),
        }
    }
    if map.contains_key("$ref")
        && let Some(extra) = map.keys().find(|k| *k != "$ref" && *k != "description")
    {
        errors.push(format!("{path}: `$ref` node also carries `{extra}`"));
    }
}

fn check(root: &Value, schema: &Value, instance: &Value, path: &str, errors: &mut Vec<String>) {
    let schema = match resolve(root, schema) {
        Ok(schema) => schema,
        Err(message) => {
            errors.push(format!("{path}: {message}"));
            return;
        }
    };
    let Some(map) = schema.as_object() else {
        errors.push(format!("{path}: schema node is not an object"));
        return;
    };
    if let Some(expected) = map.get("type") {
        let names = expected.as_str_list();
        if names.is_empty() {
            errors.push(format!("{path}: `type` must be a name or a list of names"));
        } else if !names.iter().any(|name| type_matches(name, instance)) {
            errors.push(format!(
                "{path}: expected type {} but found {}",
                names.join("|"),
                type_name(instance)
            ));
            return;
        }
    }
    if let Some(constant) = map.get("const")
        && constant != instance
    {
        errors.push(format!("{path}: value does not equal the schema `const`"));
    }
    if let Some(Value::Array(allowed)) = map.get("enum")
        && !allowed.contains(instance)
    {
        errors.push(format!("{path}: value is not one of the `enum` members"));
    }
    if let Some(Value::Array(required)) = map.get("required") {
        for key in required.iter().filter_map(Value::as_str) {
            if instance.get(key).is_none() {
                errors.push(format!("{path}: missing required property `{key}`"));
            }
        }
    }
    if let Some(object) = instance.as_object() {
        let properties = map.get("properties").and_then(Value::as_object);
        if let Some(properties) = properties {
            for (key, child_schema) in properties {
                if let Some(child) = object.get(key) {
                    check(root, child_schema, child, &format!("{path}/{key}"), errors);
                }
            }
        }
        // `additionalProperties: false` stands on its own: with no
        // `properties` beside it, *every* property is additional.
        if map.get("additionalProperties") == Some(&Value::Bool(false)) {
            for key in object.keys() {
                if !properties.is_some_and(|p| p.contains_key(key)) {
                    errors.push(format!("{path}: additional property `{key}` is not allowed"));
                }
            }
        }
    }
    if let Some(items) = instance.as_array() {
        if let Some(Value::Int(min)) = map.get("minItems")
            && (items.len() as i64) < *min
        {
            errors.push(format!("{path}: fewer than minItems={min} items"));
        }
        if let Some(item_schema) = map.get("items") {
            for (index, item) in items.iter().enumerate() {
                check(root, item_schema, item, &format!("{path}/{index}"), errors);
            }
        }
    }
}

/// Validate `instance` against `schema`. `Err` lists every violation found.
/// The schema itself is swept first, so an unsupported keyword anywhere in
/// it — reached by this instance or not — is an error.
pub fn validate(schema: &Value, instance: &Value) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    sweep(schema, "#", &mut errors);
    if !errors.is_empty() {
        return Err(errors);
    }
    check(schema, schema, instance, "#", &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::parse;

    #[test]
    fn required_types_enums_refs_and_additional_properties_are_enforced() {
        let schema = parse(
            r##"{"type":"object","required":["a","b"],"additionalProperties":false,
                "properties":{"a":{"type":"string","enum":["x","y"]},"b":{"$ref":"#/$defs/pair"},"c":{"type":["string","null"]}},
                "$defs":{"pair":{"type":"array","items":{"type":"integer"},"minItems":1}}}"##,
        )
        .unwrap();
        assert!(validate(&schema, &parse(r#"{"a":"x","b":[1,2],"c":null}"#).unwrap()).is_ok());
        let errors = validate(&schema, &parse(r#"{"a":"z","b":[],"d":1}"#).unwrap()).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("enum")), "{errors:?}");
        assert!(errors.iter().any(|e| e.contains("minItems")), "{errors:?}");
        assert!(errors.iter().any(|e| e.contains("additional property `d`")), "{errors:?}");
        let errors = validate(&schema, &parse(r#"{"a":"x"}"#).unwrap()).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("missing required property `b`")), "{errors:?}");
        // An unsupported keyword is an error, never a silent pass.
        let odd = parse(r#"{"type":"object","patternProperties":{}}"#).unwrap();
        assert!(validate(&odd, &parse("{}").unwrap()).is_err());
    }

    #[test]
    fn keywords_apply_on_their_own_and_unreached_branches_are_still_swept() {
        // `additionalProperties: false` with no `properties` beside it.
        let bare = parse(r#"{"type":"object","additionalProperties":false}"#).unwrap();
        assert!(validate(&bare, &parse(r#"{"x":1}"#).unwrap()).is_err());
        assert!(validate(&bare, &parse("{}").unwrap()).is_ok());
        // `minItems` with no `items` beside it.
        let min = parse(r#"{"type":"array","minItems":1}"#).unwrap();
        assert!(validate(&min, &parse("[]").unwrap()).is_err());
        assert!(validate(&min, &parse("[1]").unwrap()).is_ok());
        // An unsupported keyword under an absent property is still an error.
        let deep = parse(r#"{"type":"object","properties":{"a":{"type":"string","pattern":"x"}}}"#).unwrap();
        assert!(validate(&deep, &parse("{}").unwrap()).is_err());
        // Unsupported keywords under `$defs` and `items` are swept too.
        let defs = parse(r#"{"type":"object","$defs":{"p":{"type":"string","format":"date"}}}"#).unwrap();
        assert!(validate(&defs, &parse("{}").unwrap()).is_err());
        let items = parse(r#"{"type":"array","items":{"type":"string","maxLength":3}}"#).unwrap();
        assert!(validate(&items, &parse("[]").unwrap()).is_err());
        // A schema-valued additionalProperties is refused, not ignored.
        let schema_ap = parse(r#"{"type":"object","additionalProperties":{"type":"string"}}"#).unwrap();
        assert!(validate(&schema_ap, &parse(r#"{"x":1}"#).unwrap()).is_err());
        // `$ref` with a sibling keyword is refused, not silently narrowed.
        let sibling = parse(r##"{"type":"object","properties":{"a":{"$ref":"#/$defs/p","enum":["x"]}},"$defs":{"p":{"type":"string"}}}"##).unwrap();
        assert!(validate(&sibling, &parse(r#"{"a":"y"}"#).unwrap()).is_err());
    }
}
