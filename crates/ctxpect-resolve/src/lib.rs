//! Static instructions resolvers for the frozen anchors.
//!
//! The crate holds one grammar per anchor ([`ANCHORS`]): Codex CLI 0.147.0
//! (`AGENTS.override.md` / `AGENTS.md`, G1–G5) and Claude Code CLI 2.1.259
//! (`CLAUDE.md` / `.claude/CLAUDE.md` / `CLAUDE.local.md`, CL1–CL6, in
//! [`claude_code`]). [`anchor_for`] dispatches a coordinate to its grammar;
//! [`coordinate_unknown_reason`] says why a coordinate has none. The grammar
//! documents live in `docs/adapters/grammar/`.
//!
//! The Codex grammar turns a declared project root, a cwd inside it, an
//! optional `--codex-home` root and a `.ctxpect-ignore` list into a structured
//! [`Resolution`]. It discovers `AGENTS.override.md` / `AGENTS.md` by
//! classifying those named paths with [`ctxpect_fs::Root::classify`] and
//! reads selected files with [`ctxpect_fs::read_contained`]. It does not
//! start processes, follow `HOME`/`CODEX_HOME`, full-tree `scan` the project,
//! or read Codex `config.toml`.
//!
//! # What the tests observe
//!
//! - Layers run from the project root toward cwd; each layer considers
//!   `AGENTS.override.md` then `AGENTS.md` and adopts at most one file.
//! - `--cwd` outside the project is refused by containment, not guessed.
//! - Same-layer `AGENTS.override.md` marks `AGENTS.md` as overridden-by and
//!   shadowed-by, with both edges pointing at the override file. An unknown
//!   (non-regular / unreadable) override still occupies the layer winner
//!   slot: `AGENTS.md` is overridden-by and not adopted, and the
//!   instructions claim stays indeterminate.
//! - Aggregated body bytes stop at [`PROJECT_DOC_MAX_BYTES`]; the truncated
//!   file and byte offset are recorded as truncated-after. Aggregate bodies
//!   at or below the cap emit no truncated-after edge. A zero-byte file
//!   after a full cap does not emit truncated-after; `truncated` stays
//!   false.
//! - A `.ctxpect-ignore` exact relative path excludes that file from
//!   aggregation and is labelled a product user exclusion, not a Codex native
//!   rule. The excluded file is not read; its evidence digest is null.
//!   Exclusion only narrows Contexpect's observation scope: when nothing is
//!   adopted the claim is indeterminate with `observation_scope_excluded`
//!   (C-F01), never a native absence. Lines that are absolute, contain a
//!   `..` segment, or include control characters are ignored; warnings record
//!   only the 1-based line number and never echo the original line.
//! - Without an explicit codex-home root, the global layer is indeterminate
//!   with `permission_not_granted`; the crate never consults `HOME` or
//!   `CODEX_HOME`. The global layer's root kind is always codex-home.
//!   CLI output-boundary redaction of project / codex-home root forms and
//!   HOME (the same path-prefix token-boundary rule on both success and
//!   error writes; not a substring of `instructions`,
//!   `<project>/target/tmp`, `--project`, `--require`, or the command
//!   token `inspect`) lives in `ctxpect-cli`; this crate's resolution
//!   records are placeholder-relative.
//! - An explicit `--codex-home` root is classified only at `AGENTS.md` /
//!   `AGENTS.override.md`; sibling files are not listed or read. Project
//!   layers use the same named inventory; the project tree is not scanned.
//! - Symlink, FIFO, and multiply-linked candidates are not reported as
//!   missing regular files; they emit unknown-because and leave the
//!   instructions claim indeterminate.
//! - Every emitted [`ctxpect_core::Claim`] has an empty `violations()` list.
//!   Runtime facets stay indeterminate with `runtime_snapshot_missing`.
//!
//! # What they do not observe
//!
//! - Fallback filenames configured in Codex `config.toml`.
//! - `project_doc_max_bytes` overrides from config; the 32768 default is the
//!   official-spec value recorded as an assumption.
//! - Other OS lanes or surfaces as live execution; those coordinates are
//!   classified, not scanned, by [`coordinate_unknown_reason`].
//! - Native oracle reconciliation (`codex debug prompt-input`).
//! - Skills, plugins, MCP, commands, agents, memory, or any capability other
//!   than `instructions`.
//! - glob, negation, or comments in `.ctxpect-ignore`.
//! - Echoing a rejected `.ctxpect-ignore` line; only its line number is
//!   recorded.
//! - Non-UTF-8 ignore bytes as a distinct case; they are read as lossy UTF-8
//!   and then subject to the same relative-path grammar.
//! - Whether a file that exists below cwd, or on a sibling path, would have
//!   been read by Codex; only the root-to-cwd chain is walked.
//! - Windows junctions, case folding, or other OS-lane filesystem behaviour.
//! - That the default 32768-byte cap is the right product policy; the tests
//!   only show that this slice applies the documented official-spec default.
//! - Socket or device candidates as a separate integration case; FIFO covers
//!   the not-regular mapping, and collect's Withheld::NotRegular is shared.
//! - A full-tree [`ctxpect_collect::scan`] of the project; named instruction
//!   files are classified instead.
//! - CLI root-form or `HOME` path-prefix redaction; this crate does not
//!   read `HOME` or `CODEX_HOME` and does not emit host home paths. The
//!   `replace('\\', "/")` calls in ignore parsing only normalize path
//!   separators on `.ctxpect-ignore` lines.

use ctxpect_collect::{Entry, Inventory, Withheld};
use ctxpect_core::{
    Claim, ClaimKind, ClaimSource, Coverage, ExperimentRef, KnowledgeStatus, LifecycleStage,
    Precision, Provenance, TruthState, UnknownReason,
};
use ctxpect_fs::{EntryKind, Refusal, Root, is_missing, read_contained};
use std::fmt;
use std::fs;
use std::path::Path;

