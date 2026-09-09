//! Effect Lab. Decisions come only from a frozen ExperimentContract and
//! externally supplied per-run results; nothing here runs a harness or
//! manufactures an observation.
//!
//! # Honesty rules
//!
//! - Without a runs document there is no experiment: the result says
//!   `executed: false` with `effect.runs_required` and carries no decision.
//! - A runs document is checked against the contract (F-15): sample size,
//!   arm balance, confounder lock (code / model / harness / tool
//!   availability), run time after the contract freeze, duplicate runs. Any
//!   protocol deviation makes the experiment `inconclusive` with a specific
//!   reason code; it never yields `supported-*`.
//! - Estimation is an [`Estimator`] trait. The product build carries one
//!   frozen, in-tree implementation, [`PairedExactBinomial`] (ADR 0006): the
//!   exact conditional test on discordant pairs (McNemar exact / sign test)
//!   for direction, and an exact Clopper–Pearson interval on the paired
//!   difference for equivalence. No third-party statistics crate is used.
//!   Anything the frozen procedure cannot support is `inconclusive` with
//!   `effect.estimator_inconclusive` and the numbers it looked at.
//! - Non-completion outcomes (`timeout`, `crash`, `refusal`, `missing`) are
//!   handled by the contract's ITT rule and listed in `deviations[]`.

pub use ctxpect_schema::Value;
use ctxpect_schema::{array, object, string};
use std::collections::BTreeMap;

/// Schema of the runs document `ctxpect experiment --runs <file>` and
/// `POST /api/v1/lab` accept.
pub const RUNS_SCHEMA: &str = "ctxpect-effect-runs-v1";
/// Schema of the result document.
pub const RESULT_SCHEMA: &str = "ctxpect-effect-result-v1";
/// Schema of the frozen contract.
pub const CONTRACT_SCHEMA: &str = "experiment-contract-v1";

/// The only decisions the lab may reach. `inconclusive` is the default.
pub const DECISIONS: &[&str] = &[
    "supported-beneficial",
    "supported-harmful",
    "supported-equivalent-within-margin",
    "inconclusive",
];

/// Run outcomes a runs document may carry.
pub const OUTCOMES: &[&str] = &["pass", "fail", "timeout", "crash", "refusal", "missing"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectError {
    pub code: &'static str,
    pub message: String,
}

impl EffectError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// Values locked before the first run; a run that drifts from any of them
/// is a confounder deviation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LockedConfounders {
    pub code_digest: String,
    pub model: String,
    pub harness: String,
    pub tool_availability_digest: String,
}

/// The frozen pre-registration (PRD F-15).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperimentContract {
    pub experiment_id: String,
    /// The single primary outcome, e.g. `task-pass`.
    pub primary_outcome: String,
    /// Minimum effect of interest / equivalence margin, in percentage points
    /// of the primary outcome rate.
    pub margin_pp: i64,
    /// `paired-by-task` or `randomized-order`.
    pub pairing: String,
    /// Planned number of control/treatment pairs.
    pub n_planned: i64,
    /// Pre-registered alpha and power, kept as the decimal text they were
    /// registered with; no arithmetic is done on them here.
    pub alpha: String,
    pub power: String,
    /// `none` or a named correction.
    pub multiplicity: String,
    /// Intention-to-treat rule for non-completion outcomes: `count-as-fail`
    /// or `exclude-pair`.
    pub itt: String,
    pub locked: LockedConfounders,
    /// When the contract was frozen (`secs.millisZ`, the store clock format,
    /// or RFC 3339); runs must start at or after it.
    pub frozen_at: String,
    /// Conditions that invalidate the experiment as pre-registered.
    pub invalidation: Vec<String>,
}

impl ExperimentContract {
    pub fn to_value(&self) -> Value {
        object([
            ("schema", string(CONTRACT_SCHEMA)),
            ("experiment_id", string(&self.experiment_id)),
            ("primary_outcome", string(&self.primary_outcome)),
            ("margin_pp", Value::Int(self.margin_pp)),
            ("pairing", string(&self.pairing)),
            ("n_planned", Value::Int(self.n_planned)),
            ("alpha", string(&self.alpha)),
            ("power", string(&self.power)),
            ("multiplicity", string(&self.multiplicity)),
            ("itt", string(&self.itt)),
            (
                "locked",
                object([
                    ("code_digest", string(&self.locked.code_digest)),
                    ("model", string(&self.locked.model)),
                    ("harness", string(&self.locked.harness)),
                    (
                        "tool_availability_digest",
                        string(&self.locked.tool_availability_digest),
                    ),
                ]),
            ),
            ("frozen_at", string(&self.frozen_at)),
            (
                "invalidation",
                array(self.invalidation.iter().map(string)),
            ),
            ("n_locked", Value::Bool(true)),
        ])
    }

