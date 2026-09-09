//! A deterministic estimator injected from the test namespace, alongside the
//! product's frozen `PairedExactBinomial`: the injection proves the trait
//! seam, and the comparison shows the frozen procedure does not hand out a
//! `supported-*` on the tiny samples a naive difference would.

use ctxpect_effect::{decide, decide_with, exit_code, Estimate, Estimator, ExperimentContract, Pair, RunsDocument, Value};
use ctxpect_schema::{canonical_json, parse};

/// Pass-rate difference in percentage points against the contract margin.
/// Strictly inside the margin → equivalent; exactly on it → inconclusive
/// (a boundary is not evidence); beyond it → beneficial / harmful.
struct PassRateDifference;

impl Estimator for PassRateDifference {
    fn name(&self) -> &str {
        "test-pass-rate-difference"
    }
    fn estimate(&self, contract: &ExperimentContract, pairs: &[Pair]) -> Estimate {
        let n = i64::try_from(pairs.len()).unwrap_or(0).max(1);
        let control = i64::try_from(pairs.iter().filter(|p| p.control_pass).count()).unwrap_or(0) * 100 / n;
        let treatment = i64::try_from(pairs.iter().filter(|p| p.treatment_pass).count()).unwrap_or(0) * 100 / n;
        let effect_pp = treatment - control;
        let decision = if effect_pp > contract.margin_pp {
            "supported-beneficial"
        } else if effect_pp < -contract.margin_pp {
            "supported-harmful"
        } else if effect_pp.abs() < contract.margin_pp {
            "supported-equivalent-within-margin"
        } else {
            "inconclusive"
        };
        Estimate {
            decision,
            effect_pp,
            note: "test estimator: pass-rate difference vs margin".into(),
            detail: Value::Null,
        }
    }
}

fn document(outcomes: &[(&str, &str)]) -> RunsDocument {
    let contract = r#"{"schema":"experiment-contract-v1","experiment_id":"e","primary_outcome":"task-pass","margin_pp":10,"pairing":"paired-by-task","n_planned":4,"alpha":"0.05","power":"0.80","multiplicity":"none","itt":"count-as-fail","locked":{"code_digest":"c","model":"m","harness":"h","tool_availability_digest":"t"},"frozen_at":"100.0Z","invalidation":[]}"#;
    let mut runs = Vec::new();
    for (index, (control, treatment)) in outcomes.iter().enumerate() {
        for (arm, outcome) in [("control", control), ("treatment", treatment)] {
            runs.push(format!(
                r#"{{"run_id":"{arm}-{index}","arm":"{arm}","task_id":"t{index}","outcome":"{outcome}","code_digest":"c","model":"m","harness":"h","tool_availability_digest":"t","started_at":"101.0Z","ended_at":"102.0Z"}}"#
            ));
        }
    }
    let text = format!(r#"{{"schema":"ctxpect-effect-runs-v1","contract":{contract},"runs":[{}]}}"#, runs.join(","));
    RunsDocument::from_value(&parse(&text).unwrap()).unwrap()
}

#[test]
fn an_injected_estimator_reaches_all_four_decisions_and_the_frozen_one_disagrees_on_tiny_samples() {
    type Case = (&'static [(&'static str, &'static str)], &'static str, i32);
    let cases: [Case; 4] = [
        (&[("fail", "pass"), ("fail", "pass"), ("fail", "pass"), ("fail", "pass")], "supported-beneficial", 0),
        (&[("pass", "fail"), ("pass", "fail"), ("pass", "fail"), ("pass", "fail")], "supported-harmful", 0),
        (&[("pass", "pass"), ("fail", "fail"), ("pass", "pass"), ("fail", "fail")], "supported-equivalent-within-margin", 0),
        // Boundary: exactly the margin (10 pp) is not evidence either way.
        (&[("fail", "pass"), ("pass", "pass"), ("pass", "pass"), ("pass", "pass"), ("pass", "pass"), ("pass", "pass"), ("pass", "pass"), ("pass", "pass"), ("pass", "pass"), ("pass", "pass")], "inconclusive", 3),
    ];
    for (outcomes, expected, exit) in cases {
        let doc = document(outcomes);
        let injected = decide_with(&doc, Some(&PassRateDifference));
        assert_eq!(injected.get("decision").and_then(Value::as_str), Some(expected), "{}", canonical_json(&injected));
        assert_eq!(exit_code(&injected), exit);
        assert_eq!(injected.pointer(&["estimator", "name"]).and_then(Value::as_str), Some("test-pass-rate-difference"));
        // The margin is the contract's. The product path runs the frozen
        // exact estimator, whose verdict on these tiny samples differs from
        // the test estimator's: four discordant pairs are not evidence.
        assert_eq!(injected.pointer(&["contract", "margin_pp"]).and_then(Value::as_i64), Some(10));
        let product = decide(&doc);
        assert_eq!(product.pointer(&["estimator", "name"]).and_then(Value::as_str), Some("paired-exact-binomial-v2"));
        if outcomes.len() <= 4 {
            assert_eq!(product.get("decision").and_then(Value::as_str), Some("inconclusive"), "{}", canonical_json(&product));
            assert_eq!(product.get("reason_code").and_then(Value::as_str), Some("effect.estimator_inconclusive"));
        }
    }
    // A protocol deviation overrides the estimator.
    let mut doc = document(&[("fail", "pass"), ("fail", "pass"), ("fail", "pass"), ("fail", "pass")]);
    doc.runs[0].model = "other".into();
    let out = decide_with(&doc, Some(&PassRateDifference));
    assert_eq!(out.get("decision").and_then(Value::as_str), Some("inconclusive"));
    assert_eq!(out.get("reason_code").and_then(Value::as_str), Some("effect.confounder_drift"));
}