/// Official-spec default for Codex `project_doc_max_bytes` (32 KiB).
pub const PROJECT_DOC_MAX_BYTES: u64 = 32768;

pub mod claude_code;

/// Which grammar an anchor resolves with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grammar {
    /// `docs/adapters/grammar/codex-cli-0.147.0-instructions.md`
    CodexInstructions,
    /// `docs/adapters/grammar/claude-code-cli-2.1.259-instructions.md`
    ClaudeCodeInstructions,
}

/// One frozen coordinate this crate resolves authoritatively. Version is a
/// user-attested / official-spec coordinate, not native evidence that the
/// binary is installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anchor {
    pub harness: &'static str,
    pub version: &'static str,
    pub surface: &'static str,
    pub os_lane: &'static str,
    pub grammar: Grammar,
    /// Capabilities the grammar resolves. Everything else is unimplemented.
    pub capabilities: &'static [&'static str],
    /// Human name used in explanations.
    pub display: &'static str,
}

impl Anchor {
    #[must_use]
    pub fn coordinate_id(&self) -> String {
        format!("{}/{}/{}/{}", self.harness, self.version, self.surface, self.os_lane)
    }
}

pub const INSTRUCTIONS: &str = "instructions";

/// This resolver crate's version, recorded by conformance reports so a
/// result is bound to the implementation that produced it.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Every anchor, in dispatch order. The first is the CLI default.
pub const ANCHORS: &[Anchor] = &[
    Anchor {
        harness: "codex",
        version: "0.147.0",
        surface: "cli",
        os_lane: "macos-27-arm64",
        grammar: Grammar::CodexInstructions,
        capabilities: &[INSTRUCTIONS],
        display: "Codex",
    },
    Anchor {
        harness: "claude-code",
        version: "2.1.259",
        surface: "cli",
        os_lane: "macos-27-arm64",
        grammar: Grammar::ClaudeCodeInstructions,
        capabilities: &[INSTRUCTIONS],
        display: "Claude Code",
    },
];

/// The anchor `inspect` uses when no coordinate flags are given.
pub const DEFAULT_ANCHOR: &Anchor = &ANCHORS[0];

/// The anchor that resolves this exact coordinate, if any.
#[must_use]
pub fn anchor_for(harness: &str, version: &str, surface: &str, os_lane: &str) -> Option<&'static Anchor> {
    ANCHORS.iter().find(|anchor| {
        anchor.harness == harness
            && anchor.version == version
            && anchor.surface == surface
            && anchor.os_lane == os_lane
    })
}

fn family_anchors(harness: &str) -> impl Iterator<Item = &'static Anchor> {
    ANCHORS.iter().filter(move |anchor| anchor.harness == harness)
}

const OVERRIDE_NAME: &str = "AGENTS.override.md";
const AGENTS_NAME: &str = "AGENTS.md";
const IGNORE_NAME: &str = ".ctxpect-ignore";
const GLOBAL_LAYER: &str = "global";

/// Inputs for one static resolution.
pub struct ResolveRequest<'a> {
    pub project: &'a Root,
    /// Path of cwd relative to the project root, `/` separated. Empty means
    /// the project root itself.
    pub cwd_rel: &'a str,
    /// Explicit Codex home root. Read only by the Codex grammar.
    pub codex_home: Option<&'a Root>,
    pub project_doc_max_bytes: u64,
    /// The grammar of the anchor being resolved (see [`anchor_for`]).
    pub grammar: Grammar,
}

/// One edge explaining why a file was adopted, skipped, cut, or unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub kind: EdgeKind,
    pub rule_id: &'static str,
    pub path: String,
    pub related_path: Option<String>,
    pub offset: Option<u64>,
    pub file_offset: Option<u64>,
    pub note: Option<String>,
    pub root_kind: RootKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    IncludedBy,
    ExcludedBy,
    OverriddenBy,
    ShadowedBy,
    TruncatedAfter,
    UnknownBecause,
}

impl EdgeKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            EdgeKind::IncludedBy => "included-by",
            EdgeKind::ExcludedBy => "excluded-by",
            EdgeKind::OverriddenBy => "overridden-by",
            EdgeKind::ShadowedBy => "shadowed-by",
            EdgeKind::TruncatedAfter => "truncated-after",
            EdgeKind::UnknownBecause => "unknown-because",
        }
    }
}

impl fmt::Display for EdgeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A file considered at a layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub path: String,
    pub name: String,
    pub existed: bool,
    pub adopted: bool,
}

/// One search layer: global, project root, or a directory on the way to cwd.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layer {
    pub id: String,
    pub rel: String,
    pub root_kind: RootKind,
    pub candidates: Vec<Candidate>,
    pub adopted: Option<String>,
    pub unknown: Option<UnknownReason>,
}

/// Evidence identity for a file that was seen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    pub root_kind: RootKind,
    pub path: String,
    pub content_digest: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootKind {
    Project,
    /// The Codex home root (`--codex-home`).
    CodexHome,
    /// A harness home root this slice does not read (Claude Code's
    /// `~/.claude`); layers under it are always permission-not-granted.
    HarnessHome,
}

impl RootKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            RootKind::Project => "project",
            RootKind::CodexHome => "codex-home",
            RootKind::HarnessHome => "harness-home",
        }
    }
}

/// A recorded default or limitation of this slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assumption {
    pub key: String,
    pub value: String,
    pub provenance: &'static str,
}

/// Claims for the six lifecycle facets plus the corpus-facing primary claim.
#[derive(Debug, Clone)]
pub struct FacetClaims {
    pub primary: Claim,
    pub installed: Claim,
    pub discoverable: Claim,
    pub eligible: Claim,
    pub model_visible: Claim,
    pub use_evidence: Claim,
    pub outcome_affecting: Claim,
}

