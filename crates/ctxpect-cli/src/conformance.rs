//! Static-corpus conformance runner, shared by the `corpus-conformance`
//! required gate and `ctxpect adapter test --adapter <family>`.
//!
//! Every row of `acceptance/corpus/development/static/*.jsonl` is run through
//! the product's in-process `inspect` at the row's coordinate and lands in
//! exactly one of these buckets per coordinate × capability:
//!
//! - **implemented-pass**: the product implements this family × capability at
//!   this coordinate and its `expected_output` / `expected_claim` match a
//!   golden row whose claim is present / absent / not-applicable.
//! - **unknown-honesty-pass**: implemented, matches, but the golden claim is
//!   `indeterminate` (an honesty cell). Kept apart so honesty never inflates
//!   the implemented count.
//! - **fail**: implemented, but the answer differs.
//! - **unimplemented**: the product does not implement this family ×
//!   capability. Its honest Unknown answer is checked but never counted as a
//!   pass; a present/absent claim from an unimplemented path is a fail.
//!
//! The denominator is every row. `not_executed` reports what this runner
//! does **not** execute: sealed fixtures (answers withheld), live recipes and
//! development oracle fixtures (no harness is run here).
//!
//! The support level comes from `acceptance/compatibility-matrix.yaml`, the
//! implemented set from `ctxpect_resolve::implemented_capabilities`. The
//! Python golden parser is not consulted: it is the thing being checked
//! against, not the checker.

use crate::args::InspectArgs;
use crate::inspect::inspect;
use ctxpect_resolve::{implemented_capabilities, is_supported_coordinate};
use ctxpect_schema::{array, object, parse, sha256_hex, string, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const AXES: &[&str] = &[
    "claim_kind",
    "coverage",
    "knowledge_status",
    "lifecycle_stage",
    "precision",
    "provenance",
    "truth_state",
];

const OUTPUT_KEYS: &[&str] = &[
    "included",
    "native_paths_used",
    "truth_state",
    "unknown_reason_code",
    "loss_report_required",
];

/// Counts for one coordinate × capability.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub total: usize,
    pub implemented_pass: usize,
    pub unknown_honesty_pass: usize,
    pub fail: usize,
    pub unimplemented: usize,
}

impl Cell {
    /// Every matching row, implemented or honesty.
    #[must_use]
    pub fn pass(&self) -> usize {
        self.implemented_pass + self.unknown_honesty_pass
    }

    fn add(&mut self, outcome: &Outcome) {
        self.total += 1;
        match outcome {
            Outcome::ImplementedPass => self.implemented_pass += 1,
            Outcome::UnknownHonestyPass => self.unknown_honesty_pass += 1,
            Outcome::Fail(_) => self.fail += 1,
            Outcome::Unimplemented => self.unimplemented += 1,
        }
    }

    fn to_value(&self) -> Value {
        object([
            ("total", count(self.total)),
            ("implemented_pass", count(self.implemented_pass)),
            ("unknown_honesty_pass", count(self.unknown_honesty_pass)),
            ("fail", count(self.fail)),
            ("unimplemented", count(self.unimplemented)),
        ])
    }
}

fn count(n: usize) -> Value {
    Value::Int(i64::try_from(n).unwrap_or(0))
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    ImplementedPass,
    UnknownHonestyPass,
    Fail(String),
    Unimplemented,
}

/// Fixtures the static runner does not execute, by corpus.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct NotExecuted {
    pub sealed: usize,
    pub live: usize,
    pub oracle: usize,
    /// Development Doctor rows: executed by the `doctor-corpus` gate, not here.
    pub doctor: usize,
}

impl NotExecuted {
    fn to_value(&self) -> Value {
        object([
            ("sealed", count(self.sealed)),
            ("live", count(self.live)),
            ("oracle", count(self.oracle)),
            ("doctor", count(self.doctor)),
        ])
    }
}

