//! Doctor suppressions: an accepted, owned, time-boxed exception to a
//! finding (PRD F-08: "支持 suppress/exception，但必须记录 owner、理由、
//! scope、期限和 evidence").
//!
//! A suppression never makes a finding disappear. The finding is still
//! reported with its real `confirmation`, marked `suppressed: true` and
//! carrying the suppression's owner / reason / expiry; the diagnosis keeps
//! the factual counts (`confirmed`, `blocking`) and adds the *active* ones
//! (`active_confirmed`, `active_blocking`, `suppressed`), which is what
//! `blocking_exit` judges. A suppression that is malformed, expired, or
//! whose `evidence_digest` no longer matches the file is invalid: it
//! suppresses nothing and is itself reported (`doctor.suppression_invalid`,
//! non-blocking, `suspected`, so it does not turn `--fail-on confirmed`
//! red by itself).
//!
//! Blocking rules require `evidence_digest` (the sha256 of the file's bytes,
//! or of a symlink's target text): the acceptance is bound to the exact
//! content that was reviewed, and one changed byte revives the finding. The
//! file lives at `.ctxpect/doctor-suppressions.json`, under a control path
//! projection cannot write, so an `apply` exception cannot mint its own
//! suppressions. Expiry is judged against the wall clock at evaluation, never
//! against the acceptance cutoff. A suppression is a project-level
//! acceptance of risk; it does not relax any policy layer above the project,
//! the projection secret gate, path containment or the passive-scan rules.

use crate::rules::is_blocking_rule;
use ctxpect_schema::{array, object, string, Value};
use std::collections::BTreeMap;

/// Schema of the suppressions document.
pub const SUPPRESSIONS_SCHEMA: &str = "ctxpect-doctor-suppressions-v1";
/// Project-relative path of the suppressions document.
pub const SUPPRESSIONS_PATH: &str = ".ctxpect/doctor-suppressions.json";
/// `rule_id` of the finding an invalid suppression produces.
pub const SUPPRESSION_INVALID_RULE: &str = "doctor.suppression_invalid";

/// What the caller knows when suppressions are applied.
pub struct SuppressionInput<'a> {
    /// The parsed document, when the file exists and parses.
    pub document: Option<&'a Value>,
    /// Why the file could not be used, when it exists but is unusable.
    pub file_error: Option<String>,
    /// sha256 of the file's bytes, when it exists.
    pub file_digest: Option<String>,
    /// Evidence digest per project-relative path: sha256 of a regular
    /// file's bytes, or of a symlink's target text.
    pub evidence: &'a BTreeMap<String, String>,
    /// Wall-clock seconds since the Unix epoch at evaluation.
    pub now_secs: i64,
}

struct Entry {
    rule_id: String,
    path: String,
    owner: String,
    reason: String,
    expires_at: String,
    evidence_digest: Option<String>,
}

fn text(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Seconds since the epoch for `<secs>.<ms>Z` (store clock) or RFC 3339
/// (`YYYY-MM-DDTHH:MM:SS[.frac](Z|±HH:MM)`), or a plain integer.
fn epoch_seconds(text: &str) -> Option<i64> {
    let text = text.trim();
    if !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()) {
        return text.parse().ok();
    }
    if let Some(rest) = text.strip_suffix('Z')
        && let Some((secs, millis)) = rest.split_once('.')
        && !millis.is_empty()
        && millis.bytes().all(|b| b.is_ascii_digit())
        && !secs.is_empty()
        && secs.bytes().all(|b| b.is_ascii_digit())
    {
        return secs.parse().ok();
    }
    let bytes = text.as_bytes();
    if bytes.len() < 20 || bytes[4] != b'-' || bytes[7] != b'-' || (bytes[10] != b'T' && bytes[10] != b't') {
        return None;
    }
    let num = |from: usize, to: usize| -> Option<i64> {
        let slice = text.get(from..to)?;
        if slice.bytes().all(|b| b.is_ascii_digit()) { slice.parse().ok() } else { None }
    };
    let (year, month, day) = (num(0, 4)?, num(5, 7)?, num(8, 10)?);
    let (hour, minute, second) = (num(11, 13)?, num(14, 16)?, num(17, 19)?);
    if bytes[13] != b':' || bytes[16] != b':' || !(1..=12).contains(&month) || !(1..=31).contains(&day)
        || hour > 23 || minute > 59 || second > 60
    {
        return None;
    }
    let mut i = 19;
    if bytes.get(i) == Some(&b'.') {
        i += 1;
        let start = i;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if i == start {
            return None;
        }
    }
    let offset = match bytes.get(i) {
        Some(b'Z' | b'z') if i + 1 == bytes.len() => 0,
        Some(b'+' | b'-') if i + 6 == bytes.len() && bytes[i + 3] == b':' => {
            let sign = if bytes[i] == b'+' { 1 } else { -1 };
            let (oh, om) = (num(i + 1, i + 3)?, num(i + 4, i + 6)?);
            if oh > 23 || om > 59 {
                return None;
            }
            sign * (oh * 3600 + om * 60)
        }
        _ => return None,
    };
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 86_400 + hour * 3600 + minute * 60 + second - offset)
}

