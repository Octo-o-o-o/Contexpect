//! The two frozen product schemas (`docs/schemas/ctxpect-receipt-v1.schema.json`,
//! `docs/schemas/ctxpect-doctor-v1.schema.json`) validate what the product
//! actually emits, and reject a document that drifts from them.

use ctxpect_cli::{inspect, parse, persist_inspect, project_doctor_findings, InspectArgs, Store, Value};
use ctxpect_doctor::{diagnose, with_project_findings};
use ctxpect_fs::Root;
use ctxpect_schema::validate;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn schema(name: &str) -> Value {
    let path = repo_root().join("docs/schemas").join(name);
    parse(&fs::read_to_string(&path).expect("schema file")).expect("schema JSON")
}

struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new(label: &str) -> Scratch {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cx-schema-{label}-{}-{}", std::process::id(), nanos % 1_000_000));
        fs::create_dir_all(&path).expect("mkdir");
        Scratch {
            path: fs::canonicalize(&path).expect("canon"),
        }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn inspect_snapshot(project: &Path) -> Value {
    inspect(InspectArgs {
        json: true,
        offline: true,
        project: project.to_path_buf(),
        cwd: None,
        harness: "codex".into(),
        surface: "cli".into(),
        version: "0.147.0".into(),
        version_explicit: false,
        codex_home: None,
        require: vec!["instructions".into()],
        os_lane: "macos-27-arm64".into(),
        store: None,
    })
    .unwrap_or_else(|err| panic!("inspect: {} {}", err.code(), err.message()))
    .envelope
}

fn without(value: &Value, key: &str) -> Value {
    match value {
        Value::Object(map) => {
            let mut copy = map.clone();
            copy.remove(key);
            Value::Object(copy)
        }
        other => other.clone(),
    }
}

#[test]
fn a_real_receipt_and_its_tombstone_validate_and_a_drifted_one_does_not() {
    let scratch = Scratch::new("receipt");
    fs::write(scratch.path.join("AGENTS.md"), "hello\n").unwrap();
    fs::write(scratch.path.join("token.md"), "sk-abcdefghijklmnopqrstuvwxyz0123\n").unwrap();
    let store = Store::open(&scratch.path.join("store")).expect("store");
    let receipt = persist_inspect(&store, &inspect_snapshot(&scratch.path), "one-shot")
        .unwrap_or_else(|err| panic!("persist: {} {}", err.code(), err.message()));
    let schema = schema("ctxpect-receipt-v1.schema.json");
    validate(&schema, &receipt).unwrap_or_else(|errors| panic!("receipt: {errors:#?}"));

    // C-F04: the static resolver's claims name their producer and domain, and
    // the frozen schema accepts the additive optional `source` object.
    let claim = receipt
        .get("claims")
        .and_then(Value::as_array)
        .and_then(|claims| claims.first())
        .expect("receipt claims");
    assert_eq!(
        claim.pointer(&["source", "producer"]).and_then(Value::as_str),
        Some("ctxpect-resolve::resolved_claim"),
        "{claim:?}"
    );
    assert_eq!(
        claim.pointer(&["source", "source_domain"]).and_then(Value::as_str),
        Some("static-resolution"),
        "{claim:?}"
    );

    // The stored copy validates too.
    let id = receipt.get("receipt_id").and_then(Value::as_str).unwrap();
    let stored = store.get_receipt(id).unwrap();
    validate(&schema, &stored).unwrap_or_else(|errors| panic!("stored receipt: {errors:#?}"));

    // A tombstone keeps the schema and empties the bodies.
    let stone = store.delete_receipt(id, "schema-test").unwrap();
    validate(&schema, &stone).unwrap_or_else(|errors| panic!("tombstone: {errors:#?}"));
    validate(
        schema.pointer(&["$defs", "tombstoned"]).unwrap(),
        stone.get("tombstone").unwrap(),
    )
    .unwrap_or_else(|errors| panic!("tombstone body: {errors:#?}"));

    // Drift is caught: a Receipt without its manifest, and one whose
    // signature claims organizational identity.
    let errors = validate(&schema, &without(&receipt, "manifest")).unwrap_err();
    assert!(errors.iter().any(|e| e.contains("manifest")), "{errors:?}");
    let mut forged = match receipt.clone() {
        Value::Object(map) => map,
        _ => unreachable!(),
    };
    if let Some(Value::Object(sig)) = forged.get_mut("signature") {
        sig.insert("org_identity".into(), Value::Bool(true));
    }
    let errors = validate(&schema, &Value::Object(forged)).unwrap_err();
    assert!(errors.iter().any(|e| e.contains("signature/org_identity")), "{errors:?}");
}

#[test]
fn a_real_diagnosis_with_project_findings_validates_and_a_drifted_one_does_not() {
    let scratch = Scratch::new("doctor");
    fs::write(scratch.path.join("AGENTS.md"), "hello\n").unwrap();
    fs::write(scratch.path.join("NOTES.md"), "token=ghp_fixture_not_a_real_secret_00\n").unwrap();
    fs::write(scratch.path.join("OLD.md"), "---\nupdated: 2019-01-01\n---\nold\n").unwrap();
    let snapshot = inspect_snapshot(&scratch.path);
    let root = Root::new(&scratch.path).unwrap();
    let project = project_doctor_findings(&root)
        .unwrap_or_else(|err| panic!("findings: {} {}", err.code(), err.message()));
    assert!(project.len() >= 2, "both a blocking and a non-blocking finding expected");
    let diagnosis = with_project_findings(diagnose(&snapshot), &project);
    let schema = schema("ctxpect-doctor-v1.schema.json");
    validate(&schema, &diagnosis).unwrap_or_else(|errors| panic!("diagnosis: {errors:#?}"));

    // A finding whose severity is `unknown` is exactly what the schema
    // forbids (C03: Unknown is not a severity).
    let mut drifted = match diagnosis.clone() {
        Value::Object(map) => map,
        _ => unreachable!(),
    };
    if let Some(Value::Array(items)) = drifted.get_mut("findings")
        && let Some(Value::Object(first)) = items.first_mut()
    {
        first.insert("severity".into(), ctxpect_schema::string("unknown"));
    }
    let errors = validate(&schema, &Value::Object(drifted)).unwrap_err();
    assert!(errors.iter().any(|e| e.contains("severity")), "{errors:?}");
}
