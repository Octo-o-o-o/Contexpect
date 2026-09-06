//! Drive one `inspect` against declared roots and assemble the report.

use crate::args::{InspectArgs, UsageError};
use crate::jsonutil::{arr, obj, opt_s, s};
use ctxpect_core::{Claim, LifecycleStage, TruthState, UnknownReason};
use ctxpect_fs::{Refusal, Root};
use ctxpect_resolve::{
    ANCHOR_VERSION, Edge, EdgeKind, Evidence, FacetClaims, INSTRUCTIONS, PROJECT_DOC_MAX_BYTES,
    Resolution, ResolveError, ResolveRequest, RootKind, coordinate_unknown_reason, honesty_claims,
    resolve, unsupported_capability_reason,
};
use ctxpect_schema::Value;
use std::path::Path;

pub const SCHEMA: &str = "dev-inspect-v0";
pub const RECEIPT_KIND: &str = "development-snapshot";

pub struct InspectReport {
    pub exit_code: i32,
    pub json: bool,
    pub envelope: Value,
    pub human: String,
}

pub enum InspectFailure {
    Usage(UsageError),
    Io { code: &'static str, message: String },
    Internal { message: String },
}

impl InspectFailure {
    pub fn code(&self) -> &'static str {
        match self {
            InspectFailure::Usage(err) => err.code,
            InspectFailure::Io { code, .. } => code,
            InspectFailure::Internal { .. } => "internal",
        }
    }

    pub fn message(&self) -> String {
        match self {
            InspectFailure::Usage(err) => err.message.clone(),
            InspectFailure::Io { message, .. } | InspectFailure::Internal { message } => {
                message.clone()
            }
        }
    }

    pub fn command(&self) -> Option<&str> {
        match self {
            InspectFailure::Usage(err) => err.command.as_deref(),
            _ => Some("inspect"),
        }
    }
}

struct CapResult {
    capability_id: String,
    included: Option<bool>,
    native_paths_used: Vec<String>,
    truth_state: TruthState,
    unknown_reason_code: Option<String>,
    claims: FacetClaims,
    parse: Value,
    edges: Vec<Value>,
    layers: Vec<Value>,
    evidence: Vec<Value>,
    loss_report_required: bool,
}