/// The report of one run.
#[derive(Debug)]
pub struct Report {
    pub cells: BTreeMap<(String, String), Cell>,
    pub failures: Vec<String>,
    pub total: usize,
    /// Files that were run, with their sha256.
    pub files: Vec<(String, String)>,
    /// sha256 over the run files' digests (sorted by path), so one value
    /// binds the whole corpus that was executed.
    pub corpus_digest: String,
    /// sha256 of `acceptance/compatibility-matrix.yaml`.
    pub matrix_digest: String,
    pub not_executed: NotExecuted,
}

impl Report {
    #[must_use]
    pub fn totals(&self) -> Cell {
        let mut out = Cell::default();
        for cell in self.cells.values() {
            out.total += cell.total;
            out.implemented_pass += cell.implemented_pass;
            out.unknown_honesty_pass += cell.unknown_honesty_pass;
            out.fail += cell.fail;
            out.unimplemented += cell.unimplemented;
        }
        out
    }

    /// The report as a JSON document.
    #[must_use]
    pub fn to_value(&self) -> Value {
        let cells: Vec<Value> = self
            .cells
            .iter()
            .map(|((coordinate, capability), cell)| {
                match cell.to_value() {
                    Value::Object(mut map) => {
                        map.insert("coordinate_id".into(), string(coordinate));
                        map.insert("capability_id".into(), string(capability));
                        Value::Object(map)
                    }
                    other => other,
                }
            })
            .collect();
        object([
            ("schema", string("ctxpect-conformance-report-v1")),
            ("total", count(self.total)),
            ("counts", self.totals().to_value()),
            ("cells", array(cells)),
            ("failures", array(self.failures.iter().map(string))),
            (
                "files",
                array(self.files.iter().map(|(path, digest)| {
                    object([("path", string(path)), ("sha256", string(digest))])
                })),
            ),
            ("corpus_digest", string(&self.corpus_digest)),
            ("matrix_digest", string(&self.matrix_digest)),
            ("not_executed", self.not_executed.to_value()),
            ("resolver_version", string(ctxpect_resolve::VERSION)),
            ("live_oracle_executed", Value::Bool(false)),
        ])
    }

    /// The text table the gate prints: one line per coordinate × capability,
    /// a totals line and the not-executed line.
    #[must_use]
    pub fn render_table(&self) -> String {
        let mut out = String::new();
        out.push_str("coordinate | capability | total | implemented-pass | unknown-honesty-pass | fail | unimplemented\n");
        for ((coordinate, capability), cell) in &self.cells {
            out.push_str(&format!(
                "{coordinate} | {capability} | {} | {} | {} | {} | {}\n",
                cell.total, cell.implemented_pass, cell.unknown_honesty_pass, cell.fail, cell.unimplemented
            ));
        }
        let totals = self.totals();
        out.push_str(&format!(
            "TOTAL | - | {} | {} | {} | {} | {}\n",
            self.total, totals.implemented_pass, totals.unknown_honesty_pass, totals.fail, totals.unimplemented
        ));
        out.push_str(&format!(
            "NOT_EXECUTED | sealed={} live={} oracle={} doctor={} (doctor rows run under the doctor-corpus gate)\n",
            self.not_executed.sealed, self.not_executed.live, self.not_executed.oracle, self.not_executed.doctor
        ));
        for failure in &self.failures {
            out.push_str(&format!("FAIL {failure}\n"));
        }
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceError {
    pub code: &'static str,
    pub message: String,
}

fn err(code: &'static str, message: impl Into<String>) -> ConformanceError {
    ConformanceError {
        code,
        message: message.into(),
    }
}

fn str_field<'a>(row: &'a Value, key: &str) -> &'a str {
    row.get(key).and_then(Value::as_str).unwrap_or_default()
}

fn load_jsonl(path: &Path) -> Result<Vec<Value>, ConformanceError> {
    let text = fs::read_to_string(path)
        .map_err(|error| err("adapter.corpus_missing", format!("read {}: {error}", path.display())))?;
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| parse(line).map_err(|error| err("adapter.corpus_malformed", format!("parse: {error}"))))
        .collect()
}

