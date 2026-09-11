//! Frozen 18-family catalog. Live status is never invented from this table.

use ctxpect_schema::{array, object, string, Value};

#[derive(Clone, Copy)]
pub struct Family {
    pub id: &'static str,
    pub name: &'static str,
    pub cohort: &'static str,
    pub default_surface: &'static str,
    pub default_version: &'static str,
}

pub const FAMILIES: &[Family] = &[
    Family { id: "codex", name: "Codex", cohort: "anchor", default_surface: "cli", default_version: "0.147.0" },
    Family { id: "claude-code", name: "Claude Code", cohort: "anchor", default_surface: "cli", default_version: "2.1.259" },
    Family { id: "cursor", name: "Cursor", cohort: "anchor", default_surface: "ide", default_version: "unknown" },
    Family { id: "grok-build", name: "Grok Build", cohort: "anchor", default_surface: "cli", default_version: "unknown" },
    Family { id: "opencode", name: "OpenCode", cohort: "expansion", default_surface: "cli", default_version: "unknown" },
    Family { id: "kimi-code", name: "Kimi Code", cohort: "expansion", default_surface: "cli", default_version: "unknown" },
    Family { id: "zcode", name: "ZCode", cohort: "expansion", default_surface: "app", default_version: "unknown" },
    Family { id: "qwen-code", name: "Qwen Code", cohort: "expansion", default_surface: "cli", default_version: "unknown" },
    Family { id: "goose", name: "Goose", cohort: "expansion", default_surface: "cli", default_version: "unknown" },
    Family { id: "gemini-cli", name: "Gemini CLI", cohort: "expansion", default_surface: "cli", default_version: "unknown" },
    Family { id: "github-copilot-cli", name: "GitHub Copilot CLI", cohort: "expansion", default_surface: "cli", default_version: "unknown" },
    Family { id: "kiro", name: "Kiro", cohort: "expansion", default_surface: "cli-app", default_version: "unknown" },
    Family { id: "deepseek-harness", name: "DeepSeek Harness", cohort: "hermetic", default_surface: "cli", default_version: "0.1.2-rc.1" },
    Family { id: "cline", name: "Cline", cohort: "hermetic", default_surface: "cli", default_version: "3.0.61" },
    Family { id: "aider", name: "Aider", cohort: "hermetic", default_surface: "cli", default_version: "0.86.2" },
    Family { id: "openhands", name: "OpenHands", cohort: "hermetic", default_surface: "sdk", default_version: "unknown" },
    Family { id: "windsurf", name: "Windsurf", cohort: "hermetic", default_surface: "ide", default_version: "unknown" },
    Family { id: "coze", name: "Coze", cohort: "connector", default_surface: "connector", default_version: "unknown" },
];

#[must_use]
pub fn integrations_json(active_harness: &str, inspect_supported: bool) -> Value {
    let items: Vec<Value> = FAMILIES
        .iter()
        .map(|family| {
            // A family is "supported" when a resolver grammar exists for it
            // (`ctxpect_resolve::ANCHORS`), not by name.
            let supported = inspect_supported
                && !ctxpect_resolve::implemented_capabilities(family.id).is_empty();
            // The only two declared, repeatable native oracles in this freeze.
            let has_native_oracle = matches!(family.id, "codex" | "grok-build");
            // A grouping label for display, not a truth value: it summarizes
            // the declared catalog tier (not a live observation) so callers
            // do not have to re-derive it from `reason_code`/`native_oracle`.
            let evidence_capability = if has_native_oracle {
                "native"
            } else if supported {
                "static-only"
            } else if family.cohort == "connector" {
                "connector-required"
            } else {
                "unsupported"
            };
            let (install, auth, connector, version, surface, reason) = if supported {
                (
                    "unknown-not-claimed-from-config",
                    "unknown-not-scanned",
                    "not-required",
                    family.default_version,
                    family.default_surface,
                    "static-resolver-available",
                )
            } else if family.cohort == "connector" {
                (
                    "unknown",
                    "unknown",
                    "connector_required",
                    family.default_version,
                    family.default_surface,
                    "connector_required",
                )
            } else {
                (
                    "unknown",
                    "unknown",
                    "unknown",
                    family.default_version,
                    family.default_surface,
                    "unsupported_harness_version",
                )
            };
            object([
                ("family_id", string(family.id)),
                ("family_name", string(family.name)),
                ("cohort", string(family.cohort)),
                (
                    "installation",
                    object([
                        ("status", string(install)),
                        (
                            "config_residue_is_installed",
                            Value::Bool(false),
                        ),
                    ]),
                ),
                ("authentication", object([("status", string(auth))])),
                ("connector", object([("status", string(connector))])),
                ("version", object([("declared", string(version)), ("status", string("user-attested-or-unknown"))])),
                ("surface", object([("declared", string(surface))])),
                ("reason_code", string(reason)),
                ("evidence_capability", string(evidence_capability)),
                ("active_coordinate", Value::Bool(family.id == active_harness)),
                (
                    "native_oracle",
                    if has_native_oracle {
                        string(if family.id == "codex" {
                            "debug prompt-input"
                        } else {
                            "inspect --json"
                        })
                    } else {
                        string("none-declared-repeatable")
                    },
                ),
            ])
        })
        .collect();
    object([
        ("count", Value::Int(i64::try_from(FAMILIES.len()).unwrap_or(18))),
        ("families", array(items)),
        (
            "fixture_note",
            string("Doctor visual fixture 4/9/5 is not live coverage. Counts describe declared catalog tiers, not installation or collected native evidence."),
        ),
    ])
}

#[must_use]
pub fn family_entry(id: &str, active_harness: &str, inspect_supported: bool) -> Option<Value> {
    let catalog = integrations_json(active_harness, inspect_supported);
    catalog
        .get("families")
        .and_then(Value::as_array)
        .and_then(|items| {
            items
                .iter()
                .find(|item| item.get("family_id").and_then(Value::as_str) == Some(id))
                .cloned()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capability(catalog: &Value, id: &str) -> String {
        catalog
            .get("families")
            .and_then(Value::as_array)
            .and_then(|items| {
                items.iter().find(|item| {
                    item.get("family_id").and_then(Value::as_str) == Some(id)
                })
            })
            .and_then(|item| item.get("evidence_capability"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned()
    }

    #[test]
    fn evidence_capability_matches_resolver_and_oracle_facts() {
        let catalog = integrations_json("codex", true);
        // Declared, repeatable native oracles.
        assert_eq!(capability(&catalog, "codex"), "native");
        assert_eq!(capability(&catalog, "grok-build"), "native");
        assert!(!ctxpect_resolve::implemented_capabilities("codex").is_empty());
        assert!(!ctxpect_resolve::implemented_capabilities("claude-code").is_empty());
        assert!(ctxpect_resolve::implemented_capabilities("grok-build").is_empty());
        // A resolver grammar exists but no native oracle is declared.
        assert_eq!(capability(&catalog, "claude-code"), "static-only");
        // Neither resolver grammar nor native oracle.
        assert_eq!(capability(&catalog, "deepseek-harness"), "unsupported");
        // The connector cohort never claims more than "needs connector".
        assert_eq!(capability(&catalog, "coze"), "connector-required");
        // Native oracles do not depend on resolver support.
        let cold = integrations_json("codex", false);
        assert_eq!(capability(&cold, "codex"), "native");
        // Without inspect support nothing resolves statically either.
        assert_eq!(capability(&cold, "claude-code"), "unsupported");
    }
}
