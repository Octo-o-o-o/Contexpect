//! Claude Code CLI 2.1.259 `instructions` grammar (rules CL1–CL6).
//!
//! Canonical rule text: `docs/adapters/grammar/claude-code-cli-2.1.259-instructions.md`.
//! Sources are the Claude Code memory documentation as recorded in
//! `docs/research/2026-09-04-context-management-research-ledger.md` §6.2 and
//! the frozen fixtures in `acceptance/corpus/development/static/claude-code__cli.jsonl`.
//!
//! - CL1 `CLAUDE.md`, CL2 `.claude/CLAUDE.md`, CL3 `CLAUDE.local.md` at each
//!   layer from the project root toward cwd are **additive**: every regular,
//!   non-excluded candidate is adopted. Nothing overrides anything.
//! - CL4 a `.ctxpect-ignore` exact relative path excludes a candidate; this is
//!   a Contexpect product rule, not a Claude Code native rule. When nothing
//!   remains adopted the claim is indeterminate with
//!   `observation_scope_excluded` (C-F01), never a native absence.
//! - CL5 the user memory root (`~/.claude/CLAUDE.md`) is not read in this
//!   slice; the global layer is `permission_not_granted` and does not block.
//! - CL6 there is no official hard byte cap; a Contexpect `budget.json`
//!   declaration (`{path, max_bytes}`) at the project root marks an adopted
//!   file whose bytes exceed `max_bytes` as truncated-after. The declaration
//!   is a native path used and evidence, so a preview can show it.
//!
//! Not observed here (documented in the grammar file): `@path` imports,
//! subdirectory files loaded lazily when Claude reads there, managed policy
//! memory, and any global root.

use crate::{
    candidate_kind, classify_candidate, edge, ensure_expressible, instruction_claims, load_ignore,
    named_paths_inventory, push_unique, Adopted, Assumption, Candidate, CandidateOutcome, Edge,
    EdgeKind, Evidence, Layer, Resolution, ResolveError, ResolveRequest, RootKind,
    honesty_claims, layer_file, layers_toward_cwd, ANCHORS,
};
use ctxpect_core::UnknownReason;
use ctxpect_fs::{read_contained, EntryKind, Root};
use ctxpect_schema::{parse, Value};
use std::path::Path;

const CLAUDE_NAME: &str = "CLAUDE.md";
const DOT_CLAUDE_NAME: &str = ".claude/CLAUDE.md";
const LOCAL_NAME: &str = "CLAUDE.local.md";
const IGNORE_NAME: &str = ".ctxpect-ignore";
const BUDGET_NAME: &str = "budget.json";
/// The namespaced spelling of the budget declaration (no `schema` needed).
const BUDGET_NAMESPACED: &str = ".ctxpect/budget.json";
/// The `schema` a root `budget.json` must carry to be a declaration at all
/// (the Doctor reads declarations by the same rule).
const BUDGET_SCHEMA: &str = "ctxpect-budget-v1";
const GLOBAL_LAYER: &str = "global";

/// Candidate names per layer, in adoption order, with their rule ids.
const CANDIDATES: &[(&str, &str)] = &[
    (CLAUDE_NAME, "CL1"),
    (DOT_CLAUDE_NAME, "CL2"),
    (LOCAL_NAME, "CL3"),
];

struct BudgetDeclaration {
    /// Where the declaration was read from (`budget.json` or its namespaced form).
    source: String,
    path: String,
    max_bytes: u64,
    digest: String,
}

/// The budget declaration, if any: `.ctxpect/budget.json`, or a root
/// `budget.json` whose `schema` is `ctxpect-budget-v1`. A root file without
/// the schema is not a declaration (the Doctor reads it the same way), so
/// resolver and Doctor never disagree about the same file. The namespaced
/// form wins when both exist.
fn load_budget(project: &Root) -> Result<Option<BudgetDeclaration>, ResolveError> {
    for source in [BUDGET_NAMESPACED, BUDGET_NAME] {
        if let Some(found) = load_budget_from(project, source)? {
            return Ok(Some(found));
        }
    }
    Ok(None)
}

fn load_budget_from(project: &Root, source: &str) -> Result<Option<BudgetDeclaration>, ResolveError> {
    match candidate_kind(project, source)? {
        Some(EntryKind::File) => {
            let content = read_contained(project, Path::new(source))?;
            let Ok(doc) = parse(&String::from_utf8_lossy(&content.bytes)) else {
                return Ok(None);
            };
            let declared = doc.get("schema").and_then(Value::as_str);
            let accepted = match declared {
                Some(schema) => schema == BUDGET_SCHEMA,
                None => source == BUDGET_NAMESPACED,
            };
            if !accepted {
                return Ok(None);
            }
            let path = doc.get("path").and_then(Value::as_str).unwrap_or("").to_string();
            let max_bytes = doc
                .get("max_bytes")
                .and_then(Value::as_i64)
                .and_then(|n| u64::try_from(n).ok());
            match max_bytes {
                Some(max_bytes) if !path.is_empty() => Ok(Some(BudgetDeclaration {
                    source: source.to_string(),
                    path,
                    max_bytes,
                    digest: content.whole_digest,
                })),
                _ => Ok(None),
            }
        }
        _ => Ok(None),
    }
}