impl FacetClaims {
    /// Every claim this resolver is willing to emit.
    pub fn all(&self) -> [&Claim; 7] {
        [
            &self.primary,
            &self.installed,
            &self.discoverable,
            &self.eligible,
            &self.model_visible,
            &self.use_evidence,
            &self.outcome_affecting,
        ]
    }
}

/// Structured result of one static instructions resolution.
#[derive(Debug, Clone)]
pub struct Resolution {
    pub layers: Vec<Layer>,
    pub edges: Vec<Edge>,
    pub aggregated_bytes: u64,
    pub truncated: bool,
    pub ignore_used: bool,
    pub ignore_warnings: Vec<IgnoreWarning>,
    pub included: bool,
    pub native_paths_used: Vec<String>,
    pub parse_path: String,
    pub assumptions: Vec<Assumption>,
    pub evidence: Vec<Evidence>,
    pub claims: FacetClaims,
    pub project_inventory: Inventory,
    pub codex_home_inventory: Option<Inventory>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    Fs(Refusal),
    Internal(String),
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolveError::Fs(refusal) => write!(f, "{refusal}"),
            ResolveError::Internal(detail) => write!(f, "internal resolver error: {detail}"),
        }
    }
}

impl std::error::Error for ResolveError {}

impl From<Refusal> for ResolveError {
    fn from(refusal: Refusal) -> Self {
        ResolveError::Fs(refusal)
    }
}

/// Why this coordinate cannot be resolved authoritatively, if it cannot.
///
/// Order is load-bearing for unknown-honesty cells: OS lane first, then
/// surface, then harness, then version. A family with no anchor at all is
/// `official_distribution_not_captured`; a family with anchors is judged
/// against the lanes, surfaces and versions those anchors declare.
#[must_use]
pub fn coordinate_unknown_reason(
    harness: &str,
    version: &str,
    surface: &str,
    os_lane: &str,
) -> Option<UnknownReason> {
    if !ANCHORS.iter().any(|anchor| anchor.os_lane == os_lane) {
        return Some(UnknownReason::OfficialDistributionNotCaptured);
    }
    let mut anchors = family_anchors(harness).peekable();
    if anchors.peek().is_none() {
        // The lane is captured but the family is not.
        if !ANCHORS.iter().any(|anchor| anchor.surface == surface) {
            return Some(UnknownReason::SurfaceNotExposed);
        }
        return Some(UnknownReason::OfficialDistributionNotCaptured);
    }
    let anchors: Vec<&Anchor> = anchors.collect();
    if !anchors.iter().any(|anchor| anchor.os_lane == os_lane) {
        return Some(UnknownReason::OfficialDistributionNotCaptured);
    }
    if !anchors.iter().any(|anchor| anchor.surface == surface) {
        return Some(UnknownReason::SurfaceNotExposed);
    }
    if !anchors.iter().any(|anchor| anchor.version == version) {
        return Some(UnknownReason::UnsupportedHarnessVersion);
    }
    None
}

/// The capabilities this crate actually resolves for a family.
///
/// Anything not listed is **unimplemented**: the product answers Unknown for
/// it, and a conformance runner counts that as unimplemented, never as a pass
/// on the strength of the honesty branch.
#[must_use]
pub fn implemented_capabilities(harness: &str) -> &'static [&'static str] {
    family_anchors(harness)
        .next()
        .map_or(&[], |anchor| anchor.capabilities)
}

/// Whether this exact coordinate is one the crate resolves authoritatively.
#[must_use]
pub fn is_supported_coordinate(harness: &str, version: &str, surface: &str, os_lane: &str) -> bool {
    anchor_for(harness, version, surface, os_lane).is_some()
}

/// Why a capability other than `instructions` is not resolved here.
#[must_use]
pub fn unsupported_capability_reason(capability: &str) -> Option<UnknownReason> {
    if capability == INSTRUCTIONS {
        None
    } else {
        Some(UnknownReason::SurfaceNotExposed)
    }
}

/// Claims for a coordinate or capability this slice will not resolve.
#[must_use]
pub fn honesty_claims(reason: UnknownReason, surface_unexposed: bool) -> FacetClaims {
    let discoverable = resolved_claim(
        LifecycleStage::Discoverable,
        TruthState::Indeterminate,
        Some(reason),
        surface_unexposed,
    );
    let eligible = resolved_claim(
        LifecycleStage::Eligible,
        TruthState::Indeterminate,
        Some(reason),
        surface_unexposed,
    );
    FacetClaims {
        primary: discoverable.clone(),
        installed: resolved_claim(
            LifecycleStage::Installed,
            TruthState::Indeterminate,
            Some(UnknownReason::NotInstalled),
            false,
        ),
        discoverable,
        eligible,
        model_visible: runtime_claim(),
        use_evidence: runtime_claim_stage(LifecycleStage::UseEvidence),
        outcome_affecting: runtime_claim_stage(LifecycleStage::OutcomeAffecting),
    }
}

/// Directories from the project root to cwd, inclusive, as `/`-separated
/// relative paths. The root itself is the empty string.
#[must_use]
pub fn layers_toward_cwd(cwd_rel: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let trimmed = cwd_rel.trim_matches('/');
    if trimmed.is_empty() {
        return out;
    }
    let mut acc = String::new();
    for part in trimmed.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if !acc.is_empty() {
            acc.push('/');
        }
        acc.push_str(part);
        out.push(acc.clone());
    }
    out
}

/// Join a layer directory with a file name using `/`.
#[must_use]
pub fn layer_file(layer: &str, name: &str) -> String {
    if layer.is_empty() {
        name.to_string()
    } else {
        format!("{layer}/{name}")
    }
}

/// A skipped `.ctxpect-ignore` line. The original text is not stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnoreWarning {
    pub line: usize,
    pub code: &'static str,
}

