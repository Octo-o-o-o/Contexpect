//! Actual pinned oracle invocation. Metadata summaries are not automatic Claims.
use crate::{
    args::ProductArgs,
    tool_process::{self, Scratch},
};
use ctxpect_schema::{Value, array, hmac_sha256_hex, object, parse, string};
use std::{fs, path::Path, time::Duration};

type Result<T> = std::result::Result<T, &'static str>;

pub(crate) fn summarize(family: &str, value: &Value, key: &[u8]) -> Result<Value> {
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

/// Explicit one-shot development entry; outside every passive scan path.
pub(crate) mod integration {
    // Versioned one-shot integration. Receipt authority stays in the engine.
    use crate::{
        InspectArgs, dispatch, inspect,
        tool_process::{self, Scratch},
    };
    use ctxpect_schema::{
        Value, array, canonical_json, digest_value, object, parse, sha256_hex, string,
    };
    use std::{
        fs,
        io::{self, Read},
        path::{Path, PathBuf},
        time::Duration,
    };

    type Result<T> = std::result::Result<T, &'static str>;
    const PROFILE: &str = "isolated-default-trusted-instructions-v1";
    const RESOLVER: &str = "codex-0.153.3-default-instructions-v1";
    fn field<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
        v.get(k)
            .and_then(Value::as_str)
            .ok_or("integration.field_required")
    }
    fn hash_file(p: &Path) -> Result<String> {
        let mut f = fs::File::open(p).map_err(|_| "integration.file_unavailable")?;
        let mut h = ctxpect_schema::Hasher::new();
        let mut b = [0; 65536];
        loop {
            let n = f.read(&mut b).map_err(|_| "integration.file_unavailable")?;
            if n == 0 {
                break;
            }
            h.update(&b[..n]);
        }
        Ok(h.finish())
    }
    fn capabilities() -> Value {
        object([
            ("schema_major", Value::Int(1)),
            ("profile", string(PROFILE)),
            ("resolver", string(RESOLVER)),
            (
                "coordinates",
                array(ctxpect_resolve::DEVELOPMENT.iter().map(|a| {
                    object([
                        ("harness", string(a.harness)),
                        ("version", string(a.version)),
                        ("surface", string(a.surface)),
                        ("os_lane", string(a.os_lane)),
                        (
                            "capabilities",
                            array(a.capabilities.iter().map(|c| string(*c))),
                        ),
                        ("lane", string("development")),
                    ])
                })),
            ),
        ])
    }
    fn named_manifest(root: &Path, cwd: &Path) -> Result<Value> {
        let rel = cwd
            .strip_prefix(root)
            .map_err(|_| "integration.cwd_escape")?;
        let mut paths = vec![".ctxpect-ignore".to_string()];
        for layer in ctxpect_resolve::layers_toward_cwd(&rel.to_string_lossy()) {
            for name in ["AGENTS.override.md", "AGENTS.md", ".git"] {
                paths.push(ctxpect_resolve::layer_file(&layer, name));
            }
        }
        let mut entries = Vec::new();
        for rel in paths {
            let p = root.join(&rel);
            let state = match fs::symlink_metadata(&p) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => "missing".into(),
                Err(_) => return Err("integration.input_unreadable"),
                Ok(m) if m.file_type().is_symlink() => return Err("integration.symlink_refused"),
                Ok(_) if rel == ".git" || rel.ends_with("/.git") => "marker".into(),
                Ok(m) if m.is_file() => {
                    let canonical = p
                        .canonicalize()
                        .map_err(|_| "integration.input_unreadable")?;
                    if !canonical.starts_with(root) {
                        return Err("integration.input_escape");
                    }
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::MetadataExt;
                        if m.nlink() != 1 {
                            return Err("integration.hardlink_refused");
                        }
                    }
                    if m.len() > 1024 * 1024 {
                        return Err("integration.input_too_large");
                    }
                    hash_file(&p)?
                }
                _ => return Err("integration.input_not_regular"),
            };
            entries.push(object([("path", string(rel)), ("state", string(state))]));
        }
        Ok(array(entries))
    }
    fn native(
        request: &Value,
        project: &Path,
        cwd: &Path,
        manifest: &Value,
        key: &[u8],
    ) -> Result<Value> {
        let tool =
            tool_process::pinned_tool(request, "harness_tool").map_err(|_| "native.tool_drift")?;
        if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            return Err("native.os_unavailable");
        }
        let scratch = Scratch::new().map_err(|_| "native.scratch")?;
        let mirror = scratch.0.join("project");
        let home = scratch.0.join("codex");
        fs::create_dir_all(&mirror).map_err(|_| "native.scratch")?;
        fs::create_dir(&home).map_err(|_| "native.scratch")?;
        for entry in manifest.as_array().ok_or("native.manifest")? {
            let rel = field(entry, "path")?;
            let state = field(entry, "state")?;
            if state == "missing" || rel == ".ctxpect-ignore" {
                continue;
            }
            let dst = mirror.join(rel);
            if state == "marker" {
                fs::create_dir_all(&dst).map_err(|_| "native.scratch")?;
            } else {
                let bytes =
                    tool_process::read_small(&project.join(rel)).map_err(|_| "native.input")?;
                if sha256_hex(&bytes) != state {
                    return Err("integration.unstable");
                }
                fs::create_dir_all(dst.parent().ok_or("native.scratch")?)
                    .map_err(|_| "native.scratch")?;
                tool_process::private_write(&dst, &bytes).map_err(|_| "native.scratch")?;
            }
        }
        let working = mirror.join(cwd.strip_prefix(project).map_err(|_| "native.cwd")?);
        fs::create_dir_all(&working).map_err(|_| "native.scratch")?;
        let environment = vec![
            ("HOME".into(), scratch.0.to_string_lossy().into_owned()),
            ("CODEX_HOME".into(), home.to_string_lossy().into_owned()),
            (
                "XDG_CONFIG_HOME".into(),
                scratch.0.join("config").to_string_lossy().into_owned(),
            ),
        ];
        let version = tool_process::run_with_env(
            &tool,
            &["--version".into()],
            &[],
            &mirror,
            Duration::from_secs(10),
            &environment,
        )
        .map_err(|_| "native.version")?;
        if version.code != Some(0)
            || std::str::from_utf8(&version.stdout).ok().map(str::trim) != Some("codex-cli 0.153.3")
        {
            return Err("native.version_drift");
        }
        // The OS sandbox blocks network even if a future diagnostic implementation tries it.
        let command = vec![
            "-p".into(),
            "(version 1)(allow default)(deny network*)".into(),
            tool.to_string_lossy().into_owned(),
            "-c".into(),
            format!(
                "projects.{}.trust_level=\"trusted\"",
                canonical_json(&string(mirror.to_string_lossy()))
            ),
            "-c".into(),
            "features.bundled_skills=false".into(),
            "debug".into(),
            "prompt-input".into(),
        ];
        let out = tool_process::run_with_env(
            Path::new("/usr/bin/sandbox-exec"),
            &command,
            &[],
            &working,
            Duration::from_secs(25),
            &environment,
        )
        .map_err(|_| "native.execution_failed")?;
        if out.timed_out {
            return Err("native.timeout");
        }
        if out.code != Some(0) {
            return Err("native.execution_failed");
        }
        let value = ctxpect_schema::parse_preserving_numbers(
            std::str::from_utf8(&out.stdout).map_err(|_| "native.utf8")?,
        )
        .map_err(|_| "native.json_parse")?;
        let summary = crate::native_oracle::summarize("codex", &value, key)?;
        let os = tool_process::run(
            Path::new("/usr/bin/sw_vers"),
            &["-productVersion".into()],
            &[],
            &mirror,
            Duration::from_secs(5),
        )
        .map_err(|_| "native.os_unavailable")?;
        let build = tool_process::run(
            Path::new("/usr/bin/sw_vers"),
            &["-buildVersion".into()],
            &[],
            &mirror,
            Duration::from_secs(5),
        )
        .map_err(|_| "native.os_unavailable")?;
        Ok(object([
            (
                "actual_os_version",
                string(String::from_utf8_lossy(&os.stdout).trim()),
            ),
            (
                "actual_os_build",
                string(String::from_utf8_lossy(&build.stdout).trim()),
            ),
            ("actual_tool_version", string("0.153.3")),
            (
                "tool_sha256",
                request
                    .pointer(&["harness_tool", "sha256"])
                    .cloned()
                    .unwrap_or(Value::Null),
            ),
            ("status", string("observed")),
            ("profile", string(PROFILE)),
            ("source", string("native-runtime")),
            (
                "scope",
                string("mirrored-instructions-debug-construction-only"),
            ),
            ("model_executed", Value::Bool(false)),
            ("provider_wire_payload", Value::Bool(false)),
            ("claims_upgraded", Value::Bool(false)),
            ("raw_body_saved", Value::Bool(false)),
            (
                "raw_output_hmac",
                string(ctxpect_schema::hmac_sha256_hex(key, &out.stdout)),
            ),
            (
                "digest_basis",
                string("local-keyed-not-cross-device-comparable"),
            ),
            ("exit_code", Value::Int(0)),
            ("observation", summary),
        ]))
    }
    fn run_request(r: &Value) -> Result<Value> {
        if r.get("schema_major").and_then(Value::as_i64) != Some(1) {
            return Err("integration.schema_major");
        }
        let action = field(r, "action")?;
        if action == "capabilities" {
            return Ok(capabilities());
        }
        if action != "inspect" {
            return Err("integration.action");
        }
        for k in ["request_id", "run_id", "project_id"] {
            let s = field(r, k)?;
            if s.is_empty()
                || s.len() > 128
                || !s
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"-_.:".contains(&c))
            {
                return Err("integration.identity");
            }
        }
        if field(r, "profile")? != PROFILE {
            return Err("integration.profile");
        }
        let project = PathBuf::from(field(r, "project")?)
            .canonicalize()
            .map_err(|_| "integration.project")?;
        if Path::new(field(r, "project")?) != project {
            return Err("integration.project_not_canonical");
        }
        let cwd = project
            .join(field(r, "cwd")?)
            .canonicalize()
            .map_err(|_| "integration.cwd")?;
        if !project.is_dir() || !cwd.starts_with(&project) || !cwd.is_dir() {
            return Err("integration.cwd_escape");
        }
        let store_path = PathBuf::from(field(r, "store")?);
        if !store_path.is_absolute()
            || store_path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err("integration.store");
        }
        let existing = store_path
            .ancestors()
            .find(|p| p.exists())
            .ok_or("integration.store")?;
        if existing
            .canonicalize()
            .map_err(|_| "integration.store")?
            .starts_with(&project)
            || store_path.is_symlink()
        {
            return Err("integration.store_in_project");
        }
        let version = field(r, "version")?;
        let lane = field(r, "os_lane")?;
        let engine = std::env::current_exe().map_err(|_| "integration.engine")?;
        let engine_digest = hash_file(&engine)?;
        let tool = tool_process::pinned_tool(r, "harness_tool")
            .map_err(|_| "integration.harness_drift")?;
        let harness_digest = hash_file(&tool)?;
        let scratch = Scratch::new().map_err(|_| "integration.profile")?;
        let store = ctxpect_store::Store::open(&store_path).map_err(|_| "integration.store")?;
        let observed_at = ctxpect_store::now_rfc3339();
        for attempt in 0..2 {
            let before = named_manifest(&project, &cwd)?;
            let report = inspect(InspectArgs {
                json: true,
                offline: true,
                project: project.clone(),
                cwd: Some(cwd.clone()),
                harness: "codex".into(),
                surface: "cli".into(),
                version: version.into(),
                version_explicit: true,
                codex_home: Some(scratch.0.clone()),
                require: vec!["instructions".into()],
                os_lane: lane.into(),
                store: None,
            })
            .map_err(|_| "integration.inspect")?;
            let key = store.continuity_key().map_err(|_| "integration.store")?;
            let native_result = if r.get("native").and_then(Value::as_bool) == Some(true)
                && version == "0.153.3"
                && lane == "macos-27-arm64"
            {
                match native(r, &project, &cwd, &before, &key.secret) {
                    Ok(v) => v,
                    Err(code) => object([("status", string("Unknown")), ("reason", string(code))]),
                }
            } else {
                object([
                    ("status", string("Unknown")),
                    ("reason", string("runtime_snapshot_missing")),
                ])
            };
            let after = named_manifest(&project, &cwd)?;
            if before != after || hash_file(&tool)? != harness_digest {
                if attempt == 0 {
                    continue;
                }
                return Err("integration.unstable");
            }
            let receipt =
                dispatch::persist_inspect_in(&store, &report.envelope, "one-shot", Some(&project))
                    .map_err(|_| "integration.receipt")?;
            ctxpect_receipt::verify_local_continuity(&receipt, &key)
                .map_err(|_| "integration.integrity_failed")?;
            let receipt_id = field(&receipt, "receipt_id")?;
            let stored_receipt = store_path
                .join("receipts")
                .join(format!("{receipt_id}.json"));
            return Ok(object([
                ("schema_major", Value::Int(1)),
                ("request_id", string(field(r, "request_id")?)),
                ("run_id", string(field(r, "run_id")?)),
                ("project_id", string(field(r, "project_id")?)),
                (
                    "process",
                    object([
                        ("status", string("completed")),
                        ("exit_code", Value::Int(0)),
                    ]),
                ),
                ("diagnostic", report.envelope),
                ("diagnostic_exit", Value::Int(i64::from(report.exit_code))),
                ("profile", string(PROFILE)),
                ("resolver", string(RESOLVER)),
                ("engine_sha256", string(engine_digest)),
                ("harness_sha256", string(harness_digest)),
                ("version", string(version)),
                ("os_lane", string(lane)),
                ("cwd", string(field(r, "cwd")?)),
                ("input_manifest", before.clone()),
                ("input_digest", string(digest_value(&before))),
                ("native", native_result),
                (
                    "formal_receipt",
                    object([
                        ("id", string(receipt_id)),
                        ("sha256", string(hash_file(&stored_receipt)?)),
                        ("verification", string("local-continuity")),
                        ("org_identity", Value::Bool(false)),
                        (
                            "created_at",
                            receipt.get("created_at").cloned().unwrap_or(Value::Null),
                        ),
                        ("document", receipt),
                    ]),
                ),
                ("observed_at", string(observed_at)),
                ("persisted_at", string(ctxpect_store::now_rfc3339())),
                ("attempts", Value::Int(attempt + 1)),
                ("capabilities", capabilities()),
            ]));
        }
        Err("integration.unstable")
    }
    pub fn run() -> i32 {
        let mut raw = String::new();
        let result = io::stdin()
            .take(65537)
            .read_to_string(&mut raw)
            .map_err(|_| "integration.input")
            .and_then(|_| {
                if raw.len() > 65536 {
                    return Err("integration.input_too_large");
                }
                let request = parse(&raw).map_err(|_| "integration.bad_json")?;
                run_request(&request)
            });
        match result {
            Ok(v) => {
                println!("{}", canonical_json(&v));
                0
            }
            Err(code) => {
                println!(
                    "{}",
                    canonical_json(&object([
                        ("schema_major", Value::Int(1)),
                        ("error", string(code))
                    ]))
                );
                1
            }
        }
    }
}