    /// Parse and validate a contract object. Every F-15 field is required.
    pub fn from_value(value: &Value) -> Result<Self, EffectError> {
        let text = |key: &str| -> Result<String, EffectError> {
            value
                .get(key)
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .ok_or_else(|| {
                    EffectError::new(
                        "effect.contract_invalid",
                        format!("contract lacks `{key}` (F-15 requires it)"),
                    )
                })
        };
        let int = |key: &str| -> Result<i64, EffectError> {
            value.get(key).and_then(Value::as_i64).ok_or_else(|| {
                EffectError::new(
                    "effect.contract_invalid",
                    format!("contract lacks integer `{key}` (F-15 requires it)"),
                )
            })
        };
        if value.get("schema").and_then(Value::as_str) != Some(CONTRACT_SCHEMA) {
            return Err(EffectError::new(
                "effect.contract_invalid",
                format!("contract.schema must be {CONTRACT_SCHEMA}"),
            ));
        }
        let locked = value
            .get("locked")
            .ok_or_else(|| EffectError::new("effect.contract_invalid", "contract lacks `locked` confounders"))?;
        let locked_text = |key: &str| -> Result<String, EffectError> {
            locked
                .get(key)
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .ok_or_else(|| {
                    EffectError::new(
                        "effect.contract_invalid",
                        format!("contract.locked lacks `{key}`"),
                    )
                })
        };
        let pairing = text("pairing")?;
        if pairing != "paired-by-task" && pairing != "randomized-order" {
            return Err(EffectError::new(
                "effect.contract_invalid",
                "contract.pairing must be paired-by-task or randomized-order",
            ));
        }
        let itt = text("itt")?;
        if itt != "count-as-fail" && itt != "exclude-pair" {
            return Err(EffectError::new(
                "effect.contract_invalid",
                "contract.itt must be count-as-fail or exclude-pair",
            ));
        }
        let n_planned = int("n_planned")?;
        let margin_pp = int("margin_pp")?;
        if n_planned < 1 || !(0..=100).contains(&margin_pp) {
            return Err(EffectError::new(
                "effect.contract_invalid",
                "contract.n_planned must be >= 1 and margin_pp within 0..=100",
            ));
        }
        let alpha = text("alpha")?;
        if !alpha.parse::<f64>().is_ok_and(|a| a > 0.0 && a <= MAX_ALPHA) {
            return Err(EffectError::new(
                "effect.contract_invalid",
                format!("contract.alpha must be a decimal in (0, {MAX_ALPHA}] (F-15: alpha <= 0.05)"),
            ));
        }
        let frozen_at = text("frozen_at")?;
        if epoch_seconds(&frozen_at).is_none() {
            return Err(EffectError::new(
                "effect.contract_invalid",
                "contract.frozen_at must be a store clock reading (`<secs>.<ms>Z`) or an RFC 3339 timestamp",
            ));
        }
        Ok(Self {
            experiment_id: text("experiment_id")?,
            primary_outcome: text("primary_outcome")?,
            margin_pp,
            pairing,
            n_planned,
            alpha,
            power: text("power")?,
            multiplicity: text("multiplicity")?,
            itt,
            locked: LockedConfounders {
                code_digest: locked_text("code_digest")?,
                model: locked_text("model")?,
                harness: locked_text("harness")?,
                tool_availability_digest: locked_text("tool_availability_digest")?,
            },
            frozen_at,
            invalidation: value
                .get("invalidation")
                .and_then(Value::as_array)
                .map(|items| items.iter().filter_map(Value::as_str).map(str::to_string).collect())
                .unwrap_or_default(),
        })
    }
}

/// One externally observed run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub run_id: String,
    pub arm: String,
    pub task_id: String,
    pub outcome: String,
    pub code_digest: String,
    pub model: String,
    pub harness: String,
    pub tool_availability_digest: String,
    pub started_at: String,
    pub ended_at: String,
    pub receipt_id: Option<String>,
}

impl Run {
    fn from_value(value: &Value, index: usize) -> Result<Self, EffectError> {
        let text = |key: &str| -> Result<String, EffectError> {
            value
                .get(key)
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .ok_or_else(|| {
                    EffectError::new(
                        "effect.run_malformed",
                        format!("runs[{index}] lacks `{key}`"),
                    )
                })
        };
        let arm = text("arm")?;
        if arm != "control" && arm != "treatment" {
            return Err(EffectError::new(
                "effect.run_malformed",
                format!("runs[{index}].arm must be control or treatment"),
            ));
        }
        let outcome = text("outcome")?;
        if !OUTCOMES.contains(&outcome.as_str()) {
            return Err(EffectError::new(
                "effect.run_malformed",
                format!("runs[{index}].outcome is not one of {OUTCOMES:?}"),
            ));
        }
        Ok(Self {
            run_id: text("run_id")?,
            arm,
            task_id: text("task_id")?,
            outcome,
            code_digest: text("code_digest")?,
            model: text("model")?,
            harness: text("harness")?,
            tool_availability_digest: text("tool_availability_digest")?,
            started_at: text("started_at")?,
            ended_at: text("ended_at")?,
            receipt_id: value.get("receipt_id").and_then(Value::as_str).map(str::to_string),
        })
    }

    /// The run summary that is persisted: identifiers and outcome, no body.
    fn summary(&self, counted_as: &str) -> Value {
        object([
            ("run_id", string(&self.run_id)),
            ("arm", string(&self.arm)),
            ("task_id", string(&self.task_id)),
            ("outcome", string(&self.outcome)),
            ("counted_as", string(counted_as)),
            ("started_at", string(&self.started_at)),
            ("ended_at", string(&self.ended_at)),
            (
                "receipt_id",
                self.receipt_id.as_deref().map_or(Value::Null, string),
            ),
        ])
    }
}

/// A parsed runs document: the frozen contract plus the observed runs.
#[derive(Debug, Clone)]
pub struct RunsDocument {
    pub contract: ExperimentContract,
    pub runs: Vec<Run>,
}

impl RunsDocument {
    /// Parse a `ctxpect-effect-runs-v1` document. Structural problems are
    /// errors (the input is not an experiment); protocol problems are judged
    /// later by [`decide_with`] and yield `inconclusive`.
    pub fn from_value(value: &Value) -> Result<Self, EffectError> {
        if value.get("schema").and_then(Value::as_str) != Some(RUNS_SCHEMA) {
            return Err(EffectError::new(
                "effect.runs_invalid",
                format!("runs document schema must be {RUNS_SCHEMA}"),
            ));
        }
        let contract = ExperimentContract::from_value(
            value
                .get("contract")
                .ok_or_else(|| EffectError::new("effect.runs_invalid", "runs document lacks `contract`"))?,
        )?;
        let runs = value
            .get("runs")
            .and_then(Value::as_array)
            .ok_or_else(|| EffectError::new("effect.runs_invalid", "runs document lacks `runs[]`"))?
            .iter()
            .enumerate()
            .map(|(index, run)| Run::from_value(run, index))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { contract, runs })
    }
}

/// A control/treatment pair for one task after ITT handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pair {
    pub control_pass: bool,
    pub treatment_pass: bool,
}

/// What an estimator says about a set of pairs.
#[derive(Debug, Clone, PartialEq)]
pub struct Estimate {
    /// One of [`DECISIONS`].
    pub decision: &'static str,
    /// Point estimate of the treatment effect in percentage points.
    pub effect_pp: i64,
    pub note: String,
    /// The numbers the decision rests on (test statistic, interval, counts),
    /// recorded verbatim so the decision can be re-derived.
    pub detail: Value,
}

/// What an estimator says about a set of pairs.
impl Estimate {
    fn inconclusive(effect_pp: i64, note: impl Into<String>) -> Self {
        Self {
            decision: "inconclusive",
            effect_pp,
            note: note.into(),
            detail: Value::Null,
        }
    }
}