/// Parsed `.ctxpect-ignore` paths plus skipped-line warnings.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedIgnore {
    pub paths: Vec<String>,
    pub warnings: Vec<IgnoreWarning>,
}

/// Parse `.ctxpect-ignore`: one exact relative path per non-empty line.
///
/// Absolute paths, `..` segments, and control characters are skipped and
/// recorded as [`IgnoreWarning`] values that carry only a 1-based line number.
#[must_use]
pub fn parse_ignore(bytes: &[u8]) -> ParsedIgnore {
    let mut paths = Vec::new();
    let mut warnings = Vec::new();
    for (index, line) in String::from_utf8_lossy(bytes).lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !ignore_line_allowed(trimmed) {
            warnings.push(IgnoreWarning {
                line: index + 1,
                code: "ignore.invalid_line",
            });
            continue;
        }
        paths.push(trimmed.replace('\\', "/"));
    }
    ParsedIgnore { paths, warnings }
}

fn ignore_line_allowed(line: &str) -> bool {
    if line.chars().any(char::is_control) {
        return false;
    }
    let normalized = line.replace('\\', "/");
    if normalized.starts_with('/') {
        return false;
    }
    !normalized.split('/').any(|segment| segment == "..")
}

/// Resolve `instructions` for `request` with the anchor's grammar.
pub fn resolve(request: &ResolveRequest<'_>) -> Result<Resolution, ResolveError> {
    match request.grammar {
        Grammar::CodexInstructions => resolve_codex(request),
        Grammar::ClaudeCodeInstructions => claude_code::resolve_claude_code(request),
    }
}

/// Resolve Codex `instructions` for `request`.
///
/// Reads the project-root `.ctxpect-ignore` through containment first. Project
/// layers and a granted codex-home are not walked: only `AGENTS.override.md` /
/// `AGENTS.md` are classified and, when regular and not excluded, read through
/// containment.
fn resolve_codex(request: &ResolveRequest<'_>) -> Result<Resolution, ResolveError> {
    let loaded_ignore = load_ignore(request.project)?;
    let ignore = loaded_ignore.paths;
    let ignore_used = loaded_ignore.used;
    let ignore_digest = loaded_ignore.digest;
    let ignore_warnings = loaded_ignore.warnings;
    let layer_rels = layers_toward_cwd(request.cwd_rel);
    let mut project_names = Vec::new();
    for layer_rel in &layer_rels {
        project_names.push(layer_file(layer_rel, OVERRIDE_NAME));
        project_names.push(layer_file(layer_rel, AGENTS_NAME));
    }
    let project_inventory = named_paths_inventory(request.project, project_names)?;

    let mut edges: Vec<Edge> = Vec::new();
    let mut layers: Vec<Layer> = Vec::new();
    let mut evidence: Vec<Evidence> = Vec::new();
    let mut native_paths: Vec<String> = Vec::new();
    let mut adopted_files: Vec<Adopted> = Vec::new();
    let mut blocking_unknown: Option<UnknownReason> = None;
    let mut excluded_by_product = false;

    if ignore_used {
        push_unique(&mut native_paths, IGNORE_NAME.to_string());
        if let Some(digest) = ignore_digest {
            evidence.push(Evidence {
                root_kind: RootKind::Project,
                path: IGNORE_NAME.to_string(),
                content_digest: Some(digest),
            });
        }
    }

    let mut codex_home_inventory = None;
    match request.codex_home {
        None => {
            layers.push(Layer {
                id: GLOBAL_LAYER.to_string(),
                rel: String::new(),
                root_kind: RootKind::CodexHome,
                candidates: vec![Candidate {
                    path: AGENTS_NAME.to_string(),
                    name: AGENTS_NAME.to_string(),
                    existed: false,
                    adopted: false,
                }],
                adopted: None,
                unknown: Some(UnknownReason::PermissionNotGranted),
            });
            edges.push(Edge {
                kind: EdgeKind::UnknownBecause,
                rule_id: "G5",
                path: AGENTS_NAME.to_string(),
                related_path: None,
                offset: None,
                file_offset: None,
                note: Some(UnknownReason::PermissionNotGranted.as_str().to_string()),
                root_kind: RootKind::CodexHome,
            });
        }
        Some(home) => {
            let inventory = named_agents_inventory(home)?;
            consider_layer(&mut LayerWork {
                root: home,
                inventory: &inventory,
                root_kind: RootKind::CodexHome,
                layer_id: GLOBAL_LAYER,
                layer_rel: "",
                ignore: &[],
                layers: &mut layers,
                edges: &mut edges,
                evidence: &mut evidence,
                native_paths: &mut native_paths,
                adopted_files: &mut adopted_files,
                blocking_unknown: &mut blocking_unknown,
                excluded_by_product: &mut excluded_by_product,
            })?;
            codex_home_inventory = Some(inventory);
        }
    }

    for layer_rel in layer_rels {
        let layer_id = if layer_rel.is_empty() {
            "project-root".to_string()
        } else {
            layer_rel.clone()
        };
        consider_layer(&mut LayerWork {
            root: request.project,
            inventory: &project_inventory,
            root_kind: RootKind::Project,
            layer_id: &layer_id,
            layer_rel: &layer_rel,
            ignore: &ignore,
            layers: &mut layers,
            edges: &mut edges,
            evidence: &mut evidence,
            native_paths: &mut native_paths,
            adopted_files: &mut adopted_files,
            blocking_unknown: &mut blocking_unknown,
            excluded_by_product: &mut excluded_by_product,
        })?;
    }

    let (aggregated_bytes, truncated) =
        apply_cap(request.project_doc_max_bytes, &adopted_files, &mut edges);

    let included = !adopted_files.is_empty();
    let parse_path = adopted_files
        .last()
        .map(|file| file.path.clone())
        .or_else(|| {
            native_paths
                .iter()
                .rev()
                .find(|path| path.ends_with(AGENTS_NAME) || path.ends_with(OVERRIDE_NAME))
                .cloned()
        })
        .unwrap_or_else(|| AGENTS_NAME.to_string());

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
            key: "project_doc_max_bytes".to_string(),
            value: request.project_doc_max_bytes.to_string(),
            provenance: "official-spec",
        },
        Assumption {
            key: "project_doc_max_bytes.source".to_string(),
            value: "Codex AGENTS.md official docs as recorded in docs/research/2026-09-04-context-management-research-ledger.md §6.1 (accessed 2026-09-04); config.toml was not read"
                .to_string(),
            provenance: "official-spec",
        },
        Assumption {
            key: "fallback_filenames".to_string(),
            value: "not applied in this slice".to_string(),
            provenance: "official-spec",
        },
        Assumption {
            key: "anchor".to_string(),
            value: ANCHORS[0].coordinate_id(),
            provenance: "official-spec",
        },
    ];

    Ok(Resolution {
        layers,
        edges,
        aggregated_bytes,
        truncated,
        ignore_used,
        ignore_warnings,
        included,
        native_paths_used: native_paths,
        parse_path,
        assumptions,
        evidence,
        claims,
        project_inventory,
        codex_home_inventory,
    })
}