/// Compare the product's first result against the golden row.
fn compare(result: &Value, row: &Value) -> Result<(), String> {
    let expected_output = row.get("expected_output").ok_or("row lacks expected_output")?;
    for key in OUTPUT_KEYS {
        if result.get(key) != expected_output.get(key) {
            return Err(format!(
                "expected_output.{key}: golden {:?} != product {:?}",
                expected_output.get(key),
                result.get(key)
            ));
        }
    }
    // `parse` is compared on the keys the golden declares; the product may
    // carry more detail than the golden, never less.
    if let Some(expected_parse) = expected_output.get("parse").and_then(Value::as_object) {
        let parsed = result.get("parse").and_then(Value::as_object).ok_or("product lacks parse")?;
        for (key, value) in expected_parse {
            if parsed.get(key) != Some(value) {
                return Err(format!(
                    "expected_output.parse.{key}: golden {value:?} != product {:?}",
                    parsed.get(key)
                ));
            }
        }
    }
    let expected_claim = row.get("expected_claim").ok_or("row lacks expected_claim")?;
    let claim = result.get("claim").ok_or("product lacks claim")?;
    for axis in AXES {
        if claim.get(axis) != expected_claim.get(axis) {
            return Err(format!(
                "claim.{axis}: golden {:?} != product {:?}",
                expected_claim.get(axis),
                claim.get(axis)
            ));
        }
    }
    if expected_claim.get("unknown_reason_code").is_some()
        && claim.get("unknown_reason_code") != expected_claim.get("unknown_reason_code")
    {
        return Err(format!(
            "claim.unknown_reason_code: golden {:?} != product {:?}",
            expected_claim.get("unknown_reason_code"),
            claim.get("unknown_reason_code")
        ));
    }
    Ok(())
}

/// Run one golden row through the product.
#[must_use]
pub fn run_row(root: &Path, row: &Value, static_support: Option<&str>) -> Outcome {
    let family = str_field(row, "family_id");
    let capability = str_field(row, "capability_id");
    let version = str_field(row, "version");
    let surface = str_field(row, "surface");
    let os_lane = str_field(row, "os_lane");
    let input = root.join(str_field(row, "input_path"));

    let implemented_capability = implemented_capabilities(family).contains(&capability);
    // A family × capability the product resolves is exercised at its anchor
    // and at its honesty coordinates alike: the coordinate reasoning is part
    // of the implementation. Anything else is unimplemented.
    let implemented = implemented_capability
        && (is_supported_coordinate(family, version, surface, os_lane)
            || static_support == Some("required-unknown-honesty"));

    let report = match inspect(InspectArgs {
        json: true,
        offline: true,
        project: input,
        cwd: None,
        harness: family.to_string(),
        surface: surface.to_string(),
        version: version.to_string(),
        version_explicit: true,
        codex_home: None,
        require: vec![capability.to_string()],
        os_lane: os_lane.to_string(),
        store: None,
    }) {
        Ok(report) => report,
        Err(error) => {
            return Outcome::Fail(format!("inspect failed: {} {}", error.code(), error.message()));
        }
    };
    let result = report
        .envelope
        .get("results")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .cloned()
        .unwrap_or(Value::Null);

    if !implemented {
        // An unimplemented path must not answer present/absent. That would be
        // a fabricated claim, whichever way the golden happens to point.
        return match result.get("truth_state").and_then(Value::as_str) {
            Some("indeterminate") => Outcome::Unimplemented,
            other => Outcome::Fail(format!(
                "unimplemented {family}/{capability} answered {other:?} instead of indeterminate"
            )),
        };
    }
    match compare(&result, row) {
        Ok(()) => {
            let golden_truth = row
                .pointer(&["expected_claim", "truth_state"])
                .and_then(Value::as_str);
            if golden_truth == Some("indeterminate") {
                Outcome::UnknownHonestyPass
            } else {
                Outcome::ImplementedPass
            }
        }
        Err(reason) => Outcome::Fail(reason),
    }
}