/// A frozen statistical procedure.
pub trait Estimator {
    /// Name recorded in the result.
    fn name(&self) -> &str;
    fn estimate(&self, contract: &ExperimentContract, pairs: &[Pair]) -> Estimate;
}

/// Reason a build without an estimator cannot estimate.
pub const ESTIMATOR_UNAVAILABLE_NOTE: &str =
    "no statistical implementation is wired; the decision stays inconclusive";

/// The frozen in-tree estimator (ADR 0006), version 1.
///
/// Inputs are the pairs after ITT handling; only the primary binary outcome
/// (`pass` / not) is used. With `b` = pairs where treatment passed and
/// control failed, `c` = the reverse, `m = b + c` discordant pairs and `n`
/// pairs in total:
///
/// - **Direction**: the exact two-sided test of `b` against
///   `Binomial(m, 1/2)` (McNemar's exact conditional test, equivalently the
///   sign test). `p < alpha` with `b > c` → `supported-beneficial`, with
///   `c > b` → `supported-harmful`.
/// - **Equivalence**: an exact Clopper–Pearson interval at confidence
///   `1 - 2·alpha` (two one-sided tests at `alpha`) for `b / m`, mapped to
///   the paired difference `d = (2·b/m − 1)·m/n` in percentage points; when
///   `m = 0` the bound is the one-sided exact upper limit of the discordance
///   rate, `1 − alpha^(1/n)`. The interval strictly inside
///   `(−margin_pp, +margin_pp)` → `supported-equivalent-within-margin`.
/// - Otherwise `inconclusive`. "Not significant" is never "equivalent".
///
/// Multiplicity corrections other than `none` are not implemented in v1 and
/// yield `inconclusive`. Power is pre-registered in the contract and is not
/// recomputed here. Everything is `f64` arithmetic in log space; no
/// dependency is used.
pub struct PairedExactBinomial;

/// The name recorded for [`PairedExactBinomial`]. v1 read the discordant
/// rate `m/n` as a known quantity, so eleven pairs with one discordant pair
/// came out "equivalent"; v2 bounds that rate exactly as well, and the name
/// changed with the rule (ADR 0006).
pub const PAIRED_EXACT_BINOMIAL_V2: &str = "paired-exact-binomial-v2";

/// The largest alpha a contract may pre-register (F-15: alpha <= 0.05).
pub const MAX_ALPHA: f64 = 0.05;

impl Estimator for PairedExactBinomial {
    fn name(&self) -> &str {
        PAIRED_EXACT_BINOMIAL_V2
    }

    fn estimate(&self, contract: &ExperimentContract, pairs: &[Pair]) -> Estimate {
        let n = pairs.len();
        let b = pairs.iter().filter(|p| p.treatment_pass && !p.control_pass).count();
        let c = pairs.iter().filter(|p| p.control_pass && !p.treatment_pass).count();
        let m = b + c;
        let effect_pp = if n == 0 {
            0
        } else {
            ((b as f64 - c as f64) * 100.0 / n as f64).round() as i64
        };
        let Some(alpha) = contract.alpha.parse::<f64>().ok().filter(|a| *a > 0.0 && *a < 0.5) else {
            return Estimate::inconclusive(effect_pp, "contract.alpha is not a decimal in (0, 0.5)");
        };
        if contract.multiplicity != "none" {
            return Estimate::inconclusive(
                effect_pp,
                format!("multiplicity correction `{}` is not implemented in {PAIRED_EXACT_BINOMIAL_V2}", contract.multiplicity),
            );
        }
        if n == 0 {
            return Estimate::inconclusive(0, "no pairs");
        }
        let margin = contract.margin_pp as f64;
        // Direction: exact two-sided binomial test on the discordant pairs.
        let p_value = if m == 0 {
            1.0
        } else {
            (2.0 * binomial_cdf(b.min(c), m, 0.5)).min(1.0)
        };
        // Equivalence: a conservative exact interval for the paired
        // difference d = (2q − 1)·r, where r = discordant rate (m/n) and
        // q = treatment-only share of the discordant pairs (b/m). Both r and
        // q get an exact Clopper–Pearson interval at one-sided `alpha`; d is
        // bilinear in (q, r), so its range over the box is attained at the
        // corners. Reading r as its point estimate — v1's mistake — let a
        // single discordant pair among eleven pass as "equivalent".
        let (r_lo, r_hi) = if m == 0 {
            (0.0, 1.0 - alpha.powf(1.0 / n as f64))
        } else {
            clopper_pearson(m, n, alpha)
        };
        let (lo_pp, hi_pp) = if m == 0 {
            (-r_hi * 100.0, r_hi * 100.0)
        } else {
            let (q_lo, q_hi) = clopper_pearson(b, m, alpha);
            let corners = [
                (2.0 * q_lo - 1.0) * r_lo,
                (2.0 * q_lo - 1.0) * r_hi,
                (2.0 * q_hi - 1.0) * r_lo,
                (2.0 * q_hi - 1.0) * r_hi,
            ];
            let lo = corners.iter().cloned().fold(f64::INFINITY, f64::min);
            let hi = corners.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            (lo * 100.0, hi * 100.0)
        };
        let detail = object([
            ("pairs", Value::Int(n as i64)),
            ("discordant", Value::Int(m as i64)),
            ("treatment_only_pass", Value::Int(b as i64)),
            ("control_only_pass", Value::Int(c as i64)),
            ("p_value", string(format!("{p_value:.6}"))),
            ("alpha", string(&contract.alpha)),
            ("difference_ci_pp", array([string(format!("{lo_pp:.2}")), string(format!("{hi_pp:.2}"))])),
            ("discordant_rate_ci", array([string(format!("{r_lo:.4}")), string(format!("{r_hi:.4}"))])),
            ("decision_precedence", string("direction-before-equivalence")),
            ("ci_confidence", string(format!("{:.3}", 1.0 - 2.0 * alpha))),
            ("margin_pp", Value::Int(contract.margin_pp)),
        ]);
        let (decision, note) = if p_value < alpha && b != c {
            if b > c {
                ("supported-beneficial", "exact paired test rejects no-difference in favour of treatment")
            } else {
                ("supported-harmful", "exact paired test rejects no-difference in favour of control")
            }
        } else if lo_pp > -margin && hi_pp < margin {
            (
                "supported-equivalent-within-margin",
                "the exact interval for the paired difference lies strictly inside the equivalence margin",
            )
        } else {
            (
                "inconclusive",
                "neither a direction at alpha nor equivalence within the margin is supported by these pairs; not-significant is not equivalent",
            )
        };
        Estimate {
            decision,
            effect_pp,
            note: note.into(),
            detail,
        }
    }
}

