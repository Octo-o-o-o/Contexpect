//! Integration tests that start the built `ctxpect` binary.

use ctxpect_cli::{Value, canonical_json, parse, strip_time_fields};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ctxpect"))
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
        let path = std::env::temp_dir().join(format!(
            "cx-cli-{label}-{}-{}",
            std::process::id(),
            nanos % 1_000_000
        ));
        fs::create_dir_all(&path).expect("create scratch");
        Scratch {
            path: fs::canonicalize(&path).expect("canonicalize"),
        }
    }

    fn write(&self, rel: &str, contents: impl AsRef<[u8]>) -> PathBuf {
        let target = self.path.join(rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).expect("parent");
        }
        fs::write(&target, contents).expect("write");
        target
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

struct Run {
    code: i32,
    json: Value,
    stdout: String,
    #[allow(dead_code)]
    stderr: String,
}

fn run_inspect(project: &Path, extra: &[&str]) -> Run {
    let mut args = vec![
        "inspect",
        "--json",
        "--offline",
        "--project",
        project.to_str().expect("utf8 project"),
    ];
    args.extend_from_slice(extra);
    run_args(&args, None)
}

fn run_args(args: &[&str], env: Option<&[(&str, &Path)]>) -> Run {
    let mut cmd = Command::new(bin());
    cmd.args(args);
    if let Some(pairs) = env {
        for (key, value) in pairs {
            cmd.env(key, value);
        }
    }
    let output = cmd.output().expect("spawn ctxpect");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output.status.code().unwrap_or(255);
    let json = parse(stdout.trim()).unwrap_or_else(|error| {
        panic!("parse JSON (exit {code}): {error}; stdout={stdout:?}; stderr={stderr:?}");
    });
    Run {
        code,
        json,
        stdout,
        stderr,
    }
}

fn first_result(json: &Value) -> &Value {
    json.get("results")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .expect("results[0]")
}

fn explanation_items(json: &Value) -> &[Value] {
    json.get("explanation")
        .and_then(Value::as_array)
        .expect("explanation")
}

fn has_explanation(json: &Value, kind: &str, rule_id: &str, path_substr: &str) -> bool {
    explanation_items(json).iter().any(|item| {
        item.get("kind").and_then(Value::as_str) == Some(kind)
            && item.get("rule_id").and_then(Value::as_str) == Some(rule_id)
            && item
                .get("path")
                .and_then(Value::as_str)
                .is_some_and(|path| path.contains(path_substr))
    })
}

#[test]
fn g1_layers_from_root_toward_cwd_and_rejects_outside_cwd() {
    let scratch = Scratch::new("g1");
    scratch.write("AGENTS.md", "root\n");
    scratch.write("src/AGENTS.md", "src\n");
    scratch.write("src/app/AGENTS.md", "app\n");
    scratch.write("other/AGENTS.md", "other\n");

    let nested = run_inspect(
        &scratch.path,
        &["--cwd", scratch.path.join("src").to_str().unwrap()],
    );
    assert_eq!(nested.code, 0, "{}", nested.stdout);
    let result = first_result(&nested.json);
    assert_eq!(result.get("included"), Some(&Value::Bool(true)));
    let used = result
        .get("native_paths_used")
        .and_then(Value::as_array)
        .expect("native_paths_used");
    let used: Vec<&str> = used.iter().filter_map(Value::as_str).collect();
    assert!(used.contains(&"AGENTS.md"));
    assert!(used.contains(&"src/AGENTS.md"));
    assert!(!used.contains(&"src/app/AGENTS.md"));
    assert!(!used.contains(&"other/AGENTS.md"));
    assert!(has_explanation(&nested.json, "included", "G1", "AGENTS.md"));

    let outside = Scratch::new("g1-out");
    outside.write("keep", "x\n");
    let rejected = run_inspect(&scratch.path, &["--cwd", outside.path.to_str().unwrap()]);
    assert_eq!(rejected.code, 1);
    assert_eq!(
        rejected
            .json
            .get("error")
            .and_then(|error| error.get("code"))
            .and_then(Value::as_str),
        Some("io.cwd_outside_project")
    );
}

#[test]
fn g2_override_and_shadow_edges_point_at_the_override_file() {
    let scratch = Scratch::new("g2");
    scratch.write("AGENTS.override.md", "override\n");
    scratch.write("AGENTS.md", "base\n");
    let run = run_inspect(&scratch.path, &[]);
    assert_eq!(run.code, 0, "{}", run.stdout);
    let edges = first_result(&run.json)
        .get("edges")
        .and_then(Value::as_array)
        .expect("edges");
    assert!(edges.iter().any(|edge| {
        edge.get("kind").and_then(Value::as_str) == Some("included-by")
            && edge.get("path").and_then(Value::as_str) == Some("AGENTS.override.md")
            && edge.get("rule_id").and_then(Value::as_str) == Some("G1")
    }));
    assert!(edges.iter().any(|edge| {
        edge.get("kind").and_then(Value::as_str) == Some("overridden-by")
            && edge.get("path").and_then(Value::as_str) == Some("AGENTS.md")
            && edge.get("related_path").and_then(Value::as_str) == Some("AGENTS.override.md")
            && edge.get("rule_id").and_then(Value::as_str) == Some("G2")
    }));
    assert!(edges.iter().any(|edge| {
        edge.get("kind").and_then(Value::as_str) == Some("shadowed-by")
            && edge.get("path").and_then(Value::as_str) == Some("AGENTS.md")
            && edge.get("related_path").and_then(Value::as_str) == Some("AGENTS.override.md")
            && edge.get("rule_id").and_then(Value::as_str) == Some("G2")
    }));
    assert!(has_explanation(
        &run.json,
        "overridden-by",
        "G2",
        "AGENTS.md"
    ));
}

#[test]
fn g3_truncates_at_32768_and_records_offset() {
    let scratch = Scratch::new("g3");
    scratch.write("AGENTS.md", "y".repeat(33000));
    let run = run_inspect(&scratch.path, &[]);
    assert_eq!(run.code, 0, "{}", run.stdout);
    let edges = first_result(&run.json)
        .get("edges")
        .and_then(Value::as_array)
        .expect("edges");
    assert!(edges.iter().any(|edge| {
        edge.get("kind").and_then(Value::as_str) == Some("truncated-after")
            && edge.get("path").and_then(Value::as_str) == Some("AGENTS.md")
            && edge.get("offset") == Some(&Value::Int(32768))
            && edge.get("rule_id").and_then(Value::as_str) == Some("G3")
    }));
    assert!(has_explanation(&run.json, "truncated", "G3", "AGENTS.md"));
}

#[test]
fn g4_ignore_is_product_exclusion_not_native_rule() {
    let scratch = Scratch::new("g4");
    scratch.write("AGENTS.md", "ignored-body\n");
    scratch.write(".ctxpect-ignore", "AGENTS.md\n");
    let run = run_inspect(&scratch.path, &[]);
    assert_eq!(run.code, 2, "{}", run.stdout);
    let result = first_result(&run.json);
    assert_eq!(result.get("included"), Some(&Value::Bool(false)));
    assert_eq!(
        result.get("truth_state").and_then(Value::as_str),
        Some("absent")
    );
    let item = explanation_items(&run.json)
        .iter()
        .find(|item| item.get("kind").and_then(Value::as_str) == Some("excluded"))
        .expect("excluded explanation");
    assert_eq!(item.get("rule_id").and_then(Value::as_str), Some("G4"));
    assert!(
        item.get("path")
            .and_then(Value::as_str)
            .is_some_and(|path| path.contains("AGENTS.md"))
    );
    assert_eq!(
        item.get("exclusion_class").and_then(Value::as_str),
        Some("product-user-exclusion")
    );
    assert!(
        item.get("why")
            .and_then(Value::as_str)
            .is_some_and(|why| why.contains("not a Codex native rule"))
    );
}