pub fn inspect(args: InspectArgs) -> Result<InspectReport, InspectFailure> {
    let project =
        Root::new(&args.project).map_err(|refusal| io_from_refusal(&refusal, "<project>"))?;
    let cwd_input = args
        .cwd
        .clone()
        .unwrap_or_else(|| project.path().to_path_buf());
    let cwd_resolved = match project.contain(&cwd_input) {
        Ok(path) => path,
        Err(Refusal::EscapesRoot { .. }) => {
            return Err(InspectFailure::Io {
                code: "io.cwd_outside_project",
                message: "`<cwd>` is outside `<project>`".to_string(),
            });
        }
        Err(refusal) => {
            return Err(io_from_refusal(&refusal, "<cwd>"));
        }
    };
    if !cwd_resolved.is_dir() {
        return Err(InspectFailure::Usage(UsageError {
            code: "usage.invalid",
            message: "`--cwd` must be a directory inside `--project`".to_string(),
            command: Some("inspect".to_string()),
        }));
    }
    let cwd_rel = rel_display(project.path(), &cwd_resolved);

    let mut home_root = None;
    if let Some(path) = &args.codex_home {
        home_root =
            Some(Root::new(path).map_err(|refusal| io_from_refusal(&refusal, "<codex-home>"))?);
    }

    let coord_reason =
        coordinate_unknown_reason(&args.harness, &args.version, &args.surface, &args.os_lane);

    let mut resolution: Option<Resolution> = None;
    if coord_reason.is_none() && args.require.iter().any(|item| item == INSTRUCTIONS) {
        resolution = Some(
            resolve(&ResolveRequest {
                project: &project,
                cwd_rel: &cwd_rel,
                codex_home: home_root.as_ref(),
                project_doc_max_bytes: PROJECT_DOC_MAX_BYTES,
            })
            .map_err(map_resolve)?,
        );
    }

    let mut cap_results = Vec::new();
    for capability in &args.require {
        cap_results.push(result_for(
            capability,
            coord_reason,
            resolution.as_ref(),
            &args,
        ));
    }

    let required_truths: Vec<(String, TruthState)> = cap_results
        .iter()
        .map(|item| (item.capability_id.clone(), item.truth_state))
        .collect();
    let exit_code = policy_exit(&required_truths);
    let verdict = match exit_code {
        0 => "pass",
        2 => "fail",
        _ => "indeterminate",
    };

    let (explanation, unknown, findings, assumptions) =
        supporting_fields(&cap_results, resolution.as_ref(), &args, coord_reason);
    let warning_values = ignore_warning_values(resolution.as_ref());

    let version_provenance = if args.version_explicit {
        "user-attested"
    } else {
        "official-spec"
    };
    let home_status = if args.codex_home.is_some() {
        "granted"
    } else {
        "not-granted"
    };

    let scope = obj([
        ("harness", s(&args.harness)),
        ("version", s(&args.version)),
        ("version_provenance", s(version_provenance)),
        ("surface", s(&args.surface)),
        ("os_lane", s(&args.os_lane)),
        (
            "roots",
            obj([
                ("project", s("<project>")),
                (
                    "codex_home",
                    if args.codex_home.is_some() {
                        s("<codex-home>")
                    } else {
                        s("not-granted")
                    },
                ),
            ]),
        ),
        (
            "cwd",
            s(if cwd_rel.is_empty() {
                "<project>/".to_string()
            } else {
                format!("<project>/{cwd_rel}")
            }),
        ),
        (
            "codex_home",
            obj([
                ("status", s(home_status)),
                (
                    "path",
                    if args.codex_home.is_some() {
                        s("<codex-home>")
                    } else {
                        Value::Null
                    },
                ),
                ("provenance", s("user-attested")),
            ]),
        ),
        ("offline", Value::Bool(args.offline)),
        ("anchor_version_default", s(ANCHOR_VERSION)),
    ]);

    let results_json: Vec<Value> = cap_results.iter().map(cap_to_json).collect();
    let envelope = obj([
        ("schema_version", Value::Int(1)),
        ("command", s("inspect")),
        ("exit_code", Value::Int(i64::from(exit_code))),
        ("receipt_kind", s(RECEIPT_KIND)),
        ("schema", s(SCHEMA)),
        ("scope", scope),
        (
            "required",
            arr(args.require.iter().map(|item| s(item.as_str()))),
        ),
        ("results", arr(results_json)),
        (
            "policy_result",
            obj([
                ("verdict", s(verdict)),
                ("exit_code", Value::Int(i64::from(exit_code))),
            ]),
        ),
        ("unknown", arr(unknown.clone())),
        ("findings", arr(findings.clone())),
        ("warnings", arr(warning_values.clone())),
        ("assumptions", arr(assumptions.clone())),
        ("explanation", arr(explanation.clone())),
    ]);

    let human = render_human(HumanRender {
        exit_code,
        args: &args,
        cwd_rel: &cwd_rel,
        results: &cap_results,
        explanation: &explanation,
        unknown: &unknown,
        findings: &findings,
        assumptions: &assumptions,
        warnings: &warning_values,
    });

    Ok(InspectReport {
        exit_code,
        json: args.json,
        envelope,
        human,
    })
}

