//! Versioned command adapter: the external runner owns its sandbox and gates.
use crate::tool_process::{self, Scratch};
use ctxpect_effect::{ExperimentContract, RunsDocument};
use ctxpect_schema::{Value, array, canonical_json, object, parse, sha256_text, string};
use ctxpect_store::{Store, now_rfc3339};
use std::{
    collections::BTreeSet,
    fs,
    path::{Component, PathBuf},
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, &'static str>;
fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v.get(key)
        .and_then(Value::as_str)
        .ok_or("effect.runner_invalid")
}
fn number(v: &Value, key: &str) -> Result<i64> {
    v.get(key)
        .and_then(Value::as_i64)
        .ok_or("effect.runner_invalid")
}
pub struct Plan {
    pub request: Value,
    pub contract: ExperimentContract,
    pub digest: String,
    tool: PathBuf,
    timeout: u64,
    total: u64,
}
impl Plan {
    pub fn parse(request: Value) -> Result<Self> {
        if text(&request, "schema")? != "ctxpect-command-runner-v1" {
            return Err("effect.runner_invalid");
        }
        if ctxpect_doctor::contains_secret(&canonical_json(&request)) {
            return Err("effect.secret_in_input");
        }
        let contract =
            ExperimentContract::from_value(request.get("contract").ok_or("effect.runner_invalid")?)
                .map_err(|e| e.code)?;
        if contract.pairing != "paired-by-task" {
            return Err("effect.runner_pairing_unsupported");
        }
        let tasks = request
            .get("tasks")
            .and_then(Value::as_array)
            .ok_or("effect.runner_invalid")?;
        if tasks.len() as i64 != contract.n_planned || tasks.len() > 256 {
            return Err("effect.n_locked");
        }
        let mut ids = BTreeSet::new();
        for task in tasks {
            let id = text(task, "id")?;
            if id.is_empty() || !ids.insert(id) || task.as_object().is_none_or(|m| m.len() != 3) {
                return Err("effect.runner_tasks_invalid");
            }
            text(task, "control")?;
            text(task, "treatment")?;
        }
        let files = request
            .get("files")
            .and_then(Value::as_object)
            .ok_or("effect.runner_invalid")?;
        for (name, body) in files {
            if name.is_empty()
                || name.contains('\\')
                || PathBuf::from(name)
                    .components()
                    .any(|c| !matches!(c, Component::Normal(_)))
                || name.split('/').any(|s| s == ".git" || s == ".ctxpect")
                || body.as_str().is_none()
            {
                return Err("effect.runner_path_invalid");
            }
        }
        let code_digest = sha256_text(&canonical_json(
            request.get("files").ok_or("effect.runner_invalid")?,
        ));
        if code_digest != contract.locked.code_digest {
            return Err("effect.confounder_drift");
        }
        let runner = request.get("runner").ok_or("effect.runner_invalid")?;
        if text(runner, "version")? != "ctxpect-command-runner-v1" {
            return Err("effect.runner_version_unknown");
        }
        let tool =
            tool_process::pinned_tool(&request, "runner").map_err(|_| "effect.runner_drift")?;
        if text(runner, "sha256")? != contract.locked.tool_availability_digest {
            return Err("effect.confounder_drift");
        }
        let budget = request.get("budget").ok_or("effect.runner_invalid")?;
        if number(budget, "max_runs")? != 2 * contract.n_planned {
            return Err("effect.budget_invalid");
        }
        let timeout = number(budget, "timeout_seconds")?;
        let total = number(budget, "total_seconds")?;
        if !(1..=300).contains(&timeout) || !(1..=3600).contains(&total) {
            return Err("effect.budget_invalid");
        }
        if !matches!(
            text(runner, "sandbox")?,
            "external-runner" | "none-explicit"
        ) {
            return Err("effect.sandbox_unspecified");
        }
        let digest = sha256_text(&canonical_json(&request));
        Ok(Self {
            request,
            contract,
            digest,
            tool,
            timeout: timeout as u64,
            total: total as u64,
        })
    }
}

