//! Effect Lab. Decisions come only from a frozen ExperimentContract.

use ctxpect_schema::{array, object, string, Value};

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

#[derive(Debug, Clone)]
pub struct ExperimentContract {
    pub experiment_id: String,
    pub n_planned: i64,
    pub margin: i64,
    pub metric: String,
}

impl ExperimentContract {
    pub fn to_value(&self) -> Value {
        object([
            ("schema", string("experiment-contract-v1")),
            ("experiment_id", string(&self.experiment_id)),
            ("n_planned", Value::Int(self.n_planned)),
            ("margin", Value::Int(self.margin)),
            ("metric", string(&self.metric)),
            ("n_locked", Value::Bool(true)),
        ])
    }
}

/// Four preconstructed decision fixtures plus a live local runner result.
pub fn decide(
    contract: &ExperimentContract,
    control: &[i64],
    treatment: &[i64],
    n_was_changed_after_results: bool,
) -> Result<Value, EffectError> {
    if n_was_changed_after_results {
        return Err(EffectError::new(
            "effect.n_locked",
            "sample size cannot change after results are observed",
        ));
    }
    let n = i64::try_from(control.len().min(treatment.len())).unwrap_or(0);
    if n < contract.n_planned {
        return Ok(decision(
            contract,
            "inconclusive",
            n,
            "n below frozen contract; not equivalent and not causal",
        ));
    }
    if n == 1 {
        return Ok(decision(
            contract,
            "inconclusive",
            n,
            "a single A/B pair cannot support a causal decision",
        ));
    }
    let c = mean(control);
    let t = mean(treatment);
    let delta = t - c;
    let label = if delta > contract.margin {
        "supported-beneficial"
    } else if delta < -contract.margin {
        "supported-harmful"
    } else {
        "supported-equivalent-within-margin"
    };
    Ok(decision(contract, label, n, "decision follows the frozen contract only"))
}

fn decision(contract: &ExperimentContract, label: &str, n: i64, note: &str) -> Value {
    object([
        ("schema", string("ctxpect-effect-result-v1")),
        ("experiment_id", string(&contract.experiment_id)),
        ("decision", string(label)),
        ("n", Value::Int(n)),
        ("n_planned", Value::Int(contract.n_planned)),
        ("causal", Value::Bool(false)),
        (
            "single_ab_is_causal",
            Value::Bool(false),
        ),
        (
            "inconclusive_is_absent",
            Value::Bool(false),
        ),
        ("note", string(note)),
        ("contract", contract.to_value()),
        ("runs", array([])),
    ])
}

fn mean(values: &[i64]) -> i64 {
    if values.is_empty() {
        return 0;
    }
    values.iter().sum::<i64>() / i64::try_from(values.len()).unwrap_or(1)
}

/// Local runner: presence of AGENTS.md as a toy success metric (0/1).
pub fn run_local_instructions_probe(has_agents: bool) -> i64 {
    i64::from(has_agents)
}

pub fn preconstructed() -> Value {
    let contract = ExperimentContract {
        experiment_id: "fx-preconstructed".into(),
        n_planned: 4,
        margin: 1,
        metric: "task-pass".into(),
    };
    array([
        decide(&contract, &[0, 0, 0, 0], &[2, 2, 2, 2], false)
            .unwrap()
            .get("decision")
            .cloned()
            .unwrap_or(string("supported-beneficial")),
        string("supported-harmful"),
        string("supported-equivalent-within-margin"),
        string("inconclusive"),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_ab_is_inconclusive_and_n_cannot_change() {
        let contract = ExperimentContract {
            experiment_id: "e1".into(),
            n_planned: 4,
            margin: 1,
            metric: "x".into(),
        };
        let one = decide(&contract, &[0], &[1], false).unwrap();
        assert_eq!(
            one.get("decision").and_then(Value::as_str),
            Some("inconclusive")
        );
        assert_eq!(one.get("causal").and_then(Value::as_bool), Some(false));
        let err = decide(&contract, &[0, 0, 0, 0], &[1, 1, 1, 1], true).expect_err("n");
        assert_eq!(err.code, "effect.n_locked");
        let eq = decide(&contract, &[1, 1, 1, 1], &[1, 1, 1, 1], false).unwrap();
        assert_eq!(
            eq.get("decision").and_then(Value::as_str),
            Some("supported-equivalent-within-margin")
        );
        assert_eq!(
            eq.get("inconclusive_is_absent").and_then(Value::as_bool),
            Some(false)
        );
    }
}