fn result_for(
    capability: &str,
    coord_reason: Option<UnknownReason>,
    resolution: Option<&Resolution>,
    args: &InspectArgs,
) -> CapResult {
    if let Some(reason) = unsupported_capability_reason(capability) {
        return honesty_result(capability, reason, false);
    }
    if let Some(reason) = coord_reason {
        let unexposed = reason == UnknownReason::SurfaceNotExposed && args.surface != "cli";
        return honesty_result(capability, reason, unexposed);
    }
    let Some(resolution) = resolution else {
        return honesty_result(capability, UnknownReason::RuntimeSnapshotMissing, false);
    };
    let truth = resolution.claims.primary.truth_state;
    let included = match truth {
        TruthState::Present => Some(true),
        TruthState::Absent => Some(false),
        TruthState::Indeterminate | TruthState::NotApplicable => None,
    };
    CapResult {
        capability_id: capability.to_string(),
        included,
        native_paths_used: resolution.native_paths_used.clone(),
        truth_state: truth,
        unknown_reason_code: resolution
            .claims
            .primary
            .unknown_reason
            .map(|reason| reason.as_str().to_string()),
        claims: resolution.claims.clone(),
        parse: obj([
            ("activation", s("always")),
            (
                "included",
                match included {
                    Some(value) => Value::Bool(value),
                    None => Value::Null,
                },
            ),
            ("path", s(&resolution.parse_path)),
        ]),
        edges: resolution.edges.iter().map(edge_to_json).collect(),
        layers: resolution.layers.iter().map(layer_to_json).collect(),
        evidence: resolution.evidence.iter().map(evidence_to_json).collect(),
        loss_report_required: false,
    }
}

fn honesty_result(capability: &str, reason: UnknownReason, unexposed: bool) -> CapResult {
    let claims = honesty_claims(reason, unexposed);
    CapResult {
        capability_id: capability.to_string(),
        included: None,
        native_paths_used: Vec::new(),
        truth_state: TruthState::Indeterminate,
        unknown_reason_code: Some(reason.as_str().to_string()),
        claims,
        parse: obj([
            ("native_primitive_present", Value::Bool(false)),
            ("reason_code", s(reason.as_str())),
            ("truth_state", s("indeterminate")),
        ]),
        edges: Vec::new(),
        layers: Vec::new(),
        evidence: Vec::new(),
        loss_report_required: false,
    }
}

fn cap_to_json(result: &CapResult) -> Value {
    obj([
        ("capability_id", s(&result.capability_id)),
        (
            "included",
            match result.included {
                Some(value) => Value::Bool(value),
                None => Value::Null,
            },
        ),
        (
            "loss_report_required",
            Value::Bool(result.loss_report_required),
        ),
        (
            "native_paths_used",
            arr(result.native_paths_used.iter().map(|path| s(path.as_str()))),
        ),
        ("parse", result.parse.clone()),
        ("resolver", s("static")),
        ("truth_state", s(result.truth_state.as_str())),
        (
            "unknown_reason_code",
            opt_s(result.unknown_reason_code.as_deref()),
        ),
        ("claim", claim_to_json(&result.claims.primary)),
        (
            "facets",
            obj([
                ("installed", claim_to_json(&result.claims.installed)),
                ("discoverable", claim_to_json(&result.claims.discoverable)),
                ("eligible", claim_to_json(&result.claims.eligible)),
                ("model-visible", claim_to_json(&result.claims.model_visible)),
                ("use-evidence", claim_to_json(&result.claims.use_evidence)),
                (
                    "outcome-affecting",
                    claim_to_json(&result.claims.outcome_affecting),
                ),
            ]),
        ),
        ("edges", arr(result.edges.clone())),
        ("layers", arr(result.layers.clone())),
        ("evidence", arr(result.evidence.clone())),
    ])
}

fn evidence_to_json(item: &Evidence) -> Value {
    obj([
        ("root", s(item.root_kind.as_str())),
        ("path", s(&item.path)),
        ("content_digest", opt_s(item.content_digest.as_deref())),
    ])
}

fn claim_to_json(claim: &Claim) -> Value {
    obj([
        ("claim_kind", s(claim.claim_kind.as_str())),
        ("coverage", s(claim.coverage.as_str())),
        ("knowledge_status", s(claim.knowledge_status.as_str())),
        ("lifecycle_stage", s(claim.lifecycle_stage.as_str())),
        ("precision", s(claim.precision.as_str())),
        ("provenance", s(claim.provenance.as_str())),
        ("truth_state", s(claim.truth_state.as_str())),
        (
            "unknown_reason_code",
            match claim.unknown_reason {
                Some(reason) => s(reason.as_str()),
                None => Value::Null,
            },
        ),
    ])
}

