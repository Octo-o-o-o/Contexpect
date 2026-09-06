//! Unique stdout/stderr redaction boundary for inspect output.
//!
//! Every JSON and human byte written by this crate passes through
//! [`redact_output`] before it is printed. `snapshot_digest` is computed on
//! the redacted JSON.

use crate::args::InspectArgs;
use crate::jsonutil::{arr, obj, s, strip_time_fields, with_snapshot_digest};
use ctxpect_schema::{Value, canonical_json, parse};
use std::ffi::OsString;
use std::fs;
use std::path::Path;

const RESIDUAL_CODE: &str = "redaction.residual_absolute_path";

/// Project / codex-home forms (original absolute and canonicalized) to replace.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RedactRoots {
    pub project: Vec<String>,
    pub codex_home: Vec<String>,
}

/// Result of the unique output boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactOutcome {
    pub text: String,
    pub residual: bool,
}

impl RedactRoots {
    #[must_use]
    pub fn new(project: Option<&Path>, codex_home: Option<&Path>) -> Self {
        Self {
            project: path_forms(project),
            codex_home: path_forms(codex_home),
        }
    }

    #[must_use]
    pub fn from_inspect_args(args: &InspectArgs) -> Self {
        Self::new(Some(args.project.as_path()), args.codex_home.as_deref())
    }

    /// Best-effort roots from argv so usage errors still have replacement forms.
    #[must_use]
    pub fn from_argv(args: &[OsString]) -> Self {
        let mut project = None;
        let mut codex_home = None;
        let mut i = 0usize;
        while i < args.len() {
            let Some(token) = args[i].to_str() else {
                i += 1;
                continue;
            };
            if let Some(value) = token.strip_prefix("--project=") {
                project = Some(value.to_string());
            } else if token == "--project" {
                if let Some(value) = args.get(i + 1).and_then(|item| item.to_str())
                    && !value.starts_with("--")
                {
                    project = Some(value.to_string());
                    i += 1;
                }
            } else if let Some(value) = token.strip_prefix("--codex-home=") {
                codex_home = Some(value.to_string());
            } else if token == "--codex-home"
                && let Some(value) = args.get(i + 1).and_then(|item| item.to_str())
                && !value.starts_with("--")
            {
                codex_home = Some(value.to_string());
                i += 1;
            }
            i += 1;
        }
        Self::new(
            project.as_deref().map(Path::new),
            codex_home.as_deref().map(Path::new),
        )
    }
}

/// Replace declared roots, HOME at path boundaries, then residual abs paths.
///
/// Root forms and HOME share `replace_at_path_boundaries`: a match must
/// sit at the start of the string or after a declared left boundary, end at
/// the end of the string or a declared right boundary, and must not start
/// inside an already-emitted placeholder token. Longer root forms are
/// applied first. Residual `/` and Windows-drive matches still become
/// `<abs>` after those replacements.
#[must_use]
pub fn redact_output(text: &str, roots: &RedactRoots) -> RedactOutcome {
    let home = std::env::var("HOME").ok();
    redact_output_with_home(text, roots, home.as_deref())
}

fn redact_output_with_home(text: &str, roots: &RedactRoots, home: Option<&str>) -> RedactOutcome {
    let mut text = text.to_string();
    text = replace_forms(text, &roots.project, "<project>");
    text = replace_forms(text, &roots.codex_home, "<codex-home>");
    if let Some(home) = home {
        text = replace_home_at_path_boundaries(text, home);
    }
    let (text, unix) = replace_residual_unix(text);
    let (text, windows) = replace_residual_windows(text);
    RedactOutcome {
        text,
        residual: unix || windows,
    }
}

/// Replace `HOME` only when it is an absolute path prefix at a token boundary.
///
/// `HOME` must start with `/` and have length > 1. Matching uses the same
/// path-boundary and placeholder-skip rules as root-form replacement.
fn replace_home_at_path_boundaries(text: String, home: &str) -> String {
    if !home.starts_with('/') || home.len() <= 1 {
        return text;
    }
    replace_at_path_boundaries(text, home, "<home>")
}