/// Validate one entry. `Err` names the defect.
fn parse_entry(value: &Value, input: &SuppressionInput<'_>) -> Result<Entry, String> {
    let rule_id = text(value, "rule_id").ok_or("lacks rule_id")?;
    let path = text(value, "path").ok_or("lacks path")?;
    if path.starts_with('/') || path.split('/').any(|seg| seg == ".." || seg.is_empty()) {
        return Err("path must be a project-relative path without `..`".into());
    }
    let owner = text(value, "owner").ok_or("lacks owner")?;
    let reason = text(value, "reason").ok_or("lacks reason")?;
    let expires_at = text(value, "expires_at").ok_or("lacks expires_at")?;
    let expiry = epoch_seconds(&expires_at).ok_or("expires_at is neither a store clock reading nor RFC 3339")?;
    if expiry <= input.now_secs {
        return Err(format!("expired at {expires_at}"));
    }
    let evidence_digest = text(value, "evidence_digest");
    match (&evidence_digest, input.evidence.get(&path)) {
        (Some(declared), Some(actual)) if declared != actual => {
            return Err("evidence_digest does not match the current content; the accepted bytes changed".into());
        }
        (Some(_), None) => {
            return Err("evidence_digest names a path that is not in the project scan".into());
        }
        (None, _) if is_blocking_rule(&rule_id) => {
            return Err("a blocking rule needs evidence_digest (sha256 of the reviewed bytes)".into());
        }
        _ => {}
    }
    Ok(Entry {
        rule_id,
        path,
        owner,
        reason,
        expires_at,
        evidence_digest,
    })
}

fn invalid_finding(index: usize, why: &str) -> Value {
    let id = format!(
        "f_{}",
        &ctxpect_schema::sha256_text(&format!("{SUPPRESSION_INVALID_RULE}|{index}|{why}"))[..12]
    );
    let title = format!("suppression #{index} is invalid and suppresses nothing: {why}");
    object([
        ("finding_id", string(id)),
        ("rule_id", string(SUPPRESSION_INVALID_RULE)),
        ("rule_namespace", string("doctor-suppressions")),
        ("title", string(title)),
        ("path", string(SUPPRESSIONS_PATH)),
        ("blocking", Value::Bool(false)),
        // A configuration defect, not a deterministic content finding.
        ("confirmation", string("suspected")),
        ("severity", string("suspected")),
        ("unknown_is_severity", Value::Bool(false)),
        ("affected_surfaces", array([string(SUPPRESSIONS_PATH)])),
        ("evidence_state", string("static-resolution")),
        ("impact", string("advisory")),
        ("first_seen", string("current-receipt")),
        ("reason_code", string(SUPPRESSION_INVALID_RULE)),
        ("facet", string("project-content")),
        (
            "treatment",
            object([
                ("locked", Value::Bool(false)),
                ("lock_reason", string("none")),
                ("unlocks_via_export", Value::Bool(false)),
                ("unlocks_via_advisor", Value::Bool(false)),
                ("unlocks_via_user_attestation_alone", Value::Bool(false)),
            ]),
        ),
        (
            "next_evidence",
            array([object([
                ("action", string("Fix or remove the entry in .ctxpect/doctor-suppressions.json.")),
                ("kind", string("evidence")),
            ])]),
        ),
        (
            "placement",
            object([
                ("authority", string("unknown")),
                ("target", string(SUPPRESSIONS_PATH)),
                ("loss", string("unknown")),
            ]),
        ),
        ("suppressed", Value::Bool(false)),
    ])
}