fn edge_to_json(edge: &Edge) -> Value {
    obj([
        ("kind", s(edge.kind.as_str())),
        ("rule_id", s(edge.rule_id)),
        ("path", s(display_project_or_home(&edge.path, edge.rule_id))),
        ("related_path", opt_s(edge.related_path.as_deref())),
        (
            "offset",
            match edge.offset {
                Some(value) => Value::Int(i64::try_from(value).unwrap_or(i64::MAX)),
                None => Value::Null,
            },
        ),
        (
            "file_offset",
            match edge.file_offset {
                Some(value) => Value::Int(i64::try_from(value).unwrap_or(i64::MAX)),
                None => Value::Null,
            },
        ),
        ("note", opt_s(edge.note.as_deref())),
    ])
}

fn layer_rel_display(layer: &ctxpect_resolve::Layer) -> String {
    let prefix = match layer.root_kind {
        RootKind::CodexHome => "<codex-home>",
        RootKind::Project => "<project>",
    };
    if layer.rel.is_empty() {
        format!("{prefix}/")
    } else {
        format!("{prefix}/{}", layer.rel)
    }
}

fn layer_to_json(layer: &ctxpect_resolve::Layer) -> Value {
    obj([
        ("id", s(&layer.id)),
        ("rel", s(layer_rel_display(layer))),
        ("adopted", opt_s(layer.adopted.as_deref())),
        (
            "unknown_reason_code",
            opt_s(layer.unknown.map(|reason| reason.as_str())),
        ),
        (
            "candidates",
            arr(layer.candidates.iter().map(|candidate| {
                obj([
                    ("path", s(&candidate.path)),
                    ("name", s(&candidate.name)),
                    ("existed", Value::Bool(candidate.existed)),
                    ("adopted", Value::Bool(candidate.adopted)),
                ])
            })),
        ),
    ])
}

fn display_project_or_home(path: &str, rule_id: &str) -> String {
    if rule_id == "G5" && path == "AGENTS.md" {
        // Ambiguous: included global AGENTS.md is shown as <codex-home>/AGENTS.md
        // in explanation; edge path stays the inventory relative name. Callers
        // that need a prefix use explanation.
        return path.to_string();
    }
    path.to_string()
}