/// `ln(k!)` by summation; exact enough for the sample sizes a lab sees.
fn ln_factorial(k: usize) -> f64 {
    (1..=k).map(|i| (i as f64).ln()).sum()
}

/// `P(X = k)` for `X ~ Binomial(m, p)`, computed in log space.
fn binomial_pmf(k: usize, m: usize, p: f64) -> f64 {
    if p <= 0.0 {
        return if k == 0 { 1.0 } else { 0.0 };
    }
    if p >= 1.0 {
        return if k == m { 1.0 } else { 0.0 };
    }
    let log = ln_factorial(m) - ln_factorial(k) - ln_factorial(m - k)
        + k as f64 * p.ln()
        + (m - k) as f64 * (1.0 - p).ln();
    log.exp()
}

/// `P(X <= k)` for `X ~ Binomial(m, p)`.
fn binomial_cdf(k: usize, m: usize, p: f64) -> f64 {
    (0..=k.min(m)).map(|i| binomial_pmf(i, m, p)).sum::<f64>().min(1.0)
}

/// Exact Clopper–Pearson bounds for a proportion `b / m`, each side at
/// level `alpha` (so the interval has confidence `1 − 2·alpha`), found by
/// bisection on the exact binomial tails.
fn clopper_pearson(b: usize, m: usize, alpha: f64) -> (f64, f64) {
    let lower = if b == 0 {
        0.0
    } else {
        // Smallest p with P(X >= b | p) = alpha; P(X >= b) increases in p.
        bisect(|p| 1.0 - binomial_cdf(b - 1, m, p) - alpha)
    };
    let upper = if b == m {
        1.0
    } else {
        // Largest p with P(X <= b | p) = alpha; P(X <= b) decreases in p.
        bisect(|p| alpha - binomial_cdf(b, m, p))
    };
    (lower, upper)
}

