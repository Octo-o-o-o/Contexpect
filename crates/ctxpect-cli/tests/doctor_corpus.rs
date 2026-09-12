//! Doctor-corpus runner (required gate `doctor-corpus`).
//!
//! Every row of `acceptance/corpus/development/doctor/doctor-corpus.jsonl` is
//! scanned with the product's own collector-backed scan and evaluated by
//! `ctxpect_doctor::project_findings`. Findings are compared to the golden
//! `expected_findings` as `(rule_id, path)` pairs, and per-rule precision and
//! recall are printed.
//!
//! Two invariants are enforced, not just printed:
//!
//! - every rule in `BLOCKING_RULES` has precision 1.00 on this corpus (PRD
//!   §17.1: a blocking rule has zero false positives), so `ci` cannot exit 2
//!   on a look-alike;
//! - the exit `ctxpect ci` / `ctxpect doctor` would take from the findings
//!   equals the row's `expected_exit`, computed by the one shared judgement
//!   `ctxpect_doctor::blocking_exit`.
//!
//! The Python golden parser is not consulted here: it produced the golden and
//! is not a substitute for the product under test.

use ctxpect_cli::{parse, scan_project_for_doctor, Value};
use ctxpect_doctor::{
    blocking_exit, project_findings, render_project_findings, with_project_findings,
    BLOCKING_RULES, NON_BLOCKING_RULES, STALE_CUTOFF,
};
use ctxpect_fs::Root;
use ctxpect_schema::object;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

#[derive(Default, Debug)]
struct RuleStats {
    tp: usize,
    fp: usize,
    fn_: usize,
}

impl RuleStats {
    fn precision(&self) -> Option<f64> {
        let denom = self.tp + self.fp;
        (denom > 0).then(|| self.tp as f64 / denom as f64)
    }
    fn recall(&self) -> Option<f64> {
        let denom = self.tp + self.fn_;
        (denom > 0).then(|| self.tp as f64 / denom as f64)
    }
}

#[test]
fn every_doctor_corpus_row_is_scored_and_blocking_rules_have_no_false_positive() {
    let root = repo_root();
    let text = fs::read_to_string(root.join("acceptance/corpus/development/doctor/doctor-corpus.jsonl"))
        .expect("doctor corpus");
    let rows: Vec<Value> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| parse(line).expect("row"))
        .collect();
    assert_eq!(rows.len(), 707, "the Doctor corpus has 707 rows");

    let mut stats: BTreeMap<String, RuleStats> = BTreeMap::new();
    for rule in BLOCKING_RULES.iter().chain(NON_BLOCKING_RULES.iter()) {
        stats.entry((*rule).to_string()).or_default();
    }
    let mut mismatches: Vec<String> = Vec::new();
    let mut exit_mismatches: Vec<String> = Vec::new();

    for row in &rows {
        let id = row.get("id").and_then(Value::as_str).unwrap_or("?");
        let input = root.join(row.get("input_path").and_then(Value::as_str).expect("input_path"));
        let project = Root::new(&input).expect("input dir");
        let files = scan_project_for_doctor(&project).unwrap_or_else(|err| panic!("scan {id}: {} {}", err.code(), err.message()));
        // The corpus runs against the frozen reference date (the acceptance
        // cutoff), not the wall clock, so the golden rows stay reproducible.
        let found = project_findings(&files, STALE_CUTOFF);
        let got: BTreeSet<(String, String)> = found
            .iter()
            .map(|f| (f.rule_id.to_string(), f.path.clone()))
            .collect();
        let want: BTreeSet<(String, String)> = row
            .get("expected_findings")
            .and_then(Value::as_array)
            .unwrap_or(&[])
            .iter()
            .map(|f| {
                (
                    f.get("rule_id").and_then(Value::as_str).unwrap_or("").to_string(),
                    f.get("path").and_then(Value::as_str).unwrap_or("").to_string(),
                )
            })
            .collect();
        for (rule, path) in got.intersection(&want) {
            stats.entry(rule.clone()).or_default().tp += 1;
            let _ = path;
        }
        for (rule, path) in got.difference(&want) {
            stats.entry(rule.clone()).or_default().fp += 1;
            mismatches.push(format!("{id}: false positive {rule} @ {path}"));
        }
        for (rule, path) in want.difference(&got) {
            stats.entry(rule.clone()).or_default().fn_ += 1;
            mismatches.push(format!("{id}: missed {rule} @ {path}"));
        }

        // The exit the product would take, through the shared judgement.
        let diagnosis = with_project_findings(
            object([("findings", ctxpect_schema::array([])), ("counts", object([("confirmed", Value::Int(0))]))]),
            &render_project_findings(&found),
        );
        let exit = blocking_exit(&diagnosis, None);
        let expected_exit = i32::try_from(row.get("expected_exit").and_then(Value::as_i64).unwrap_or(0)).unwrap_or(0);
        if exit != expected_exit {
            exit_mismatches.push(format!("{id}: exit {exit} != expected {expected_exit}"));
        }
    }

    println!("rule | blocking | tp | fp | fn | precision | recall");
    let fmt = |v: Option<f64>| v.map_or("n/a".to_string(), |x| format!("{x:.2}"));
    for (rule, s) in &stats {
        println!(
            "{rule} | {} | {} | {} | {} | {} | {}",
            BLOCKING_RULES.contains(&rule.as_str()),
            s.tp,
            s.fp,
            s.fn_,
            fmt(s.precision()),
            fmt(s.recall())
        );
    }
    for line in &mismatches {
        println!("MISMATCH {line}");
    }
    for line in &exit_mismatches {
        println!("EXIT {line}");
    }

    for rule in BLOCKING_RULES {
        let s = stats.get(*rule).expect("stats");
        assert_eq!(
            s.precision(),
            Some(1.0),
            "blocking rule {rule} has a false positive on the corpus; it must be demoted to NON_BLOCKING_RULES ({s:?})"
        );
        assert!(s.tp > 0, "blocking rule {rule} never fired on the corpus; it is not implemented");
    }
    for rule in NON_BLOCKING_RULES {
        let s = stats.get(*rule).expect("stats");
        assert!(s.tp > 0, "non-blocking rule {rule} never fired on the corpus; it is not implemented");
    }
    assert!(mismatches.is_empty(), "{} finding mismatches:\n{}", mismatches.len(), mismatches.join("\n"));
    assert!(exit_mismatches.is_empty(), "{} exit mismatches:\n{}", exit_mismatches.len(), exit_mismatches.join("\n"));
}
