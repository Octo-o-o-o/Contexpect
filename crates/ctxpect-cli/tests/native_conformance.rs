//! Native-synthetic corpus conformance (required gate `native_conformance`).
//!
//! The fixture under `acceptance/corpus/development/native/` was produced by
//! DeepSeek Harness's own public API (`scripts/fixtures/deepseek-harness/
//! generate.ts` run inside the pinned checkout; `meta.json` records the SHA).
//! `expected.json` holds, for every request (one per `step/start`, the
//! header being the latest snapshot before that step's first provider
//! output — a step whose header did not change logs none), the canonical
//! header digest, the derived message digests, the surface nodes and
//! replacements, plus the interrupted-tail closers. This gate checks that the Rust importer
//! reconstructs exactly those values, that every precise partial answer has
//! a negative that turns red, and that nothing of the bodies reaches the
//! store. It executes no harness: the corpus is synthetic and
//! `live_tested: false`.

use ctxpect_cli::{canonical_json, parse, Value};
use ctxpect_importer::deepseek_harness::{import_deepseek_harness, TOOL_OUTCOME_UNKNOWN};
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .to_path_buf()
}

const FIXTURE: &str = "acceptance/corpus/development/native/deepseek-harness-cli-0.1.2-rc.1";

fn fixture(name: &str) -> Vec<u8> {
    fs::read(repo_root().join(FIXTURE).join(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn fixture_json(name: &str) -> Value {
    parse(std::str::from_utf8(&fixture(name)).unwrap()).unwrap()
}

fn import(bytes: &[u8]) -> Value {
    import_deepseek_harness(bytes, "s_native").unwrap_or_else(|e| panic!("{} {}", e.code, e.message))
}

fn reasons(session: &Value) -> Vec<String> {
    session
        .get("unknown")
        .and_then(Value::as_array)
        .unwrap_or(&[])
        .iter()
        .filter_map(|u| u.get("reason_code").and_then(Value::as_str))
        .map(str::to_string)
        .collect()
}

/// `tail.closers[index]` — `Value::pointer` addresses objects only.
fn closer(session: &Value, index: usize) -> Value {
    session
        .pointer(&["tail", "closers"])
        .and_then(Value::as_array)
        .and_then(|c| c.get(index))
        .cloned()
        .unwrap_or(Value::Null)
}

fn seqs(value: &Value) -> Vec<i64> {
    value.as_array().unwrap_or(&[]).iter().filter_map(Value::as_i64).collect()
}

/// Replace one event line (by seq) with a transformed copy.
fn edit_line(jsonl: &str, seq: usize, f: impl Fn(&str) -> String) -> String {
    let mut out = Vec::new();
    for (index, line) in jsonl.lines().enumerate() {
        // Line 0 is the header; event seq n is line n + 1.
        if index == seq + 1 {
            out.push(f(line));
        } else {
            out.push(line.to_string());
        }
    }
    out.join("\n") + "\n"
}

#[test]
fn the_importer_reconstructs_every_request_prefix_exactly_as_dsh_did() {
    let meta = fixture_json("meta.json");
    assert_eq!(meta.get("live_tested"), Some(&Value::Bool(false)));
    assert_eq!(meta.get("license").and_then(Value::as_str), Some("Apache-2.0"));
    assert_eq!(meta.get("dsh_sha").and_then(Value::as_str).map(str::len), Some(40));
    let jsonl = fixture("session.jsonl");
    assert_eq!(
        meta.get("session_jsonl_sha256").and_then(Value::as_str),
        Some(ctxpect_schema::sha256_hex(&jsonl).as_str()),
        "the fixture on disk is the one the generator wrote"
    );
    let expected = fixture_json("expected.json");
    let session = import(&jsonl);

    assert_eq!(session.get("mapping_id").and_then(Value::as_str), Some("deepseek-harness-cli"));
    assert_eq!(session.pointer(&["format", "version"]), expected.get("format_version"));
    assert_eq!(session.get("event_count"), expected.get("event_count"));
    assert_eq!(session.pointer(&["reconstruction", "complete"]), Some(&Value::Bool(true)));

    let want = expected.get("requests").and_then(Value::as_array).unwrap();
    let got = session.get("requests").and_then(Value::as_array).unwrap();
    assert_eq!(got.len(), want.len(), "request count");
    for (g, w) in got.iter().zip(want) {
        let seq = w.get("seq").and_then(Value::as_i64).unwrap();
        assert_eq!(g.get("seq"), w.get("seq"));
        assert_eq!(g.get("turn"), w.get("turn"), "seq {seq}");
        assert_eq!(g.get("step"), w.get("step"), "seq {seq}");
        assert_eq!(g.get("header_seq"), w.get("header_seq"), "seq {seq}: the effective header event");
        assert_eq!(g.get("header_logged_in_step"), w.get("header_logged_in_step"), "seq {seq}");
        assert_eq!(g.get("dispatch_evidence"), w.get("dispatch_seq"), "seq {seq}: dispatch evidence");
        assert_eq!(g.get("reason"), w.get("reason"), "seq {seq}");
        assert_eq!(g.get("header_digest"), w.get("header_digest"), "seq {seq}: header digest");
        assert_eq!(g.get("message_count"), w.get("message_count"), "seq {seq}: message count");
        let want_digests: Vec<&str> = w
            .get("messages")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .map(|m| m.get("digest").and_then(Value::as_str).unwrap())
            .collect();
        let got_digests: Vec<&str> = g
            .get("message_digests")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert_eq!(got_digests, want_digests, "seq {seq}: message digests in order");
        assert_eq!(seqs(g.get("surface_nodes").unwrap()), seqs(w.get("surface_nodes").unwrap()), "seq {seq}: surface nodes");
        let want_repl = w.get("replacements").and_then(Value::as_array).unwrap();
        let got_repl = g.get("replaced_ranges").and_then(Value::as_array).unwrap();
        assert_eq!(got_repl.len(), want_repl.len(), "seq {seq}: replacements");
        for (gr, wr) in got_repl.iter().zip(want_repl) {
            assert_eq!(gr.get("seq"), wr.get("seq"));
            assert_eq!(gr.get("start"), wr.get("start"));
            assert_eq!(gr.get("end"), wr.get("end"));
            assert_eq!(seqs(gr.get("shadowed").unwrap()), seqs(wr.get("shadowed").unwrap()));
        }
    }
    assert_eq!(seqs(session.get("final_surface").unwrap()), seqs(expected.get("final_surface").unwrap()));

    // Four steps, four requests. Step 3 of turn 1 logged no header of its
    // own: it reuses the step-2 snapshot (seq 14) and is still a request.
    assert_eq!(got.len(), 4);
    let header_seqs: Vec<Option<i64>> = got.iter().map(|g| g.get("header_seq").and_then(Value::as_i64)).collect();
    assert_eq!(header_seqs, vec![Some(5), Some(14), Some(14), Some(25)]);
    assert_eq!(got[2].get("header_logged_in_step"), Some(&Value::Bool(false)));
    assert_eq!(got[2].get("header_digest"), got[1].get("header_digest"), "the reused snapshot has the same digest");
    assert_ne!(got[2].get("message_digests"), got[1].get("message_digests"), "but the surface moved on");
    let dispatch: Vec<Option<i64>> = got.iter().map(|g| g.get("dispatch_evidence").and_then(Value::as_i64)).collect();
    assert_eq!(dispatch, vec![Some(6), Some(15), Some(18), Some(26)]);
    // Usage is provider-reported and travels with the request it answers.
    assert_eq!(got[1].pointer(&["usage", "input_tokens"]).and_then(Value::as_i64), Some(210));
    assert_eq!(got[1].pointer(&["usage", "cache_read_tokens"]).and_then(Value::as_i64), Some(100));
    assert_eq!(session.pointer(&["usage", "cumulative_input_is_occupancy"]), Some(&Value::Bool(false)));
    assert_eq!(session.pointer(&["usage", "context_window"]).and_then(Value::as_i64), Some(128000));
    assert_eq!(session.pointer(&["usage", "context_window_is_used"]), Some(&Value::Bool(false)));

    // Tail: DSH's closers, listed and not appended.
    let want_tail = expected.get("tail").unwrap();
    let got_tail = session.get("tail").unwrap();
    assert_eq!(got_tail.get("interrupted"), want_tail.get("interrupted"));
    assert_eq!(got_tail.get("closers_appended"), Some(&Value::Bool(false)));
    let want_closers: Vec<(&str, Option<&str>)> = want_tail
        .get("closers")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .map(|c| (c.get("type").and_then(Value::as_str).unwrap(), c.get("error_code").and_then(Value::as_str)))
        .collect();
    let got_closers: Vec<(&str, Option<&str>)> = got_tail
        .get("closers")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .map(|c| (c.get("type").and_then(Value::as_str).unwrap(), c.get("error_code").and_then(Value::as_str)))
        .collect();
    assert_eq!(got_closers, want_closers);
    assert_eq!(got_closers[0].1, Some(TOOL_OUTCOME_UNKNOWN));
    assert_eq!(closer(&session, 0).get("call_seq").and_then(Value::as_i64), Some(27));

    // Claims: prepared present for every request; model-visible present only
    // with dispatch evidence; use-evidence / outcome-affecting Unknown.
    let claims = session.get("claims").and_then(Value::as_array).unwrap();
    assert_eq!(claims.len(), 4 * 4);
    for claim in claims {
        let stage = claim.get("lifecycle_stage").and_then(Value::as_str).unwrap();
        let truth = claim.get("truth_state").and_then(Value::as_str).unwrap();
        assert_eq!(claim.get("provenance").and_then(Value::as_str), Some("native-log"));
        assert!(claim.get("evidence_id").and_then(Value::as_str).is_some_and(|id| id.len() == 64));
        match stage {
            "eligible" | "model-visible" => assert_eq!(truth, "present", "{claim:?}"),
            _ => {
                assert_eq!(truth, "indeterminate", "{claim:?}");
                assert_eq!(claim.get("unknown_reason_code").and_then(Value::as_str), Some("runtime_snapshot_missing"));
            }
        }
    }
    let evidence = session.get("evidence").and_then(Value::as_array).unwrap();
    assert_eq!(evidence.len(), 8, "a prepared and a dispatch evidence entry per request");
    assert!(evidence.iter().all(|e| e.get("collection").and_then(Value::as_str) == Some("import")));
    assert!(evidence.iter().all(|e| e.get("importer_version").and_then(Value::as_str).is_some()));
    assert!(evidence.iter().all(|e| e.get("quality").and_then(Value::as_str) == Some("unknown")));
}

#[test]
fn the_persisted_record_is_metadata_only() {
    let jsonl = fixture("session.jsonl");
    let session = import(&jsonl);
    let text = canonical_json(&session);
    assert_eq!(session.get("bodies_stored"), Some(&Value::Bool(false)));
    for body in [
        "List the files in the project.",
        "keep answers brief",
        "AGENTS.md README.md src",
        "Summary of the earlier conversation",
        "Now show the README",
        "Nothing else to list",
        "cat README.md",
        "/tmp/ctxpect-fixture",
        "ctxpect-synthetic-0001",
        "You are a coding agent",
        "call-0001",
        "deepseek-chat",
    ] {
        assert!(!text.contains(body), "body fragment `{body}` reached the record");
    }
    for row in session.get("timeline").and_then(Value::as_array).unwrap() {
        let keys: Vec<&String> = row.as_object().unwrap().keys().collect();
        assert_eq!(keys, vec!["digest", "len", "seq", "surface_op", "type"], "timeline rows carry type/seq/len/digest (+ surface placement) only");
    }
}

#[test]
fn every_precise_partial_answer_has_a_red_negative() {
    let bytes = fixture("session.jsonl");
    let jsonl = String::from_utf8(bytes.clone()).unwrap();

    // Encoding: a Zstandard frame is refused by its magic, never half-read.
    let mut zstd = vec![0x28, 0xB5, 0x2F, 0xFD];
    zstd.extend_from_slice(&bytes);
    let err = import_deepseek_harness(&zstd, "s").unwrap_err();
    assert_eq!(err.code, "import.encoding_unsupported");
    assert!(err.message.contains("Zstandard"), "{}", err.message);

    // Encoding: a packed chunk row is refused, not skipped.
    let packed = edit_line(&jsonl, 7, |_| {
        r#"{"type":"text-chunks","seq0":7,"time0":1700000007000,"data":{"turn":1,"step":1,"index":0,"dt":[1000,1000],"texts":["a","b","c"]}}"#.to_string()
    });
    let err = import_deepseek_harness(packed.as_bytes(), "s").unwrap_err();
    assert_eq!(err.code, "import.encoding_unsupported");
    assert!(err.message.contains("packed"), "{}", err.message);

    // Format version: anything but the pinned version is refused.
    let bumped = jsonl.replacen("\"version\":0", "\"version\":1", 1);
    let err = import_deepseek_harness(bumped.as_bytes(), "s").unwrap_err();
    assert_eq!(err.code, "import.format_version_unsupported");

    // Missing header: provider output in a step with no request/header
    // anywhere in the prefix. The step is listed (it happened) with a null
    // header and no claims; later steps have the step-2 header to reuse.
    let no_header = edit_line(&jsonl, 5, |_| {
        r#"{"type":"request/context","data":{"provider":"deepseek","model":"deepseek-chat"},"seq":5,"time":1700000005000}"#.to_string()
    });
    let session = import(no_header.as_bytes());
    assert!(reasons(&session).contains(&"request_prefix_incomplete".to_string()), "{:?}", reasons(&session));
    let listed = session.get("requests").and_then(Value::as_array).unwrap();
    assert_eq!(listed.len(), 4);
    assert_eq!(listed[0].get("header_digest"), Some(&Value::Null));
    assert_eq!(listed[1].get("header_seq").and_then(Value::as_i64), Some(14));
    assert_eq!(session.get("claims").and_then(Value::as_array).map(<[Value]>::len), Some(3 * 4), "no claims for the header-less step");
    assert_eq!(session.get("partial"), Some(&Value::Bool(true)));

    // Unknown event type without `ignorable`: refused from that event on.
    let alien = edit_line(&jsonl, 4, |line| line.replacen("\"type\":\"request/context\"", "\"type\":\"future/thing\"", 1));
    let session = import(alien.as_bytes());
    let unknown = session.get("unknown").and_then(Value::as_array).unwrap();
    let hit = unknown
        .iter()
        .find(|u| u.get("reason_code").and_then(Value::as_str) == Some("import_parse_failed"))
        .expect("import_parse_failed");
    assert_eq!(hit.get("type_len").and_then(Value::as_i64), Some(12));
    assert_eq!(hit.get("seq").and_then(Value::as_i64), Some(4));
    assert_eq!(session.pointer(&["reconstruction", "stopped_at_seq"]).and_then(Value::as_i64), Some(4));
    // The step that started at seq 3 is listed, header-less: nothing after
    // seq 4 was read, so no header, no dispatch, no claims.
    let listed = session.get("requests").and_then(Value::as_array).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].get("header_digest"), Some(&Value::Null));
    assert_eq!(session.get("claims").and_then(Value::as_array).map(<[Value]>::len), Some(0));
    assert_eq!(session.get("partial"), Some(&Value::Bool(true)));
    assert!(!canonical_json(&session).contains("future/thing"), "the unknown type is not echoed");
    // ... but an `ignorable` one is skipped and reconstruction continues.
    let ignorable = edit_line(&jsonl, 4, |line| {
        line.replacen("\"type\":\"request/context\"", "\"type\":\"future/thing\",\"ignorable\":true", 1)
    });
    let session = import(ignorable.as_bytes());
    assert!(reasons(&session).contains(&"import_event_ignorable_skipped".to_string()));
    assert_eq!(session.pointer(&["reconstruction", "complete"]), Some(&Value::Bool(true)));
    assert_eq!(session.get("requests").and_then(Value::as_array).map(<[Value]>::len), Some(4));

    // Seq discontinuity: the gap is recorded and reconstruction stops there.
    let gap = edit_line(&jsonl, 13, |line| line.replacen("\"seq\":13", "\"seq\":15", 1));
    let session = import(gap.as_bytes());
    let hit = session
        .get("unknown")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .find(|u| u.get("reason_code").and_then(Value::as_str) == Some("seq_discontinuity"))
        .expect("seq_discontinuity");
    assert_eq!(hit.get("expected_seq").and_then(Value::as_i64), Some(13));
    assert_eq!(hit.get("found_seq").and_then(Value::as_i64), Some(15));
    assert_eq!(session.get("requests").and_then(Value::as_array).map(<[Value]>::len), Some(1));
    assert_eq!(session.pointer(&["reconstruction", "stopped_at_seq"]).and_then(Value::as_i64), Some(13));

    // A tool/call without a tool/result is TOOL_OUTCOME_UNKNOWN (the fixture
    // tail); one whose call was never recorded is TOOL_NOT_STARTED.
    let session = import(&bytes);
    assert_eq!(closer(&session, 0).get("error_code").and_then(Value::as_str), Some(TOOL_OUTCOME_UNKNOWN));
    let never_started: String = jsonl.lines().take(28).collect::<Vec<_>>().join("\n") + "\n";
    let session = import(never_started.as_bytes());
    assert_eq!(closer(&session, 0).get("error_code").and_then(Value::as_str), Some("TOOL_NOT_STARTED"));
    assert_eq!(closer(&session, 0).get("call_seq"), Some(&Value::Null));

    // A balanced log has no closers.
    let balanced: String = jsonl.lines().take(22).collect::<Vec<_>>().join("\n") + "\n";
    let session = import(balanced.as_bytes());
    assert_eq!(session.pointer(&["tail", "interrupted"]), Some(&Value::Bool(false)));
    assert_eq!(session.get("partial"), Some(&Value::Bool(false)));

    // Replacement provenance pointing at a seq that does not exist.
    let dangling = edit_line(&jsonl, 21, |line| line.replacen("\"sourceEventSeqs\":[1,2,9,11]", "\"sourceEventSeqs\":[1,2,9,99]", 1));
    let session = import(dangling.as_bytes());
    let hit = session
        .get("unknown")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .find(|u| u.get("reason_code").and_then(Value::as_str) == Some("provenance_incomplete"))
        .expect("provenance_incomplete");
    assert_eq!(hit.get("seq").and_then(Value::as_i64), Some(21));
    assert_eq!(session.get("requests").and_then(Value::as_array).map(<[Value]>::len), Some(3), "the request after the bad replacement is not reconstructed");
    // ... and one that omits a shadowed node.
    let short = edit_line(&jsonl, 21, |line| line.replacen("\"sourceEventSeqs\":[1,2,9,11]", "\"sourceEventSeqs\":[1,2,9]", 1));
    let session = import(short.as_bytes());
    let hit = session
        .get("unknown")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .find(|u| u.get("reason_code").and_then(Value::as_str) == Some("provenance_incomplete"))
        .expect("provenance_incomplete");
    assert_eq!(hit.get("missing_source_seq").and_then(Value::as_i64), Some(11));
}

/// Importing the same artifact twice through the CLI lands in one session
/// record; deleting it removes requests and insights alike.
#[test]
fn import_is_idempotent_and_delete_cascades() {
    use std::process::Command;
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let scratch = std::env::temp_dir().join(format!("cx-native-{}-{}", std::process::id(), nanos % 1_000_000));
    fs::create_dir_all(&scratch).unwrap();
    let store = scratch.join("store");
    fs::create_dir_all(store.join("policies")).unwrap();
    fs::create_dir_all(store.join("exceptions")).unwrap();
    fs::write(
        store.join("policies/active.json"),
        r#"[{"layer":"organization","mode":"enforceable","rules":[]},{"layer":"team","mode":"enforceable","rules":[]},{"layer":"project","mode":"enforceable","rules":[]},{"layer":"user","mode":"detect-only","rules":[]},{"layer":"session","mode":"detect-only","rules":[]}]"#,
    )
    .unwrap();
    let digest = ctxpect_cli::project_scope_digest(&store, None);
    for action in ["sessions.import", "sessions.delete"] {
        let id = format!("ex-{}", action.replace('.', "-"));
        fs::write(
            store.join(format!("exceptions/{id}.json")),
            format!(
                r#"{{"exception_id":"{id}","requester":"operator","action":"{action}","project_digest":"{digest}","target":"*","state":"approved","created_at":1,"expires_at":4102444800,"reason":"test grant","approver":"enrolled-out-of-band"}}"#
            ),
        )
        .unwrap();
    }
    let artifact = repo_root().join(FIXTURE).join("session.jsonl");
    let run = |args: &[&str]| -> (i32, Value) {
        let out = Command::new(env!("CARGO_BIN_EXE_ctxpect")).args(args).output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        (out.status.code().unwrap_or(255), parse(text.trim()).unwrap_or(Value::Null))
    };
    let store_s = store.to_str().unwrap();
    let artifact_s = artifact.to_str().unwrap();
    let (code, first) = run(&["import", "--json", "--store", store_s, "--from", artifact_s, "--mapping", "deepseek-harness-cli"]);
    assert_eq!(code, 0, "{first:?}");
    let id = first.get("session_id").and_then(Value::as_str).unwrap().to_string();
    let (code, second) = run(&["import", "--json", "--store", store_s, "--from", artifact_s, "--mapping", "deepseek-harness-cli"]);
    assert_eq!(code, 0);
    assert_eq!(second.get("session_id").and_then(Value::as_str), Some(id.as_str()), "same bytes, same session id");
    let sessions: Vec<_> = fs::read_dir(store.join("sessions")).unwrap().collect();
    assert_eq!(sessions.len(), 1, "one record, not two");
    assert_eq!(first.get("requests").and_then(Value::as_array).map(<[Value]>::len), Some(4));
    assert_eq!(first.get("bodies_stored"), Some(&Value::Bool(false)));
    let stored = fs::read_to_string(store.join(format!("sessions/{id}.json"))).unwrap();
    assert!(!stored.contains("List the files"), "no body in the store");
    assert!(!stored.contains("/tmp/ctxpect-fixture"), "no path in the store");

    let (code, _) = run(&["sessions", "--json", "--store", store_s, "--session", &id, "--reason", "delete"]);
    assert_eq!(code, 0);
    assert!(!store.join(format!("sessions/{id}.json")).exists());
    assert!(!store.join(format!("insights/{id}.json")).exists());
    let (code, gone) = run(&["sessions", "--json", "--store", store_s, "--session", &id]);
    assert_eq!(code, 1);
    assert_eq!(gone.pointer(&["error", "code"]).and_then(Value::as_str), Some("store.missing"));
    let _ = fs::remove_dir_all(&scratch);
}