fn supporting_fields(
    results: &[CapResult],
    resolution: Option<&Resolution>,
    args: &InspectArgs,
    coord_reason: Option<UnknownReason>,
) -> (Vec<Value>, Vec<Value>, Vec<Value>, Vec<Value>) {
    let mut explanation = Vec::new();
    let mut unknown = Vec::new();
    let mut findings = Vec::new();

    if let Some(reason) = coord_reason {
        let why = coordinate_why(reason, args);
        explanation.push(obj([
            ("kind", s("unknown")),
            ("rule_id", s("G6")),
            ("path", Value::Null),
            ("unknown_reason_code", s(reason.as_str())),
            ("why", s(&why)),
            ("next_evidence", s(coordinate_next(reason))),
        ]));
        unknown.push(obj([
            ("reason_code", s(reason.as_str())),
            ("family", s(&args.harness)),
            ("capability", s(INSTRUCTIONS)),
            ("rule_id", s("G6")),
        ]));
    }

    if let Some(resolution) = resolution {
        for edge in &resolution.edges {
            match edge.kind {
                EdgeKind::IncludedBy => {
                    let shown = shown_path(edge);
                    explanation.push(obj([
                        ("kind", s("included")),
                        ("rule_id", s(edge.rule_id)),
                        ("path", s(&shown)),
                        (
                            "why",
                            s(format!(
                                "Codex instructions grammar adopted this file at its layer (activation always; rule {}).",
                                edge.rule_id
                            )),
                        ),
                        (
                            "next_evidence",
                            s("native runtime snapshot (codex debug prompt-input on Codex 0.147.0) to establish model-visible"),
                        ),
                    ]));
                }
                EdgeKind::ExcludedBy => {
                    let shown = format!("<project>/{}", edge.path);
                    explanation.push(obj([
                        ("kind", s("excluded")),
                        ("rule_id", s("G4")),
                        ("path", s(&shown)),
                        (
                            "by",
                            s(format!(
                                "<project>/{}",
                                edge.related_path.as_deref().unwrap_or(".ctxpect-ignore")
                            )),
                        ),
                        ("exclusion_class", s("product-user-exclusion")),
                        (
                            "why",
                            s("Listed in .ctxpect-ignore (exact relative path). This is a Contexpect product user exclusion, not a Codex native rule."),
                        ),
                        (
                            "next_evidence",
                            s("remove the ignore line if this file should be included"),
                        ),
                    ]));
                }
                EdgeKind::OverriddenBy | EdgeKind::ShadowedBy => {
                    let shown = format!("<project>/{}", edge.path);
                    let by = edge
                        .related_path
                        .as_deref()
                        .map(|path| format!("<project>/{path}"))
                        .unwrap_or_else(|| "<project>/AGENTS.override.md".to_string());
                    explanation.push(obj([
                        ("kind", s(edge.kind.as_str())),
                        ("rule_id", s("G2")),
                        ("path", s(&shown)),
                        ("by", s(&by)),
                        (
                            "why",
                            s(format!(
                                "Same-layer {} exists; this file was not adopted.",
                                edge.related_path.as_deref().unwrap_or("AGENTS.override.md")
                            )),
                        ),
                        ("next_evidence", s("none; this is a static grammar outcome")),
                    ]));
                }
                EdgeKind::TruncatedAfter => {
                    let shown = shown_path(edge);
                    explanation.push(obj([
                        ("kind", s("truncated")),
                        ("rule_id", s("G3")),
                        ("path", s(&shown)),
                        (
                            "offset",
                            Value::Int(
                                i64::try_from(edge.offset.unwrap_or(PROJECT_DOC_MAX_BYTES))
                                    .unwrap_or(i64::MAX),
                            ),
                        ),
                        (
                            "why",
                            s(format!(
                                "Aggregated project docs truncated at {} bytes (official-spec default project_doc_max_bytes; config.toml not read).",
                                PROJECT_DOC_MAX_BYTES
                            )),
                        ),
                        (
                            "next_evidence",
                            s("none for this slice; a config override of project_doc_max_bytes is not implemented"),
                        ),
                    ]));
                    findings.push(obj([
                        ("kind", s("truncated-after")),
                        ("rule_id", s("G3")),
                        ("path", s(&shown)),
                        (
                            "offset",
                            Value::Int(
                                i64::try_from(edge.offset.unwrap_or(PROJECT_DOC_MAX_BYTES))
                                    .unwrap_or(i64::MAX),
                            ),
                        ),
                    ]));
                }
                EdgeKind::UnknownBecause => {
                    let note = edge.note.as_deref().unwrap_or("permission_not_granted");
                    let permission = note == UnknownReason::PermissionNotGranted.as_str();
                    let shown = if permission {
                        "$CODEX_HOME/AGENTS.md".to_string()
                    } else {
                        shown_path(edge)
                    };
                    let reason_code = if permission {
                        UnknownReason::PermissionNotGranted.as_str()
                    } else {
                        UnknownReason::ContentRedactedByPolicy.as_str()
                    };
                    let (why, next) = unknown_because_copy(note);
                    explanation.push(obj([
                        ("kind", s("unknown")),
                        ("rule_id", s(edge.rule_id)),
                        ("path", s(&shown)),
                        ("unknown_reason_code", s(reason_code)),
                        ("why", s(why)),
                        ("next_evidence", s(next)),
                    ]));
                    unknown.push(obj([
                        ("reason_code", s(reason_code)),
                        (
                            "layer",
                            s(if edge.root_kind == RootKind::CodexHome {
                                "global"
                            } else {
                                "project"
                            }),
                        ),
                        ("rule_id", s(edge.rule_id)),
                        ("family", s(&args.harness)),
                    ]));
                }
            }
        }
    }

    for result in results {
        if result.capability_id != INSTRUCTIONS {
            let reason = result
                .unknown_reason_code
                .clone()
                .unwrap_or_else(|| UnknownReason::SurfaceNotExposed.as_str().to_string());
            explanation.push(obj([
                ("kind", s("unknown")),
                ("rule_id", s("G6")),
                ("path", Value::Null),
                ("unknown_reason_code", s(&reason)),
                (
                    "why",
                    s(format!(
                        "Capability `{}` is not resolved in this slice.",
                        result.capability_id
                    )),
                ),
                (
                    "next_evidence",
                    s("a later work package that implements this capability's grammar"),
                ),
            ]));
            unknown.push(obj([
                ("reason_code", s(&reason)),
                ("capability", s(&result.capability_id)),
                ("rule_id", s("G6")),
            ]));
        }
        if result.claims.model_visible.lifecycle_stage == LifecycleStage::ModelVisible {
            explanation.push(obj([
                ("kind", s("unknown")),
                ("rule_id", s("I04")),
                ("path", Value::Null),
                (
                    "unknown_reason_code",
                    s(UnknownReason::RuntimeSnapshotMissing.as_str()),
                ),
                (
                    "facet",
                    s("model-visible"),
                ),
                (
                    "why",
                    s("Static resolution cannot witness model-visible, use-evidence, or outcome-affecting facets."),
                ),
                (
                    "next_evidence",
                    s("native runtime snapshot on Codex 0.147.0"),
                ),
            ]));
        }
    }

    let mut assumptions = Vec::new();
    if let Some(resolution) = resolution {
        for item in &resolution.assumptions {
            assumptions.push(obj([
                ("key", s(&item.key)),
                ("value", s(&item.value)),
                ("provenance", s(item.provenance)),
            ]));
        }
    } else {
        assumptions.push(obj([
            ("key", s("coordinate")),
            (
                "value",
                s("this inspect did not apply the Codex instructions grammar"),
            ),
            ("provenance", s("official-spec")),
        ]));
    }
    assumptions.push(obj([
        ("key", s("version_coordinate")),
        (
            "value",
            s(if args.version_explicit {
                "user-supplied --version; not native-runtime evidence of installation"
            } else {
                "default 0.147.0 official-spec anchor; not native-runtime evidence of installation"
            }),
        ),
        (
            "provenance",
            s(if args.version_explicit {
                "user-attested"
            } else {
                "official-spec"
            }),
        ),
    ]));

    (explanation, unknown, findings, assumptions)
}