/// Called only while holding mutation authorization. Reusing an id never
/// starts another call, including after an interrupted invocation.
pub fn execute(plan: &Plan, store: &Store) -> Result<Value> {
    let id = &plan.contract.experiment_id;
    let ids = store
        .list_named("runnerjobs")
        .map_err(|_| "effect.job_unreadable")?;
    if ids.contains(id) {
        return Err("effect.execution_exists");
    }
    if ctxpect_effect::epoch_seconds(&plan.contract.frozen_at)
        .is_none_or(|t| t > crate::dispatch::now_unix())
    {
        return Err("effect.contract_not_frozen");
    }
    let mut job = object([
        ("request_digest", string(&plan.digest)),
        ("contract", plan.contract.to_value()),
        ("state", string("running")),
        ("started_at", string(now_rfc3339())),
        ("attempts", Value::Int(0)),
        ("runs", array([])),
        ("executor_receipts", array([])),
    ]);
    // Freeze before the first process starts, including the full budget and
    // treatment/order specification by digest. An interrupted job is retained.
    store
        .put_named("runnerjobs", id, &job)
        .map_err(|_| "effect.freeze_failed")?;
    let started = Instant::now();
    let mut runs = Vec::new();
    let mut receipts = Vec::new();
    let mut attempts = 0;
    let mut budget_exhausted = false;
    let mut protocol_invalid = false;
    let tasks = plan
        .request
        .get("tasks")
        .and_then(Value::as_array)
        .ok_or("effect.runner_invalid")?;
    for (index, task) in tasks.iter().enumerate() {
        // Counterbalance order deterministically; its procedure is part of v1.
        let order = if index % 2 == 0 {
            ["control", "treatment"]
        } else {
            ["treatment", "control"]
        };
        for arm in order {
            let run_id = format!("run-{index}-{arm}");
            let run_started = now_rfc3339();
            let mut outcome = "missing";
            let mut actual = plan.contract.locked.clone();
            let mut receipt = object([
                ("run_id", string(&run_id)),
                ("launched", Value::Bool(false)),
                ("reason", string("budget-exhausted")),
            ]);
            if started.elapsed().as_secs() < plan.total {
                let scratch = Scratch::new().map_err(|_| "effect.workdir_failed")?;
                for (name, body) in plan
                    .request
                    .get("files")
                    .and_then(Value::as_object)
                    .ok_or("effect.runner_invalid")?
                {
                    let path = scratch.0.join(name);
                    if let Some(parent) = path.parent() {
                        fs::create_dir_all(parent).map_err(|_| "effect.workdir_failed")?;
                    }
                    tool_process::private_write(
                        &path,
                        body.as_str().ok_or("effect.runner_invalid")?.as_bytes(),
                    )
                    .map_err(|_| "effect.workdir_failed")?;
                }
                // Recheck the executable pin immediately before each attempt.
                tool_process::pinned_tool(&plan.request, "runner")
                    .map_err(|_| "effect.runner_drift")?;
                attempts += 1;
                if let Value::Object(m) = &mut job {
                    m.insert("attempts".into(), Value::Int(attempts));
                    m.insert("active_run".into(), string(&run_id));
                }
                store
                    .put_named("runnerjobs", id, &job)
                    .map_err(|_| "effect.checkpoint_failed")?;
                let request = object([
                    ("schema", string("ctxpect-runner-input-v1")),
                    ("run_id", string(&run_id)),
                    (
                        "task_id",
                        task.get("id").cloned().ok_or("effect.runner_invalid")?,
                    ),
                    ("arm", string(arm)),
                    (
                        "input",
                        task.get(arm).cloned().ok_or("effect.runner_invalid")?,
                    ),
                    ("contract", plan.contract.to_value()),
                    ("request_digest", string(&plan.digest)),
                ]);
                let remaining = Duration::from_secs(plan.total).saturating_sub(started.elapsed());
                let output = tool_process::run(
                    &plan.tool,
                    &[],
                    canonical_json(&request).as_bytes(),
                    &scratch.0,
                    Duration::from_secs(plan.timeout).min(remaining),
                );
                let mut result = Value::Null;
                let mut output_digest = Value::Null;
                let mut exit = Value::Null;
                match output {
                    Ok(out) => {
                        output_digest = string(out.stdout_digest);
                        exit = out
                            .code
                            .map(|c| Value::Int(i64::from(c)))
                            .unwrap_or(Value::Null);
                        outcome = if out.timed_out {
                            "timeout"
                        } else if out.code != Some(0) {
                            "crash"
                        } else {
                            "refusal"
                        };
                        if !out.timed_out && out.code == Some(0) {
                            result = std::str::from_utf8(&out.stdout)
                                .ok()
                                .and_then(|s| parse(s).ok())
                                .unwrap_or(Value::Null);
                            if result.get("schema").and_then(Value::as_str)
                                == Some("ctxpect-runner-result-v1")
                                && result.get("run_id").and_then(Value::as_str) == Some(&run_id)
                                && result.get("request_digest").and_then(Value::as_str)
                                    == Some(&plan.digest)
                                && result.get("sandbox")
                                    == plan.request.pointer(&["runner", "sandbox"])
                            {
                                outcome = match result.get("outcome").and_then(Value::as_str) {
                                    Some("pass") => "pass",
                                    Some("fail") => "fail",
                                    Some("timeout") => "timeout",
                                    Some("crash") => "crash",
                                    Some("refusal") => "refusal",
                                    _ => {
                                        protocol_invalid = true;
                                        "refusal"
                                    }
                                };
                                // Missing or changed observed locks remain deviations, never
                                // silently replaced with the requested confounders.
                                let lock = result.get("observed").unwrap_or(&Value::Null);
                                actual.code_digest = lock
                                    .get("code_digest")
                                    .and_then(Value::as_str)
                                    .unwrap_or("unobserved")
                                    .into();
                                actual.model = lock
                                    .get("model")
                                    .and_then(Value::as_str)
                                    .unwrap_or("unobserved")
                                    .into();
                                actual.harness = lock
                                    .get("harness")
                                    .and_then(Value::as_str)
                                    .unwrap_or("unobserved")
                                    .into();
                                actual.tool_availability_digest = lock
                                    .get("tool_availability_digest")
                                    .and_then(Value::as_str)
                                    .unwrap_or("unobserved")
                                    .into();
                            } else {
                                protocol_invalid = true;
                            }
                        }
                    }
                    Err(_) => outcome = "crash",
                }
                let gate_digest = result
                    .get("gate_evidence_digest")
                    .and_then(Value::as_str)
                    .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()));
                if matches!(outcome, "pass" | "fail") && gate_digest.is_none() {
                    protocol_invalid = true;
                    outcome = "refusal";
                }
                receipt = object([
                    ("run_id", string(&run_id)),
                    ("launched", Value::Bool(true)),
                    ("stdout_digest", output_digest),
                    ("process_exit", exit),
                    (
                        "runner_digest",
                        string(text(
                            plan.request.get("runner").ok_or("effect.runner_invalid")?,
                            "sha256",
                        )?),
                    ),
                    (
                        "sandbox",
                        plan.request
                            .pointer(&["runner", "sandbox"])
                            .cloned()
                            .unwrap_or(Value::Null),
                    ),
                    (
                        "sandbox_evidence_kind",
                        string("runner-attested-not-host-verified"),
                    ),
                    (
                        "gate_evidence_digest",
                        gate_digest.map(string).unwrap_or(Value::Null),
                    ),
                    ("raw_response_saved", Value::Bool(false)),
                ]);
            }
            if outcome == "missing" {
                budget_exhausted = true;
            }
            let receipt_id = sha256_text(&canonical_json(&receipt));
            runs.push(object([
                ("run_id", string(&run_id)),
                (
                    "task_id",
                    task.get("id").cloned().ok_or("effect.runner_invalid")?,
                ),
                ("arm", string(arm)),
                ("outcome", string(outcome)),
                ("started_at", string(run_started)),
                ("ended_at", string(now_rfc3339())),
                ("code_digest", string(actual.code_digest)),
                ("model", string(actual.model)),
                ("harness", string(actual.harness)),
                (
                    "tool_availability_digest",
                    string(actual.tool_availability_digest),
                ),
                ("receipt_id", string(receipt_id)),
            ]));
            receipts.push(receipt);
            if let Value::Object(m) = &mut job {
                m.insert("runs".into(), array(runs.clone()));
                m.insert("executor_receipts".into(), array(receipts.clone()));
                m.insert("active_run".into(), Value::Null);
            }
            store
                .put_named("runnerjobs", id, &job)
                .map_err(|_| "effect.checkpoint_failed")?;
        }
    }
    let document = object([
        ("schema", string(ctxpect_effect::RUNS_SCHEMA)),
        ("contract", plan.contract.to_value()),
        ("runs", array(runs)),
    ]);
    let parsed = RunsDocument::from_value(&document).map_err(|e| e.code)?;
    let mut result = ctxpect_effect::decide(&parsed);
    if let Value::Object(m) = &mut result {
        m.insert("runner_adapter".into(), string("command-v1"));
        m.insert("request_digest".into(), string(&plan.digest));
        m.insert("executor_receipts".into(), array(receipts));
        m.insert("process_attempts".into(), Value::Int(attempts));
        m.insert("sandbox_host_verified".into(), Value::Bool(false));
        if protocol_invalid {
            m.insert("decision".into(), string("inconclusive"));
            m.insert(
                "reason_code".into(),
                string("effect.runner_protocol_invalid"),
            );
        }
        if budget_exhausted {
            m.insert("decision".into(), string("inconclusive"));
            m.insert("reason_code".into(), string("effect.budget_exhausted"));
        }
    }
    store
        .put_named("experiments", id, &result)
        .map_err(|_| "effect.result_persist_failed")?;
    if let Value::Object(m) = &mut job {
        m.insert("state".into(), string("completed"));
        m.insert("ended_at".into(), string(now_rfc3339()));
    }
    store
        .put_named("runnerjobs", id, &job)
        .map_err(|_| "effect.checkpoint_failed")?;
    Ok(result)
}