/// Apply the suppressions document to a diagnosis (see the module docs).
#[must_use]
pub fn apply_suppressions(diagnosis: Value, input: &SuppressionInput<'_>) -> Value {
    let Value::Object(mut map) = diagnosis else {
        return diagnosis;
    };
    let mut findings = map
        .get("findings")
        .and_then(Value::as_array)
        .map(<[Value]>::to_vec)
        .unwrap_or_default();

    let mut entries: Vec<Value> = Vec::new();
    let mut valid: Vec<Entry> = Vec::new();
    let mut invalid = 0i64;
    let record_invalid = |index: usize, why: String, entries: &mut Vec<Value>, findings: &mut Vec<Value>| {
        entries.push(object([
            ("index", Value::Int(i64::try_from(index).unwrap_or(0))),
            ("status", string("invalid")),
            ("why", string(&why)),
        ]));
        findings.push(invalid_finding(index, &why));
    };
    match (input.document, &input.file_error) {
        (Some(doc), _) => {
            if doc.get("schema").and_then(Value::as_str) != Some(SUPPRESSIONS_SCHEMA) {
                invalid += 1;
                record_invalid(0, format!("document schema is not {SUPPRESSIONS_SCHEMA}"), &mut entries, &mut findings);
            } else {
                let items: Vec<Value> = doc.get("suppressions").and_then(Value::as_array).map(<[Value]>::to_vec).unwrap_or_default();
                for (index, item) in items.iter().enumerate() {
                    match parse_entry(item, input) {
                        Ok(entry) => valid.push(entry),
                        Err(why) => {
                            invalid += 1;
                            record_invalid(index, why, &mut entries, &mut findings);
                        }
                    }
                }
            }
        }
        (None, Some(why)) => {
            invalid += 1;
            record_invalid(0, why.clone(), &mut entries, &mut findings);
        }
        (None, None) => {}
    }

    let mut applied = 0i64;
    let mut matched: Vec<bool> = vec![false; valid.len()];
    for finding in &mut findings {
        let rule = finding.get("rule_id").and_then(Value::as_str).unwrap_or("").to_string();
        let paths: Vec<String> = finding
            .get("path")
            .and_then(Value::as_str)
            .map(str::to_string)
            .into_iter()
            .chain(
                finding
                    .get("affected_surfaces")
                    .and_then(Value::as_array)
                    .map(|items| items.iter().filter_map(Value::as_str).map(str::to_string).collect::<Vec<_>>())
                    .unwrap_or_default(),
            )
            .collect();
        let hit = valid
            .iter()
            .enumerate()
            .find(|(_, entry)| entry.rule_id == rule && paths.contains(&entry.path));
        let Value::Object(item) = finding else { continue };
        match hit {
            Some((index, entry)) => {
                matched[index] = true;
                applied += 1;
                item.insert("suppressed".into(), Value::Bool(true));
                item.insert(
                    "suppression".into(),
                    object([
                        ("owner", string(&entry.owner)),
                        ("reason", string(&entry.reason)),
                        ("expires_at", string(&entry.expires_at)),
                        (
                            "evidence_digest",
                            entry.evidence_digest.as_deref().map_or(Value::Null, string),
                        ),
                    ]),
                );
            }
            None => {
                item.entry("suppressed".to_string()).or_insert(Value::Bool(false));
            }
        }
    }
    for (index, entry) in valid.iter().enumerate() {
        entries.push(object([
            ("rule_id", string(&entry.rule_id)),
            ("path", string(&entry.path)),
            ("owner", string(&entry.owner)),
            ("expires_at", string(&entry.expires_at)),
            ("status", string(if matched[index] { "applied" } else { "unmatched" })),
        ]));
    }
    let unmatched = matched.iter().filter(|m| !**m).count() as i64;

    // Factual counts stay; active counts are what the exit judges.
    let mut confirmed = 0i64;
    let mut suspected = 0i64;
    let mut blocking = 0i64;
    let mut active_confirmed = 0i64;
    let mut active_blocking = 0i64;
    let mut suppressed = 0i64;
    for item in &findings {
        let is_suppressed = item.get("suppressed").and_then(Value::as_bool) == Some(true);
        let is_confirmed = item.get("confirmation").and_then(Value::as_str) == Some("confirmed");
        let is_blocking = item.get("blocking").and_then(Value::as_bool) == Some(true);
        if is_confirmed {
            confirmed += 1;
            if !is_suppressed {
                active_confirmed += 1;
            }
        }
        if item.get("confirmation").and_then(Value::as_str) == Some("suspected") {
            suspected += 1;
        }
        if is_blocking {
            blocking += 1;
            if !is_suppressed {
                active_blocking += 1;
            }
        }
        if is_suppressed {
            suppressed += 1;
        }
    }
    if let Some(Value::Object(counts)) = map.get_mut("counts") {
        counts.insert("confirmed".into(), Value::Int(confirmed));
        counts.insert("suspected".into(), Value::Int(suspected));
        counts.insert("blocking".into(), Value::Int(blocking));
        counts.insert("active_confirmed".into(), Value::Int(active_confirmed));
        counts.insert("active_blocking".into(), Value::Int(active_blocking));
        counts.insert("suppressed".into(), Value::Int(suppressed));
    }
    map.insert("findings".into(), array(findings));
    map.insert(
        "suppressions".into(),
        object([
            ("schema", string(SUPPRESSIONS_SCHEMA)),
            ("file", string(SUPPRESSIONS_PATH)),
            ("present", Value::Bool(input.document.is_some() || input.file_error.is_some())),
            ("file_digest", input.file_digest.as_deref().map_or(Value::Null, string)),
            ("evaluated_at_secs", Value::Int(input.now_secs)),
            ("applied", Value::Int(applied)),
            ("unmatched", Value::Int(unmatched)),
            ("invalid", Value::Int(invalid)),
            ("entries", array(entries)),
            (
                "scope",
                string("project-level acceptance of risk; findings stay reported and counted, only the active counts and the exit change; policy layers above the project, the projection secret gate and path containment are not relaxed"),
            ),
        ]),
    );
    Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctxpect_schema::parse;

    fn diagnosis() -> Value {
        parse(
            r#"{"findings":[
                {"finding_id":"f1","rule_id":"secret_literal","path":"NOTES.md","affected_surfaces":["NOTES.md"],"confirmation":"confirmed","blocking":true},
                {"finding_id":"f2","rule_id":"stale","path":"OLD.md","affected_surfaces":["OLD.md"],"confirmation":"confirmed","blocking":false}
              ],"counts":{"confirmed":2,"suspected":0,"blocking":1}}"#,
        )
        .unwrap()
    }

    fn evidence() -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        map.insert("NOTES.md".to_string(), "aa".repeat(32));
        map.insert("OLD.md".to_string(), "bb".repeat(32));
        map
    }

    fn doc(entries: &str) -> Value {
        parse(&format!(r#"{{"schema":"{SUPPRESSIONS_SCHEMA}","suppressions":[{entries}]}}"#)).unwrap()
    }

    fn run(doc: Option<&Value>, now: i64) -> Value {
        let evidence = evidence();
        apply_suppressions(
            diagnosis(),
            &SuppressionInput { document: doc, file_error: None, file_digest: Some("ff".repeat(32)), evidence: &evidence, now_secs: now },
        )
    }

    #[test]
    fn a_valid_suppression_keeps_the_finding_and_only_changes_the_active_counts() {
        let entry = format!(
            r#"{{"rule_id":"secret_literal","path":"NOTES.md","owner":"alice","reason":"fixture token","expires_at":"2030-01-01T00:00:00Z","evidence_digest":"{}"}}"#,
            "aa".repeat(32)
        );
        let out = run(Some(&doc(&entry)), 1_800_000_000);
        let findings = out.get("findings").and_then(Value::as_array).unwrap();
        assert_eq!(findings.len(), 2, "nothing disappears");
        assert_eq!(findings[0].get("suppressed"), Some(&Value::Bool(true)));
        assert_eq!(findings[0].get("confirmation").and_then(Value::as_str), Some("confirmed"), "the fact stays");
        assert_eq!(findings[0].pointer(&["suppression", "owner"]).and_then(Value::as_str), Some("alice"));
        assert_eq!(out.pointer(&["counts", "confirmed"]).and_then(Value::as_i64), Some(2));
        assert_eq!(out.pointer(&["counts", "blocking"]).and_then(Value::as_i64), Some(1));
        assert_eq!(out.pointer(&["counts", "active_blocking"]).and_then(Value::as_i64), Some(0));
        assert_eq!(out.pointer(&["counts", "active_confirmed"]).and_then(Value::as_i64), Some(1));
        assert_eq!(out.pointer(&["counts", "suppressed"]).and_then(Value::as_i64), Some(1));
        assert_eq!(crate::blocking_exit(&out, None), 0);
        assert_eq!(crate::blocking_exit(&out, Some("confirmed")), 2, "OLD.md is still an active confirmed finding");
        assert_eq!(out.pointer(&["suppressions", "applied"]).and_then(Value::as_i64), Some(1));
    }

    #[test]
    fn every_defect_revives_the_finding_and_is_itself_reported() {
        let cases = [
            // one changed byte
            (format!(r#"{{"rule_id":"secret_literal","path":"NOTES.md","owner":"a","reason":"r","expires_at":"2030-01-01T00:00:00Z","evidence_digest":"{}"}}"#, "ab".repeat(32)), "changed"),
            // expired
            (format!(r#"{{"rule_id":"secret_literal","path":"NOTES.md","owner":"a","reason":"r","expires_at":"2020-01-01T00:00:00Z","evidence_digest":"{}"}}"#, "aa".repeat(32)), "expired"),
            // blocking rule without evidence
            (r#"{"rule_id":"secret_literal","path":"NOTES.md","owner":"a","reason":"r","expires_at":"2030-01-01T00:00:00Z"}"#.to_string(), "evidence_digest"),
            // missing owner
            (format!(r#"{{"rule_id":"secret_literal","path":"NOTES.md","reason":"r","expires_at":"2030-01-01T00:00:00Z","evidence_digest":"{}"}}"#, "aa".repeat(32)), "owner"),
            // path escaping the project
            (format!(r#"{{"rule_id":"secret_literal","path":"../NOTES.md","owner":"a","reason":"r","expires_at":"2030-01-01T00:00:00Z","evidence_digest":"{}"}}"#, "aa".repeat(32)), "relative"),
        ];
        for (entry, why) in cases {
            let out = run(Some(&doc(&entry)), 1_800_000_000);
            let findings = out.get("findings").and_then(Value::as_array).unwrap();
            assert_eq!(findings[0].get("suppressed"), Some(&Value::Bool(false)), "{why}");
            assert_eq!(out.pointer(&["counts", "active_blocking"]).and_then(Value::as_i64), Some(1), "{why}");
            assert_eq!(crate::blocking_exit(&out, None), 2, "{why}");
            let invalid = findings.iter().find(|f| f.get("rule_id").and_then(Value::as_str) == Some(SUPPRESSION_INVALID_RULE)).unwrap_or_else(|| panic!("{why}"));
            assert!(invalid.get("title").and_then(Value::as_str).unwrap().contains(why), "{invalid:?}");
            assert_eq!(invalid.get("blocking"), Some(&Value::Bool(false)));
            assert_eq!(invalid.get("confirmation").and_then(Value::as_str), Some("suspected"));
            assert_eq!(out.pointer(&["suppressions", "invalid"]).and_then(Value::as_i64), Some(1));
        }
        // No document at all: nothing changes but the active counts exist.
        let out = run(None, 1_800_000_000);
        assert_eq!(out.pointer(&["counts", "active_blocking"]).and_then(Value::as_i64), Some(1));
        assert_eq!(out.pointer(&["suppressions", "present"]), Some(&Value::Bool(false)));
        // A non-blocking rule may be suppressed without evidence; an unmatched
        // entry is reported as such.
        let entry = r#"{"rule_id":"stale","path":"OLD.md","owner":"a","reason":"r","expires_at":"1900000000"},{"rule_id":"stale","path":"NONE.md","owner":"a","reason":"r","expires_at":"1900000000"}"#;
        let out = run(Some(&doc(entry)), 1_800_000_000);
        assert_eq!(out.pointer(&["counts", "active_confirmed"]).and_then(Value::as_i64), Some(1));
        assert_eq!(out.pointer(&["suppressions", "unmatched"]).and_then(Value::as_i64), Some(1));
        assert_eq!(epoch_seconds("2026-09-09T08:00:00+09:00"), Some(1_788_912_000 - 3600));
    }
}