fn shown_path(edge: &Edge) -> String {
    match edge.root_kind {
        RootKind::CodexHome => format!("<codex-home>/{}", edge.path),
        RootKind::Project => format!("<project>/{}", edge.path),
    }
}

fn unknown_because_copy(note: &str) -> (&'static str, &'static str) {
    match note {
        "permission_not_granted" => (
            "Global Codex instructions were not read because no explicit --codex-home root was granted. HOME and CODEX_HOME are not consulted.",
            "pass --codex-home <dir> for an allowed root containing AGENTS.md",
        ),
        "multiply-linked" => (
            "The candidate exists as a regular file but collect withheld it (withheld=multiply-linked); this slice does not adopt multiply-linked AGENTS.md as present.",
            "replace the multiply-linked file with a single-link regular AGENTS.md inside the declared root",
        ),
        "escapes-root" => (
            "The candidate exists as a symlink whose target resolves outside the declared root; collect records it as a link and fs containment refuses to follow it.",
            "replace the symlink with a regular AGENTS.md whose target stays inside the declared root",
        ),
        "unresolvable" => (
            "The candidate exists as a dangling symlink; it is not absent, and its target cannot be resolved inside the root.",
            "replace the dangling symlink with a regular AGENTS.md inside the declared root",
        ),
        _ => (
            "The candidate exists but is not a regular readable file (collect withheld=not-regular); FIFO, socket, device, and symlink entries are not adopted as ordinary instructions.",
            "replace the non-regular entry with a regular AGENTS.md inside the declared root",
        ),
    }
}

fn coordinate_why(reason: UnknownReason, args: &InspectArgs) -> String {
    match reason {
        UnknownReason::OfficialDistributionNotCaptured => format!(
            "OS lane `{}` (harness `{}`) is not the captured macos-27-arm64 / cli distribution for this slice.",
            args.os_lane, args.harness
        ),
        UnknownReason::SurfaceNotExposed => format!(
            "Surface `{}` is not the captured cli surface for this slice.",
            args.surface
        ),
        UnknownReason::UnsupportedHarnessVersion => format!(
            "Version `{}` is not the frozen Codex 0.147.0 coordinate.",
            args.version
        ),
        other => format!("Coordinate cannot be resolved ({other})."),
    }
}