pub(crate) fn resolve_claude_code(request: &ResolveRequest<'_>) -> Result<Resolution, ResolveError> {
    let loaded_ignore = load_ignore(request.project)?;
    let ignore = loaded_ignore.paths;
    let layer_rels = layers_toward_cwd(request.cwd_rel);

    let mut names = Vec::new();
    for layer_rel in &layer_rels {
        for (name, _) in CANDIDATES {
            names.push(layer_file(layer_rel, name));
        }
    }
    let project_inventory = named_paths_inventory(request.project, names)?;

    let mut edges: Vec<Edge> = Vec::new();
    let mut layers: Vec<Layer> = Vec::new();
    let mut evidence: Vec<Evidence> = Vec::new();
    let mut native_paths: Vec<String> = Vec::new();
    let mut adopted_files: Vec<Adopted> = Vec::new();
    let mut blocking_unknown: Option<UnknownReason> = None;
    let mut excluded_by_product = false;

    if loaded_ignore.used {
        push_unique(&mut native_paths, IGNORE_NAME.to_string());
        if let Some(digest) = loaded_ignore.digest {
            evidence.push(Evidence {
                root_kind: RootKind::Project,
                path: IGNORE_NAME.to_string(),
                content_digest: Some(digest),
            });
        }
    }

    // CL5: the user memory root is never read in this slice.
    layers.push(Layer {
        id: GLOBAL_LAYER.to_string(),
        rel: String::new(),
        root_kind: RootKind::HarnessHome,
        candidates: vec![Candidate {
            path: CLAUDE_NAME.to_string(),
            name: CLAUDE_NAME.to_string(),
            existed: false,
            adopted: false,
        }],
        adopted: None,
        unknown: Some(UnknownReason::PermissionNotGranted),
    });
    edges.push(edge(
        EdgeKind::UnknownBecause,
        "CL5",
        CLAUDE_NAME.to_string(),
        RootKind::HarnessHome,
        None,
        None,
        None,
        Some(UnknownReason::PermissionNotGranted.as_str().to_string()),
    ));

    for layer_rel in &layer_rels {
        let layer_id = if layer_rel.is_empty() {
            "project-root".to_string()
        } else {
            layer_rel.clone()
        };
        let mut candidates = Vec::new();
        let mut layer_adopted: Option<String> = None;
        let mut layer_unknown: Option<UnknownReason> = None;
        for (name, rule_id) in CANDIDATES {
            let path = layer_file(layer_rel, name);
            let Some(kind) = candidate_kind(request.project, &path)? else {
                candidates.push(Candidate {
                    path,
                    name: (*name).to_string(),
                    existed: false,
                    adopted: false,
                });
                continue;
            };
            push_unique(&mut native_paths, path.clone());
            if ignore.iter().any(|item| item == &path) {
                excluded_by_product = true;
                edges.push(edge(
                    EdgeKind::ExcludedBy,
                    "CL4",
                    path.clone(),
                    RootKind::Project,
                    Some(IGNORE_NAME.to_string()),
                    None,
                    None,
                    Some("product-user-exclusion".to_string()),
                ));
                evidence.push(Evidence {
                    root_kind: RootKind::Project,
                    path: path.clone(),
                    content_digest: None,
                });
                candidates.push(Candidate {
                    path,
                    name: (*name).to_string(),
                    existed: true,
                    adopted: false,
                });
                continue;
            }
            match classify_candidate(request.project, &path, kind)? {
                CandidateOutcome::Adopt { len, digest } => {
                    adopted_files.push(Adopted {
                        path: path.clone(),
                        len,
                        root_kind: RootKind::Project,
                    });
                    evidence.push(Evidence {
                        root_kind: RootKind::Project,
                        path: path.clone(),
                        content_digest: digest,
                    });
                    edges.push(edge(
                        EdgeKind::IncludedBy,
                        rule_id,
                        path.clone(),
                        RootKind::Project,
                        None,
                        None,
                        None,
                        None,
                    ));
                    // Additive: the layer records its last adopted file, but
                    // every adopted file counts.
                    layer_adopted = Some(path.clone());
                    candidates.push(Candidate {
                        path,
                        name: (*name).to_string(),
                        existed: true,
                        adopted: true,
                    });
                }
                CandidateOutcome::Unknown { note } => {
                    blocking_unknown = Some(UnknownReason::ContentRedactedByPolicy);
                    layer_unknown = Some(UnknownReason::ContentRedactedByPolicy);
                    evidence.push(Evidence {
                        root_kind: RootKind::Project,
                        path: path.clone(),
                        content_digest: None,
                    });
                    edges.push(edge(
                        EdgeKind::UnknownBecause,
                        rule_id,
                        path.clone(),
                        RootKind::Project,
                        None,
                        None,
                        None,
                        Some(note.to_string()),
                    ));
                    candidates.push(Candidate {
                        path,
                        name: (*name).to_string(),
                        existed: true,
                        adopted: false,
                    });
                }
            }
        }
        layers.push(Layer {
            id: layer_id,
            rel: layer_rel.clone(),
            root_kind: RootKind::Project,
            candidates,
            adopted: layer_adopted,
            unknown: layer_unknown,
        });
    }

    // CL6: a Contexpect cap declaration, applied to the adopted file it names.
    let mut aggregated_bytes: u64 = adopted_files.iter().map(|file| file.len).sum();
    let mut truncated = false;
    let mut cap_assumption = Assumption {
        key: "byte_cap".to_string(),
        value: "no official hard byte cap is recorded for Claude Code memory files; none applied".to_string(),
        provenance: "official-spec",
    };
    if let Some(budget) = load_budget(request.project)?
        && let Some(file) = adopted_files.iter().find(|file| file.path == budget.path)
    {
        push_unique(&mut native_paths, budget.source.clone());
        evidence.push(Evidence {
            root_kind: RootKind::Project,
            path: budget.source.clone(),
            content_digest: Some(budget.digest.clone()),
        });
        cap_assumption = Assumption {
            key: "byte_cap".to_string(),
            value: format!(
                "contexpect {} declaration: {} capped at {} bytes (not a Claude Code native cap)",
                budget.source, budget.path, budget.max_bytes
            ),
            provenance: "official-spec",
        };
        if file.len > budget.max_bytes {
            truncated = true;
            aggregated_bytes = aggregated_bytes.saturating_sub(file.len) + budget.max_bytes;
            edges.push(edge(
                EdgeKind::TruncatedAfter,
                "CL6",
                file.path.clone(),
                RootKind::Project,
                Some(budget.source.clone()),
                Some(budget.max_bytes),
                Some(budget.max_bytes),
                None,
            ));
        }
    }

    let included = !adopted_files.is_empty();
    let parse_path = adopted_files
        .first()
        .map(|file| file.path.clone())
        .or_else(|| {
            native_paths
                .iter()
                .find(|path| path.ends_with(".md"))
                .cloned()
        })
        .unwrap_or_else(|| CLAUDE_NAME.to_string());

    let claims = if !included {
        if let Some(reason) = blocking_unknown {
            honesty_claims(reason, false)
        } else if excluded_by_product {
            // C-F01: `.ctxpect-ignore` only removes the file from Contexpect's
            // observation scope; it says nothing about the harness's native
            // load behaviour, so the claim is indeterminate, never absent.
            honesty_claims(UnknownReason::ObservationScopeExcluded, false)
        } else {
            instruction_claims(false)
        }
    } else {
        instruction_claims(true)
    };
    ensure_expressible(&claims)?;

    let assumptions = vec![
        Assumption {
            key: "instruction_files".to_string(),
            value: "CLAUDE.md, .claude/CLAUDE.md and CLAUDE.local.md at each layer from the project root toward cwd, all additive (Claude Code memory docs as recorded in docs/research/2026-09-04-context-management-research-ledger.md §6.2)".to_string(),
            provenance: "official-spec",
        },
        cap_assumption,
        Assumption {
            key: "imports".to_string(),
            value: "@path imports are not followed in this slice".to_string(),
            provenance: "official-spec",
        },
        Assumption {
            key: "user_memory".to_string(),
            value: "~/.claude/CLAUDE.md is not read; the global layer is permission_not_granted".to_string(),
            provenance: "official-spec",
        },
        Assumption {
            key: "anchor".to_string(),
            value: ANCHORS[1].coordinate_id(),
            provenance: "official-spec",
        },
    ];

    Ok(Resolution {
        layers,
        edges,
        aggregated_bytes,
        truncated,
        ignore_used: loaded_ignore.used,
        ignore_warnings: loaded_ignore.warnings,
        included,
        native_paths_used: native_paths,
        parse_path,
        assumptions,
        evidence,
        claims,
        project_inventory,
        codex_home_inventory: None,
    })
}
