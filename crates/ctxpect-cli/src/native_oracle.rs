//! Actual pinned oracle invocation. Metadata summaries are not automatic Claims.
use crate::{
    args::ProductArgs,
    tool_process::{self, Scratch},
};
use ctxpect_schema::{Value, array, hmac_sha256_hex, object, parse, string};
use std::{fs, path::Path, time::Duration};

type Result<T> = std::result::Result<T, &'static str>;

fn summarize(family: &str, value: &Value, key: &[u8]) -> Result<Value> {
    match family {
        "codex" => {
            let messages = value.as_array().ok_or("native.shape_mismatch")?;
            let mut summaries = Vec::new();
            for item in messages {
                if item.get("type").and_then(Value::as_str) != Some("message") {
                    return Err("native.shape_mismatch");
                }
                let role = item
                    .get("role")
                    .and_then(Value::as_str)
                    .ok_or("native.shape_mismatch")?;
                if !matches!(role, "system" | "developer" | "user" | "assistant") {
                    return Err("native.shape_mismatch");
                }
                let content = item
                    .get("content")
                    .and_then(Value::as_array)
                    .ok_or("native.shape_mismatch")?;
                let mut bytes = 0;
                let mut blocks = Vec::new();
                for block in content {
                    let kind = block
                        .get("type")
                        .and_then(Value::as_str)
                        .ok_or("native.shape_mismatch")?;
                    if !matches!(kind, "input_text" | "output_text") {
                        return Err("native.unsupported_content");
                    }
                    let text = block
                        .get("text")
                        .and_then(Value::as_str)
                        .ok_or("native.shape_mismatch")?;
                    bytes += text.len() as i64;
                    blocks.push(string(hmac_sha256_hex(key, text.as_bytes())));
                }
                summaries.push(object([
                    ("role", string(role)),
                    ("text_bytes", Value::Int(bytes)),
                    ("text_block_digests", array(blocks)),
                ]));
            }
            Ok(object([
                ("normalization", string("codex-prompt-array-v1")),
                ("observed_facet", string("debug-prompt-input-records")),
                ("messages", array(summaries)),
                ("provider_wire_payload", Value::Bool(false)),
            ]))
        }
        "grok-build" => {
            let entries = value
                .get("projectInstructions")
                .and_then(Value::as_array)
                .ok_or("native.shape_mismatch")?;
            let mut summaries = Vec::new();
            for item in entries {
                let path = item
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or("native.shape_mismatch")?;
                let size = item
                    .get("sizeBytes")
                    .and_then(Value::as_i64)
                    .filter(|v| *v >= 0)
                    .ok_or("native.shape_mismatch")?;
                summaries.push(object([
                    ("path_digest", string(hmac_sha256_hex(key, path.as_bytes()))),
                    ("size_bytes", Value::Int(size)),
                ]));
            }
            Ok(object([
                ("normalization", string("grok-inspect-object-v1")),
                ("observed_facet", string("instruction-discovery-metadata")),
                ("instructions", array(summaries)),
                ("instruction_body_observed", Value::Bool(false)),
            ]))
        }
        _ => Err("native.adapter_unknown"),
    }
}