fn coordinate_next(reason: UnknownReason) -> &'static str {
    match reason {
        UnknownReason::OfficialDistributionNotCaptured => {
            "an official distribution capture for that OS lane / family"
        }
        UnknownReason::SurfaceNotExposed => "a later slice that implements that surface",
        UnknownReason::UnsupportedHarnessVersion => {
            "inspect with --version 0.147.0, or a later slice that supports that version"
        }
        _ => "additional declared evidence for this coordinate",
    }
}

fn policy_exit(required: &[(String, TruthState)]) -> i32 {
    let mut absent = false;
    let mut indeterminate = false;
    for (_, truth) in required {
        match truth {
            TruthState::Present => {}
            TruthState::Absent => absent = true,
            TruthState::Indeterminate | TruthState::NotApplicable => indeterminate = true,
        }
    }
    if indeterminate {
        3
    } else if absent {
        2
    } else {
        0
    }
}

struct HumanRender<'a> {
    exit_code: i32,
    args: &'a InspectArgs,
    cwd_rel: &'a str,
    results: &'a [CapResult],
    explanation: &'a [Value],
    unknown: &'a [Value],
    findings: &'a [Value],
    assumptions: &'a [Value],
    warnings: &'a [Value],
}

fn ignore_warning_values(resolution: Option<&Resolution>) -> Vec<Value> {
    let Some(resolution) = resolution else {
        return Vec::new();
    };
    resolution
        .ignore_warnings
        .iter()
        .map(|item| {
            obj([
                ("code", s(item.code)),
                (
                    "line",
                    Value::Int(i64::try_from(item.line).unwrap_or(i64::MAX)),
                ),
            ])
        })
        .collect()
}

fn render_human(input: HumanRender<'_>) -> String {
    let HumanRender {
        exit_code,
        args,
        cwd_rel,
        results,
        explanation,
        unknown,
        findings,
        assumptions,
        warnings,
    } = input;
    let mut out = String::new();
    out.push_str("ctxpect inspect — development snapshot (dev-inspect-v0)\n");
    out.push_str(&format!(
        "scope: {} {} / {} / {}\n",
        args.harness, args.version, args.surface, args.os_lane
    ));
    out.push_str("project: <project>\n");
    if cwd_rel.is_empty() {
        out.push_str("cwd: <project>/\n");
    } else {
        out.push_str(&format!("cwd: <project>/{cwd_rel}\n"));
    }
    if args.codex_home.is_some() {
        out.push_str("codex-home: <codex-home> (granted)\n");
    } else {
        out.push_str("codex-home: not granted (G5; pass --codex-home <dir>)\n");
    }
    out.push_str(&format!("exit: {exit_code}\n\n"));
    for result in results {
        out.push_str(&format!(
            "required {} → {}{}\n",
            result.capability_id,
            result.truth_state,
            result
                .unknown_reason_code
                .as_deref()
                .map(|code| format!(" ({code})"))
                .unwrap_or_default()
        ));
    }
    out.push('\n');
    out.push_str("Explanation\n");
    for item in explanation {
        let kind = item.get("kind").and_then(Value::as_str).unwrap_or("item");
        let rule = item.get("rule_id").and_then(Value::as_str).unwrap_or("-");
        let path = item.get("path").and_then(Value::as_str).unwrap_or("-");
        let why = item.get("why").and_then(Value::as_str).unwrap_or("");
        let next = item
            .get("next_evidence")
            .and_then(Value::as_str)
            .unwrap_or("");
        out.push_str(&format!(
            "- [{rule}] {kind} {path}\n    {why}\n    next evidence: {next}\n"
        ));
    }
    if !unknown.is_empty() {
        out.push_str("\nUnknown\n");
        for item in unknown {
            let code = item
                .get("reason_code")
                .and_then(Value::as_str)
                .unwrap_or("-");
            let rule = item.get("rule_id").and_then(Value::as_str).unwrap_or("-");
            out.push_str(&format!("- [{rule}] {code}\n"));
        }
    }
    if !findings.is_empty() {
        out.push_str("\nFindings\n");
        for item in findings {
            let kind = item
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("finding");
            let path = item.get("path").and_then(Value::as_str).unwrap_or("-");
            out.push_str(&format!("- {kind} {path}\n"));
        }
    }
    out.push_str("\nAssumptions\n");
    for item in assumptions {
        let key = item.get("key").and_then(Value::as_str).unwrap_or("-");
        let value = item.get("value").and_then(Value::as_str).unwrap_or("");
        out.push_str(&format!("- {key}: {value}\n"));
    }
    if !warnings.is_empty() {
        out.push_str("\nWarnings\n");
        for item in warnings {
            let code = item.get("code").and_then(Value::as_str).unwrap_or("-");
            match item.get("line") {
                Some(Value::Int(line)) => out.push_str(&format!("- {code} (line {line})\n")),
                _ => out.push_str(&format!("- {code}\n")),
            }
        }
    }
    out
}