#[test]
fn g5_home_and_codex_home_env_are_not_read_without_explicit_root() {
    let project = Scratch::new("g5p");
    project.write("AGENTS.md", "project-agents\n");
    let sentinel = "SENTINEL_HOME_AGENTS_UNIQUE_9f3c";
    let fake_home = Scratch::new("g5h");
    fake_home.write("AGENTS.md", sentinel);
    let output = Command::new(bin())
        .args([
            "inspect",
            "--json",
            "--offline",
            "--project",
            project.path.to_str().unwrap(),
        ])
        .env("HOME", &fake_home.path)
        .env("CODEX_HOME", &fake_home.path)
        .output()
        .expect("spawn");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert_eq!(output.status.code(), Some(0), "{stdout}{stderr}");
    assert!(
        !stdout.contains(sentinel) && !stderr.contains(sentinel),
        "sentinel leaked: stdout={stdout} stderr={stderr}"
    );
    assert!(!stdout.contains(fake_home.path.to_str().unwrap()));
    let json = parse(stdout.trim()).expect("json");
    assert!(has_explanation(&json, "unknown", "G5", "AGENTS.md"));
    let unknown = json
        .get("unknown")
        .and_then(Value::as_array)
        .expect("unknown");
    assert!(unknown.iter().any(|item| {
        item.get("reason_code").and_then(Value::as_str) == Some("permission_not_granted")
            && item.get("rule_id").and_then(Value::as_str) == Some("G5")
    }));
}

#[test]
fn g5_explicit_codex_home_is_included() {
    let project = Scratch::new("g5pe");
    project.write("README.md", "no agents here\n");
    let home = Scratch::new("g5he");
    home.write("AGENTS.md", "from-granted-home\n");
    let run = run_inspect(
        &project.path,
        &["--codex-home", home.path.to_str().unwrap()],
    );
    assert_eq!(run.code, 0, "{}", run.stdout);
    assert!(has_explanation(
        &run.json,
        "included",
        "G5",
        "<codex-home>/AGENTS.md"
    ));
    assert!(!run.stdout.contains(home.path.to_str().unwrap()));
    let used = first_result(&run.json)
        .get("native_paths_used")
        .and_then(Value::as_array)
        .expect("native_paths_used");
    assert!(
        used.iter()
            .any(|item| item.as_str() == Some("$CODEX_HOME/AGENTS.md"))
    );
}

#[test]
fn unimplemented_flags_and_commands_are_refused() {
    let scratch = Scratch::new("unimpl");
    scratch.write("AGENTS.md", "x\n");
    let project = scratch.path.to_str().unwrap();
    for args in [
        vec!["inspect", "--json", "--project", project, "--config", "x"],
        vec![
            "inspect",
            "--json",
            "--project",
            project,
            "--privacy",
            "strict",
        ],
        vec!["inspect", "--json", "--project", project, "--allow-unknown"],
        vec!["inspect", "--json", "--project", project, "--force"],
    ] {
        let run = run_args(&args, None);
        assert_eq!(run.code, 1, "{args:?} {}", run.stdout);
        assert_eq!(
            run.json
                .get("error")
                .and_then(|error| error.get("code"))
                .and_then(Value::as_str),
            Some("usage.unimplemented"),
            "{args:?}"
        );
    }
}

#[test]
fn exit_codes_present_absent_indeterminate_and_usage() {
    let present = Scratch::new("e0");
    present.write("AGENTS.md", "yes\n");
    assert_eq!(run_inspect(&present.path, &[]).code, 0);

    let absent = Scratch::new("e2");
    absent.write("AGENTS.md", "no\n");
    absent.write(".ctxpect-ignore", "AGENTS.md\n");
    assert_eq!(run_inspect(&absent.path, &[]).code, 2);

    let unsupported = run_inspect(&present.path, &["--version", "0.99.0"]);
    assert_eq!(unsupported.code, 3);
    assert_eq!(
        first_result(&unsupported.json)
            .get("unknown_reason_code")
            .and_then(Value::as_str),
        Some("unsupported_harness_version")
    );

    let usage = run_args(&["inspect", "--json"], None);
    assert_eq!(usage.code, 1);
    assert_eq!(
        usage
            .json
            .get("error")
            .and_then(|error| error.get("code"))
            .and_then(Value::as_str),
        Some("usage.invalid")
    );
}

#[test]
fn snapshot_digest_is_stable_and_moves_with_content() {
    let scratch = Scratch::new("digest");
    scratch.write("AGENTS.md", "stable-body\n");
    let first = run_inspect(&scratch.path, &[]);
    let second = run_inspect(&scratch.path, &[]);
    assert_eq!(first.code, 0);
    let a = canonical_json(&strip_time_fields(&first.json));
    let b = canonical_json(&strip_time_fields(&second.json));
    assert_eq!(a, b);
    let digest_a = first
        .json
        .get("snapshot_digest")
        .and_then(Value::as_str)
        .expect("digest");
    let digest_b = second
        .json
        .get("snapshot_digest")
        .and_then(Value::as_str)
        .expect("digest");
    assert_eq!(digest_a, digest_b);

    scratch.write("AGENTS.md", "stable-body!\n");
    let third = run_inspect(&scratch.path, &[]);
    let digest_c = third
        .json
        .get("snapshot_digest")
        .and_then(Value::as_str)
        .expect("digest");
    assert_ne!(digest_a, digest_c);
}

#[test]
fn explanation_included_excluded_unknown_are_field_complete() {
    let included = Scratch::new("exp-in");
    included.write("AGENTS.md", "keep\n");
    let run = run_inspect(&included.path, &[]);
    let item = explanation_items(&run.json)
        .iter()
        .find(|item| item.get("kind").and_then(Value::as_str) == Some("included"))
        .expect("included");
    assert_eq!(item.get("rule_id").and_then(Value::as_str), Some("G1"));
    assert!(
        item.get("path")
            .and_then(Value::as_str)
            .is_some_and(|path| path == "<project>/AGENTS.md")
    );
    assert!(item.get("why").and_then(Value::as_str).is_some());
    assert!(item.get("next_evidence").and_then(Value::as_str).is_some());

    let excluded = Scratch::new("exp-ex");
    excluded.write("AGENTS.md", "no\n");
    excluded.write(".ctxpect-ignore", "AGENTS.md\n");
    let run = run_inspect(&excluded.path, &[]);
    let item = explanation_items(&run.json)
        .iter()
        .find(|item| item.get("kind").and_then(Value::as_str) == Some("excluded"))
        .expect("excluded");
    assert_eq!(item.get("rule_id").and_then(Value::as_str), Some("G4"));
    assert!(
        item.get("path")
            .and_then(Value::as_str)
            .is_some_and(|path| path.contains("AGENTS.md"))
    );
    assert_eq!(
        item.get("exclusion_class").and_then(Value::as_str),
        Some("product-user-exclusion")
    );

    let unknown = run_inspect(&included.path, &["--os-lane", "ubuntu-24.04-x86_64"]);
    let item = explanation_items(&unknown.json)
        .iter()
        .find(|item| {
            item.get("kind").and_then(Value::as_str) == Some("unknown")
                && item.get("unknown_reason_code").and_then(Value::as_str)
                    == Some("official_distribution_not_captured")
        })
        .expect("unknown");
    assert_eq!(item.get("rule_id").and_then(Value::as_str), Some("G6"));
    assert!(item.get("why").and_then(Value::as_str).is_some());
    assert!(item.get("next_evidence").and_then(Value::as_str).is_some());
}

#[test]
fn envelope_marks_development_snapshot_and_user_attested_version() {
    let scratch = Scratch::new("env");
    scratch.write("AGENTS.md", "x\n");
    let run = run_inspect(&scratch.path, &["--version", "0.147.0"]);
    assert_eq!(
        run.json.get("schema").and_then(Value::as_str),
        Some("dev-inspect-v0")
    );
    assert_eq!(
        run.json.get("receipt_kind").and_then(Value::as_str),
        Some("development-snapshot")
    );
    let provenance = run
        .json
        .get("scope")
        .and_then(|scope| scope.get("version_provenance"))
        .and_then(Value::as_str);
    assert_eq!(provenance, Some("user-attested"));
    let claim = first_result(&run.json).get("claim").expect("claim");
    assert_eq!(
        claim.get("provenance").and_then(Value::as_str),
        Some("official-spec")
    );
    assert_ne!(
        claim.get("provenance").and_then(Value::as_str),
        Some("native-runtime")
    );
    assert_eq!(
        first_result(&run.json)
            .get("facets")
            .and_then(|facets| facets.get("model-visible"))
            .and_then(|claim| claim.get("truth_state"))
            .and_then(Value::as_str),
        Some("indeterminate")
    );
}

#[test]
fn human_output_names_rule_file_and_next_evidence() {
    let scratch = Scratch::new("human");
    scratch.write("AGENTS.md", "keep\n");
    let output = Command::new(bin())
        .args([
            "inspect",
            "--offline",
            "--project",
            scratch.path.to_str().unwrap(),
        ])
        .output()
        .expect("spawn");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    assert!(stdout.contains("[G1]"));
    assert!(stdout.contains("<project>/AGENTS.md"));
    assert!(stdout.contains("next evidence"));
    assert!(!stdout.contains(scratch.path.to_str().unwrap()));
}