/// Replace `needle` only as a path prefix at a declared token boundary.
///
/// A match must sit at the start of the string or after `"`, `'`, ASCII
/// whitespace, `(`, `=`, or `:`; the byte after the match must be the end of
/// the string or `/`, `"`, `'`, ASCII whitespace, `)`, or `,`. Placeholder
/// tokens already in the text (`<project>`, `<codex-home>`, `<home>`, `<abs>`)
/// are copied through and are not scanned for a second replacement.
fn replace_at_path_boundaries(text: String, needle: &str, placeholder: &str) -> String {
    if needle.len() < 2 {
        return text;
    }
    let needle_bytes = needle.as_bytes();
    let placeholder_bytes = placeholder.as_bytes();
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if let Some(skip) = placeholder_token_len(bytes, i) {
            out.extend_from_slice(&bytes[i..i + skip]);
            i += skip;
            continue;
        }
        let found = bytes[i..].starts_with(needle_bytes)
            && path_left_ok(bytes, i)
            && path_right_ok(bytes, i + needle_bytes.len());
        if found {
            out.extend_from_slice(placeholder_bytes);
            i += needle_bytes.len();
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).expect("redaction preserves UTF-8")
}

fn placeholder_token_len(bytes: &[u8], i: usize) -> Option<usize> {
    const MARKS: [&[u8]; 4] = [b"<project>", b"<codex-home>", b"<home>", b"<abs>"];
    for mark in MARKS {
        if bytes[i..].starts_with(mark) {
            return Some(mark.len());
        }
    }
    None
}

fn path_left_ok(bytes: &[u8], i: usize) -> bool {
    if i == 0 {
        return true;
    }
    let prev = bytes[i - 1];
    matches!(prev, b'"' | b'\'' | b'(' | b'=' | b':') || prev.is_ascii_whitespace()
}

fn path_right_ok(bytes: &[u8], after: usize) -> bool {
    if after == bytes.len() {
        return true;
    }
    let next = bytes[after];
    matches!(next, b'/' | b'"' | b'\'' | b')' | b',') || next.is_ascii_whitespace()
}

/// Apply [`redact_output`] to canonical JSON, record residual warnings, then digest.
#[must_use]
pub fn redact_json_envelope(envelope: Value, roots: &RedactRoots) -> Value {
    let mut body = strip_time_fields(&envelope);
    if let Value::Object(map) = &mut body {
        map.remove("snapshot_digest");
    }
    let outcome = redact_output(&canonical_json(&body), roots);
    let mut parsed = match parse(&outcome.text) {
        Ok(value) => value,
        Err(_) => body,
    };
    if outcome.residual {
        append_residual_warning(&mut parsed);
        let again = redact_output(&canonical_json(&strip_time_fields(&parsed)), roots);
        if let Ok(value) = parse(&again.text) {
            parsed = value;
        }
    }
    with_snapshot_digest(parsed)
}

fn path_forms(path: Option<&Path>) -> Vec<String> {
    let Some(path) = path else {
        return Vec::new();
    };
    let mut forms = Vec::new();
    push_abs_form(&mut forms, path.to_str());
    if let Ok(canon) = fs::canonicalize(path) {
        push_abs_form(&mut forms, canon.to_str());
    }
    forms.sort_by_key(|item| std::cmp::Reverse(item.len()));
    forms.dedup();
    forms
}

fn push_abs_form(forms: &mut Vec<String>, raw: Option<&str>) {
    let Some(raw) = raw else {
        return;
    };
    if raw.len() < 2 {
        return;
    }
    if !(raw.starts_with('/') || looks_like_windows_abs(raw)) {
        return;
    }
    let trimmed = raw.trim_end_matches(['/', '\\']);
    if trimmed.len() >= 2 {
        push_unique(forms, trimmed.to_string());
    }
    push_unique(forms, raw.to_string());
}

fn looks_like_windows_abs(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

fn push_unique(items: &mut Vec<String>, value: String) {
    if !items.iter().any(|item| item == &value) {
        items.push(value);
    }
}

fn replace_forms(mut text: String, forms: &[String], placeholder: &str) -> String {
    let mut ordered = forms.to_vec();
    ordered.sort_by_key(|item| std::cmp::Reverse(item.len()));
    for form in ordered {
        if form.len() < 2 {
            continue;
        }
        text = replace_at_path_boundaries(text, &form, placeholder);
    }
    text
}

fn is_placeholder_prev(byte: u8) -> bool {
    // `<project>/rel` and `<codex-home>/rel` are the I03 placeholder form.
    // The slash sits after `>`, not `<`; both marker characters are exempt.
    byte == b'<' || byte == b'>' || byte.is_ascii_alphanumeric() || byte == b'_'
}

fn is_abs_start(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'.'
}

fn is_unix_abs_cont(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'/' | b'-')
}