pub(crate) struct Adopted {
    pub(crate) path: String,
    pub(crate) len: u64,
    pub(crate) root_kind: RootKind,
}

/// Classify only the two instruction filenames under `root`. Does not list
/// the directory or read any other name.
fn named_agents_inventory(root: &Root) -> Result<Inventory, ResolveError> {
    named_paths_inventory(root, [OVERRIDE_NAME.to_string(), AGENTS_NAME.to_string()])
}

/// Classify the given relative names. Does not list the directory or read
/// file bytes.
pub(crate) fn named_paths_inventory(
    root: &Root,
    rels: impl IntoIterator<Item = String>,
) -> Result<Inventory, ResolveError> {
    let mut entries = Vec::new();
    for path in rels {
        if let Some(kind) = candidate_kind(root, &path)? {
            let withheld = match kind {
                EntryKind::File => None,
                other => Some(Withheld::NotRegular(other)),
            };
            entries.push(Entry {
                path,
                kind,
                content_digest: None,
                len: None,
                truncated: false,
                withheld,
                link_target: None,
                link_count: None,
                identity: None,
            });
        }
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    // This listing is built from an explicit candidate set, not a walk, so no
    // file limit applies and it is complete by construction.
    Ok(Inventory {
        entries,
        truncated: false,
        file_limit: None,
    })
}

pub(crate) fn candidate_kind(root: &Root, rel: &str) -> Result<Option<EntryKind>, ResolveError> {
    match root.classify(Path::new(rel)) {
        Ok(kind) => Ok(Some(kind)),
        Err(Refusal::Unresolvable { .. }) => {
            let joined = root.path().join(rel);
            match fs::symlink_metadata(&joined) {
                Err(err) if is_missing(&err) => Ok(None),
                Err(err) => Err(Refusal::Unresolvable {
                    detail: format!("{}: {err}", joined.display()),
                }
                .into()),
                Ok(meta) => Ok(Some(EntryKind::from_symlink_metadata(&meta))),
            }
        }
        Err(other) => Err(other.into()),
    }
}

pub(crate) struct LoadedIgnore {
    pub(crate) paths: Vec<String>,
    pub(crate) used: bool,
    pub(crate) digest: Option<String>,
    pub(crate) warnings: Vec<IgnoreWarning>,
}

pub(crate) fn load_ignore(project: &Root) -> Result<LoadedIgnore, ResolveError> {
    match candidate_kind(project, IGNORE_NAME)? {
        None => Ok(LoadedIgnore {
            paths: Vec::new(),
            used: false,
            digest: None,
            warnings: Vec::new(),
        }),
        Some(EntryKind::File) => {
            let content = read_contained(project, Path::new(IGNORE_NAME))?;
            let parsed = parse_ignore(&content.bytes);
            Ok(LoadedIgnore {
                paths: parsed.paths,
                used: true,
                digest: Some(content.whole_digest),
                warnings: parsed.warnings,
            })
        }
        Some(_) => Err(ResolveError::Internal(format!(
            "{IGNORE_NAME} exists but cannot be read as a regular file"
        ))),
    }
}

pub(crate) struct LayerWork<'a> {
    pub(crate) root: &'a Root,
    pub(crate) inventory: &'a Inventory,
    pub(crate) root_kind: RootKind,
    pub(crate) layer_id: &'a str,
    pub(crate) layer_rel: &'a str,
    pub(crate) ignore: &'a [String],
    pub(crate) layers: &'a mut Vec<Layer>,
    pub(crate) edges: &'a mut Vec<Edge>,
    pub(crate) evidence: &'a mut Vec<Evidence>,
    pub(crate) native_paths: &'a mut Vec<String>,
    pub(crate) adopted_files: &'a mut Vec<Adopted>,
    pub(crate) blocking_unknown: &'a mut Option<UnknownReason>,
    pub(crate) excluded_by_product: &'a mut bool,
}