#[test]
fn non_required_indeterminate_does_not_change_present_exit() {
    let scratch = Scratch::new("nr");
    scratch.write("AGENTS.md", "yes\n");
    let run = run_inspect(&scratch.path, &[]);
    assert_eq!(run.code, 0);
    assert!(
        run.json
            .get("unknown")
            .and_then(Value::as_array)
            .is_some_and(|items| items.iter().any(|item| {
                item.get("reason_code").and_then(Value::as_str) == Some("permission_not_granted")
            }))
    );
}

#[test]
fn extra_required_capability_is_indeterminate_exit_3() {
    let scratch = Scratch::new("skills");
    scratch.write("AGENTS.md", "yes\n");
    let run = run_inspect(
        &scratch.path,
        &["--require", "instructions", "--require", "skills"],
    );
    assert_eq!(run.code, 3, "{}", run.stdout);
}

fn error_code(run: &Run) -> Option<&str> {
    run.json
        .get("error")
        .and_then(|error| error.get("code"))
        .and_then(Value::as_str)
}

#[test]
fn listed_nested_commands_and_command_flags_are_unimplemented() {
    let scratch = Scratch::new("listed");
    scratch.write("AGENTS.md", "x\n");
    let project = scratch.path.to_str().unwrap();
    for args in [
        vec!["doctor", "--json", "--sarif"],
        vec!["inspect", "--json", "--project", project, "--sarif"],
    ] {
        let run = run_args(&args, None);
        assert_eq!(run.code, 1, "{args:?} {}", run.stdout);
        assert_eq!(error_code(&run), Some("usage.unimplemented"), "{args:?}");
        let message = run
            .json
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(
            message.contains("listed in the CLI contract")
                && message.contains("not implemented in this development slice"),
            "{args:?} {message}"
        );
    }
}

#[test]
fn unknown_command_and_flag_are_invalid() {
    let scratch = Scratch::new("invalid");
    scratch.write("AGENTS.md", "x\n");
    let project = scratch.path.to_str().unwrap();
    for args in [
        vec!["definitely-not-a-ctxpect-command", "--json"],
        vec![
            "inspect",
            "--json",
            "--project",
            project,
            "--definitely-not-a-flag",
        ],
        vec!["receipt", "not-a-listed-subcommand", "--json"],
    ] {
        let run = run_args(&args, None);
        assert_eq!(run.code, 1, "{args:?} {}", run.stdout);
        assert_eq!(error_code(&run), Some("usage.invalid"), "{args:?}");
    }
}

#[test]
fn g3_does_not_truncate_at_or_below_32768() {
    for size in [16_usize, 32768] {
        let scratch = Scratch::new(&format!("g3-ok-{size}"));
        scratch.write("AGENTS.md", "n".repeat(size));
        let run = run_inspect(&scratch.path, &[]);
        assert_eq!(run.code, 0, "size={size} {}", run.stdout);
        let edges = first_result(&run.json)
            .get("edges")
            .and_then(Value::as_array)
            .expect("edges");
        assert!(
            edges.iter().all(|edge| {
                edge.get("kind").and_then(Value::as_str) != Some("truncated-after")
                    && edge.get("offset") == Some(&Value::Null)
            }),
            "size={size}"
        );
        assert!(!has_explanation(&run.json, "truncated", "G3", "AGENTS.md"));
    }
}

#[test]
fn g3_truncation_of_codex_home_agents_uses_codex_home_prefix() {
    let project = Scratch::new("g3-home-p");
    project.write("README.md", "no agents\n");
    let home = Scratch::new("g3-home-h");
    home.write("AGENTS.md", "y".repeat(33000));
    let run = run_inspect(
        &project.path,
        &["--codex-home", home.path.to_str().unwrap()],
    );
    assert_eq!(run.code, 0, "{}", run.stdout);
    assert!(has_explanation(
        &run.json,
        "truncated",
        "G3",
        "<codex-home>/AGENTS.md"
    ));
    let item = explanation_items(&run.json)
        .iter()
        .find(|item| item.get("kind").and_then(Value::as_str) == Some("truncated"))
        .expect("truncated explanation");
    assert_eq!(
        item.get("path").and_then(Value::as_str),
        Some("<codex-home>/AGENTS.md")
    );
}

#[test]
fn explicit_codex_home_does_not_surface_sibling_sentinels() {
    let project = Scratch::new("home-sent-p");
    project.write("README.md", "x\n");
    let home = Scratch::new("home-sent-h");
    home.write("AGENTS.md", "from-granted-home\n");
    let session = "SENTINEL_SESSION_UNIQ_rsb03_do_not_read";
    let auth = "SENTINEL_AUTH_UNIQ_rsb03_do_not_read";
    let config = "SENTINEL_CONFIG_UNIQ_rsb03_do_not_read";
    home.write("sessions/2026.jsonl", session);
    home.write("auth.json", auth);
    home.write("config.toml", config);
    let json_run = run_inspect(
        &project.path,
        &["--codex-home", home.path.to_str().unwrap()],
    );
    assert_eq!(json_run.code, 0, "{}", json_run.stdout);
    let combined = format!("{}{}", json_run.stdout, json_run.stderr);
    for needle in [session, auth, config, "sessions/2026.jsonl"] {
        assert!(!combined.contains(needle), "leaked {needle}: {combined}");
    }
    let evidence = first_result(&json_run.json)
        .get("evidence")
        .and_then(Value::as_array)
        .expect("evidence");
    assert!(evidence.iter().all(|item| {
        item.get("path")
            .and_then(Value::as_str)
            .is_some_and(|path| path == "AGENTS.md" || path == ".ctxpect-ignore")
    }));
    let used = first_result(&json_run.json)
        .get("native_paths_used")
        .and_then(Value::as_array)
        .expect("native_paths_used");
    assert!(
        used.iter()
            .any(|item| item.as_str() == Some("$CODEX_HOME/AGENTS.md"))
    );
    assert!(used.iter().all(|item| {
        item.as_str().is_some_and(|path| {
            !path.contains("sessions")
                && path != "auth.json"
                && path != "config.toml"
                && !path.ends_with("/auth.json")
                && !path.ends_with("/config.toml")
        })
    }));
    assert!(has_explanation(
        &json_run.json,
        "included",
        "G5",
        "<codex-home>/AGENTS.md"
    ));

    let human = Command::new(bin())
        .args([
            "inspect",
            "--offline",
            "--project",
            project.path.to_str().unwrap(),
            "--codex-home",
            home.path.to_str().unwrap(),
        ])
        .output()
        .expect("spawn");
    let human_out = format!(
        "{}{}",
        String::from_utf8_lossy(&human.stdout),
        String::from_utf8_lossy(&human.stderr)
    );
    for needle in [session, auth, config, "sessions/2026.jsonl"] {
        assert!(!human_out.contains(needle), "human leaked {needle}");
    }
}

#[test]
fn missing_absolute_project_path_is_redacted() {
    let home = std::env::var("HOME").unwrap_or_default();
    let missing = if home.is_empty() {
        std::env::temp_dir().join(format!(
            "cx-missing-project-{}-does-not-exist",
            std::process::id()
        ))
    } else {
        Path::new(&home).join(format!(
            "cx-missing-project-{}-does-not-exist",
            std::process::id()
        ))
    };
    let missing_text = missing.to_str().expect("utf8 missing");
    let json_out = Command::new(bin())
        .args(["inspect", "--json", "--offline", "--project", missing_text])
        .output()
        .expect("spawn");
    let stdout = String::from_utf8_lossy(&json_out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&json_out.stderr).into_owned();
    assert_eq!(json_out.status.code(), Some(1), "{stdout}{stderr}");
    assert!(!stdout.contains(missing_text), "json leaked path: {stdout}");
    assert!(
        !stderr.contains(missing_text),
        "stderr leaked path: {stderr}"
    );
    if !home.is_empty() {
        assert!(!stdout.contains(&home), "json leaked HOME: {stdout}");
        assert!(!stderr.contains(&home), "stderr leaked HOME: {stderr}");
    }
    let json = parse(stdout.trim()).expect("json");
    assert_eq!(
        json.get("error")
            .and_then(|error| error.get("code"))
            .and_then(Value::as_str),
        Some("io.missing")
    );
    let message = json
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(message.contains("<project>"), "{message}");
    assert!(!message.contains(missing_text), "{message}");

    let human = Command::new(bin())
        .args(["inspect", "--offline", "--project", missing_text])
        .output()
        .expect("spawn");
    let human_out = format!(
        "{}{}",
        String::from_utf8_lossy(&human.stdout),
        String::from_utf8_lossy(&human.stderr)
    );
    assert!(
        !human_out.contains(missing_text),
        "human leaked path: {human_out}"
    );
    if !home.is_empty() {
        assert!(!human_out.contains(&home), "human leaked HOME: {human_out}");
    }
}