/// Root of a function that is increasing on [0, 1].
fn bisect(f: impl Fn(f64) -> f64) -> f64 {
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    for _ in 0..80 {
        let mid = (lo + hi) / 2.0;
        if f(mid) < 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

/// Judge a runs document with the frozen in-tree estimator: the product path.
pub fn decide(document: &RunsDocument) -> Value {
    decide_with(document, Some(&PairedExactBinomial))
}

/// Judge a runs document against its contract, then hand valid pairs to the
/// estimator if there is one. Every protocol deviation is recorded; the
/// first invalidating one fixes the reason code.
pub fn decide_with(document: &RunsDocument, estimator: Option<&dyn Estimator>) -> Value {
    let contract = &document.contract;
    let mut deviations: Vec<Value> = Vec::new();
    let mut invalidating: Option<&'static str> = None;
    let mut note = |code: &'static str, detail: String, invalidates: bool, out: &mut Vec<Value>| {
        out.push(object([
            ("reason_code", string(code)),
            ("detail", string(detail)),
            ("invalidates", Value::Bool(invalidates)),
        ]));
        if invalidates && invalidating.is_none() {
            invalidating = Some(code);
        }
    };

    // Duplicate run ids.
    let mut seen = std::collections::BTreeSet::new();
    for run in &document.runs {
        if !seen.insert(run.run_id.as_str()) {
            note(
                "effect.run_duplicate",
                format!("run_id `{}` appears more than once", run.run_id),
                true,
                &mut deviations,
            );
        }
    }
    // Confounder lock and freeze time.
    for run in &document.runs {
        let mut drift = Vec::new();
        if run.code_digest != contract.locked.code_digest {
            drift.push("code_digest");
        }
        if run.model != contract.locked.model {
            drift.push("model");
        }
        if run.harness != contract.locked.harness {
            drift.push("harness");
        }
        if run.tool_availability_digest != contract.locked.tool_availability_digest {
            drift.push("tool_availability_digest");
        }
        if !drift.is_empty() {
            note(
                "effect.confounder_drift",
                format!("run `{}` differs from the locked values in {}", run.run_id, drift.join(", ")),
                true,
                &mut deviations,
            );
        }
        match (epoch_seconds(&run.started_at), epoch_seconds(&run.ended_at), epoch_seconds(&contract.frozen_at)) {
            (Some(started), Some(ended), Some(frozen)) => {
                if started < frozen {
                    note(
                        "effect.run_precedes_freeze",
                        format!("run `{}` started before the contract was frozen", run.run_id),
                        true,
                        &mut deviations,
                    );
                }
                if ended < started {
                    note(
                        "effect.run_time_invalid",
                        format!("run `{}` ended before it started", run.run_id),
                        true,
                        &mut deviations,
                    );
                }
            }
            _ => note(
                "effect.run_time_invalid",
                format!("run `{}` has a time that is neither a store clock reading nor RFC 3339", run.run_id),
                true,
                &mut deviations,
            ),
        }
    }
    // Every (task, arm) is one observation; a second run for the same slot
    // would otherwise silently replace the first, and the order of the
    // document would pick the result.
    {
        let mut seen: BTreeMap<(&str, &str), &str> = BTreeMap::new();
        for run in &document.runs {
            if let Some(first) = seen.insert((run.task_id.as_str(), run.arm.as_str()), run.run_id.as_str()) {
                note(
                    "effect.run_duplicate",
                    format!("runs `{first}` and `{}` both observe task `{}` arm `{}`", run.run_id, run.task_id, run.arm),
                    true,
                    &mut deviations,
                );
            }
        }
    }
    // Arms and pairs.
    let controls = document.runs.iter().filter(|r| r.arm == "control").count();
    let treatments = document.runs.iter().filter(|r| r.arm == "treatment").count();
    if controls != treatments {
        note(
            "effect.arm_unbalanced",
            format!("{controls} control runs vs {treatments} treatment runs"),
            true,
            &mut deviations,
        );
    }
    let mut by_task: BTreeMap<&str, (Option<&Run>, Option<&Run>)> = BTreeMap::new();
    for run in &document.runs {
        let slot = by_task.entry(run.task_id.as_str()).or_default();
        if run.arm == "control" {
            slot.0 = Some(run);
        } else {
            slot.1 = Some(run);
        }
    }
    let mut pairs = Vec::new();
    let mut run_rows = Vec::new();
    for (task, (control, treatment)) in &by_task {
        let (Some(control), Some(treatment)) = (control, treatment) else {
            note(
                "effect.pair_incomplete",
                format!("task `{task}` has only one arm"),
                true,
                &mut deviations,
            );
            for run in [control, treatment].into_iter().flatten() {
                run_rows.push(run.summary("unpaired"));
            }
            continue;
        };
        let non_completion = [control, treatment]
            .iter()
            .any(|run| run.outcome != "pass" && run.outcome != "fail");
        let counted = if non_completion {
            for run in [control, treatment] {
                if run.outcome != "pass" && run.outcome != "fail" {
                    note(
                        "effect.itt_applied",
                        format!(
                            "run `{}` outcome `{}` handled by ITT rule `{}`",
                            run.run_id, run.outcome, contract.itt
                        ),
                        false,
                        &mut deviations,
                    );
                }
            }
            if contract.itt == "exclude-pair" {
                "excluded-pair"
            } else {
                "counted-as-fail"
            }
        } else {
            "observed"
        };
        run_rows.push(control.summary(counted));
        run_rows.push(treatment.summary(counted));
        if counted == "excluded-pair" {
            continue;
        }
        pairs.push(Pair {
            control_pass: control.outcome == "pass",
            treatment_pass: treatment.outcome == "pass",
        });
    }
    let n = i64::try_from(pairs.len()).unwrap_or(0);
    if n < contract.n_planned {
        note(
            "effect.n_insufficient",
            format!("{n} usable pairs, contract plans {}", contract.n_planned),
            true,
            &mut deviations,
        );
    } else if n > contract.n_planned {
        // A fixed-sample contract is fixed in both directions: adding pairs
        // until significance is optional stopping, not the pre-registered
        // test.
        note(
            "effect.n_mismatch",
            format!("{n} usable pairs, contract plans exactly {}; a fixed-sample design cannot take extra observations", contract.n_planned),
            true,
            &mut deviations,
        );
    }

    let (decision, reason, estimator_doc, effect_pp) = match (invalidating, estimator) {
        (Some(code), _) => ("inconclusive", code, estimator_note(estimator), Value::Null),
        (None, None) => (
            "inconclusive",
            "effect.estimator_unavailable",
            estimator_note(None),
            Value::Null,
        ),
        (None, Some(estimator)) => {
            let estimate = estimator.estimate(contract, &pairs);
            let decision = if DECISIONS.contains(&estimate.decision) {
                estimate.decision
            } else {
                "inconclusive"
            };
            (
                decision,
                if decision == "inconclusive" { "effect.estimator_inconclusive" } else { "ok" },
                object([
                    ("available", Value::Bool(true)),
                    ("name", string(estimator.name())),
                    ("note", string(&estimate.note)),
                    ("detail", estimate.detail),
                ]),
                Value::Int(estimate.effect_pp),
            )
        }
    };
    result(
        contract,
        Some(decision),
        reason,
        n,
        run_rows,
        deviations,
        estimator_doc,
        effect_pp,
    )
}

fn estimator_note(estimator: Option<&dyn Estimator>) -> Value {
    match estimator {
        Some(estimator) => object([
            ("available", Value::Bool(true)),
            ("name", string(estimator.name())),
        ]),
        None => object([
            ("available", Value::Bool(false)),
            ("reason_code", string("effect.estimator_unavailable")),
            ("note", string(ESTIMATOR_UNAVAILABLE_NOTE)),
        ]),
    }
}

/// The result for an experiment that was never executed: no runs, no
/// decision. `experiment_id` is all that is known.
#[must_use]
pub fn not_executed(experiment_id: &str, n_planned: Option<i64>) -> Value {
    object([
        ("schema", string(RESULT_SCHEMA)),
        ("experiment_id", string(experiment_id)),
        ("executed", Value::Bool(false)),
        ("decision", Value::Null),
        ("reason_code", string("effect.runs_required")),
        (
            "note",
            string("no per-run results were supplied; the lab does not run a harness and does not manufacture observations"),
        ),
        ("n", Value::Int(0)),
        ("n_planned", n_planned.map_or(Value::Null, Value::Int)),
        ("causal", Value::Bool(false)),
        ("single_ab_is_causal", Value::Bool(false)),
        ("inconclusive_is_absent", Value::Bool(false)),
        ("runs", array([])),
        ("deviations", array([])),
        ("estimator", estimator_note(None)),
    ])
}

#[allow(clippy::too_many_arguments)]
fn result(
    contract: &ExperimentContract,
    decision: Option<&str>,
    reason: &str,
    n: i64,
    runs: Vec<Value>,
    deviations: Vec<Value>,
    estimator: Value,
    effect_pp: Value,
) -> Value {
    object([
        ("schema", string(RESULT_SCHEMA)),
        ("experiment_id", string(&contract.experiment_id)),
        ("executed", Value::Bool(true)),
        ("decision", decision.map_or(Value::Null, string)),
        ("reason_code", string(reason)),
        ("effect_pp", effect_pp),
        ("n", Value::Int(n)),
        ("n_planned", Value::Int(contract.n_planned)),
        ("causal", Value::Bool(false)),
        ("single_ab_is_causal", Value::Bool(false)),
        ("inconclusive_is_absent", Value::Bool(false)),
        (
            "note",
            string("a decision follows the frozen contract only and applies to this experiment coordinate; not-significant is not equivalent"),
        ),
        ("contract", contract.to_value()),
        ("runs", array(runs)),
        ("deviations", array(deviations)),
        ("estimator", estimator),
    ])
}

/// Seconds since the Unix epoch for the two time formats a runs document
/// may carry: the store clock reading `<secs>.<millis>Z` and RFC 3339
/// (`YYYY-MM-DDTHH:MM:SS[.frac](Z|±HH:MM)`). Both land on one axis, so a
/// run in one format is compared correctly with a freeze in the other.
/// `None` for anything else — never "sorts first".
#[must_use]
pub fn epoch_seconds(text: &str) -> Option<i64> {
    let text = text.trim();
    if let Some(rest) = text.strip_suffix('Z')
        && let Some((secs, millis)) = rest.split_once('.')
        && !millis.is_empty()
        && millis.bytes().all(|b| b.is_ascii_digit())
        && !secs.is_empty()
        && secs.bytes().all(|b| b.is_ascii_digit())
    {
        return secs.parse::<i64>().ok();
    }
    let bytes = text.as_bytes();
    if bytes.len() < 20 || bytes[4] != b'-' || bytes[7] != b'-' || (bytes[10] != b'T' && bytes[10] != b't') {
        return None;
    }
    let num = |from: usize, to: usize| -> Option<i64> {
        let slice = text.get(from..to)?;
        if slice.bytes().all(|b| b.is_ascii_digit()) { slice.parse().ok() } else { None }
    };
    let (year, month, day) = (num(0, 4)?, num(5, 7)?, num(8, 10)?);
    let (hour, minute, second) = (num(11, 13)?, num(14, 16)?, num(17, 19)?);
    if bytes[13] != b':' || bytes[16] != b':' || !(1..=12).contains(&month) || !(1..=31).contains(&day)
        || hour > 23 || minute > 59 || second > 60
    {
        return None;
    }
    let mut i = 19;
    if bytes.get(i) == Some(&b'.') {
        i += 1;
        let start = i;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if i == start {
            return None;
        }
    }
    let offset = match bytes.get(i) {
        Some(b'Z' | b'z') if i + 1 == bytes.len() => 0,
        Some(b'+' | b'-') if i + 6 == bytes.len() && bytes[i + 3] == b':' => {
            let sign = if bytes[i] == b'+' { 1 } else { -1 };
            let (oh, om) = (num(i + 1, i + 3)?, num(i + 4, i + 6)?);
            if oh > 23 || om > 59 {
                return None;
            }
            sign * (oh * 3600 + om * 60)
        }
        _ => return None,
    };
    let days = days_from_civil(year, month, day);
    Some(days * 86_400 + hour * 3600 + minute * 60 + second - offset)
}

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant's
/// algorithm), valid for the years a session log can carry.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The exit code the CLI reports for a result: `supported-*` → 0, anything
/// else (inconclusive or not executed) → 3.
#[must_use]
pub fn exit_code(result: &Value) -> i32 {
    match result.get("decision").and_then(Value::as_str) {
        Some(decision) if decision.starts_with("supported-") => 0,
        _ => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctxpect_schema::parse;

    fn contract_value() -> Value {
        parse(
            r#"{"schema":"experiment-contract-v1","experiment_id":"e1","primary_outcome":"task-pass","margin_pp":10,"pairing":"paired-by-task","n_planned":2,"alpha":"0.05","power":"0.80","multiplicity":"none","itt":"count-as-fail","locked":{"code_digest":"c","model":"m","harness":"h","tool_availability_digest":"t"},"frozen_at":"100.0Z","invalidation":["harness update"]}"#,
        )
        .unwrap()
    }

    fn run(id: &str, arm: &str, task: &str, outcome: &str, started: &str) -> String {
        format!(
            r#"{{"run_id":"{id}","arm":"{arm}","task_id":"{task}","outcome":"{outcome}","code_digest":"c","model":"m","harness":"h","tool_availability_digest":"t","started_at":"{started}","ended_at":"{started}"}}"#
        )
    }

    fn document(runs: &[String]) -> RunsDocument {
        let text = format!(
            r#"{{"schema":"ctxpect-effect-runs-v1","contract":{},"runs":[{}]}}"#,
            ctxpect_schema::canonical_json(&contract_value()),
            runs.join(",")
        );
        RunsDocument::from_value(&parse(&text).unwrap()).unwrap()
    }

    #[test]
    fn one_discordant_pair_among_eleven_is_not_equivalence() {
        // v1 read the discordant rate as known and called this "equivalent".
        let mut contract = ExperimentContract::from_value(&contract_value()).unwrap();
        contract.n_planned = 11;
        let mut pairs = vec![Pair { control_pass: false, treatment_pass: true }];
        pairs.extend((0..10).map(|_| Pair { control_pass: true, treatment_pass: true }));
        let e = PairedExactBinomial.estimate(&contract, &pairs);
        assert_eq!(e.decision, "inconclusive", "{}", e.note);
        assert!(e.detail.pointer(&["discordant_rate_ci"]).is_some());
        // With no discordant pair the bound is the same as before: n=30 clears
        // a 10 pp margin, n=20 does not.
        let equal = |n: usize| (0..n).map(|_| Pair { control_pass: true, treatment_pass: true }).collect::<Vec<_>>();
        contract.n_planned = 20;
        assert_eq!(PairedExactBinomial.estimate(&contract, &equal(20)).decision, "inconclusive");
        contract.n_planned = 30;
        assert_eq!(PairedExactBinomial.estimate(&contract, &equal(30)).decision, "supported-equivalent-within-margin");
        // Direction still needs the exact test: 9 vs 1 discordant is beneficial.
        contract.n_planned = 10;
        let mut skewed: Vec<Pair> = (0..9).map(|_| Pair { control_pass: false, treatment_pass: true }).collect();
        skewed.push(Pair { control_pass: true, treatment_pass: false });
        assert_eq!(PairedExactBinomial.estimate(&contract, &skewed).decision, "supported-beneficial");
    }

    #[test]
    fn extra_pairs_duplicate_slots_and_unreadable_times_all_invalidate() {
        // More pairs than planned is optional stopping, not the registered test.
        let mut runs = Vec::new();
        for i in 0..3 {
            runs.push(run(&format!("c{i}"), "control", &format!("t{i}"), "fail", "101.0Z"));
            runs.push(run(&format!("x{i}"), "treatment", &format!("t{i}"), "pass", "101.0Z"));
        }
        let out = decide(&document(&runs));
        assert_eq!(out.get("reason_code").and_then(Value::as_str), Some("effect.n_mismatch"));
        assert_eq!(out.get("decision").and_then(Value::as_str), Some("inconclusive"));

        // A second run for the same (task, arm) is a duplicate observation.
        let out = decide(&document(&[
            run("a", "control", "t1", "pass", "101.0Z"),
            run("b", "treatment", "t1", "fail", "101.0Z"),
            run("c", "control", "t1", "fail", "101.0Z"),
            run("d", "treatment", "t2", "pass", "101.0Z"),
            run("e", "control", "t2", "fail", "101.0Z"),
        ]));
        assert_eq!(out.get("reason_code").and_then(Value::as_str), Some("effect.run_duplicate"));

        // Times: RFC 3339 and store readings share one axis; an offset counts.
        assert_eq!(epoch_seconds("100.0Z"), Some(100));
        assert_eq!(epoch_seconds("2026-09-09T00:00:00Z"), Some(1_788_912_000));
        assert_eq!(epoch_seconds("2026-09-09T08:00:00+09:00"), Some(1_788_912_000 - 3600));
        assert_eq!(epoch_seconds("2026-09-09T00:00:00.250Z"), Some(1_788_912_000));
        assert_eq!(epoch_seconds("garbage"), None);
        assert_eq!(epoch_seconds("2026-13-01T00:00:00Z"), None);
        let out = decide(&document(&[
            run("a", "control", "t1", "pass", "not a time"),
            run("b", "treatment", "t1", "fail", "101.0Z"),
            run("c", "control", "t2", "pass", "101.0Z"),
            run("d", "treatment", "t2", "fail", "101.0Z"),
        ]));
        assert_eq!(out.get("reason_code").and_then(Value::as_str), Some("effect.run_time_invalid"));
        // A contract frozen at an unreadable time is not a contract.
        let mut broken = contract_value();
        if let Value::Object(map) = &mut broken {
            map.insert("frozen_at".into(), string("whenever"));
        }
        assert_eq!(ExperimentContract::from_value(&broken).unwrap_err().code, "effect.contract_invalid");
        if let Value::Object(map) = &mut broken {
            map.insert("frozen_at".into(), string("100.0Z"));
            map.insert("alpha".into(), string("0.49"));
        }
        assert_eq!(ExperimentContract::from_value(&broken).unwrap_err().code, "effect.contract_invalid");
    }

    #[test]
    fn two_pairs_are_not_evidence_and_the_estimator_says_so_with_its_numbers() {
        let doc = document(&[
            run("r1", "control", "t1", "fail", "101.0Z"),
            run("r2", "treatment", "t1", "pass", "101.0Z"),
            run("r3", "control", "t2", "fail", "101.0Z"),
            run("r4", "treatment", "t2", "pass", "101.0Z"),
        ]);
        let out = decide(&doc);
        assert_eq!(out.get("executed"), Some(&Value::Bool(true)));
        assert_eq!(out.get("decision").and_then(Value::as_str), Some("inconclusive"));
        assert_eq!(
            out.get("reason_code").and_then(Value::as_str),
            Some("effect.estimator_inconclusive")
        );
        assert_eq!(out.pointer(&["estimator", "available"]), Some(&Value::Bool(true)));
        assert_eq!(out.pointer(&["estimator", "name"]).and_then(Value::as_str), Some(PAIRED_EXACT_BINOMIAL_V2));
        // b = 2, c = 0, m = 2: two-sided exact p = 2 * 0.25 = 0.5.
        assert_eq!(out.pointer(&["estimator", "detail", "p_value"]).and_then(Value::as_str), Some("0.500000"));
        assert_eq!(out.get("n").and_then(Value::as_i64), Some(2));
        assert_eq!(out.get("runs").and_then(Value::as_array).map(<[Value]>::len), Some(4));
        assert_eq!(exit_code(&out), 3);
        // Without any estimator the answer names that, not a number.
        let bare = decide_with(&doc, None);
        assert_eq!(bare.get("reason_code").and_then(Value::as_str), Some("effect.estimator_unavailable"));
    }

    fn pairs(b: usize, c: usize, both_pass: usize, both_fail: usize) -> Vec<Pair> {
        let mut out = Vec::new();
        out.extend((0..b).map(|_| Pair { control_pass: false, treatment_pass: true }));
        out.extend((0..c).map(|_| Pair { control_pass: true, treatment_pass: false }));
        out.extend((0..both_pass).map(|_| Pair { control_pass: true, treatment_pass: true }));
        out.extend((0..both_fail).map(|_| Pair { control_pass: false, treatment_pass: false }));
        out
    }

    #[test]
    fn the_exact_binomial_matches_known_values() {
        // P(X <= 2 | 10, 0.5) = 56/1024.
        assert!((binomial_cdf(2, 10, 0.5) - 56.0 / 1024.0).abs() < 1e-12);
        // Clopper–Pearson for 0/10 at alpha 0.05: upper = 1 - 0.05^(1/10).
        let (lo, hi) = clopper_pearson(0, 10, 0.05);
        assert_eq!(lo, 0.0);
        assert!((hi - (1.0 - 0.05f64.powf(0.1))).abs() < 1e-9, "{hi}");
        // 10/10: lower = 0.05^(1/10).
        let (lo, hi) = clopper_pearson(10, 10, 0.05);
        assert!((lo - 0.05f64.powf(0.1)).abs() < 1e-9, "{lo}");
        assert_eq!(hi, 1.0);
        // Symmetry: bounds for b/m mirror those for (m-b)/m.
        let (lo1, hi1) = clopper_pearson(3, 12, 0.05);
        let (lo2, hi2) = clopper_pearson(9, 12, 0.05);
        assert!((lo1 - (1.0 - hi2)).abs() < 1e-9 && (hi1 - (1.0 - lo2)).abs() < 1e-9);
    }

    #[test]
    fn the_frozen_estimator_reaches_every_decision_for_the_right_reasons() {
        let mut contract = ExperimentContract::from_value(&contract_value()).unwrap();
        contract.n_planned = 1;
        let est = PairedExactBinomial;
        // 10 discordant pairs all favouring treatment: p = 2 * 0.5^10 < 0.05.
        let e = est.estimate(&contract, &pairs(10, 0, 0, 0));
        assert_eq!(e.decision, "supported-beneficial");
        assert_eq!(e.effect_pp, 100);
        // ... and all favouring control.
        let e = est.estimate(&contract, &pairs(0, 10, 0, 0));
        assert_eq!(e.decision, "supported-harmful");
        assert_eq!(e.effect_pp, -100);
        // 4 discordant pairs: p = 0.125, not evidence; and the interval is
        // wide, so not equivalent either.
        let e = est.estimate(&contract, &pairs(4, 0, 0, 0));
        assert_eq!(e.decision, "inconclusive");
        // 200 concordant pairs, no discordance: bound 1 - 0.05^(1/200) = 1.5 pp
        // inside a 10 pp margin → equivalent.
        let e = est.estimate(&contract, &pairs(0, 0, 100, 100));
        assert_eq!(e.decision, "supported-equivalent-within-margin");
        assert_eq!(e.effect_pp, 0);
        // 200 pairs with 2 discordant each way: still equivalent within 10 pp.
        let e = est.estimate(&contract, &pairs(2, 2, 98, 98));
        assert_eq!(e.decision, "supported-equivalent-within-margin", "{:?}", e.detail);
        // 8 concordant pairs: bound 1 - 0.05^(1/8) = 31 pp → inconclusive,
        // because absence of discordance in a tiny sample is not equivalence.
        let e = est.estimate(&contract, &pairs(0, 0, 4, 4));
        assert_eq!(e.decision, "inconclusive");
        // A multiplicity correction v1 does not implement: inconclusive, said so.
        contract.multiplicity = "bonferroni".into();
        let e = est.estimate(&contract, &pairs(10, 0, 0, 0));
        assert_eq!(e.decision, "inconclusive");
        assert!(e.note.contains("multiplicity"));
    }

    #[test]
    fn every_protocol_deviation_is_inconclusive_with_its_own_reason() {
        // n insufficient
        let doc = document(&[
            run("r1", "control", "t1", "fail", "101.0Z"),
            run("r2", "treatment", "t1", "pass", "101.0Z"),
        ]);
        assert_eq!(decide(&doc).get("reason_code").and_then(Value::as_str), Some("effect.n_insufficient"));
        // arm unbalanced
        let doc = document(&[
            run("r1", "control", "t1", "fail", "101.0Z"),
            run("r2", "treatment", "t1", "pass", "101.0Z"),
            run("r3", "control", "t2", "fail", "101.0Z"),
            run("r4", "treatment", "t2", "pass", "101.0Z"),
            run("r5", "treatment", "t3", "pass", "101.0Z"),
        ]);
        assert_eq!(decide(&doc).get("reason_code").and_then(Value::as_str), Some("effect.arm_unbalanced"));
        // confounder drift
        let drifted = run("r2", "treatment", "t1", "pass", "101.0Z").replace("\"model\":\"m\"", "\"model\":\"other\"");
        let doc = document(&[
            run("r1", "control", "t1", "fail", "101.0Z"),
            drifted,
            run("r3", "control", "t2", "fail", "101.0Z"),
            run("r4", "treatment", "t2", "pass", "101.0Z"),
        ]);
        let out = decide(&doc);
        assert_eq!(out.get("reason_code").and_then(Value::as_str), Some("effect.confounder_drift"));
        assert!(out.get("deviations").and_then(Value::as_array).is_some_and(|d| d.iter().any(|item| {
            item.get("detail").and_then(Value::as_str).is_some_and(|t| t.contains("model"))
        })));
        // run before freeze
        let doc = document(&[
            run("r1", "control", "t1", "fail", "99.0Z"),
            run("r2", "treatment", "t1", "pass", "101.0Z"),
            run("r3", "control", "t2", "fail", "101.0Z"),
            run("r4", "treatment", "t2", "pass", "101.0Z"),
        ]);
        assert_eq!(decide(&doc).get("reason_code").and_then(Value::as_str), Some("effect.run_precedes_freeze"));
        // duplicate run id
        let doc = document(&[
            run("r1", "control", "t1", "fail", "101.0Z"),
            run("r1", "treatment", "t1", "pass", "101.0Z"),
            run("r3", "control", "t2", "fail", "101.0Z"),
            run("r4", "treatment", "t2", "pass", "101.0Z"),
        ]);
        assert_eq!(decide(&doc).get("reason_code").and_then(Value::as_str), Some("effect.run_duplicate"));
        // ITT: a timeout is counted as fail and listed, not silently dropped.
        let doc = document(&[
            run("r1", "control", "t1", "timeout", "101.0Z"),
            run("r2", "treatment", "t1", "pass", "101.0Z"),
            run("r3", "control", "t2", "fail", "101.0Z"),
            run("r4", "treatment", "t2", "pass", "101.0Z"),
        ]);
        let out = decide(&doc);
        assert_eq!(out.get("n").and_then(Value::as_i64), Some(2));
        assert!(out.get("deviations").and_then(Value::as_array).is_some_and(|d| d.iter().any(|item| {
            item.get("reason_code").and_then(Value::as_str) == Some("effect.itt_applied")
        })));
        assert!(out.get("runs").and_then(Value::as_array).is_some_and(|r| r.iter().any(|item| {
            item.get("counted_as").and_then(Value::as_str) == Some("counted-as-fail")
        })));
    }

    #[test]
    fn structural_problems_are_errors_not_experiments() {
        let err = RunsDocument::from_value(&parse(r#"{"schema":"nope"}"#).unwrap()).unwrap_err();
        assert_eq!(err.code, "effect.runs_invalid");
        let mut contract = match contract_value() {
            Value::Object(map) => map,
            _ => unreachable!(),
        };
        contract.remove("alpha");
        let text = format!(
            r#"{{"schema":"ctxpect-effect-runs-v1","contract":{},"runs":[]}}"#,
            ctxpect_schema::canonical_json(&Value::Object(contract))
        );
        let err = RunsDocument::from_value(&parse(&text).unwrap()).unwrap_err();
        assert_eq!(err.code, "effect.contract_invalid");
        let text = format!(
            r#"{{"schema":"ctxpect-effect-runs-v1","contract":{},"runs":[{{"run_id":"x","arm":"left"}}]}}"#,
            ctxpect_schema::canonical_json(&contract_value())
        );
        let err = RunsDocument::from_value(&parse(&text).unwrap()).unwrap_err();
        assert_eq!(err.code, "effect.run_malformed");
    }

    #[test]
    fn not_executed_carries_no_decision() {
        let out = not_executed("e", Some(4));
        assert_eq!(out.get("executed"), Some(&Value::Bool(false)));
        assert_eq!(out.get("decision"), Some(&Value::Null));
        assert_eq!(out.get("reason_code").and_then(Value::as_str), Some("effect.runs_required"));
        assert_eq!(exit_code(&out), 3);
    }
}