/// `coordinate id → static_support` from the compatibility matrix, plus the
/// matrix file's digest.
fn matrix_support(root: &Path) -> Result<(BTreeMap<String, String>, String), ConformanceError> {
    let path = root.join("acceptance/compatibility-matrix.yaml");
    let text = fs::read_to_string(&path)
        .map_err(|error| err("adapter.corpus_missing", format!("read {}: {error}", path.display())))?;
    let matrix = parse(&text).map_err(|error| err("adapter.corpus_malformed", format!("matrix: {error}")))?;
    let support = matrix
        .get("coordinates")
        .and_then(Value::as_array)
        .ok_or_else(|| err("adapter.corpus_malformed", "matrix lacks coordinates"))?
        .iter()
        .map(|coord| {
            (
                str_field(coord, "id").to_string(),
                str_field(coord, "static_support").to_string(),
            )
        })
        .collect();
    Ok((support, sha256_hex(text.as_bytes())))
}

/// What the manifest lists that this runner does not execute.
fn not_executed(root: &Path) -> NotExecuted {
    let mut out = NotExecuted::default();
    let Ok(text) = fs::read_to_string(root.join("acceptance/corpus-manifest.json")) else {
        return out;
    };
    let Ok(manifest) = parse(&text) else {
        return out;
    };
    for item in manifest.get("fixtures").and_then(Value::as_array).unwrap_or(&[]) {
        match (str_field(item, "corpus"), str_field(item, "kind")) {
            ("sealed", _) => out.sealed += 1,
            ("live", _) => out.live += 1,
            ("development", "oracle") => out.oracle += 1,
            // Executed by the `doctor-corpus` gate, not by this runner.
            ("development", "doctor") => out.doctor += 1,
            _ => {}
        }
    }
    out
}

/// Run the development static corpus under `root` (the repository root that
/// holds `acceptance/`). With `family`, only that family's files run.
pub fn run_static_corpus(root: &Path, family: Option<&str>) -> Result<Report, ConformanceError> {
    let (support, matrix_digest) = matrix_support(root)?;
    let dir = root.join("acceptance/corpus/development/static");
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .map_err(|error| err("adapter.corpus_missing", format!("read {}: {error}", dir.display())))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("jsonl"))
        .filter(|path| match family {
            Some(family) => path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|name| name.starts_with(&format!("{family}__"))),
            None => true,
        })
        .collect();
    files.sort();

    let mut cells: BTreeMap<(String, String), Cell> = BTreeMap::new();
    let mut failures = Vec::new();
    let mut total = 0usize;
    let mut digests = Vec::new();
    for file in &files {
        let bytes = fs::read(file)
            .map_err(|error| err("adapter.corpus_missing", format!("read {}: {error}", file.display())))?;
        let rel = file
            .strip_prefix(root)
            .unwrap_or(file)
            .to_string_lossy()
            .replace('\\', "/");
        digests.push((rel, sha256_hex(&bytes)));
        for row in load_jsonl(file)? {
            total += 1;
            let coordinate = str_field(&row, "coordinate_id").to_string();
            let capability = str_field(&row, "capability_id").to_string();
            let static_support = support.get(&coordinate).map(String::as_str);
            let outcome = run_row(root, &row, static_support);
            if let Outcome::Fail(reason) = &outcome {
                failures.push(format!("{}: {reason}", str_field(&row, "id")));
            }
            cells.entry((coordinate, capability)).or_default().add(&outcome);
        }
    }
    let corpus_digest = sha256_hex(
        digests
            .iter()
            .map(|(path, digest)| format!("{digest}  {path}\n"))
            .collect::<String>()
            .as_bytes(),
    );
    Ok(Report {
        cells,
        failures,
        total,
        files: digests,
        corpus_digest,
        matrix_digest,
        not_executed: not_executed(root),
    })
}