pub fn capture(args: &ProductArgs, key: &[u8]) -> Result<Value> {
    let (adapter, family, version, command) = match args.adapter.as_deref() {
        Some("codex-prompt-input-v1") => (
            "codex-prompt-input-v1",
            "codex",
            "0.147.0",
            vec!["debug".into(), "prompt-input".into()],
        ),
        Some("grok-inspect-v1") => (
            "grok-inspect-v1",
            "grok-build",
            "1.0.13",
            vec!["inspect".into(), "--json".into()],
        ),
        _ => return Err("native.adapter_unknown"),
    };
    if args.harness != family
        || args.version != version
        || args.surface != "cli"
        || args.os_lane != "macos-27-arm64"
    {
        return Err("native.coordinate_mismatch");
    }
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return Err("native.os_lane_unavailable");
    }
    let path = Path::new(args.profile.as_deref().ok_or("native.profile_required")?);
    let profile = crate::secure_sync::read_json(path).map_err(|_| "native.profile_invalid")?;
    if profile.get("schema").and_then(Value::as_str) != Some("ctxpect-native-tool-profile-v1") {
        return Err("native.profile_invalid");
    }
    let tool = tool_process::pinned_tool(&profile, "tool").map_err(|_| "native.tool_drift")?;
    let scratch = Scratch::new().map_err(|_| "native.profile_unavailable")?;
    let codex = scratch.0.join("codex");
    fs::create_dir(&codex).map_err(|_| "native.profile_unavailable")?;
    let env = vec![
        ("HOME".into(), scratch.0.to_string_lossy().into_owned()),
        ("CODEX_HOME".into(), codex.to_string_lossy().into_owned()),
        (
            "GROK_HOME".into(),
            scratch.0.join("grok").to_string_lossy().into_owned(),
        ),
        (
            "XDG_CONFIG_HOME".into(),
            scratch.0.join("config").to_string_lossy().into_owned(),
        ),
    ];
    let project = args.project.as_ref().ok_or("native.project_required")?;
    let actual = tool_process::run_with_env(
        &tool,
        &["--version".into()],
        &[],
        project,
        Duration::from_secs(10),
        &env,
    )
    .map_err(|_| "native.version_unavailable")?;
    let actual = std::str::from_utf8(&actual.stdout).map_err(|_| "native.version_unavailable")?;
    if actual.split_whitespace().nth(1) != Some(version) {
        return Err("native.version_drift");
    }
    let mut command = command;
    if family == "codex"
        && let Some(task) = &args.task
    {
        if ctxpect_doctor::contains_secret(task) {
            return Err("native.secret_in_task");
        }
        command.push(task.clone());
    }
    let os = tool_process::run(
        Path::new("/usr/bin/sw_vers"),
        &["-productVersion".into()],
        &[],
        project,
        Duration::from_secs(5),
    )
    .map_err(|_| "native.os_lane_unavailable")?;
    let os_version = std::str::from_utf8(&os.stdout)
        .map_err(|_| "native.os_lane_unavailable")?
        .trim();
    if os_version != "27.0" {
        return Err("native.os_lane_unavailable");
    }
    let build = tool_process::run(
        Path::new("/usr/bin/sw_vers"),
        &["-buildVersion".into()],
        &[],
        project,
        Duration::from_secs(5),
    )
    .map_err(|_| "native.os_lane_unavailable")?;
    let os_build = std::str::from_utf8(&build.stdout)
        .map_err(|_| "native.os_lane_unavailable")?
        .trim();
    let output =
        tool_process::run_with_env(&tool, &command, &[], project, Duration::from_secs(25), &env)
            .map_err(|_| "native.execution_failed")?;
    if output.timed_out {
        return Err("native.timeout");
    }
    if output.code != Some(0) {
        return Err("native.execution_failed");
    }
    let value = parse(std::str::from_utf8(&output.stdout).map_err(|_| "native.shape_mismatch")?)
        .map_err(|_| "native.shape_mismatch")?;
    let summary = summarize(family, &value, key)?;
    Ok(object([
        ("schema", string("ctxpect-native-observation-v1")),
        ("adapter", string(adapter)),
        ("harness", string(family)),
        ("version", string(version)),
        ("os_lane", string(&args.os_lane)),
        ("actual_os_version", string(os_version)),
        ("actual_os_build", string(os_build)),
        (
            "frozen_os_build_matches",
            Value::Bool(os_build == "26A5425a"),
        ),
        (
            "tool_digest",
            profile
                .pointer(&["tool", "sha256"])
                .cloned()
                .unwrap_or(Value::Null),
        ),
        ("captured_at", string(ctxpect_store::now_rfc3339())),
        ("exit_code", Value::Int(0)),
        ("native_executed", Value::Bool(true)),
        ("model_executed", Value::Bool(false)),
        ("coverage", string("partial-declared-surface")),
        ("profile", string("isolated-empty-user-profile")),
        (
            "raw_output_digest",
            string(hmac_sha256_hex(key, &output.stdout)),
        ),
        (
            "digest_basis",
            string("local-keyed-not-cross-device-comparable"),
        ),
        ("raw_body_saved", Value::Bool(false)),
        ("claims_upgraded", Value::Bool(false)),
        ("observation", summary),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctxpect_schema::canonical_json;
    #[test]
    fn real_array_shape_is_distinct_from_synthetic_messages_wrapper() {
        let value=parse(r#"[{"type":"message","role":"user","content":[{"type":"input_text","text":"private source body"}]}]"#).unwrap();
        let out = summarize("codex", &value, b"local-key").unwrap();
        assert!(!canonical_json(&out).contains("private source body"));
        assert_eq!(
            out.pointer(&["messages"])
                .and_then(Value::as_array)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            summarize("codex", &object([("messages", value)]), b"key").unwrap_err(),
            "native.shape_mismatch"
        );
        let grok = parse(
            r#"{"projectInstructions":[{"path":"/private/project/AGENTS.md","sizeBytes":7}]}"#,
        )
        .unwrap();
        let out = summarize("grok-build", &grok, b"key").unwrap();
        assert_eq!(
            out.get("instruction_body_observed"),
            Some(&Value::Bool(false))
        );
        assert!(!canonical_json(&out).contains("/private"));
    }
}