fn irregular_exit_is_indeterminate(run: &Run) {
    assert_eq!(run.code, 3, "{}", run.stdout);
    assert_ne!(run.code, 0);
    assert_ne!(run.code, 2);
    let result = first_result(&run.json);
    assert_eq!(
        result.get("truth_state").and_then(Value::as_str),
        Some("indeterminate")
    );
    assert_eq!(
        result.get("unknown_reason_code").and_then(Value::as_str),
        Some("content_redacted_by_policy")
    );
    assert_eq!(result.get("included"), Some(&Value::Null));
    let layers = result
        .get("layers")
        .and_then(Value::as_array)
        .expect("layers");
    let project_root = layers
        .iter()
        .find(|layer| layer.get("id").and_then(Value::as_str) == Some("project-root"))
        .expect("project-root");
    let existed = project_root
        .get("candidates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|candidate| candidate.get("name").and_then(Value::as_str) == Some("AGENTS.md"))
        .and_then(|candidate| candidate.get("existed"))
        .and_then(Value::as_bool);
    assert_eq!(existed, Some(true), "{}", run.stdout);
}

#[cfg(unix)]
#[test]
fn irregular_symlink_escape_is_indeterminate_exit_3() {
    let outside = Scratch::new("esc-out");
    let sentinel = "SENTINEL_ESCAPE_UNIQ_rsb04_secret";
    outside.write("secret.txt", sentinel);
    let project = Scratch::new("esc-p");
    std::os::unix::fs::symlink(
        outside.path.join("secret.txt"),
        project.path.join("AGENTS.md"),
    )
    .expect("symlink");
    let run = run_inspect(&project.path, &[]);
    irregular_exit_is_indeterminate(&run);
    let combined = format!("{}{}", run.stdout, run.stderr);
    assert!(!combined.contains(sentinel), "escaped content leaked");
    assert!(
        !combined.contains(outside.path.to_str().unwrap()),
        "outside path leaked"
    );
    assert!(
        run.json
            .get("explanation")
            .and_then(Value::as_array)
            .is_some_and(|items| items.iter().any(|item| {
                item.get("unknown_reason_code").and_then(Value::as_str)
                    == Some("content_redacted_by_policy")
            }))
    );
}

#[cfg(unix)]
#[test]
fn irregular_dangling_symlink_is_indeterminate_exit_3() {
    let project = Scratch::new("dangle");
    std::os::unix::fs::symlink(
        "no-such-target-rsb04-unique",
        project.path.join("AGENTS.md"),
    )
    .expect("dangling symlink");
    let run = run_inspect(&project.path, &[]);
    irregular_exit_is_indeterminate(&run);
    let combined = format!("{}{}", run.stdout, run.stderr);
    assert!(
        !combined.contains("no-such-target-rsb04-unique"),
        "dangling target leaked"
    );
}

#[cfg(unix)]
#[test]
fn irregular_fifo_is_indeterminate_exit_3() {
    let project = Scratch::new("fifo");
    let fifo = project.path.join("AGENTS.md");
    let made = Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    assert!(made, "mkfifo must succeed for this required case");
    let run = run_inspect(&project.path, &[]);
    irregular_exit_is_indeterminate(&run);
}

#[cfg(unix)]
#[test]
fn irregular_hard_link_is_indeterminate_exit_3() {
    let project = Scratch::new("hard");
    let body = "SENTINEL_HARDLINK_UNIQ_rsb04_body\n";
    project.write("AGENTS.md", body);
    fs::hard_link(
        project.path.join("AGENTS.md"),
        project.path.join("alias.md"),
    )
    .expect("hard link");
    let run = run_inspect(&project.path, &[]);
    irregular_exit_is_indeterminate(&run);
    let combined = format!("{}{}", run.stdout, run.stderr);
    assert!(!combined.contains(body.trim()), "hard-link body leaked");
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

fn has_absolute_path_fragment(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    for (index, ch) in chars.iter().enumerate() {
        if *ch != '/' {
            continue;
        }
        let next_letter = chars
            .get(index + 1)
            .is_some_and(|c| c.is_ascii_alphabetic());
        if !next_letter {
            continue;
        }
        let prev_not_lt = index == 0 || chars[index - 1] != '<';
        if prev_not_lt {
            return true;
        }
    }
    false
}

fn assert_no_host_path_leak(text: &str) {
    for needle in ["/dev", "/private", "/etc", "/Users"] {
        assert!(!text.contains(needle), "leaked {needle}: {text}");
    }
    if let Ok(home) = std::env::var("HOME")
        && home.len() > 1
    {
        assert!(!text.contains(&home), "leaked HOME: {text}");
    }
    assert!(
        !has_absolute_path_fragment(text),
        "absolute path fragment: {text}"
    );
}

fn run_inspect_human(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(bin())
        .args(args)
        .output()
        .expect("spawn ctxpect");
    (
        output.status.code().unwrap_or(255),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn assert_unknown_override_cli(run: &Run) {
    assert_eq!(run.code, 3, "{}", run.stdout);
    let result = first_result(&run.json);
    assert_eq!(
        result.get("truth_state").and_then(Value::as_str),
        Some("indeterminate")
    );
    assert_eq!(result.get("included"), Some(&Value::Null));
    let edges = result
        .get("edges")
        .and_then(Value::as_array)
        .expect("edges");
    assert!(
        edges
            .iter()
            .all(|edge| edge.get("kind").and_then(Value::as_str) != Some("included-by")),
        "{}",
        run.stdout
    );
    assert!(edges.iter().any(|edge| {
        edge.get("kind").and_then(Value::as_str) == Some("overridden-by")
            && edge.get("path").and_then(Value::as_str) == Some("AGENTS.md")
            && edge.get("related_path").and_then(Value::as_str) == Some("AGENTS.override.md")
            && edge
                .get("note")
                .and_then(Value::as_str)
                .is_some_and(|note| note.contains("unknown"))
            && edge.get("rule_id").and_then(Value::as_str) == Some("G2")
    }));
}

#[cfg(unix)]
#[test]
fn unknown_override_escape_symlink_does_not_adopt_agents() {
    let outside = Scratch::new("cli-ov-esc-out");
    outside.write("secret.txt", "SENTINEL_CLI_OVERRIDE_ESCAPE\n");
    let project = Scratch::new("cli-ov-esc");
    std::os::unix::fs::symlink(
        outside.path.join("secret.txt"),
        project.path.join("AGENTS.override.md"),
    )
    .expect("symlink");
    project.write("AGENTS.md", "should-not-be-adopted\n");
    let run = run_inspect(&project.path, &[]);
    assert_unknown_override_cli(&run);
    let combined = format!("{}{}", run.stdout, run.stderr);
    assert!(!combined.contains("SENTINEL_CLI_OVERRIDE_ESCAPE"));
    assert!(!combined.contains("should-not-be-adopted"));
}

#[cfg(unix)]
#[test]
fn unknown_override_fifo_does_not_adopt_agents() {
    let project = Scratch::new("cli-ov-fifo");
    let fifo = project.path.join("AGENTS.override.md");
    let made = Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    assert!(made, "mkfifo must succeed for this required case");
    project.write("AGENTS.md", "should-not-be-adopted\n");
    let run = run_inspect(&project.path, &[]);
    assert_unknown_override_cli(&run);
    assert!(!format!("{}{}", run.stdout, run.stderr).contains("should-not-be-adopted"));
}

#[test]
fn regular_override_still_adopts_override_file() {
    let project = Scratch::new("cli-ov-ok");
    project.write("AGENTS.override.md", "override-body\n");
    project.write("AGENTS.md", "base-body\n");
    let run = run_inspect(&project.path, &[]);
    assert_eq!(run.code, 0, "{}", run.stdout);
    let edges = first_result(&run.json)
        .get("edges")
        .and_then(Value::as_array)
        .expect("edges");
    assert!(edges.iter().any(|edge| {
        edge.get("kind").and_then(Value::as_str) == Some("included-by")
            && edge.get("path").and_then(Value::as_str) == Some("AGENTS.override.md")
    }));
    assert!(edges.iter().any(|edge| {
        edge.get("kind").and_then(Value::as_str) == Some("overridden-by")
            && edge.get("path").and_then(Value::as_str) == Some("AGENTS.md")
            && edge.get("related_path").and_then(Value::as_str) == Some("AGENTS.override.md")
    }));
}

#[test]
fn io_errors_do_not_leak_absolute_paths() {
    let cases: &[(&[&str], &str)] = &[
        (
            &["inspect", "--json", "--offline", "--project", "/dev/stdin"],
            "io.not_a_directory",
        ),
        (
            &[
                "inspect",
                "--json",
                "--offline",
                "--project",
                "/etc/../etc/passwd",
            ],
            "io.not_a_directory",
        ),
    ];
    for (args, expected_code) in cases {
        let run = run_args(args, None);
        assert_eq!(run.code, 1, "{args:?} {}", run.stdout);
        assert_eq!(
            error_code(&run),
            Some(*expected_code),
            "{args:?} {}",
            run.stdout
        );
        assert_no_host_path_leak(&run.stdout);
        assert_no_host_path_leak(&run.stderr);
    }

    let (code, out, err) = run_inspect_human(&["inspect", "--offline", "--project", "/dev/stdin"]);
    assert_eq!(code, 1, "{out}{err}");
    assert_no_host_path_leak(&out);
    assert_no_host_path_leak(&err);
    assert!(
        err.contains("io.not_a_directory") || out.contains("io.not_a_directory"),
        "{out}{err}"
    );

    let (code, out, err) =
        run_inspect_human(&["inspect", "--offline", "--project", "/etc/../etc/passwd"]);
    assert_eq!(code, 1, "{out}{err}");
    assert_no_host_path_leak(&out);
    assert_no_host_path_leak(&err);

    let project = Scratch::new("cwd-miss");
    project.write("AGENTS.md", "x\n");
    let missing_cwd = "/var/tmp/cx-missing-cwd-rsb05-does-not-exist";
    let json_run = run_args(
        &[
            "inspect",
            "--json",
            "--offline",
            "--project",
            project.path.to_str().unwrap(),
            "--cwd",
            missing_cwd,
        ],
        None,
    );
    assert_eq!(json_run.code, 1, "{}", json_run.stdout);
    assert_eq!(error_code(&json_run), Some("io.missing"));
    assert_no_host_path_leak(&json_run.stdout);
    assert_no_host_path_leak(&json_run.stderr);
    assert!(json_run.stdout.contains("<cwd>"), "{}", json_run.stdout);

    let (code, out, err) = run_inspect_human(&[
        "inspect",
        "--offline",
        "--project",
        project.path.to_str().unwrap(),
        "--cwd",
        missing_cwd,
    ]);
    assert_eq!(code, 1, "{out}{err}");
    assert_no_host_path_leak(&out);
    assert_no_host_path_leak(&err);
}

#[test]
fn g3_zero_byte_file_after_exact_cap_is_not_truncated() {
    let scratch = Scratch::new("cli-g3-zero");
    scratch.write("AGENTS.md", "n".repeat(32768));
    scratch.write("src/AGENTS.md", "");
    let run = run_inspect(
        &scratch.path,
        &["--cwd", scratch.path.join("src").to_str().unwrap()],
    );
    assert_eq!(run.code, 0, "{}", run.stdout);
    let edges = first_result(&run.json)
        .get("edges")
        .and_then(Value::as_array)
        .expect("edges");
    assert!(
        edges
            .iter()
            .all(|edge| { edge.get("kind").and_then(Value::as_str) != Some("truncated-after") }),
        "{}",
        run.stdout
    );
    assert!(!has_explanation(&run.json, "truncated", "G3", "AGENTS.md"));
}

#[test]
fn g3_one_byte_file_after_exact_cap_is_truncated_at_file_offset_zero() {
    let scratch = Scratch::new("cli-g3-one");
    scratch.write("AGENTS.md", "n".repeat(32768));
    scratch.write("src/AGENTS.md", "x");
    let run = run_inspect(
        &scratch.path,
        &["--cwd", scratch.path.join("src").to_str().unwrap()],
    );
    assert_eq!(run.code, 0, "{}", run.stdout);
    let edges = first_result(&run.json)
        .get("edges")
        .and_then(Value::as_array)
        .expect("edges");
    assert!(edges.iter().any(|edge| {
        edge.get("kind").and_then(Value::as_str) == Some("truncated-after")
            && edge.get("path").and_then(Value::as_str) == Some("src/AGENTS.md")
            && edge.get("offset") == Some(&Value::Int(32768))
            && edge.get("file_offset") == Some(&Value::Int(0))
            && edge.get("rule_id").and_then(Value::as_str) == Some("G3")
    }));
}

#[test]
fn global_layer_rel_uses_codex_home_prefix() {
    let project = Scratch::new("layer-rel-p");
    project.write("AGENTS.md", "project-agents\n");
    let home = Scratch::new("layer-rel-h");
    home.write("AGENTS.md", "home-agents\n");
    let run = run_inspect(
        &project.path,
        &["--codex-home", home.path.to_str().unwrap()],
    );
    assert_eq!(run.code, 0, "{}", run.stdout);
    let layers = first_result(&run.json)
        .get("layers")
        .and_then(Value::as_array)
        .expect("layers");
    let global = layers
        .iter()
        .find(|layer| layer.get("id").and_then(Value::as_str) == Some("global"))
        .expect("global");
    assert_eq!(
        global.get("rel").and_then(Value::as_str),
        Some("<codex-home>/")
    );
    let project_root = layers
        .iter()
        .find(|layer| layer.get("id").and_then(Value::as_str) == Some("project-root"))
        .expect("project-root");
    assert_eq!(
        project_root.get("rel").and_then(Value::as_str),
        Some("<project>/")
    );

    let (code, out, err) = run_inspect_human(&[
        "inspect",
        "--offline",
        "--project",
        project.path.to_str().unwrap(),
        "--codex-home",
        home.path.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("<codex-home>/AGENTS.md"), "{out}");
    assert!(out.contains("<project>/AGENTS.md"), "{out}");
}

#[test]
fn ignored_corpus_agents_body_and_digest_are_absent() {
    let project = workspace_root().join(
        "acceptance/corpus/development/static/inputs/dev__static__codex__0.147.0__cli__macos-27-arm64__instructions__negative",
    );
    let body = fs::read_to_string(project.join("AGENTS.md")).expect("fixture body");
    let digest = "845777b9b2d902a75ca9fe1b6f1845b8f76596ee615f833ceb0730da103b904e";
    let json_run = run_inspect(&project, &[]);
    assert_eq!(json_run.code, 2, "{}", json_run.stdout);
    let combined = format!("{}{}", json_run.stdout, json_run.stderr);
    assert!(!combined.contains(digest), "digest leaked: {combined}");
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        assert!(
            !combined.contains(trimmed),
            "body leaked {trimmed}: {combined}"
        );
    }
    let evidence = first_result(&json_run.json)
        .get("evidence")
        .and_then(Value::as_array)
        .expect("evidence");
    let agents = evidence
        .iter()
        .find(|item| item.get("path").and_then(Value::as_str) == Some("AGENTS.md"))
        .expect("AGENTS.md evidence");
    assert_eq!(agents.get("content_digest"), Some(&Value::Null));

    let (code, out, err) = run_inspect_human(&[
        "inspect",
        "--offline",
        "--project",
        project.to_str().unwrap(),
    ]);
    assert_eq!(code, 2, "{out}{err}");
    let human = format!("{out}{err}");
    assert!(!human.contains(digest), "human digest leaked: {human}");
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        assert!(!human.contains(trimmed), "human body leaked {trimmed}");
    }
}

fn combined_text(run: &Run) -> String {
    format!("{}{}", run.stdout, run.stderr)
}

fn assert_no_echo(text: &str, original: &str) {
    assert!(!text.contains(original), "echoed {original:?}: {text}");
    assert!(!text.contains("/etc"), "leaked /etc: {text}");
    assert!(!text.contains(".."), "leaked ..: {text}");
}

#[test]
fn coordinate_params_reject_path_like_and_overlong_values_without_echo() {
    let scratch = Scratch::new("coord-reject");
    scratch.write("AGENTS.md", "yes\n");
    let project = scratch.path.to_str().unwrap().to_string();
    let long = "x".repeat(80);
    let flags = ["version", "harness", "surface", "os-lane", "require"];
    let bads = ["/etc/passwd", "../x", long.as_str()];
    for flag in flags {
        for bad in bads {
            let flag_arg = format!("--{flag}");
            let json_args = [
                "inspect",
                "--json",
                "--offline",
                "--project",
                project.as_str(),
                flag_arg.as_str(),
                bad,
            ];
            let run = run_args(&json_args, None);
            assert_eq!(run.code, 1, "{json_args:?} {}", run.stdout);
            assert_eq!(error_code(&run), Some("usage.invalid"), "{json_args:?}");
            assert_no_echo(&combined_text(&run), bad);
            let message = run
                .json
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("");
            assert!(!message.contains(bad), "{flag} {message}");
            assert!(message.contains(&format!("`--{flag}`")), "{message}");

            let (code, out, err) = run_inspect_human(&[
                "inspect",
                "--offline",
                "--project",
                project.as_str(),
                flag_arg.as_str(),
                bad,
            ]);
            assert_eq!(code, 1, "{flag} {bad} {out}{err}");
            assert_no_echo(&format!("{out}{err}"), bad);
        }
    }
}

#[test]
fn legal_coordinate_controls_keep_present_and_honesty_behaviour() {
    let scratch = Scratch::new("coord-ok");
    scratch.write("AGENTS.md", "yes\n");

    let present = run_inspect(&scratch.path, &["--version", "0.147.0"]);
    assert_eq!(present.code, 0, "{}", present.stdout);
    assert_eq!(
        present
            .json
            .get("scope")
            .and_then(|scope| scope.get("version"))
            .and_then(Value::as_str),
        Some("0.147.0")
    );

    let beta = run_inspect(&scratch.path, &["--version", "0.153.3-beta.1"]);
    assert_eq!(beta.code, 3, "{}", beta.stdout);
    assert_eq!(
        beta.json
            .get("scope")
            .and_then(|scope| scope.get("version"))
            .and_then(Value::as_str),
        Some("0.153.3-beta.1")
    );
    assert_eq!(
        first_result(&beta.json)
            .get("unknown_reason_code")
            .and_then(Value::as_str),
        Some("unsupported_harness_version")
    );

    let lane = run_inspect(&scratch.path, &["--os-lane", "ubuntu-24.04-x86_64"]);
    assert_eq!(lane.code, 3, "{}", lane.stdout);
    assert_eq!(
        first_result(&lane.json)
            .get("unknown_reason_code")
            .and_then(Value::as_str),
        Some("official_distribution_not_captured")
    );
    assert_eq!(
        lane.json
            .get("scope")
            .and_then(|scope| scope.get("os_lane"))
            .and_then(Value::as_str),
        Some("ubuntu-24.04-x86_64")
    );

    let required = run_inspect(&scratch.path, &["--require", "instructions"]);
    assert_eq!(required.code, 0, "{}", required.stdout);
    assert_eq!(
        first_result(&required.json)
            .get("truth_state")
            .and_then(Value::as_str),
        Some("present")
    );
}

#[test]
fn ignore_absolute_and_parent_lines_are_skipped_with_line_warnings() {
    let scratch = Scratch::new("ignore-skip");
    scratch.write("AGENTS.md", "keep-visible-body\n");
    scratch.write(".ctxpect-ignore", "/etc/passwd\nAGENTS.md\n../x\n");
    let json_run = run_inspect(&scratch.path, &[]);
    assert_eq!(json_run.code, 2, "{}", json_run.stdout);
    let combined = combined_text(&json_run);
    assert!(!combined.contains("/etc/passwd"), "{combined}");
    assert!(!combined.contains("../x"), "{combined}");
    assert!(!combined.contains("/etc"), "{combined}");
    let warnings = json_run
        .json
        .get("warnings")
        .and_then(Value::as_array)
        .expect("warnings");
    assert!(
        warnings.iter().any(|item| {
            item.get("code").and_then(Value::as_str) == Some("ignore.invalid_line")
                && item.get("line") == Some(&Value::Int(1))
        }),
        "{}",
        json_run.stdout
    );
    assert!(
        warnings.iter().any(|item| {
            item.get("code").and_then(Value::as_str) == Some("ignore.invalid_line")
                && item.get("line") == Some(&Value::Int(3))
        }),
        "{}",
        json_run.stdout
    );
    assert!(
        !warnings
            .iter()
            .any(|item| item.get("line") == Some(&Value::Int(2))),
        "valid line should not warn: {}",
        json_run.stdout
    );
    assert_eq!(
        first_result(&json_run.json)
            .get("truth_state")
            .and_then(Value::as_str),
        Some("absent")
    );

    let (code, out, err) = run_inspect_human(&[
        "inspect",
        "--offline",
        "--project",
        scratch.path.to_str().unwrap(),
    ]);
    assert_eq!(code, 2, "{out}{err}");
    let human = format!("{out}{err}");
    assert!(!human.contains("/etc/passwd"), "{human}");
    assert!(!human.contains("../x"), "{human}");
    assert!(human.contains("ignore.invalid_line (line 1)"), "{human}");
    assert!(human.contains("ignore.invalid_line (line 3)"), "{human}");
}

#[test]
fn unsafe_unknown_command_and_unimplemented_value_are_not_echoed() {
    let scratch = Scratch::new("unsafe-token");
    scratch.write("AGENTS.md", "x\n");
    let project = scratch.path.to_str().unwrap();
    let unknown = run_args(&["/etc/passwd", "--json"], None);
    assert_eq!(unknown.code, 1);
    assert_eq!(error_code(&unknown), Some("usage.invalid"));
    assert_no_echo(&combined_text(&unknown), "/etc/passwd");

    let config = run_args(
        &[
            "inspect",
            "--json",
            "--project",
            project,
            "--config",
            "/etc/passwd",
        ],
        None,
    );
    assert_eq!(config.code, 1);
    assert_eq!(error_code(&config), Some("usage.unimplemented"));
    assert_no_echo(&combined_text(&config), "/etc/passwd");
}

fn required_list(json: &Value) -> Vec<&str> {
    json.get("required")
        .and_then(Value::as_array)
        .expect("required")
        .iter()
        .filter_map(Value::as_str)
        .collect()
}

fn warnings_empty(json: &Value) -> bool {
    json.get("warnings")
        .and_then(Value::as_array)
        .is_some_and(|items| items.is_empty())
}

fn layer_rels(json: &Value) -> Vec<&str> {
    first_result(json)
        .get("layers")
        .and_then(Value::as_array)
        .expect("layers")
        .iter()
        .filter_map(|layer| layer.get("rel").and_then(Value::as_str))
        .collect()
}

#[test]
fn home_tmp_does_not_rewrite_target_tmp_cwd() {
    let project = Scratch::new("r11-tmp-cwd");
    project.write("AGENTS.md", "root-agents\n");
    fs::create_dir_all(project.path.join("target/tmp")).expect("target/tmp");
    let output = Command::new(bin())
        .args([
            "inspect",
            "--json",
            "--offline",
            "--project",
            project.path.to_str().unwrap(),
            "--cwd",
            "target/tmp",
        ])
        .env("HOME", "/tmp")
        .output()
        .expect("spawn ctxpect");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output.status.code().unwrap_or(255);
    let json = parse(stdout.trim()).unwrap_or_else(|error| {
        panic!("parse JSON (exit {code}): {error}; stdout={stdout:?}; stderr={stderr:?}");
    });
    assert_eq!(code, 0, "{stdout}{stderr}");
    assert_eq!(
        json.get("scope")
            .and_then(|scope| scope.get("cwd"))
            .and_then(Value::as_str),
        Some("<project>/target/tmp"),
        "{stdout}"
    );
    let rels = layer_rels(&json);
    assert!(
        rels.contains(&"<project>/target/tmp"),
        "layers missing cwd rel: {rels:?} {stdout}"
    );
    assert!(
        rels.iter().all(|rel| !rel.contains("<home>")),
        "HOME=/tmp rewrote a layer rel: {rels:?} {stdout}"
    );
    assert!(warnings_empty(&json), "{stdout}");
    assert!(!stdout.contains("<project>/target<home>"), "{stdout}");
}

#[test]
fn home_instructions_does_not_rewrite_required_capability() {
    let project = workspace_root().join(
        "acceptance/corpus/development/static/inputs/dev__static__codex__0.147.0__cli__macos-27-arm64__instructions__positive",
    );
    let output = Command::new(bin())
        .args([
            "inspect",
            "--json",
            "--offline",
            "--project",
            project.to_str().unwrap(),
        ])
        .env("HOME", "instructions")
        .output()
        .expect("spawn ctxpect");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output.status.code().unwrap_or(255);
    let json = parse(stdout.trim()).unwrap_or_else(|error| {
        panic!("parse JSON (exit {code}): {error}; stdout={stdout:?}; stderr={stderr:?}");
    });
    assert_eq!(code, 0, "{stdout}{stderr}");
    assert_eq!(required_list(&json), vec!["instructions"], "{stdout}");
    assert_eq!(
        first_result(&json)
            .get("capability_id")
            .and_then(Value::as_str),
        Some("instructions"),
        "{stdout}"
    );
}

#[test]
fn home_absolute_temp_path_still_redacted_in_error_and_evidence() {
    let home = Scratch::new("r11-keep-home");
    let home_text = home.path.to_str().expect("utf8 home").to_string();
    home.write("AGENTS.md", "SENTINEL_R11_HOME_BODY\n");

    let missing = home.path.join("missing-project-rsb11");
    let missing_text = missing.to_str().expect("utf8 missing");
    let err_out = Command::new(bin())
        .args(["inspect", "--json", "--offline", "--project", missing_text])
        .env("HOME", &home.path)
        .output()
        .expect("spawn ctxpect");
    let err_stdout = String::from_utf8_lossy(&err_out.stdout).into_owned();
    let err_stderr = String::from_utf8_lossy(&err_out.stderr).into_owned();
    assert_eq!(err_out.status.code(), Some(1), "{err_stdout}{err_stderr}");
    assert!(
        !err_stdout.contains(&home_text) && !err_stderr.contains(&home_text),
        "IO error leaked HOME: stdout={err_stdout} stderr={err_stderr}"
    );
    let err_json = parse(err_stdout.trim()).expect("error json");
    assert_eq!(
        err_json
            .get("error")
            .and_then(|error| error.get("code"))
            .and_then(Value::as_str),
        Some("io.missing")
    );
    let message = err_json
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(
        message.contains("<project>") || message.contains("<home>"),
        "{message}"
    );
    assert!(!message.contains(&home_text), "{message}");

    let project = Scratch::new("r11-keep-proj");
    project.write("AGENTS.md", format!("home-ref {home_text}\n"));
    let ok_out = Command::new(bin())
        .args([
            "inspect",
            "--json",
            "--offline",
            "--project",
            project.path.to_str().unwrap(),
        ])
        .env("HOME", &home.path)
        .output()
        .expect("spawn ctxpect");
    let ok_stdout = String::from_utf8_lossy(&ok_out.stdout).into_owned();
    let ok_stderr = String::from_utf8_lossy(&ok_out.stderr).into_owned();
    assert_eq!(ok_out.status.code(), Some(0), "{ok_stdout}{ok_stderr}");
    assert!(
        !ok_stdout.contains(&home_text) && !ok_stderr.contains(&home_text),
        "evidence path leaked HOME: stdout={ok_stdout} stderr={ok_stderr}"
    );
    let ok_json = parse(ok_stdout.trim()).expect("ok json");
    let evidence = first_result(&ok_json)
        .get("evidence")
        .and_then(Value::as_array)
        .expect("evidence");
    let agents = evidence
        .iter()
        .find(|item| item.get("path").and_then(Value::as_str) == Some("AGENTS.md"))
        .expect("AGENTS.md evidence");
    assert!(
        agents
            .get("content_digest")
            .and_then(Value::as_str)
            .is_some_and(|digest| digest.len() == 64),
        "HOME-bearing AGENTS.md was not read as evidence: {ok_stdout}"
    );
}

fn run_bin_with_home(args: &[&str], home: &str) -> (i32, String, String) {
    let output = Command::new(bin())
        .args(args)
        .env("HOME", home)
        .output()
        .expect("spawn ctxpect");
    (
        output.status.code().unwrap_or(255),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn json_error_message(stdout: &str, stderr: &str, code: i32) -> (Value, String) {
    let json = parse(stdout.trim()).unwrap_or_else(|error| {
        panic!("parse JSON (exit {code}): {error}; stdout={stdout:?}; stderr={stderr:?}");
    });
    let message = json
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    (json, message)
}

#[test]
fn home_error_tokens_are_not_rewritten_by_non_path_home() {
    let (code, stdout, stderr) = run_bin_with_home(&["inspect", "--json"], "project");
    assert_eq!(code, 1, "{stdout}{stderr}");
    let (json, message) = json_error_message(&stdout, &stderr, code);
    assert_eq!(
        json.get("error")
            .and_then(|error| error.get("code"))
            .and_then(Value::as_str),
        Some("usage.invalid"),
        "{stdout}"
    );
    assert!(
        message.contains("--project <dir>"),
        "HOME=project rewrote usage: {message}"
    );
    assert!(
        !message.contains("<home>"),
        "HOME=project injected <home>: {message}"
    );
    assert!(
        !stdout.contains("<home>") && !stderr.contains("<home>"),
        "{stdout}{stderr}"
    );

    let (code, stdout, stderr) = run_bin_with_home(&["inspect"], "project");
    let human = format!("{stdout}{stderr}");
    assert_eq!(code, 1, "{human}");
    assert!(
        human.contains("--project <dir>"),
        "HOME=project rewrote human usage: {human}"
    );
    assert!(
        !human.contains("<home>"),
        "HOME=project injected <home>: {human}"
    );

    let (code, stdout, stderr) = run_bin_with_home(&["--json"], "inspect");
    assert_eq!(code, 1, "{stdout}{stderr}");
    let (json, message) = json_error_message(&stdout, &stderr, code);
    assert_eq!(
        json.get("error")
            .and_then(|error| error.get("code"))
            .and_then(Value::as_str),
        Some("usage.invalid"),
        "{stdout}"
    );
    assert!(
        message.contains("inspect"),
        "HOME=inspect rewrote missing-command: {message}"
    );
    assert!(
        !message.contains("<home>"),
        "HOME=inspect injected <home>: {message}"
    );
    assert!(
        !stdout.contains("<home>") && !stderr.contains("<home>"),
        "{stdout}{stderr}"
    );

    let (code, stdout, stderr) = run_bin_with_home(&[], "inspect");
    let human = format!("{stdout}{stderr}");
    assert_eq!(code, 1, "{human}");
    assert!(
        human.contains("inspect"),
        "HOME=inspect rewrote human missing-command: {human}"
    );
    assert!(
        !human.contains("<home>"),
        "HOME=inspect injected <home>: {human}"
    );

    let project = Scratch::new("r11-home-require");
    project.write("AGENTS.md", "x\n");
    let project_text = project.path.to_str().expect("utf8 project");
    let (code, stdout, stderr) = run_bin_with_home(
        &[
            "inspect",
            "--json",
            "--offline",
            "--project",
            project_text,
            "--require",
            "/etc/passwd",
        ],
        "require",
    );
    assert_eq!(code, 1, "{stdout}{stderr}");
    let (json, message) = json_error_message(&stdout, &stderr, code);
    assert_eq!(
        json.get("error")
            .and_then(|error| error.get("code"))
            .and_then(Value::as_str),
        Some("usage.invalid"),
        "{stdout}"
    );
    assert!(
        message.contains("`--require`"),
        "HOME=require rewrote coordinate usage: {message}"
    );
    assert!(
        !message.contains("<home>"),
        "HOME=require injected <home>: {message}"
    );
    assert!(
        !stdout.contains("<home>") && !stderr.contains("<home>"),
        "{stdout}{stderr}"
    );

    let (code, stdout, stderr) = run_bin_with_home(
        &[
            "inspect",
            "--offline",
            "--project",
            project_text,
            "--require",
            "/etc/passwd",
        ],
        "require",
    );
    let human = format!("{stdout}{stderr}");
    assert_eq!(code, 1, "{human}");
    assert!(
        human.contains("`--require`"),
        "HOME=require rewrote human coordinate usage: {human}"
    );
    assert!(
        !human.contains("<home>"),
        "HOME=require injected <home>: {human}"
    );
}

#[test]
fn home_absolute_temp_path_still_redacted_in_error_human() {
    let home = Scratch::new("r11-keep-home-human");
    let home_text = home.path.to_str().expect("utf8 home").to_string();
    let missing = home.path.join("missing-project-rsb11-human");
    let missing_text = missing.to_str().expect("utf8 missing");
    let (code, stdout, stderr) = run_bin_with_home(
        &["inspect", "--offline", "--project", missing_text],
        &home_text,
    );
    let combined = format!("{stdout}{stderr}");
    assert_eq!(code, 1, "{combined}");
    assert!(
        !combined.contains(&home_text),
        "human IO error leaked HOME: {combined}"
    );
    assert!(
        !combined.contains(missing_text),
        "human IO error leaked project path: {combined}"
    );
    assert!(
        combined.contains("<project>") || combined.contains("<home>"),
        "{combined}"
    );
}

fn human_cwd_line(out: &str) -> Option<&str> {
    out.lines().find(|line| line.starts_with("cwd: "))
}

#[test]
fn codex_home_tmp_does_not_rewrite_target_tmp_cwd() {
    let project = Scratch::new("r12-codex-tmp-cwd");
    project.write("AGENTS.md", "root-agents\n");
    fs::create_dir_all(project.path.join("target/tmp")).expect("target/tmp");
    let project_text = project.path.to_str().expect("utf8 project");
    let output = Command::new(bin())
        .args([
            "inspect",
            "--json",
            "--offline",
            "--project",
            project_text,
            "--cwd",
            "target/tmp",
            "--codex-home",
            "/tmp",
        ])
        .env("HOME", "/var/empty")
        .output()
        .expect("spawn ctxpect");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output.status.code().unwrap_or(255);
    let json = parse(stdout.trim()).unwrap_or_else(|error| {
        panic!("parse JSON (exit {code}): {error}; stdout={stdout:?}; stderr={stderr:?}");
    });
    assert_eq!(code, 0, "{stdout}{stderr}");
    assert_eq!(
        json.get("scope")
            .and_then(|scope| scope.get("cwd"))
            .and_then(Value::as_str),
        Some("<project>/target/tmp"),
        "{stdout}"
    );
    let rels = layer_rels(&json);
    assert!(
        rels.contains(&"<project>/target/tmp"),
        "layers missing cwd rel: {rels:?} {stdout}"
    );
    assert!(
        rels.iter().all(|rel| !rel.contains("target<codex-home>")),
        "--codex-home /tmp rewrote a layer rel: {rels:?} {stdout}"
    );
    assert!(warnings_empty(&json), "{stdout}");
    assert!(!stdout.contains("<project>/target<codex-home>"), "{stdout}");

    let human = Command::new(bin())
        .args([
            "inspect",
            "--offline",
            "--project",
            project_text,
            "--cwd",
            "target/tmp",
            "--codex-home",
            "/tmp",
        ])
        .env("HOME", "/var/empty")
        .output()
        .expect("spawn ctxpect human");
    let human_out = String::from_utf8_lossy(&human.stdout).into_owned();
    let human_err = String::from_utf8_lossy(&human.stderr).into_owned();
    assert_eq!(
        human.status.code(),
        Some(0),
        "{human_out}{human_err}"
    );
    assert_eq!(
        human_cwd_line(&human_out),
        Some("cwd: <project>/target/tmp"),
        "{human_out}"
    );
    assert!(
        !human_out.contains("target<codex-home>"),
        "{human_out}"
    );
}

#[test]
fn project_root_form_does_not_rewrite_ordinary_path_suffix() {
    let project = Scratch::new("r12-proj-suffix");
    project.write("AGENTS.md", "root-agents\n");
    let project_abs = project.path.to_str().expect("utf8 project").to_string();
    let cwd_rel = format!("nested{project_abs}");
    fs::create_dir_all(project.path.join(&cwd_rel)).expect("suffix cwd");
    let output = Command::new(bin())
        .args([
            "inspect",
            "--json",
            "--offline",
            "--project",
            project_abs.as_str(),
            "--cwd",
            cwd_rel.as_str(),
        ])
        .env("HOME", "/var/empty")
        .output()
        .expect("spawn ctxpect");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let code = output.status.code().unwrap_or(255);
    let json = parse(stdout.trim()).unwrap_or_else(|error| {
        panic!("parse JSON (exit {code}): {error}; stdout={stdout:?}; stderr={stderr:?}");
    });
    assert_eq!(code, 0, "{stdout}{stderr}");
    let expected_cwd = format!("<project>/{cwd_rel}");
    assert_eq!(
        json.get("scope")
            .and_then(|scope| scope.get("cwd"))
            .and_then(Value::as_str),
        Some(expected_cwd.as_str()),
        "{stdout}"
    );
    let rels = layer_rels(&json);
    assert!(
        rels.contains(&expected_cwd.as_str()),
        "layers missing cwd rel: {rels:?} {stdout}"
    );
    assert!(
        !stdout.contains("<project>/nested<project>"),
        "project root form rewrote an ordinary suffix: {stdout}"
    );
    assert!(warnings_empty(&json), "{stdout}");

    let human = Command::new(bin())
        .args([
            "inspect",
            "--offline",
            "--project",
            project_abs.as_str(),
            "--cwd",
            cwd_rel.as_str(),
        ])
        .env("HOME", "/var/empty")
        .output()
        .expect("spawn ctxpect human");
    let human_out = String::from_utf8_lossy(&human.stdout).into_owned();
    let human_err = String::from_utf8_lossy(&human.stderr).into_owned();
    assert_eq!(
        human.status.code(),
        Some(0),
        "{human_out}{human_err}"
    );
    let expected_human = format!("cwd: {expected_cwd}");
    assert_eq!(
        human_cwd_line(&human_out),
        Some(expected_human.as_str()),
        "{human_out}"
    );
    assert!(
        !human_out.contains("<project>/nested<project>"),
        "{human_out}"
    );
}

#[test]
fn explicit_root_absolute_paths_stay_redacted_in_json_and_human() {
    let project = Scratch::new("r12-roots-keep");
    project.write("AGENTS.md", "project-agents\n");
    let home = Scratch::new("r12-roots-home");
    home.write("AGENTS.md", "global-agents\n");
    let project_text = project.path.to_str().expect("utf8 project").to_string();
    let home_text = home.path.to_str().expect("utf8 home").to_string();
    let json_run = run_inspect(
        &project.path,
        &["--codex-home", home_text.as_str()],
    );
    assert_eq!(json_run.code, 0, "{}", json_run.stdout);
    let combined = format!("{}{}", json_run.stdout, json_run.stderr);
    assert!(
        !combined.contains(&project_text),
        "JSON leaked project root: {combined}"
    );
    assert!(
        !combined.contains(&home_text),
        "JSON leaked codex-home root: {combined}"
    );
    assert!(has_explanation(
        &json_run.json,
        "included",
        "G5",
        "<codex-home>/AGENTS.md"
    ));
    assert!(has_explanation(
        &json_run.json,
        "included",
        "G1",
        "AGENTS.md"
    ));
    let rels = layer_rels(&json_run.json);
    assert!(
        rels.contains(&"<codex-home>/"),
        "missing global rel: {rels:?}"
    );
    assert!(
        rels.contains(&"<project>/"),
        "missing project rel: {rels:?}"
    );
    assert!(warnings_empty(&json_run.json), "{}", json_run.stdout);

    let (code, out, err) = run_inspect_human(&[
        "inspect",
        "--offline",
        "--project",
        project_text.as_str(),
        "--codex-home",
        home_text.as_str(),
    ]);
    let human = format!("{out}{err}");
    assert_eq!(code, 0, "{human}");
    assert!(
        !human.contains(&project_text),
        "human leaked project root: {human}"
    );
    assert!(
        !human.contains(&home_text),
        "human leaked codex-home root: {human}"
    );
    assert!(human.contains("<codex-home>/AGENTS.md"), "{human}");
    assert!(human.contains("<project>/AGENTS.md"), "{human}");
    assert_eq!(human_cwd_line(&out), Some("cwd: <project>/"), "{out}");
}

/// `--help` / `-h` print usage and exit 0. Help is an additive surface: it does
/// not widen the closed argv grammar, so unimplemented CLI-reference tokens must
/// still fail closed, and it must not claim capabilities this slice lacks.
#[test]
fn help_prints_usage_without_widening_the_grammar() {
    for flag in ["--help", "-h"] {
        let (code, out, err) = run_inspect_human(&[flag]);
        assert_eq!(code, 0, "{flag}: {out}{err}");
        assert!(out.contains("ctxpect inspect"), "{flag}: {out}");
        assert!(out.contains("--codex-home"), "{flag}: {out}");
        assert!(out.contains("indeterminate"), "{flag}: {out}");
        assert!(err.is_empty(), "{flag} wrote to stderr: {err}");
    }

    // Help alongside a command still prints usage rather than running inspect.
    let (code, out, _) = run_inspect_human(&["inspect", "--help"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("用法"), "{out}");

    // The closed grammar is unchanged: an unimplemented flag still fails.
    let (code, out, err) = run_inspect_human(&["inspect", "--project", ".", "--daemon", "x"]);
    let combined = format!("{out}{err}");
    assert_eq!(code, 1, "{combined}");
    assert!(combined.contains("--daemon"), "{combined}");
}