fn replace_residual_unix(text: String) -> (String, bool) {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    let mut residual = false;
    while i < bytes.len() {
        let prev_ok = i == 0 || !is_placeholder_prev(bytes[i - 1]);
        if bytes[i] == b'/' && i + 1 < bytes.len() && is_abs_start(bytes[i + 1]) && prev_ok {
            residual = true;
            i += 1;
            while i < bytes.len() && is_unix_abs_cont(bytes[i]) {
                i += 1;
            }
            out.extend_from_slice(b"<abs>");
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    (
        String::from_utf8(out).expect("redaction preserves UTF-8"),
        residual,
    )
}

fn replace_residual_windows(text: String) -> (String, bool) {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    let mut residual = false;
    while i < bytes.len() {
        let drive = i + 3 < bytes.len()
            && bytes[i].is_ascii_alphabetic()
            && bytes[i + 1] == b':'
            && (bytes[i + 2] == b'\\' || bytes[i + 2] == b'/')
            && is_abs_start(bytes[i + 3]);
        if drive {
            residual = true;
            i += 3;
            while i < bytes.len() && (is_unix_abs_cont(bytes[i]) || bytes[i] == b'\\') {
                i += 1;
            }
            out.extend_from_slice(b"<abs>");
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    (
        String::from_utf8(out).expect("redaction preserves UTF-8"),
        residual,
    )
}

fn append_residual_warning(value: &mut Value) {
    let warning = obj([("code", s(RESIDUAL_CODE))]);
    let Value::Object(map) = value else {
        return;
    };
    match map.get_mut("warnings") {
        Some(Value::Array(items)) => {
            let already = items
                .iter()
                .any(|item| item.get("code").and_then(Value::as_str) == Some(RESIDUAL_CODE));
            if !already {
                items.push(warning);
            }
        }
        _ => {
            map.insert("warnings".to_string(), arr([warning]));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jsonutil::{arr, obj, s, with_snapshot_digest};
    use ctxpect_schema::canonical_json;

    #[test]
    fn redact_output_replaces_roots_home_and_residual_paths() {
        let roots = RedactRoots {
            project: vec!["/tmp/proj-unique-redact".to_string()],
            codex_home: vec!["/var/tmp/home-unique-redact".to_string()],
        };
        let input = "/tmp/proj-unique-redact/AGENTS.md and /etc/passwd and C:\\Users\\x\\y";
        let outcome = redact_output(input, &roots);
        assert!(outcome.residual);
        assert!(!outcome.text.contains("/tmp/proj-unique-redact"));
        assert!(!outcome.text.contains("/etc/passwd"));
        assert!(!outcome.text.contains("/etc"));
        assert!(!outcome.text.contains("C:\\Users"));
        assert!(outcome.text.contains("<project>"));
        assert!(outcome.text.contains("<abs>"));
    }

    #[test]
    fn redact_json_envelope_records_residual_warning_and_strips_abs() {
        let envelope = obj([
            ("schema_version", Value::Int(1)),
            ("path", s("/etc/passwd")),
            ("warnings", arr([])),
        ]);
        let redacted = redact_json_envelope(envelope, &RedactRoots::default());
        let text = canonical_json(&redacted);
        assert!(!text.contains("/etc/passwd"), "{text}");
        assert!(!text.contains("/etc"), "{text}");
        let warnings = redacted
            .get("warnings")
            .and_then(Value::as_array)
            .expect("warnings");
        assert!(
            warnings
                .iter()
                .any(|item| { item.get("code").and_then(Value::as_str) == Some(RESIDUAL_CODE) }),
            "{text}"
        );
        assert!(
            redacted
                .get("snapshot_digest")
                .and_then(Value::as_str)
                .is_some()
        );
    }

    #[test]
    fn redact_json_envelope_is_noop_on_placeholder_corpus_shape() {
        let envelope = obj([
            ("schema_version", Value::Int(1)),
            ("command", s("inspect")),
            ("exit_code", Value::Int(0)),
            ("receipt_kind", s("development-snapshot")),
            ("schema", s("dev-inspect-v0")),
            (
                "scope",
                obj([("cwd", s("<project>/")), ("version", s("0.147.0"))]),
            ),
            ("warnings", arr([])),
            (
                "explanation",
                arr([obj([
                    ("kind", s("included")),
                    ("path", s("<project>/AGENTS.md")),
                    ("rule_id", s("G1")),
                ])]),
            ),
        ]);
        let before = with_snapshot_digest(envelope.clone());
        let after = redact_json_envelope(envelope, &RedactRoots::default());
        assert_eq!(canonical_json(&before), canonical_json(&after));
        assert_eq!(before.get("snapshot_digest"), after.get("snapshot_digest"));
        let warnings = after
            .get("warnings")
            .and_then(Value::as_array)
            .expect("warnings");
        assert!(warnings.is_empty());
    }

    #[test]
    fn placeholders_are_not_treated_as_residual_absolute_paths() {
        let outcome = redact_output(
            "cwd: <project>/\nglobal: <codex-home>/AGENTS.md\nnative: $CODEX_HOME/AGENTS.md",
            &RedactRoots::default(),
        );
        assert!(!outcome.residual, "{}", outcome.text);
        assert_eq!(
            outcome.text,
            "cwd: <project>/\nglobal: <codex-home>/AGENTS.md\nnative: $CODEX_HOME/AGENTS.md"
        );
    }

    #[test]
    fn redact_output_replaces_home_only_at_path_boundaries() {
        let roots = RedactRoots::default();
        assert_eq!(
            replace_home_at_path_boundaries("/tmp/x /tmpfoo".to_string(), "/tmp"),
            "<home>/x /tmpfoo"
        );
        let outcome = redact_output_with_home("/tmp/x /tmpfoo", &roots, Some("/tmp"));
        assert_eq!(outcome.text, "<home>/x <abs>");
        assert!(outcome.residual);
        assert!(!outcome.text.contains("/tmp/x"));
        assert!(!outcome.text.contains("<home>foo"));
    }

    #[test]
    fn redact_output_does_not_replace_home_inside_project_placeholder() {
        let roots = RedactRoots::default();
        assert_eq!(
            replace_home_at_path_boundaries("<project>/target/tmp".to_string(), "/tmp"),
            "<project>/target/tmp"
        );
        let outcome = redact_output_with_home("<project>/target/tmp", &roots, Some("/tmp"));
        assert_eq!(outcome.text, "<project>/target/tmp");
        assert!(!outcome.residual);
    }

    #[test]
    fn redact_output_still_replaces_home_path_prefix_in_error_text() {
        let roots = RedactRoots::default();
        let home = "/var/tmp/cx-r11-home-unit";
        let outcome = redact_output_with_home(
            "failed to read /var/tmp/cx-r11-home-unit/secret (NotFound)",
            &roots,
            Some(home),
        );
        assert_eq!(outcome.text, "failed to read <home>/secret (NotFound)");
        assert!(!outcome.text.contains(home));
        assert!(!outcome.residual);
    }

    #[test]
    fn replace_forms_uses_path_boundaries_like_home() {
        assert_eq!(
            replace_forms(
                "<project>/target/tmp".to_string(),
                &["/tmp".to_string()],
                "<codex-home>",
            ),
            "<project>/target/tmp"
        );
        assert_eq!(
            replace_forms(
                "/tmp/AGENTS.md and <project>/target/tmp".to_string(),
                &["/tmp".to_string()],
                "<codex-home>",
            ),
            "<codex-home>/AGENTS.md and <project>/target/tmp"
        );
        assert_eq!(
            replace_forms(
                "<project>/nested/tmp/proj and /tmp/proj/AGENTS.md".to_string(),
                &["/tmp/proj".to_string()],
                "<project>",
            ),
            "<project>/nested/tmp/proj and <project>/AGENTS.md"
        );
    }

    #[test]
    fn replace_forms_skips_placeholder_tokens_and_keeps_longest_root() {
        assert_eq!(
            replace_forms(
                "<project>/x".to_string(),
                &["<project>".to_string()],
                "<codex-home>",
            ),
            "<project>/x"
        );
        assert_eq!(
            replace_forms(
                "/tmp/proj-long/AGENTS.md /tmp/AGENTS.md".to_string(),
                &["/tmp".to_string(), "/tmp/proj-long".to_string()],
                "<project>",
            ),
            "<project>/AGENTS.md <project>/AGENTS.md"
        );
    }
}
