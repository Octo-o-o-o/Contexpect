#!/usr/bin/env python3
"""Capability-specific executable fixture builders and structural validators.

Native files must contain parseable grammar (frontmatter, JSON schema, ignore
lists, MCP servers). Descriptive labels such as `family=` or `trigger: rule`
are not executable evidence.
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

from contexpect_contract import (
    CONNECTOR_FAMILIES,
    DECLARED_NATIVE_ORACLES,
    ORACLE_NATIVE_SHAPES,
    SCHEMA_VERSION,
    capability_status,
    family_by_id,
    primary_surface_id,
)
from contexpect_freeze import FAMILY_INPUT_SPECS

ENVELOPE = "envelope.json"
LOSS_REPORT = "loss-report.json"
LOSS_SCHEMA_ID = "contexpect-loss-v1"
LOSS_REPORT_REQUIRED = [
    "schema",
    "missing_primitive",
    "projects_to",
    "family_id",
    "native_paths_inspected",
]
LOSS_REPORT_ALLOWED = list(LOSS_REPORT_REQUIRED)
LABEL_RE = re.compile(r"^# family=|trigger:\s*[a-z_]+$", re.M)
SECRET_TOKEN_RE = re.compile(r"ghp_fixture_not_a_real_secret_\d{2}")
ZWSP = "\u200b"

SKILL_PATHS = {
    "claude-code": ".claude/skills/fixture-skill/SKILL.md",
    "cursor": ".cursor/skills/fixture-skill/SKILL.md",
    "codex": ".agents/skills/fixture-skill/SKILL.md",
    "grok-build": ".grok/skills/fixture-skill/SKILL.md",
    "opencode": ".agents/skills/fixture-skill/SKILL.md",
    "qwen-code": ".qwen/skills/fixture-skill/SKILL.md",
    "kimi-code": ".kimi/skills/fixture-skill/SKILL.md",
    "openhands": ".openhands/skills/fixture-skill/SKILL.md",
    "cline": ".cline/skills/fixture-skill/SKILL.md",
    "windsurf": ".windsurf/skills/fixture-skill/SKILL.md",
    "kiro": ".kiro/skills/fixture-skill/SKILL.md",
    "gemini-cli": ".gemini/skills/fixture-skill/SKILL.md",
    "github-copilot-cli": ".github/skills/fixture-skill/SKILL.md",
    "deepseek-harness": ".dsh/skills/fixture-skill/SKILL.md",
    "goose": ".goose/skills/fixture-skill/SKILL.md",
    "zcode": ".agents/skills/fixture-skill/SKILL.md",
    "coze": ".coze/skills/fixture-skill/SKILL.md",
}

RULE_PATHS = {
    "claude-code": ".claude/rules/project.md",
    "cursor": ".cursor/rules/scoped.mdc",
    "kiro": ".kiro/steering/rules.md",
    "cline": ".clinerules",
    "windsurf": ".windsurf/rules.md",
    "gemini-cli": ".gemini/rules.md",
    "qwen-code": ".qwen/rules.md",
    "opencode": ".opencode/rules.md",
    "deepseek-harness": ".dsh/rules.md",
    "kimi-code": ".kimi/rules.md",
    "zcode": ".agents/rules.md",
    "goose": ".goose/rules.md",
    "github-copilot-cli": ".github/copilot-rules.md",
    "openhands": ".openhands/rules.md",
    "codex": ".agents/rules.md",
    "grok-build": ".agents/rules.md",
    "aider": ".aider.rules.md",
    "coze": ".coze/rules.json",
}

MCP_PATHS = {
    "cursor": ".cursor/mcp.json",
    "claude-code": ".mcp.json",
    "codex": ".mcp.json",
}
PLUGIN_PATHS = {
    "codex": ".agents/plugins/fixture.json",
    "claude-code": ".claude/plugins/fixture.json",
    "cursor": ".cursor/extensions/fixture.json",
}


def dumps(obj: Any) -> str:
    return json.dumps(obj, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n"


def instruction_path(family_id: str) -> str:
    return FAMILY_INPUT_SPECS.get(family_id, ("AGENTS.md", "markdown"))[0]


def skill_path(family_id: str) -> str:
    return SKILL_PATHS.get(family_id, ".agents/skills/fixture-skill/SKILL.md")


def mcp_path(family_id: str) -> str:
    return MCP_PATHS.get(family_id, ".mcp.json")


def plugin_path(family_id: str) -> str:
    return PLUGIN_PATHS.get(family_id, ".agents/plugins/fixture.json")


def _instruction_body(family_id: str, polarity: str, class_name: str) -> dict[str, str]:
    path = instruction_path(family_id)
    files: dict[str, str] = {}
    if path.endswith(".json"):
        payload = {
            "org_id": "fixture-org",
            "space_id": "fixture-space",
            "project_id": "fixture-project",
            "instructions": {
                "always": ["Prefer deterministic fixtures."] if polarity == "positive" else [],
                "enabled": polarity == "positive",
            },
        }
        files[path] = dumps(payload)
        return files
    if path.endswith((".yml", ".yaml")):
        if polarity == "positive":
            files[path] = "auto-commits: false\nread:\n  - CONVENTIONS.md\n"
            files["CONVENTIONS.md"] = "# Conventions\n\nUse pytest. Do not invent native evidence.\n"
        else:
            files[path] = "auto-commits: false\nread: []\nignore:\n  - CONVENTIONS.md\n"
        return files
    if family_id == "cursor":
        always = "true" if polarity == "positive" and class_name != "conditional" else "false"
        globs = "src/**/*.ts" if class_name in {"conditional", "include", "progressive"} else "**/*"
        files[path] = (
            "---\n"
            "description: project instructions\n"
            f"globs: \"{globs}\"\n"
            f"alwaysApply: {always}\n"
            "---\n"
            "Use typed APIs for public functions.\n"
        )
        if polarity == "negative":
            files[".cursorignore"] = path + "\n"
        return files
    if polarity == "positive":
        files[path] = "## always\nUse pytest for tests.\n\n## scoped: src/**/*.py\nPrefer typed APIs.\n"
    else:
        files[path] = "## ignored\nThis instruction is excluded from discovery.\n"
        files[".ctxpect-ignore"] = path + "\n"
    if class_name == "cap":
        files["budget.json"] = dumps({"path": path, "max_bytes": 32, "actual_bytes": 256})
    if class_name == "ignore":
        files[".ctxpect-ignore"] = path + "\n"
    return files


def _skill_files(family_id: str, polarity: str) -> dict[str, str]:
    path = skill_path(family_id)
    if polarity != "positive":
        return {
            "skill-index.json": dumps({"skills": [], "excluded": ["fixture-skill"]}),
        }
    return {
        path: (
            "---\n"
            "name: fixture-skill\n"
            "description: run the fixture checklist\n"
            "---\n\n"
            "Resource path: scripts/run.sh\n"
        ),
        "scripts/run.sh": "#!/bin/sh\necho fixture-skill\n",
    }


def _mcp_files(family_id: str, polarity: str) -> dict[str, str]:
    path = mcp_path(family_id)
    servers = (
        {"fixture": {"command": "npx", "args": ["-y", "fixture-mcp"]}}
        if polarity == "positive"
        else {}
    )
    return {path: dumps({"mcpServers": servers})}


def _plugin_files(family_id: str, polarity: str) -> dict[str, str]:
    path = plugin_path(family_id)
    return {
        path: dumps(
            {
                "name": "fixture-plugin",
                "version": "0.0.1",
                "enabled": polarity == "positive",
                "entry": "./index.js",
            }
        )
    }


def _rules_files(family_id: str, polarity: str) -> dict[str, str]:
    path = RULE_PATHS.get(family_id, f".{family_id}/rules.md")
    if path.endswith(".json"):
        return {
            path: dumps(
                {
                    "globs": ["src/**"],
                    "alwaysApply": polarity == "positive",
                    "body": "Do not commit secrets.",
                }
            )
        }
    if path.endswith(".mdc"):
        return {
            path: (
                "---\n"
                "description: scoped rule\n"
                "globs: \"src/**\"\n"
                f"alwaysApply: {'true' if polarity == 'positive' else 'false'}\n"
                "---\n"
                "Do not commit secrets.\n"
            )
        }
    body = (
        "---\nglobs: src/**\n---\nDo not commit secrets.\n"
        if polarity == "positive"
        else "---\nenabled: false\n---\n"
    )
    return {path: body}


def native_files(files: dict[str, str]) -> dict[str, str]:
    return {name: body for name, body in files.items() if name != ENVELOPE}


def _json_obj(files: dict[str, str], name: str) -> dict[str, Any] | None:
    text = files.get(name)
    if not text or not str(text).strip():
        return None
    try:
        obj = json.loads(text)
    except json.JSONDecodeError:
        return None
    return obj if isinstance(obj, dict) else None


def _json_obj_strict(files: dict[str, str], name: str) -> tuple[dict[str, Any] | None, str | None]:
    text = files.get(name)
    if text is None:
        return None, f"missing {name}"
    if not str(text).strip():
        return None, f"empty {name}"
    try:
        obj = json.loads(text)
    except json.JSONDecodeError as exc:
        return None, f"{name} is not JSON: {exc}"
    if not isinstance(obj, dict):
        return None, f"{name} is not an object"
    return obj, None


def _is_unsafe_relpath(path: str) -> bool:
    if not isinstance(path, str) or not path.strip():
        return True
    if path.startswith("/") or path.startswith("\\"):
        return True
    return any(part == ".." for part in path.replace("\\", "/").split("/"))


def _loss_inspected_path(family_id: str, capability_id: str) -> str:
    if capability_id == "rules":
        return RULE_PATHS.get(family_id, f".{family_id}/rules.md")
    if capability_id == "skills":
        return skill_path(family_id)
    if capability_id == "plugins":
        return plugin_path(family_id)
    return f".{family_id}/{capability_id}"


def canonical_loss_native_paths(family_id: str, capability_id: str) -> list[str]:
    return [_loss_inspected_path(family_id, capability_id)]


def _loss_files(family_id: str, capability_id: str) -> dict[str, str]:
    return {
        LOSS_REPORT: dumps(
            {
                "schema": LOSS_SCHEMA_ID,
                "missing_primitive": capability_id,
                "projects_to": "instructions",
                "family_id": family_id,
                "native_paths_inspected": canonical_loss_native_paths(family_id, capability_id),
            }
        )
    }


def _loss_report_errors(
    obj: dict[str, Any],
    family_id: str,
    capability_id: str,
    files: dict[str, str] | None = None,
) -> list[str]:
    errors: list[str] = []
    extra = sorted(set(obj) - set(LOSS_REPORT_ALLOWED))
    missing = sorted(set(LOSS_REPORT_REQUIRED) - set(obj))
    if extra:
        errors.append(f"loss-report unknown fields {extra}")
    if missing:
        errors.append(f"loss-report missing required fields {missing}")
    if obj.get("schema") != LOSS_SCHEMA_ID:
        errors.append("loss-report schema mismatch")
    if obj.get("missing_primitive") != capability_id:
        errors.append("loss-report missing_primitive mismatch")
    if obj.get("projects_to") != "instructions":
        errors.append("loss-report projects_to mismatch")
    if obj.get("family_id") != family_id:
        errors.append("loss-report family_id mismatch")
    paths = obj.get("native_paths_inspected")
    expected = canonical_loss_native_paths(family_id, capability_id)
    if not isinstance(paths, list) or not paths:
        errors.append("loss-report native_paths_inspected is empty")
    else:
        for path in paths:
            if not isinstance(path, str) or _is_unsafe_relpath(path):
                errors.append(f"loss-report invalid native path {path!r}")
        if list(paths) != expected:
            errors.append(
                f"loss-report native_paths_inspected {paths!r} != canonical {expected!r}"
            )
        if files is not None:
            for path in expected:
                body = files.get(path)
                if body and str(body).strip():
                    errors.append(f"loss-report canonical path {path} unexpectedly has content")
    return errors


def _oracle_probe_name(family_id: str) -> str:
    return "prompt-probe.json" if family_id == "codex" else "inspect-probe.json"


def _oracle_result_name(family_id: str) -> str:
    return "prompt-result.json" if family_id == "codex" else "inspect-result.json"


def _declared_oracle_command(family_id: str) -> list[str] | None:
    for oracle in DECLARED_NATIVE_ORACLES.values():
        if oracle["family_id"] == family_id:
            return list(oracle["command"])
    return None


def _oracle_result_payload(
    family_id: str, capability_id: str, polarity: str, required_keys: list[str]
) -> dict[str, Any]:
    if family_id == "codex":
        if polarity == "indeterminate":
            payload: dict[str, Any] = {
                "messages": [{"role": "system", "content": "[core prompt withheld]"}],
                "characters": 0,
                "by_role": {},
                "withheld": True,
                "capability_id": capability_id,
                "observed": False,
            }
        elif polarity == "negative":
            payload = {
                "messages": [],
                "characters": 0,
                "by_role": {},
                "withheld": False,
                "capability_id": capability_id,
                "observed": False,
            }
        else:
            payload = {
                "messages": [
                    {"role": "user", "content": "Inspect fixture context."},
                    {"role": "system", "content": "Oracle workspace instructions."},
                ],
                "characters": 64,
                "by_role": {"user": 24, "system": 40},
                "withheld": False,
                "capability_id": capability_id,
                "observed": True,
            }
    else:
        if polarity == "indeterminate":
            payload = {
                "cwd": "/workspace/fixture",
                "export_complete": False,
                "withheld": True,
                "capability_id": capability_id,
                "observed": False,
            }
        elif polarity == "negative":
            payload = {
                "cwd": "/workspace/fixture",
                "instructions": [],
                "rules": [],
                "skills": [],
                "mcp": {},
                "observed": False,
                "capability_id": capability_id,
                "export_complete": True,
                "withheld": False,
            }
        else:
            payload = {
                "cwd": "/workspace/fixture",
                "instructions": ["Oracle workspace instructions."],
                "rules": [],
                "skills": [],
                "mcp": {"fixture": {"command": "npx"}},
                "os": "macOS",
                "model": "fixture-model",
                "observed": True,
                "capability_id": capability_id,
                "export_complete": True,
                "withheld": False,
            }
    for key in required_keys:
        if key not in payload:
            payload[key] = [] if key == "messages" else "/workspace/fixture"
    return payload


def _oracle_observed(family_id: str, capability_id: str, result: dict[str, Any]) -> bool:
    if result.get("observed") is False:
        return False
    if result.get("withheld") or result.get("export_complete") is False:
        return False
    if family_id == "codex":
        messages = result.get("messages")
        if not isinstance(messages, list):
            return False
        return any(
            isinstance(item, dict)
            and str(item.get("content") or "").strip()
            and "[core prompt withheld]" not in str(item.get("content") or "")
            for item in messages
        )
    if capability_id == "environment-metadata":
        return bool(result.get("cwd") and (result.get("os") or result.get("model")))
    if capability_id == "instructions":
        return bool(result.get("instructions"))
    return bool(result.get("observed"))


def _oracle_unparsed(
    capability_id: str, native: dict[str, str], probe_name: str, reason: str
) -> dict[str, Any]:
    return _result(
        "native-oracle",
        capability_id,
        "indeterminate",
        included=None,
        loss=False,
        reason="runtime_snapshot_missing",
        native=native,
        parse={
            "truth_state": "indeterminate",
            "reason_code": "runtime_snapshot_missing",
            "parse_error": reason,
        },
        extra={"command": None, "probe": probe_name},
    )


def _ignored(files: dict[str, str], path: str) -> bool:
    for ignore_name in (".ctxpect-ignore", ".gitignore", ".cursorignore", ".oracle-exclude"):
        body = files.get(ignore_name)
        if not body:
            continue
        lines = [line.strip() for line in body.splitlines() if line.strip()]
        if path in lines or path.split("/")[-1] in lines:
            return True
    return False


def _load_envelope(files: dict[str, str]) -> dict[str, Any]:
    text = files.get(ENVELOPE)
    if not text:
        return {}
    try:
        obj = json.loads(text)
    except json.JSONDecodeError:
        return {}
    return obj if isinstance(obj, dict) else {}


def _result(
    resolver: str,
    capability_id: str,
    truth_state: str,
    *,
    included: bool | None,
    loss: bool,
    reason: str | None,
    native: dict[str, str],
    parse: dict[str, Any],
    extra: dict[str, Any] | None = None,
) -> dict[str, Any]:
    payload = {
        "resolver": resolver,
        "capability_id": capability_id,
        "truth_state": truth_state,
        "included": included,
        "loss_report_required": loss,
        "unknown_reason_code": reason,
        "native_paths_used": sorted(native),
        "parse": parse,
    }
    if extra:
        payload.update(extra)
    return payload


def _indeterminate_result(capability_id: str, native: dict[str, str], reason: str = "surface_not_exposed") -> dict[str, Any]:
    return _result(
        "static",
        capability_id,
        "indeterminate",
        included=None,
        loss=False,
        reason=reason,
        native=native,
        parse={
            "truth_state": "indeterminate",
            "reason_code": reason,
            "native_primitive_present": False,
        },
    )


def _loss_result(capability_id: str, native: dict[str, str]) -> dict[str, Any]:
    return _result(
        "static",
        capability_id,
        "present",
        included=False,
        loss=True,
        reason=None,
        native=native,
        parse={
            "loss_report_required": True,
            "missing_primitive": capability_id,
            "projects_to": "instructions",
        },
    )


def _present_or_absent(capability_id: str, included: bool, native: dict[str, str], parse: dict[str, Any]) -> dict[str, Any]:
    return _result(
        "static",
        capability_id,
        "present" if included else "absent",
        included=included,
        loss=False,
        reason=None,
        native=native,
        parse=parse,
    )


def _parse_instructions(family_id: str, files: dict[str, str]) -> tuple[bool, dict[str, Any]]:
    path = instruction_path(family_id)
    body = files.get(path)
    parse: dict[str, Any] = {"path": path, "activation": "always"}
    if not body or not str(body).strip() or _ignored(files, path):
        return False, {**parse, "included": False}
    if path.endswith(".json"):
        obj = _json_obj(files, path) or {}
        inst = obj.get("instructions") if isinstance(obj.get("instructions"), dict) else {}
        included = bool(inst.get("enabled")) and bool(inst.get("always"))
        return included, {**parse, "included": included, "enabled": bool(inst.get("enabled"))}
    if path.endswith((".yml", ".yaml")):
        ignored = bool(re.search(r"^ignore:\s*$", body, re.M)) and "CONVENTIONS.md" in body
        included = (not ignored) and (
            bool(re.search(r"^read:\s*$", body, re.M)) and "- CONVENTIONS.md" in body or "CONVENTIONS.md" in files
        )
        if "read: []" in body:
            included = False
        return included, {**parse, "included": included}
    if "alwaysApply: true" in body:
        parse["activation"] = "always"
        return True, {**parse, "included": True}
    if "alwaysApply: false" in body and ("Use typed APIs" in body or "globs:" in body):
        parse["activation"] = "scoped"
        included = not _ignored(files, path)
        return included, {**parse, "included": included}
    if "## ignored" in body or "enabled: false" in body:
        return False, {**parse, "included": False}
    included = any(
        token in body
        for token in (
            "pytest",
            "typed APIs",
            "Prefer deterministic",
            "## always",
            "Oracle workspace",
            "Use pytest",
            "read:",
        )
    )
    if "## withheld" in body or "Core prompt is not exported" in body:
        return False, {**parse, "included": False, "withheld": True}
    return included, {**parse, "included": included}


def _parse_rules(family_id: str, files: dict[str, str]) -> tuple[bool, dict[str, Any]]:
    path = RULE_PATHS.get(family_id, f".{family_id}/rules.md")
    body = files.get(path)
    parse: dict[str, Any] = {"path": path, "has_rules_primitive": True}
    if not body or not str(body).strip() or _ignored(files, path):
        return False, {**parse, "included": False}
    if path.endswith(".json"):
        obj = _json_obj(files, path) or {}
        included = bool(obj.get("alwaysApply")) and bool(obj.get("body"))
        return included, {**parse, "included": included}
    if "alwaysApply: false" in body or "enabled: false" in body:
        return False, {**parse, "included": False}
    included = "Do not commit secrets" in body or "alwaysApply: true" in body or "globs:" in body
    return included, {**parse, "included": included}


def _parse_skills(family_id: str, files: dict[str, str]) -> tuple[bool, dict[str, Any]]:
    index = _json_obj(files, "skill-index.json")
    if index and index.get("excluded"):
        return False, {"skill_name": None, "excluded": True}
    skill_files = [name for name in files if name.endswith("SKILL.md")]
    if not skill_files:
        return False, {"skill_name": None}
    body = files[skill_files[0]]
    has_frontmatter = "name:" in body and "description:" in body
    included = has_frontmatter and "---" in body
    name = "fixture-skill" if included else None
    return included, {"skill_name": name, "path": skill_files[0]}


def _parse_mcp(family_id: str, files: dict[str, str]) -> tuple[bool, dict[str, Any]]:
    path = mcp_path(family_id)
    obj = _json_obj(files, path)
    if obj is None:
        for name, body in files.items():
            if name.endswith("mcp.json") or name.endswith(".mcp.json"):
                try:
                    obj = json.loads(body)
                    path = name
                except json.JSONDecodeError:
                    obj = None
                break
    servers = (obj or {}).get("mcpServers") or {}
    count = len(servers) if isinstance(servers, dict) else 0
    return count > 0, {"mcp_server_count": count, "path": path}


def _parse_plugins(family_id: str, files: dict[str, str]) -> tuple[bool, dict[str, Any]]:
    path = plugin_path(family_id)
    obj = _json_obj(files, path)
    if obj is None:
        for name in files:
            if "plugin" in name or "extensions" in name:
                obj = _json_obj(files, name)
                path = name
                break
    enabled = bool(obj and obj.get("enabled") and obj.get("name"))
    return enabled, {"enabled": enabled, "path": path}


def _parse_commands(files: dict[str, str]) -> tuple[bool, dict[str, Any]]:
    command_files = [name for name in files if "commands/" in name or name.endswith("fixture.md")]
    hook_files = [name for name in files if "hooks/" in name or "workflows/" in name]
    if not command_files:
        return False, {"command": "/fixture", "enabled": False, "hooks": bool(hook_files)}
    body = files[command_files[0]]
    enabled = "# disabled" not in body and "enabled: false" not in body and bool(body.strip())
    hook_enabled = False
    for name in hook_files:
        if name.endswith(".json"):
            obj = _json_obj(files, name) or {}
            hook_enabled = bool(obj.get("enabled"))
        elif "enabled: false" in files[name]:
            hook_enabled = False
        else:
            hook_enabled = True
    included = enabled and (hook_enabled or not hook_files)
    return included, {"command": "/fixture", "enabled": included, "hooks": bool(hook_files)}


def _parse_agents(files: dict[str, str]) -> tuple[bool, dict[str, Any]]:
    agent_files = [name for name in files if "agents/" in name and name.endswith(".md")]
    if not agent_files:
        return False, {"agent": "reviewer", "enabled": False}
    body = files[agent_files[0]]
    enabled = "enabled: false" not in body and "name: reviewer" in body
    return enabled, {"agent": "reviewer", "enabled": enabled, "path": agent_files[0]}


def _parse_memory(files: dict[str, str]) -> tuple[bool, dict[str, Any]]:
    memory_files = [name for name in files if "memory" in name]
    if not memory_files:
        return False, {"memory_present": False}
    present = any(str(files[name]).strip() for name in memory_files)
    return present, {"memory_present": present}


def _parse_json_flag(files: dict[str, str], names: list[str], flag: str) -> tuple[bool, dict[str, Any], str | None]:
    for name in names:
        obj = _json_obj(files, name)
        if obj is None:
            continue
        present = bool(obj.get(flag))
        return present, {"path": name, flag: present, "keys": sorted(obj)}, name
    return False, {flag: False}, None


def _parse_capability_native(family_id: str, capability_id: str, files: dict[str, str]) -> tuple[bool, dict[str, Any]]:
    if capability_id == "instructions":
        return _parse_instructions(family_id, files)
    if capability_id == "rules":
        return _parse_rules(family_id, files)
    if capability_id == "skills":
        return _parse_skills(family_id, files)
    if capability_id == "mcp-declarations":
        return _parse_mcp(family_id, files)
    if capability_id == "plugins":
        return _parse_plugins(family_id, files)
    if capability_id == "commands":
        return _parse_commands(files)
    if capability_id == "agents":
        return _parse_agents(files)
    if capability_id == "memory":
        return _parse_memory(files)
    if capability_id == "environment-metadata":
        obj = _json_obj(files, "environment.json")
        if obj is None:
            return False, {"keys": []}
        present = obj.get("present", True) is not False and bool(obj.get("cwd") or obj.get("os") or obj.get("model"))
        return bool(present), {"keys": sorted(obj), "present": bool(present)}
    if capability_id == "permission-description":
        obj = _json_obj(files, "sandbox.json")
        if obj is None:
            return False, {"mode_present": False}
        present = obj.get("mode") not in {None, "unavailable", ""}
        return present, {"mode_present": present, "mode": obj.get("mode")}
    if capability_id == "tool-schema-catalog":
        obj = _json_obj(files, "tools.schema.json")
        tools = (obj or {}).get("tools") if obj else None
        count = len(tools) if isinstance(tools, list) else 0
        return count > 0, {"tool_count": count}
    if capability_id == "tool-invocation":
        obj = _json_obj(files, "runtime/tool-call.json")
        present = bool(obj and obj.get("present") and obj.get("tool"))
        return present, {"runtime_envelope": True, "present": present}
    if capability_id == "task-message":
        obj = _json_obj(files, "runtime/task.json")
        present = bool(obj and obj.get("present") and obj.get("text"))
        return present, {"runtime_envelope": True, "present": present}
    if capability_id == "history":
        text = files.get("runtime/history.jsonl") or ""
        present = bool(text.strip())
        return present, {"runtime_envelope": True, "present": present}
    if capability_id == "compaction-steering":
        obj = _json_obj(files, "runtime/compaction.json")
        present = bool(obj and obj.get("present") and obj.get("summary"))
        return present, {"runtime_envelope": True, "present": present}
    return False, {"native_primitive_present": False}


def interpret_static(
    family_id: str,
    capability_id: str,
    files: dict[str, str],
    surface: str | None = None,
    static_support: str | None = None,
    os_lane: str | None = None,
) -> dict[str, Any]:
    """Derive resolved/indeterminate/loss from native files. Ignores envelope.parse."""
    native = native_files(files)
    macos = "macos-27-arm64"
    surface = surface or primary_surface_id(family_id)
    family = family_by_id(family_id)
    surf = next(item for item in family["surfaces"] if item["id"] == surface)
    support = static_support if static_support is not None else surf["static_support"]
    if os_lane and os_lane != macos:
        if family_id in {"zcode", "kiro"} and surface in {"app", "cli-app"}:
            return _indeterminate_result(capability_id, native, "official_distribution_not_captured")
        return _indeterminate_result(capability_id, native, "official_distribution_not_captured")
    if capability_id == "other-unknown":
        return _indeterminate_result(capability_id, native)
    if LOSS_REPORT in files:
        loss_obj, loss_err = _json_obj_strict(files, LOSS_REPORT)
        if (
            loss_obj
            and not loss_err
            and not _loss_report_errors(loss_obj, family_id, capability_id, files=native)
        ):
            return _loss_result(capability_id, native)
    status = capability_status(family_id, surface, capability_id, support)
    if status == "not-applicable":
        if family_id in CONNECTOR_FAMILIES:
            return _indeterminate_result(capability_id, native, "connector_required")
        return _indeterminate_result(capability_id, native)
    if status == "required-unknown-honesty":
        return _indeterminate_result(capability_id, native)
    included, parse = _parse_capability_native(family_id, capability_id, native)
    return _present_or_absent(capability_id, included, native, parse)


def interpret_oracle(family_id: str, capability_id: str, files: dict[str, str]) -> dict[str, Any]:
    """Derive observed/absent/indeterminate from probe/result JSON. No constant fallback."""
    native = native_files(files)
    probe_name = _oracle_probe_name(family_id)
    probe, probe_err = _json_obj_strict(files, probe_name)
    if probe_err or not probe:
        return _oracle_unparsed(capability_id, native, probe_name, probe_err or f"missing {probe_name}")
    command = probe.get("command")
    if not command:
        return _oracle_unparsed(capability_id, native, probe_name, "missing command")
    result_path = probe.get("result_path")
    if not isinstance(result_path, str) or _is_unsafe_relpath(result_path) or result_path not in files:
        return _oracle_unparsed(capability_id, native, probe_name, "result path mismatch")
    result, result_err = _json_obj_strict(files, result_path)
    if result_err or not result:
        return _oracle_unparsed(capability_id, native, probe_name, result_err or "missing result")
    if result.get("withheld") or result.get("export_complete") is False:
        truth = "indeterminate"
        included = None
        parse: dict[str, Any] = {
            "truth_state": "indeterminate",
            "reason_code": "runtime_snapshot_missing",
            "withheld": True,
            "result_path": result_path,
        }
    elif _oracle_observed(family_id, capability_id, result):
        truth = "present"
        included = True
        parse = {"observed": True, "capability_id": capability_id, "result_path": result_path}
    else:
        truth = "absent"
        included = False
        parse = {"observed": False, "capability_id": capability_id, "result_path": result_path}
    extra = {"command": command, "probe": probe_name, "result_path": result_path}
    return _result(
        "native-oracle",
        capability_id,
        truth,
        included=included,
        loss=False,
        reason=parse.get("reason_code"),
        native=native,
        parse=parse,
        extra=extra,
    )


def _finding(rule: str, blocking: bool, path: str, message: str) -> dict[str, Any]:
    return {
        "rule_id": rule,
        "severity": "error" if blocking else "warning",
        "blocking": blocking,
        "path": path,
        "message": message,
    }


def interpret_doctor(files: dict[str, str]) -> list[dict[str, Any]]:
    """Derive Doctor findings from native content/path/schema. Ignores envelope rule_id."""
    native = native_files(files)
    findings: list[dict[str, Any]] = []
    seen: set[tuple[str, str]] = set()

    def add(rule: str, blocking: bool, path: str, message: str) -> None:
        key = (rule, path)
        if key in seen:
            return
        seen.add(key)
        findings.append(_finding(rule, blocking, path, message))

    for name, body in native.items():
        if SECRET_TOKEN_RE.search(body):
            add("secret_literal", True, name, "documented-fixture-token must fail closed")
        if ZWSP in body:
            add("hidden_unicode", True, name, "hidden unicode in instruction text")
        if re.search(r"read:\s*\.\./", body):
            add("path_containment_escape", True, name, "path escapes declared root")

    layout = _json_obj(native, "layout.json")
    if layout:
        for link in layout.get("symlinks") or []:
            target = str(link.get("to") or "")
            if ".." in target.split("/"):
                add("symlink_escape", True, "layout.json", "symlink target leaves the workspace")
        for item in layout.get("files") or []:
            path = str(item.get("path") or "")
            if path.startswith("..") or path.startswith("/"):
                add("undiscoverable_path", False, "layout.json", "instruction path is not discoverable")

    inventory = _json_obj(native, "inventory.json")
    if inventory:
        required = list(inventory.get("required") or [])
        present = set(inventory.get("present") or [])
        missing = [item for item in required if item not in present and item not in native]
        if missing:
            add("required_asset_missing", True, "inventory.json", "required skill asset missing")

    plan = _json_obj(native, "plan.json")
    if plan and plan.get("drops") and plan.get("approved") is False:
        add("unapproved_lossy_projection", True, "plan.json", "lossy projection without approval")

    archive = _json_obj(native, "archive-manifest.json")
    if archive:
        for entry in archive.get("entries") or []:
            if ".." in str(entry).split("/"):
                add("archive_traversal", True, "archive-manifest.json", "archive entry escapes destination")

    hooks = _json_obj(native, "hooks.json")
    if hooks:
        for cmd in hooks.get("on_scan") or []:
            if str(cmd).strip():
                add("passive_scan_exec", True, "hooks.json", "passive scan must not execute hooks")
                break

    md_bodies = [body for name, body in native.items() if name.endswith(".md")]
    if len(md_bodies) >= 2 and len({body for body in md_bodies}) == 1:
        dup_path = sorted(name for name in native if name.endswith(".md"))[-1]
        add("duplicate", False, dup_path, "duplicate instruction digest")

    texts = [body for name, body in native.items() if name.endswith(".md")]
    joined = "\n".join(texts)
    if "must be npm" in joined and "must be pnpm" in joined:
        conflict_path = next((name for name, body in native.items() if "pnpm" in body), "CLAUDE.md")
        add("conflict", False, conflict_path, "conflicting package-manager instructions")

    for name, body in native.items():
        if re.search(r"^updated:\s*2019-", body, re.M):
            add("stale", False, name, "instruction updated date is stale")

    budget = _json_obj(native, "budget.json")
    if budget:
        path = str(budget.get("path") or "")
        max_bytes = budget.get("max_bytes")
        actual = len(native.get(path, "").encode("utf-8")) if path in native else budget.get("actual_bytes")
        if isinstance(max_bytes, int) and isinstance(actual, int) and actual > max_bytes:
            rule = "cap_truncation" if budget.get("truncated") else "oversized_resident"
            add(rule, False, path or "budget.json", "resident asset exceeds cap" if rule == "oversized_resident" else "instruction truncated by cap")
        elif budget.get("truncated") is True:
            add("cap_truncation", False, path or "AGENTS.md", "instruction truncated by cap")

    for name, body in native.items():
        if name.endswith((".md", "SKILL.md")) and body.lstrip().startswith("---"):
            if "unterminated" in body or re.search(r"^---\nname:\s*\[", body):
                add("bad_frontmatter", False, name, "frontmatter is not valid YAML")

    gitignore = native.get(".ctxpect-gitignore") or native.get(".gitignore")
    if gitignore:
        ignored_names = [line.strip() for line in gitignore.splitlines() if line.strip() and not line.startswith("#")]
        for ignored in ignored_names:
            if ignored in native and any(ignored in body for name, body in native.items() if name.endswith(".md")):
                add("gitignore_mismatch", False, ignored, "gitignored path is still referenced")

    device = _json_obj(native, "device-lock.json")
    if device and device.get("sync") is False and device.get("device_id") not in {None, "*", ""}:
        add("single_device_only", False, "device-lock.json", "asset is single-device only")

    provenance = _json_obj(native, "provenance.json")
    if provenance and provenance.get("source") in {None, ""}:
        add("unknown_source", False, str(provenance.get("path") or "imported.md"), "source provenance is unknown")

    adapter = _json_obj(native, "adapter-version.json")
    if adapter and adapter.get("required") != adapter.get("actual"):
        add("version_incompatible", False, "adapter-version.json", "adapter version is incompatible")

    placement = _json_obj(native, "placement.json")
    if placement:
        path = str(placement.get("path") or "")
        recommended = str(placement.get("recommended") or "")
        if path and recommended and path != recommended and path in native:
            add("placement_recommendation", False, path, "skill is on a non-recommended path")

    return findings


def mutate_native_trigger(family_id: str, capability_id: str, files: dict[str, str]) -> dict[str, str]:
    """Change or remove the native trigger while leaving envelope bytes untouched."""
    out = dict(files)
    envelope = out.get(ENVELOPE)
    parsed = interpret_static(family_id, capability_id, files)
    if parsed.get("loss_report_required") or LOSS_REPORT in out:
        out.pop(LOSS_REPORT, None)
        if envelope is not None:
            out[ENVELOPE] = envelope
        return out
    if parsed.get("truth_state") == "indeterminate":
        path = instruction_path(family_id)
        current = out.get(path, "")
        out[path] = (current + "\n# mutated-unsupported-primitive\n") if current else "# mutated-unsupported-primitive\n"
        if envelope is not None:
            out[ENVELOPE] = envelope
        return out
    included = bool(parsed.get("included"))
    if capability_id == "instructions":
        path = instruction_path(family_id)
        if included:
            if path.endswith(".json"):
                out[path] = dumps(
                    {
                        "org_id": "fixture-org",
                        "space_id": "fixture-space",
                        "project_id": "fixture-project",
                        "instructions": {"always": [], "enabled": False},
                    }
                )
            elif path.endswith((".yml", ".yaml")):
                out[path] = "auto-commits: false\nread: []\nignore:\n  - CONVENTIONS.md\n"
            else:
                out[path] = "## ignored\nThis instruction is excluded from discovery.\n"
                out[".ctxpect-ignore"] = path + "\n"
        else:
            rebuilt = _instruction_body(family_id, "positive", "include")
            out.update(rebuilt)
    elif capability_id == "rules":
        rebuilt = _rules_files(family_id, "negative" if included else "positive")
        out.update(rebuilt)
    elif capability_id == "skills":
        rebuilt = _skill_files(family_id, "negative" if included else "positive")
        for name in list(out):
            if name.endswith("SKILL.md") or name == "skill-index.json":
                if included:
                    del out[name]
        out.update(rebuilt)
    elif capability_id == "mcp-declarations":
        out.update(_mcp_files(family_id, "negative" if included else "positive"))
    elif capability_id == "plugins":
        out.update(_plugin_files(family_id, "negative" if included else "positive"))
    elif capability_id == "commands":
        if included:
            out[".agents/commands/fixture.md"] = "# /fixture\n\n# disabled\n"
            out[".agents/hooks/on-command.json"] = dumps(
                {"event": "user-command", "command": "/fixture", "enabled": False}
            )
        else:
            out[".agents/commands/fixture.md"] = "# /fixture\n\nRun the fixture checklist.\n"
            out[".agents/hooks/on-command.json"] = dumps(
                {"event": "user-command", "command": "/fixture", "enabled": True}
            )
            out[".agents/workflows/fixture.yaml"] = "name: fixture\nsteps:\n  - /fixture\n"
    elif capability_id == "agents":
        path = ".agents/agents/reviewer.md"
        out[path] = (
            "---\nname: reviewer\nenabled: false\n---\n"
            if included
            else "---\nname: reviewer\n---\nRead-only review subagent.\n"
        )
    elif capability_id == "memory":
        out[".agents/memory/user.md"] = "" if included else "# User memory\nPrefer small diffs.\n"
    elif capability_id == "environment-metadata":
        out["environment.json"] = dumps(
            {
                "cwd": "/workspace/fixture",
                "git_root": "/workspace/fixture",
                "os": "macOS",
                "model": "fixture-model",
                "present": not included,
            }
        )
    elif capability_id == "permission-description":
        out["sandbox.json"] = dumps(
            {
                "mode": "unavailable" if included else "workspace",
                "network": False,
                "filesystem": "workspace",
            }
        )
    elif capability_id == "tool-schema-catalog":
        tools = [] if included else [{"name": "read_file", "parameters": {"path": {"type": "string"}}}]
        out["tools.schema.json"] = dumps({"tools": tools})
    elif capability_id == "tool-invocation":
        out["runtime/tool-call.json"] = dumps(
            {"tool": "read_file", "path": "README.md", "present": not included}
        )
    elif capability_id == "task-message":
        out["runtime/task.json"] = dumps(
            {"role": "user", "text": "Inspect fixture context.", "present": not included}
        )
    elif capability_id == "history":
        out["runtime/history.jsonl"] = "" if included else dumps({"role": "user", "text": "hello"})
    elif capability_id == "compaction-steering":
        out["runtime/compaction.json"] = dumps(
            {"summary": "prior turns compacted", "present": not included}
        )
    else:
        path = instruction_path(family_id)
        out[path] = (out.get(path) or "") + "\n# mutated\n"
    if envelope is not None:
        out[ENVELOPE] = envelope
    return out


def expected_claim_from_parse(parsed: dict[str, Any], polarity: str | None = None) -> dict[str, Any]:
    truth = parsed.get("truth_state") or "indeterminate"
    if truth == "indeterminate":
        knowledge, coverage = "unknown", "unknown"
    else:
        knowledge, coverage = "current", "partial-declared-surface"
    if polarity is None:
        if parsed.get("loss_report_required"):
            polarity = "loss"
        elif truth == "indeterminate":
            polarity = "indeterminate"
        elif truth == "absent":
            polarity = "negative"
        else:
            polarity = "positive"
    claim = {
        "claim_kind": "resolved" if parsed.get("resolver") == "static" else "observed",
        "lifecycle_stage": "discoverable" if parsed.get("resolver") == "static" else "model-visible",
        "truth_state": truth,
        "provenance": "official-spec" if parsed.get("resolver") == "static" else "native-runtime",
        "coverage": coverage,
        "precision": "derived" if parsed.get("resolver") == "static" else "exact",
        "knowledge_status": knowledge,
        "polarity": polarity,
    }
    if truth == "indeterminate":
        claim["unknown_reason_code"] = parsed.get("unknown_reason_code") or "surface_not_exposed"
        claim["precision"] = "not-applicable"
    if parsed.get("loss_report_required"):
        claim["loss_report_required"] = True
    return claim


def build_static_files(family_id: str, capability_id: str, polarity: str, class_name: str) -> dict[str, str]:
    envelope: dict[str, Any] = {
        "schema_version": SCHEMA_VERSION,
        "kind": "static-fixture",
        "family_id": family_id,
        "capability_id": capability_id,
        "polarity": polarity,
        "class": class_name,
    }
    files: dict[str, str] = {}
    if polarity == "indeterminate":
        envelope["kind"] = "unknown-honesty"
        files[ENVELOPE] = dumps(envelope)
        return files
    if polarity == "loss":
        envelope["kind"] = "loss"
        files.update(_instruction_body(family_id, "positive", "include"))
        files.update(_loss_files(family_id, capability_id))
        envelope["native_paths"] = sorted(files)
        files[ENVELOPE] = dumps(envelope)
        return files

    if capability_id == "instructions":
        files.update(_instruction_body(family_id, polarity, class_name))
    elif capability_id == "rules":
        files.update(_rules_files(family_id, polarity))
    elif capability_id == "skills":
        files.update(_skill_files(family_id, polarity))
    elif capability_id == "mcp-declarations":
        files.update(_mcp_files(family_id, polarity))
    elif capability_id == "plugins":
        files.update(_plugin_files(family_id, polarity))
    elif capability_id == "commands":
        path = ".agents/commands/fixture.md"
        files[path] = "# /fixture\n\nRun the fixture checklist.\n" if polarity == "positive" else "# /fixture\n\n# disabled\n"
        files[".agents/hooks/on-command.json"] = dumps(
            {"event": "user-command", "command": "/fixture", "enabled": polarity == "positive"}
        )
        files[".agents/workflows/fixture.yaml"] = (
            "name: fixture\nsteps:\n  - /fixture\n" if polarity == "positive" else "name: fixture\nenabled: false\n"
        )
    elif capability_id == "agents":
        path = ".agents/agents/reviewer.md"
        files[path] = "---\nname: reviewer\n---\nRead-only review subagent.\n" if polarity == "positive" else "---\nname: reviewer\nenabled: false\n---\n"
    elif capability_id == "memory":
        files[".agents/memory/user.md"] = "# User memory\nPrefer small diffs.\n" if polarity == "positive" else ""
    elif capability_id == "environment-metadata":
        files["environment.json"] = dumps(
            {
                "cwd": "/workspace/fixture",
                "git_root": "/workspace/fixture",
                "os": "macOS",
                "model": "fixture-model",
                "present": polarity == "positive",
            }
        )
    elif capability_id == "permission-description":
        files["sandbox.json"] = dumps(
            {
                "mode": "workspace" if polarity == "positive" else "unavailable",
                "network": False,
                "filesystem": "workspace",
            }
        )
    elif capability_id == "tool-schema-catalog":
        tools = [{"name": "read_file", "parameters": {"path": {"type": "string"}}}] if polarity == "positive" else []
        files["tools.schema.json"] = dumps({"tools": tools})
    elif capability_id == "tool-invocation":
        files["runtime/tool-call.json"] = dumps(
            {"tool": "read_file", "path": "README.md", "present": polarity == "positive"}
        )
    elif capability_id == "task-message":
        files["runtime/task.json"] = dumps(
            {"role": "user", "text": "Inspect fixture context.", "present": polarity == "positive"}
        )
    elif capability_id == "history":
        files["runtime/history.jsonl"] = (
            dumps({"role": "user", "text": "hello"}) + dumps({"role": "assistant", "text": "ok"})
            if polarity == "positive"
            else ""
        )
    elif capability_id == "compaction-steering":
        files["runtime/compaction.json"] = dumps(
            {"summary": "prior turns compacted", "present": polarity == "positive"}
        )
    else:
        envelope["kind"] = "unknown-honesty"

    if class_name == "filesystem_edge":
        files["layout.json"] = dumps(
            {"symlinks": [{"from": "rel-link.md", "to": instruction_path(family_id)}]}
        )
    if class_name == "legacy":
        files[".cursorrules"] = "legacy project rule\n"
    envelope["native_paths"] = sorted(p for p in files)
    files[ENVELOPE] = dumps(envelope)
    return files


def build_oracle_files(family_id: str, capability_id: str, polarity: str, command: list[str], shape: dict[str, Any]) -> dict[str, str]:
    probe_name = _oracle_probe_name(family_id)
    result_name = _oracle_result_name(family_id)
    native = instruction_path(family_id)
    files: dict[str, str] = {}
    required = list((shape or {}).get("required_keys") or [])
    if not required:
        required = ["messages"] if family_id == "codex" else ["cwd"]
    if polarity == "indeterminate":
        files[native] = "## withheld\nCore prompt is not exported by this oracle.\n"
    elif polarity == "negative":
        files[native] = "## ignored\nThis item is excluded from native export.\n"
        files[".oracle-exclude"] = native + "\n"
    else:
        files[native] = "## always\nOracle workspace instructions.\n"
    files[result_name] = dumps(_oracle_result_payload(family_id, capability_id, polarity, required))
    files[probe_name] = dumps(
        {
            "command": command,
            "capability_id": capability_id,
            "live_tested": False,
            "result_path": result_name,
            "expected_native_shape": {
                "format": (shape or {}).get("format") or "json",
                "required_keys": required,
            },
        }
    )
    files[ENVELOPE] = dumps(
        {
            "schema_version": SCHEMA_VERSION,
            "kind": "oracle-fixture",
            "family_id": family_id,
            "capability_id": capability_id,
            "class": "oracle",
            "native_paths": sorted(name for name in files),
        }
    )
    return files


def build_doctor_files(rule: str | None, polarity: str, klass: str, idx: int) -> tuple[dict[str, str], list[dict]]:
    files: dict[str, str] = {}
    if klass == "clean" or (polarity == "negative" and rule is None):
        files["AGENTS.md"] = "## always\nKeep fixtures synthetic. No secret literals.\n"
        files[ENVELOPE] = dumps({"schema_version": SCHEMA_VERSION, "kind": "doctor-clean"})
        return files, interpret_doctor(files)

    if polarity == "negative" and rule:
        files, _ = _doctor_negative(rule, idx)
        files[ENVELOPE] = dumps({"schema_version": SCHEMA_VERSION, "kind": "doctor-lookalike"})
        return files, interpret_doctor(files)

    if rule == "secret_literal":
        files["AGENTS.md"] = f"## always\nDOCUMENTATION_FIXTURE_TOKEN=ghp_fixture_not_a_real_secret_{idx:02d}\n"
    elif rule == "symlink_escape":
        files["layout.json"] = dumps({"symlinks": [{"from": "escape.md", "to": "../../outside/secret"}]})
        files["AGENTS.md"] = "## always\nSee layout.json for link inventory.\n"
    elif rule == "hidden_unicode":
        files["AGENTS.md"] = f"## always\nINVIS{ZWSP}IBLE override\n"
    elif rule == "path_containment_escape":
        files["AGENTS.md"] = "## always\nread: ../outside/key\n"
    elif rule == "required_asset_missing":
        files["inventory.json"] = dumps({"required": [".agents/skills/needed/SKILL.md"], "present": []})
    elif rule == "unapproved_lossy_projection":
        files["plan.json"] = dumps({"drops": ["scoped-rule"], "approved": False, "authority": "contexpect-native"})
    elif rule == "archive_traversal":
        files["archive-manifest.json"] = dumps({"entries": ["../../etc/passwd"]})
    elif rule == "passive_scan_exec":
        files["hooks.json"] = dumps({"on_scan": ["curl http://example.invalid"]})
    elif rule == "duplicate":
        body = "## always\nDuplicate instruction body.\n"
        files["AGENTS.md"] = body
        files["AGENTS.copy.md"] = body
    elif rule == "conflict":
        files["AGENTS.md"] = "## always\nPackage manager must be npm.\n"
        files["CLAUDE.md"] = "## always\nPackage manager must be pnpm.\n"
    elif rule == "stale":
        files["AGENTS.md"] = "---\nupdated: 2019-01-01\n---\n## always\nStale instruction.\n"
    elif rule == "oversized_resident":
        files["RESIDENT.md"] = "x" * 256 + "\n"
        files["budget.json"] = dumps({"path": "RESIDENT.md", "max_bytes": 64})
    elif rule == "cap_truncation":
        files["AGENTS.md"] = "## always\n" + ("keep " * 40) + "\n"
        files["budget.json"] = dumps({"path": "AGENTS.md", "max_bytes": 40, "truncated": True})
    elif rule == "bad_frontmatter":
        files["SKILL.md"] = "---\nname: [unterminated\n---\nbody\n"
    elif rule == "gitignore_mismatch":
        files[".ctxpect-gitignore"] = "tracked-secret.md\n"
        files["tracked-secret.md"] = "not-a-real-secret\n"
        files["AGENTS.md"] = "## always\nSee tracked-secret.md\n"
    elif rule == "single_device_only":
        files["device-lock.json"] = dumps({"device_id": "only-device-A", "sync": False})
        files["AGENTS.md"] = "## always\nDevice-scoped instruction.\n"
    elif rule == "unknown_source":
        files["imported.md"] = "Imported without provenance.\n"
        files["provenance.json"] = dumps({"path": "imported.md", "source": None})
    elif rule == "version_incompatible":
        files["adapter-version.json"] = dumps({"required": "99.0.0", "actual": "0.1.0"})
        files["AGENTS.md"] = "## always\nPinned to an unsupported adapter version.\n"
    elif rule == "undiscoverable_path":
        files["layout.json"] = dumps({"files": [{"path": "../outside/AGENTS.md"}]})
    elif rule == "placement_recommendation":
        files["docs/SKILL.md"] = "---\nname: misplaced\n---\nThis skill is not under a skills directory.\n"
        files["placement.json"] = dumps({"path": "docs/SKILL.md", "recommended": ".agents/skills/misplaced/SKILL.md"})
    else:
        files["AGENTS.md"] = "## always\nUnclassified doctor fixture.\n"
    files[ENVELOPE] = dumps({"schema_version": SCHEMA_VERSION, "kind": "doctor-positive"})
    return files, interpret_doctor(files)


def _doctor_negative(rule: str, idx: int) -> tuple[dict[str, str], list[dict]]:
    files: dict[str, str] = {"AGENTS.md": "## always\nLookalike without the triggering grammar.\n"}
    if rule == "secret_literal":
        files["AGENTS.md"] = "## always\nAPI_KEY=documentation-only-placeholder\n"
    elif rule == "symlink_escape":
        files["layout.json"] = dumps({"symlinks": [{"from": "ok.md", "to": "AGENTS.md"}]})
    elif rule == "hidden_unicode":
        files["AGENTS.md"] = "## always\nVisible ASCII only.\n"
    elif rule == "path_containment_escape":
        files["AGENTS.md"] = "## always\nread: ./inside/key\n"
    elif rule == "required_asset_missing":
        files["inventory.json"] = dumps({"required": ["SKILL.md"], "present": ["SKILL.md"]})
        files["SKILL.md"] = "---\nname: present\n---\n"
    elif rule == "unapproved_lossy_projection":
        files["plan.json"] = dumps({"drops": [], "approved": True})
    elif rule == "archive_traversal":
        files["archive-manifest.json"] = dumps({"entries": ["readme.txt"]})
    elif rule == "passive_scan_exec":
        files["hooks.json"] = dumps({"on_scan": []})
    elif rule == "duplicate":
        files["AGENTS.md"] = "## always\nPrimary.\n"
        files["OTHER.md"] = "## always\nDifferent body.\n"
    elif rule == "conflict":
        files["AGENTS.md"] = "## always\nPackage manager must be npm.\n"
        files["CLAUDE.md"] = "## always\nPackage manager must be npm.\n"
    elif rule == "stale":
        files["AGENTS.md"] = "---\nupdated: 2026-09-01\n---\n## always\nCurrent instruction.\n"
    elif rule == "oversized_resident":
        files["RESIDENT.md"] = "small\n"
        files["budget.json"] = dumps({"path": "RESIDENT.md", "max_bytes": 64})
    elif rule == "cap_truncation":
        files["AGENTS.md"] = "## always\nshort\n"
        files["budget.json"] = dumps({"path": "AGENTS.md", "max_bytes": 4000, "truncated": False})
    elif rule == "bad_frontmatter":
        files["SKILL.md"] = "---\nname: ok\n---\nbody\n"
    elif rule == "gitignore_mismatch":
        files[".ctxpect-gitignore"] = "build/\n"
        files["AGENTS.md"] = "## always\nDo not ignore source.\n"
    elif rule == "single_device_only":
        files["device-lock.json"] = dumps({"device_id": "*", "sync": True})
    elif rule == "unknown_source":
        files["imported.md"] = "Imported with provenance.\n"
        files["provenance.json"] = dumps({"path": "imported.md", "source": "official-spec"})
    elif rule == "version_incompatible":
        files["adapter-version.json"] = dumps({"required": "0.147.0", "actual": "0.147.0"})
    elif rule == "undiscoverable_path":
        files["layout.json"] = dumps({"files": [{"path": "AGENTS.md"}]})
    elif rule == "placement_recommendation":
        files[".agents/skills/ok/SKILL.md"] = "---\nname: ok\n---\n"
    _ = idx
    return files, []


def load_input_files(root: Path, input_path: str) -> dict[str, str]:
    path = root / input_path
    files: dict[str, str] = {}
    if path.is_file():
        files[path.name] = path.read_text(encoding="utf-8")
        return files
    if path.is_dir():
        for child in path.rglob("*"):
            if child.is_file():
                files[child.relative_to(path).as_posix()] = child.read_text(encoding="utf-8")
    return files


def _native_text(files: dict[str, str]) -> str:
    return "\n".join(body for name, body in files.items() if name != ENVELOPE)


def validate_static_structure(row: dict[str, Any], files: dict[str, str], errors: list[str]) -> None:
    label = row.get("id", "static")
    envelope_text = files.get(ENVELOPE)
    if not envelope_text:
        errors.append(f"{label} missing {ENVELOPE}")
        return
    try:
        envelope = json.loads(envelope_text)
    except json.JSONDecodeError as exc:
        errors.append(f"{label} envelope is not JSON: {exc}")
        return
    if envelope.get("schema_version") != SCHEMA_VERSION:
        errors.append(f"{label} envelope schema_version is not {SCHEMA_VERSION}")
    if envelope.get("capability_id") != row.get("capability_id"):
        errors.append(f"{label} envelope capability_id mismatch")
    combined = "\n".join(files.values())
    if LABEL_RE.search(combined):
        errors.append(f"{label} uses label-only family=/trigger: content")
    parsed = interpret_static(
        row.get("family_id") or envelope.get("family_id") or "",
        row.get("capability_id") or envelope.get("capability_id") or "",
        files,
        surface=row.get("surface"),
        static_support=row.get("static_support"),
        os_lane=row.get("os_lane"),
    )
    polarity = row.get("polarity")
    if polarity == "indeterminate":
        if envelope.get("kind") != "unknown-honesty":
            errors.append(f"{label} indeterminate fixture must be unknown-honesty")
        if parsed.get("truth_state") != "indeterminate":
            errors.append(f"{label} native parse must stay indeterminate")
        return
    if polarity == "loss":
        obj, err = _json_obj_strict(files, LOSS_REPORT)
        if err or not obj:
            errors.append(f"{label} loss native evidence: {err or 'missing loss-report.json'}")
            return
        for item in _loss_report_errors(
            obj,
            row.get("family_id") or envelope.get("family_id") or "",
            row.get("capability_id") or "",
            files=native_files(files),
        ):
            errors.append(f"{label} {item}")
        if not parsed.get("loss_report_required"):
            errors.append(f"{label} loss fixture native parse missing loss_report_required")
        return
    if polarity == "positive" and parsed.get("truth_state") != "present" and not parsed.get("loss_report_required"):
        if parsed.get("truth_state") != "indeterminate":
            errors.append(f"{label} positive native parse is {parsed.get('truth_state')!r}, expected present")
    if polarity == "negative" and parsed.get("truth_state") not in {"absent", "indeterminate"}:
        if not parsed.get("loss_report_required"):
            errors.append(f"{label} negative native parse is {parsed.get('truth_state')!r}, expected absent")


def validate_oracle_structure(row: dict[str, Any], files: dict[str, str], errors: list[str]) -> None:
    label = row.get("id", "oracle")
    envelope_text = files.get(ENVELOPE)
    if not envelope_text:
        errors.append(f"{label} missing {ENVELOPE}")
        return
    try:
        envelope = json.loads(envelope_text)
    except json.JSONDecodeError as exc:
        errors.append(f"{label} oracle envelope is not JSON: {exc}")
        return
    if envelope.get("schema_version") != SCHEMA_VERSION:
        errors.append(f"{label} oracle envelope schema_version is not {SCHEMA_VERSION}")
    if envelope.get("kind") != "oracle-fixture":
        errors.append(f"{label} envelope kind must be oracle-fixture")
    if envelope.get("capability_id") != row.get("capability_id"):
        errors.append(f"{label} oracle envelope capability_id mismatch")
    family_id = row.get("family_id") or envelope.get("family_id") or ""
    probe_name = _oracle_probe_name(family_id)
    result_name = _oracle_result_name(family_id)
    probe, probe_err = _json_obj_strict(files, probe_name)
    if probe_err or not probe:
        errors.append(f"{label} {probe_err or 'missing command/result data'}")
        return
    command = probe.get("command")
    if not command:
        errors.append(f"{label} missing command/result data")
    else:
        if row.get("command") and command != row.get("command"):
            errors.append(f"{label} probe command {command!r} != {row.get('command')!r}")
        declared = _declared_oracle_command(family_id)
        if declared and command != declared:
            errors.append(f"{label} probe command {command!r} != declared {declared!r}")
    result_path = probe.get("result_path")
    if not result_path:
        errors.append(f"{label} missing command/result data")
        return
    if _is_unsafe_relpath(str(result_path)) or result_path != result_name:
        errors.append(f"{label} result path mismatch {result_path!r}")
    if result_path not in files:
        errors.append(f"{label} missing command/result data")
        return
    result, result_err = _json_obj_strict(files, str(result_path))
    if result_err or not result:
        errors.append(f"{label} {result_err or 'missing command/result data'}")
        return
    shape = row.get("expected_native_shape") or {}
    if not shape.get("required_keys"):
        errors.append(f"{label} missing expected native required_keys")
    oracle_id = row.get("oracle_id")
    if not oracle_id:
        oracle_id = "debug-prompt-input" if family_id == "codex" else "inspect-json"
    canonical = ORACLE_NATIVE_SHAPES.get(oracle_id)
    if canonical:
        if list(shape.get("required_keys") or []) != list(canonical["required_keys"]):
            errors.append(f"{label} bogus required keys {shape.get('required_keys')!r}")
        if shape.get("format") and shape.get("format") != canonical["format"]:
            errors.append(f"{label} native shape format mismatch")
        for key in canonical["required_keys"]:
            if key not in result:
                errors.append(f"{label} result missing required native key {key}")
    probe_shape = probe.get("expected_native_shape") if isinstance(probe.get("expected_native_shape"), dict) else {}
    if probe_shape and shape.get("required_keys") and list(probe_shape.get("required_keys") or []) != list(shape.get("required_keys") or []):
        errors.append(f"{label} probe native-shape identity != record")
    native = instruction_path(family_id)
    if native not in files and not any(name.endswith(Path(native).name) for name in files if native):
        errors.append(f"{label} missing oracle workspace instruction file")
    parsed = interpret_oracle(
        family_id,
        row.get("capability_id") or envelope.get("capability_id") or "",
        files,
    )
    if parsed.get("parse", {}).get("parse_error"):
        errors.append(f"{label} oracle native parse failed: {parsed['parse']['parse_error']}")
    if not command and parsed.get("command"):
        errors.append(f"{label} silent fallback to built-in oracle command")


def validate_doctor_structure(row: dict[str, Any], files: dict[str, str], errors: list[str]) -> None:
    label = row.get("id", "doctor")
    combined = "\n".join(files.values())
    if re.search(r"^trigger:\s*[a-z_]+$", combined, re.M):
        errors.append(f"{label} uses label-only trigger: content")
    envelope_text = files.get(ENVELOPE)
    if not envelope_text:
        errors.append(f"{label} missing {ENVELOPE}")
        return
    try:
        envelope = json.loads(envelope_text)
    except json.JSONDecodeError as exc:
        errors.append(f"{label} doctor envelope is not JSON: {exc}")
        return
    if envelope.get("schema_version") != SCHEMA_VERSION:
        errors.append(f"{label} doctor envelope schema_version is not {SCHEMA_VERSION}")
    findings = interpret_doctor(files)
    rule = row.get("rule_id")
    polarity = row.get("polarity")
    if not rule:
        if envelope.get("kind") != "doctor-clean":
            errors.append(f"{label} dedicated clean fixture must be doctor-clean")
        if findings:
            errors.append(f"{label} clean fixture native parse produced findings")
        return
    if polarity == "positive":
        if envelope.get("kind") != "doctor-positive":
            errors.append(f"{label} doctor positive envelope kind mismatch")
        if not any(item.get("rule_id") == rule for item in findings):
            errors.append(f"{label} native content does not produce rule {rule}")
        md_bodies = [body for name, body in files.items() if name.endswith(".md") and name != ENVELOPE]
        if rule == "duplicate" and (len(md_bodies) < 2 or len(set(md_bodies)) != 1):
            errors.append(f"{label} duplicate positive needs two identical markdown bodies")
    if polarity == "negative":
        if envelope.get("kind") not in {"doctor-lookalike", "doctor-clean"}:
            errors.append(f"{label} doctor negative envelope kind mismatch")
        if any(item.get("rule_id") == rule for item in findings):
            errors.append(f"{label} lookalike native parse still matches rule {rule}")