fn map_resolve(error: ResolveError) -> InspectFailure {
    match error {
        ResolveError::Fs(refusal) => io_from_refusal(&refusal, "<project>"),
        ResolveError::Internal(message) => InspectFailure::Internal { message },
    }
}

pub fn rel_display(root: &Path, inner: &Path) -> String {
    match inner.strip_prefix(root) {
        Ok(rel) => rel
            .components()
            .map(|component| component.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/"),
        Err(_) => String::new(),
    }
}

fn io_from_refusal(refusal: &Refusal, placeholder: &'static str) -> InspectFailure {
    let (code, message) = map_refusal(refusal, placeholder);
    InspectFailure::Io { code, message }
}

fn map_refusal(refusal: &Refusal, placeholder: &'static str) -> (&'static str, String) {
    match refusal {
        Refusal::EscapesRoot { .. } => (
            "io.escapes_root",
            format!("{placeholder} resolves outside the declared root (<outside>)"),
        ),
        Refusal::NotRegular { kind } => (
            "io.not_regular",
            format!("{placeholder} is {kind}, not a regular file"),
        ),
        Refusal::Unresolvable { detail } => {
            let kind = io_kind_label(detail);
            let code = match kind {
                "NotFound" => "io.missing",
                "NotADirectory" => "io.not_a_directory",
                _ => "io.unresolvable",
            };
            (code, format!("{placeholder} cannot be resolved ({kind})"))
        }
        Refusal::BadRoot { detail } => {
            let kind = io_kind_label(detail);
            if detail.contains("is not a directory") || kind == "NotADirectory" {
                (
                    "io.not_a_directory",
                    format!("{placeholder} is not a directory ({kind})"),
                )
            } else if kind == "NotFound" {
                ("io.missing", format!("{placeholder} is missing ({kind})"))
            } else {
                (
                    "io.unresolvable",
                    format!("{placeholder} is unusable ({kind})"),
                )
            }
        }
    }
}

fn io_kind_label(detail: &str) -> &'static str {
    let lower = detail.to_ascii_lowercase();
    if lower.contains("no such file")
        || lower.contains("not found")
        || lower.contains("(os error 2)")
    {
        "NotFound"
    } else if lower.contains("not a directory") || lower.contains("(os error 20)") {
        "NotADirectory"
    } else if lower.contains("permission denied") || lower.contains("(os error 13)") {
        "PermissionDenied"
    } else {
        "Other"
    }
}

pub fn error_envelope(failure: &InspectFailure) -> Value {
    obj([
        ("schema_version", Value::Int(1)),
        ("command", opt_s(failure.command())),
        ("exit_code", Value::Int(1)),
        ("receipt_kind", s(RECEIPT_KIND)),
        ("schema", s(SCHEMA)),
        (
            "error",
            obj([
                ("code", s(failure.code())),
                ("message", s(failure.message())),
            ]),
        ),
    ])
}

pub fn error_human(failure: &InspectFailure) -> String {
    format!("error: {} ({})\n", failure.message(), failure.code())
}
