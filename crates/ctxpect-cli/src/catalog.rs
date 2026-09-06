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
            let supported = family.id == "codex" && inspect_supported;
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
                ("active_coordinate", Value::Bool(family.id == active_harness)),
                (
                    "native_oracle",
                    if family.id == "codex" {
                        string("debug prompt-input")
                    } else if family.id == "grok-build" {
                        string("inspect --json")
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
            string("Doctor visual fixture 4/9/5 is not live coverage. Live counts are computed from this catalog plus inspect."),
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