fn consider_layer(work: &mut LayerWork<'_>) -> Result<(), ResolveError> {
    let names: &[&str] = if work.layer_id == GLOBAL_LAYER {
        &[AGENTS_NAME][..]
    } else {
        &[OVERRIDE_NAME, AGENTS_NAME][..]
    };

    let mut candidates = Vec::new();
    let mut adopted: Option<String> = None;
    let mut occupied: Option<String> = None;
    let mut occupied_unknown = false;
    let mut layer_unknown: Option<UnknownReason> = None;

    for name in names {
        let path = layer_file(work.layer_rel, name);
        let kind = match candidate_kind(work.root, &path)? {
            Some(kind) => kind,
            None => {
                candidates.push(Candidate {
                    path,
                    name: (*name).to_string(),
                    existed: false,
                    adopted: false,
                });
                continue;
            }
        };

        record_seen(work, &path);

        if work.ignore.iter().any(|item| item == &path) {
            *work.excluded_by_product = true;
            work.edges.push(edge(
                EdgeKind::ExcludedBy,
                "G4",
                path.clone(),
                work.root_kind,
                Some(IGNORE_NAME.to_string()),
                None,
                None,
                Some("product-user-exclusion".to_string()),
            ));
            candidates.push(Candidate {
                path,
                name: (*name).to_string(),
                existed: true,
                adopted: false,
            });
            continue;
        }

        if let Some(winner) = occupied.clone() {
            if kind == EntryKind::File {
                let (edge_kind, rule_id) =
                    if *name == AGENTS_NAME && winner.ends_with(OVERRIDE_NAME) {
                        (EdgeKind::OverriddenBy, "G2")
                    } else {
                        (EdgeKind::ShadowedBy, "G2")
                    };
                let note = if occupied_unknown && edge_kind == EdgeKind::OverriddenBy {
                    Some("override-unknown".to_string())
                } else {
                    None
                };
                work.edges.push(edge(
                    edge_kind,
                    rule_id,
                    path.clone(),
                    work.root_kind,
                    Some(winner.clone()),
                    None,
                    None,
                    note,
                ));
                if edge_kind == EdgeKind::OverriddenBy {
                    work.edges.push(edge(
                        EdgeKind::ShadowedBy,
                        "G2",
                        path.clone(),
                        work.root_kind,
                        Some(winner),
                        None,
                        None,
                        None,
                    ));
                }
            }
            candidates.push(Candidate {
                path,
                name: (*name).to_string(),
                existed: true,
                adopted: false,
            });
            continue;
        }

        match classify_candidate(work.root, &path, kind)? {
            CandidateOutcome::Adopt { len, digest } => {
                occupied = Some(path.clone());
                adopted = Some(path.clone());
                work.adopted_files.push(Adopted {
                    path: path.clone(),
                    len,
                    root_kind: work.root_kind,
                });
                if let Some(digest) = digest {
                    set_evidence_digest(work, &path, Some(digest));
                }
                work.edges.push(edge(
                    EdgeKind::IncludedBy,
                    if work.layer_id == GLOBAL_LAYER {
                        "G5"
                    } else {
                        "G1"
                    },
                    path.clone(),
                    work.root_kind,
                    None,
                    None,
                    None,
                    None,
                ));
                candidates.push(Candidate {
                    path,
                    name: (*name).to_string(),
                    existed: true,
                    adopted: true,
                });
            }
            CandidateOutcome::Unknown { note } => {
                occupied = Some(path.clone());
                occupied_unknown = true;
                *work.blocking_unknown = Some(UnknownReason::ContentRedactedByPolicy);
                layer_unknown = Some(UnknownReason::ContentRedactedByPolicy);
                work.edges.push(edge(
                    EdgeKind::UnknownBecause,
                    if work.layer_id == GLOBAL_LAYER {
                        "G5"
                    } else {
                        "G1"
                    },
                    path.clone(),
                    work.root_kind,
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

    work.layers.push(Layer {
        id: work.layer_id.to_string(),
        rel: work.layer_rel.to_string(),
        root_kind: work.root_kind,
        candidates,
        adopted,
        unknown: layer_unknown,
    });
    Ok(())
}

pub(crate) enum CandidateOutcome {
    Adopt { len: u64, digest: Option<String> },
    Unknown { note: &'static str },
}

pub(crate) fn classify_candidate(
    root: &Root,
    path: &str,
    kind: EntryKind,
) -> Result<CandidateOutcome, ResolveError> {
    match kind {
        EntryKind::File => {
            let content = read_contained(root, Path::new(path))?;
            if content.link_count > 1 {
                return Ok(CandidateOutcome::Unknown {
                    note: "multiply-linked",
                });
            }
            Ok(CandidateOutcome::Adopt {
                len: u64::try_from(content.bytes.len()).unwrap_or(u64::MAX),
                digest: Some(content.whole_digest),
            })
        }
        EntryKind::Symlink => match root.contain(path) {
            Ok(_) => Ok(CandidateOutcome::Unknown {
                note: "not-regular",
            }),
            Err(Refusal::EscapesRoot { .. }) => Ok(CandidateOutcome::Unknown {
                note: "escapes-root",
            }),
            Err(Refusal::Unresolvable { .. }) => Ok(CandidateOutcome::Unknown {
                note: "unresolvable",
            }),
            Err(Refusal::NotRegular { .. }) => Ok(CandidateOutcome::Unknown {
                note: "not-regular",
            }),
            Err(other) => Err(other.into()),
        },
        EntryKind::Fifo
        | EntryKind::Socket
        | EntryKind::Device
        | EntryKind::Other
        | EntryKind::Directory => Ok(CandidateOutcome::Unknown {
            note: "not-regular",
        }),
    }
}

pub(crate) fn record_seen(work: &mut LayerWork<'_>, path: &str) {
    let native_name = match work.root_kind {
        RootKind::Project => path.to_string(),
        RootKind::CodexHome => format!("$CODEX_HOME/{path}"),
        RootKind::HarnessHome => format!("$HARNESS_HOME/{path}"),
    };
    push_unique(work.native_paths, native_name);
    let digest = work
        .inventory
        .get(path)
        .and_then(|item| item.content_digest.clone());
    set_evidence_digest(work, path, digest);
}

pub(crate) fn set_evidence_digest(work: &mut LayerWork<'_>, path: &str, digest: Option<String>) {
    if let Some(existing) = work
        .evidence
        .iter_mut()
        .find(|item| item.root_kind == work.root_kind && item.path == path)
    {
        if existing.content_digest.is_none() {
            existing.content_digest = digest;
        }
        return;
    }
    work.evidence.push(Evidence {
        root_kind: work.root_kind,
        path: path.to_string(),
        content_digest: digest,
    });
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn edge(
    kind: EdgeKind,
    rule_id: &'static str,
    path: String,
    root_kind: RootKind,
    related_path: Option<String>,
    offset: Option<u64>,
    file_offset: Option<u64>,
    note: Option<String>,
) -> Edge {
    Edge {
        kind,
        rule_id,
        path,
        related_path,
        offset,
        file_offset,
        note,
        root_kind,
    }
}

fn apply_cap(max_bytes: u64, adopted: &[Adopted], edges: &mut Vec<Edge>) -> (u64, bool) {
    let mut used: u64 = 0;
    let mut truncated = false;
    for file in adopted {
        if file.len == 0 {
            continue;
        }
        if used >= max_bytes {
            truncated = true;
            edges.push(edge(
                EdgeKind::TruncatedAfter,
                "G3",
                file.path.clone(),
                file.root_kind,
                None,
                Some(max_bytes),
                Some(0),
                None,
            ));
            continue;
        }
        let remaining = max_bytes - used;
        if file.len > remaining {
            truncated = true;
            used = max_bytes;
            edges.push(edge(
                EdgeKind::TruncatedAfter,
                "G3",
                file.path.clone(),
                file.root_kind,
                None,
                Some(max_bytes),
                Some(remaining),
                None,
            ));
        } else {
            used = used.saturating_add(file.len);
        }
    }
    (used, truncated)
}

pub(crate) fn instruction_claims(included: bool) -> FacetClaims {
    let truth = if included {
        TruthState::Present
    } else {
        TruthState::Absent
    };
    let discoverable = resolved_claim(LifecycleStage::Discoverable, truth, None, false);
    let eligible = resolved_claim(LifecycleStage::Eligible, truth, None, false);
    FacetClaims {
        primary: discoverable.clone(),
        installed: resolved_claim(
            LifecycleStage::Installed,
            TruthState::Indeterminate,
            Some(UnknownReason::NotInstalled),
            false,
        ),
        discoverable,
        eligible,
        model_visible: runtime_claim(),
        use_evidence: runtime_claim_stage(LifecycleStage::UseEvidence),
        outcome_affecting: runtime_claim_stage(LifecycleStage::OutcomeAffecting),
    }
}

fn runtime_claim() -> Claim {
    runtime_claim_stage(LifecycleStage::ModelVisible)
}

fn runtime_claim_stage(stage: LifecycleStage) -> Claim {
    resolved_claim(
        stage,
        TruthState::Indeterminate,
        Some(UnknownReason::RuntimeSnapshotMissing),
        false,
    )
}

fn resolved_claim(
    stage: LifecycleStage,
    truth: TruthState,
    reason: Option<UnknownReason>,
    capability_unexposed: bool,
) -> Claim {
    let (coverage, knowledge, precision) = match truth {
        TruthState::Indeterminate | TruthState::NotApplicable => (
            Coverage::Unknown,
            KnowledgeStatus::Unknown,
            Precision::NotApplicable,
        ),
        TruthState::Present | TruthState::Absent => (
            Coverage::PartialDeclaredSurface,
            KnowledgeStatus::Current,
            Precision::Derived,
        ),
    };
    Claim {
        claim_kind: ClaimKind::Resolved,
        coverage,
        knowledge_status: knowledge,
        lifecycle_stage: stage,
        precision,
        provenance: Provenance::OfficialSpec,
        truth_state: truth,
        use_evidence_kind: None,
        decision: None,
        experiment: ExperimentRef::default(),
        unknown_reason: reason,
        capability_unexposed,
        has_timeline_events: false,
        contradicted_by_equal_coverage: false,
        filled_from_higher_provenance_outside_coverage: false,
        // C-F04: name the producer. Every claim this crate emits is a static
        // resolution over the grammar's declared surface; no clock reading or
        // single evidence id is wired per claim, so basis/evaluated_at stay
        // unset rather than invented.
        source: Some(ClaimSource::produced_by(
            "ctxpect-resolve::resolved_claim",
            Provenance::OfficialSpec,
        )),
    }
}

pub(crate) fn ensure_expressible(claims: &FacetClaims) -> Result<(), ResolveError> {
    for claim in claims.all() {
        let violations = claim.violations();
        if !violations.is_empty() {
            let detail = violations
                .iter()
                .map(|item| item.to_string())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(ResolveError::Internal(format!(
                "resolver produced a non-expressible claim: {detail}"
            )));
        }
    }
    Ok(())
}

pub(crate) fn push_unique(items: &mut Vec<String>, value: String) {
    if !items.iter().any(|item| item == &value) {
        items.push(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layers_include_the_root_and_each_cwd_step() {
        assert_eq!(layers_toward_cwd(""), vec![""]);
        assert_eq!(layers_toward_cwd("src/app"), vec!["", "src", "src/app"]);
        assert_eq!(layers_toward_cwd("/src/"), vec!["", "src"]);
    }

    #[test]
    fn coordinate_reason_prefers_os_lane_then_surface_then_version() {
        assert_eq!(
            coordinate_unknown_reason("codex", "0.147.0", "cli", "ubuntu-24.04-x86_64"),
            Some(UnknownReason::OfficialDistributionNotCaptured)
        );
        assert_eq!(
            coordinate_unknown_reason("codex", "unknown-honesty", "desktop", "macos-27-arm64"),
            Some(UnknownReason::SurfaceNotExposed)
        );
        assert_eq!(
            coordinate_unknown_reason("codex", "0.99.0", "cli", "macos-27-arm64"),
            Some(UnknownReason::UnsupportedHarnessVersion)
        );
        assert_eq!(
            coordinate_unknown_reason("codex", "0.147.0", "cli", "macos-27-arm64"),
            None
        );
        // The second anchor dispatches on its own coordinate; a family with
        // no anchor at all is not captured.
        assert_eq!(
            coordinate_unknown_reason("claude-code", "2.1.259", "cli", "macos-27-arm64"),
            None
        );
        assert_eq!(
            coordinate_unknown_reason("claude-code", "unknown-honesty", "cloud", "macos-27-arm64"),
            Some(UnknownReason::SurfaceNotExposed)
        );
        assert_eq!(
            coordinate_unknown_reason("claude-code", "2.1.259", "cli", "ubuntu-24.04-x86_64"),
            Some(UnknownReason::OfficialDistributionNotCaptured)
        );
        // A family with no anchor: a surface no anchor exposes is
        // surface_not_exposed; a lane no anchor captured is not captured.
        assert_eq!(
            coordinate_unknown_reason("cursor", "3.19.7", "ide", "macos-27-arm64"),
            Some(UnknownReason::SurfaceNotExposed)
        );
        assert_eq!(
            coordinate_unknown_reason("aider", "0.86.2", "cli", "ubuntu-24.04-x86_64"),
            Some(UnknownReason::OfficialDistributionNotCaptured)
        );
        assert_eq!(
            coordinate_unknown_reason("aider", "0.86.2", "cli", "macos-27-arm64"),
            Some(UnknownReason::OfficialDistributionNotCaptured)
        );
        assert_eq!(
            anchor_for("claude-code", "2.1.259", "cli", "macos-27-arm64").map(|a| a.grammar),
            Some(Grammar::ClaudeCodeInstructions)
        );
        assert_eq!(implemented_capabilities("claude-code"), &[INSTRUCTIONS]);
        assert!(implemented_capabilities("cursor").is_empty());
    }

    #[test]
    fn honesty_and_instruction_claims_are_expressible() {
        for reason in [
            UnknownReason::OfficialDistributionNotCaptured,
            UnknownReason::SurfaceNotExposed,
            UnknownReason::UnsupportedHarnessVersion,
            UnknownReason::PermissionNotGranted,
            UnknownReason::ContentRedactedByPolicy,
        ] {
            let claims = honesty_claims(reason, reason == UnknownReason::SurfaceNotExposed);
            for claim in claims.all() {
                assert!(
                    claim.violations().is_empty(),
                    "{reason}: {:?}",
                    claim.violations()
                );
            }
        }
        for included in [true, false] {
            let claims = instruction_claims(included);
            for claim in claims.all() {
                assert!(
                    claim.violations().is_empty(),
                    "included={included}: {:?}",
                    claim.violations()
                );
            }
            assert_eq!(claims.primary.claim_kind, ClaimKind::Resolved);
            assert_eq!(claims.primary.lifecycle_stage, LifecycleStage::Discoverable);
            assert!(claims.model_visible.lifecycle_stage.is_runtime_facet());
            assert_eq!(
                claims.model_visible.unknown_reason,
                Some(UnknownReason::RuntimeSnapshotMissing)
            );
        }
    }

    #[test]
    fn parse_ignore_trims_and_skips_blank_lines() {
        let parsed = parse_ignore(b"AGENTS.md\n\n  src/AGENTS.md  \n");
        assert_eq!(parsed.paths, vec!["AGENTS.md", "src/AGENTS.md"]);
        assert!(parsed.warnings.is_empty());
    }

    #[test]
    fn parse_ignore_skips_absolute_parent_and_control_lines() {
        let parsed = parse_ignore(b"/etc/passwd\nAGENTS.md\n../x\nfoo/\x07bar\n");
        assert_eq!(parsed.paths, vec!["AGENTS.md"]);
        assert_eq!(
            parsed
                .warnings
                .iter()
                .map(|item| item.line)
                .collect::<Vec<_>>(),
            vec![1, 3, 4]
        );
        assert!(
            parsed
                .warnings
                .iter()
                .all(|item| item.code == "ignore.invalid_line")
        );
        let dumped = format!("{parsed:?}");
        assert!(!dumped.contains("/etc/passwd"));
        assert!(!dumped.contains("../x"));
    }

    #[test]
    fn apply_cap_zero_byte_tail_after_exact_cap_is_not_truncated() {
        let files = [
            Adopted {
                path: "AGENTS.md".to_string(),
                len: PROJECT_DOC_MAX_BYTES,
                root_kind: RootKind::Project,
            },
            Adopted {
                path: "src/AGENTS.md".to_string(),
                len: 0,
                root_kind: RootKind::Project,
            },
        ];
        let mut edges = Vec::new();
        let (used, truncated) = apply_cap(PROJECT_DOC_MAX_BYTES, &files, &mut edges);
        assert_eq!(used, PROJECT_DOC_MAX_BYTES);
        assert!(!truncated);
        assert!(
            edges
                .iter()
                .all(|edge| edge.kind != EdgeKind::TruncatedAfter)
        );
    }

    #[test]
    fn apply_cap_byte_after_exact_cap_is_truncated_at_file_offset_zero() {
        let files = [
            Adopted {
                path: "AGENTS.md".to_string(),
                len: PROJECT_DOC_MAX_BYTES,
                root_kind: RootKind::Project,
            },
            Adopted {
                path: "src/AGENTS.md".to_string(),
                len: 1,
                root_kind: RootKind::Project,
            },
        ];
        let mut edges = Vec::new();
        let (used, truncated) = apply_cap(PROJECT_DOC_MAX_BYTES, &files, &mut edges);
        assert_eq!(used, PROJECT_DOC_MAX_BYTES);
        assert!(truncated);
        assert!(edges.iter().any(|edge| {
            edge.kind == EdgeKind::TruncatedAfter
                && edge.path == "src/AGENTS.md"
                && edge.offset == Some(PROJECT_DOC_MAX_BYTES)
                && edge.file_offset == Some(0)
        }));
    }
}
