#!/usr/bin/env python3
"""Foundation stage gate: documentation structure and honesty.

Offline. No network. No third-party packages.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(Path(__file__).resolve().parent))

from contexpect_contract import (  # noqa: E402
    ARTIFACT_DIGEST_MANIFEST,
    CANONICAL_DOCS,
    CANONICAL_STATUS,
    FEATURES,
    GATE_TABLE_DOCS,
    REQUIRED_GATES,
    PRIVATE_SECURITY_URL,
    PUBLIC_SECURITY_FALLBACK_URL,
    ROOT_OSS_FILES,
    WORK_PACKAGES,
)
from contexpect_freeze import is_forbidden_placeholder  # noqa: E402

MD_LINK_RE = re.compile(r"(?<!\!)\[([^\]]+)\]\(([^)]+)\)")
MD_IMAGE_RE = re.compile(r"!\[([^\]]*)\]\(([^)]+)\)")
MD_REF_USE_RE = re.compile(r"(?<!!)\[([^\]]+)\]\[([^\]]*)\]")
MD_REF_IMG_RE = re.compile(r"!\[([^\]]*)\]\[([^\]]*)\]")
MD_REF_DEF_RE = re.compile(r"^ {0,3}\[([^\]]+)\]:\s+<?([^\s>]+)>?(?:\s+.*)?$", re.M)
SKIP_LINK_PREFIXES = ("http://", "https://", "mailto:", "tel:")
SKIP_MD_DIRS = {
    ".octoworkflow",
    "runtime_receipts",
    "__pycache__",
    "node_modules",
}
HOME_UNIX_RE = re.compile(r"(?:^|[\s`\"'(=:,])(/(?:Users|home)/)([^/\s`\"')]+)")
HOME_WIN_RE = re.compile(r"(?:^|[\s`\"'(=:,])([A-Za-z]:\\Users\\)([^\\\s`\"')]+)")
ALLOWED_HOME_USERS = {
    "<name>",
    "<user>",
    "<username>",
    "<you>",
    "$USER",
    "${USER}",
    "NAME",
    "example",
    "placeholder",
    "*",
}
PUBLISHABLE_HOME_SUFFIXES = {".md", ".json", ".yaml", ".yml", ".csv", ".txt", ".jsonl"}

IMPLEMENTED_CLAIM_RE = re.compile(
    r"(已经实现|已经上线|already implemented|currently implements|production already ships|产品运行时已完成)",
    re.IGNORECASE,
)
PLANNED_RE = re.compile(r"(尚未实施|not implemented|规范|planned|not started|未实施)")


def fail(message: str, errors: list[str]) -> None:
    errors.append(message)


def check_exists(errors: list[str]) -> None:
    for rel in ROOT_OSS_FILES + CANONICAL_DOCS:
        path = ROOT / rel
        if not path.is_file():
            fail(f"missing required file: {rel}", errors)
        elif path.stat().st_size == 0:
            fail(f"empty required file: {rel}", errors)


def check_index(errors: list[str]) -> None:
    index = ROOT / "docs/README.md"
    if not index.is_file():
        return
    text = index.read_text(encoding="utf-8")
    for rel in CANONICAL_DOCS:
        if rel == "docs/README.md":
            continue
        needle = rel[len("docs/") :]
        if needle not in text and rel not in text:
            fail(f"docs/README.md does not index {rel}", errors)
    for rel in [
        "acceptance/compatibility-matrix.yaml",
        "acceptance/corpus-manifest.json",
        "acceptance/claim-validity-matrix.yaml",
        "acceptance/context-capability-matrix.yaml",
        "acceptance/projection-matrix.yaml",
        "acceptance/traceability.csv",
        "acceptance/reference-hardware.md",
        "acceptance/integration-contracts.yaml",
        ARTIFACT_DIGEST_MANIFEST,
        "acceptance/semantic-team-contract.yaml",
        "docs/research/2026-09-04-source-backed-coordinates.md",
    ]:
        if rel not in text and rel.split("/", 1)[-1] not in text:
            fail(f"docs/README.md does not index {rel}", errors)


def check_status_banners(errors: list[str]) -> None:
    for rel in CANONICAL_DOCS + ["README.md", "AGENTS.md"]:
        path = ROOT / rel
        if not path.is_file():
            continue
        text = path.read_text(encoding="utf-8")
        if "尚未实施" not in text and "not implemented" not in text.lower() and CANONICAL_STATUS not in text:
            fail(f"{rel} does not state that the product runtime is not implemented", errors)
        for match in IMPLEMENTED_CLAIM_RE.finditer(text):
            start = max(0, match.start() - 80)
            end = min(len(text), match.end() + 80)
            window = text[start:end]
            if not PLANNED_RE.search(window):
                fail(
                    f"{rel} presents planned behavior as implemented near: {match.group(0)!r}",
                    errors,
                )


def check_coverage(errors: list[str]) -> None:
    plan = ROOT / "docs/process/implementation-plan.md"
    if not plan.is_file():
        return
    text = plan.read_text(encoding="utf-8")
    for item in FEATURES + WORK_PACKAGES:
        if item not in text:
            fail(f"implementation plan missing {item}", errors)
    forbidden = re.compile(r"先做 MVP|裁成 MVP|MVP 替代|TODO placeholder|范围裁剪为试验版")
    if forbidden.search(text):
        fail("implementation plan appears to substitute MVP/TODO for scope", errors)

    adr = (ROOT / "docs/adr/0001-rust-tauri-react-sqlite.md").read_text(encoding="utf-8")
    for token in ["Rust", "Tauri 2", "React", "TypeScript", "SQLite", "FTS5"]:
        if token not in adr:
            fail(f"ADR 0001 does not freeze {token}", errors)


def _strip_code_fences(text: str) -> str:
    lines = []
    in_code = False
    for line in text.splitlines():
        if line.strip().startswith("```"):
            in_code = not in_code
            continue
        if not in_code:
            lines.append(line)
    return "\n".join(lines)


def _link_target(raw: str) -> str:
    target = raw.strip()
    if target.startswith("<") and target.endswith(">"):
        target = target[1:-1]
    if " " in target:
        target = target.split(" ", 1)[0]
    if target.startswith('"') or target.startswith("'"):
        return target
    return target


def publishable_markdown(root: Path) -> list[Path]:
    files = []
    for path in root.rglob("*.md"):
        rel_parts = path.relative_to(root).parts
        if any(part in SKIP_MD_DIRS for part in rel_parts):
            continue
        if path.name.endswith(".log"):
            continue
        files.append(path)
    return files


def _check_relative_target(path: Path, root: Path, target: str, errors: list[str], kind: str) -> None:
    if not target or target.startswith("#"):
        return
    if any(target.lower().startswith(prefix) for prefix in SKIP_LINK_PREFIXES):
        return
    if "://" in target:
        return
    file_part = target.split("#", 1)[0]
    if not file_part:
        return
    resolved = (path.parent / file_part).resolve()
    try:
        resolved.relative_to(root.resolve())
    except ValueError:
        fail(f"{path.relative_to(root)} {kind} escapes repo: {target}", errors)
        return
    if not resolved.exists():
        fail(f"{path.relative_to(root)} broken relative {kind}: {target}", errors)


def check_markdown_links(errors: list[str], root: Path | None = None) -> None:
    root = root or ROOT
    for path in publishable_markdown(root):
        text = _strip_code_fences(path.read_text(encoding="utf-8"))
        definitions: dict[str, str] = {}
        for match in MD_REF_DEF_RE.finditer(text):
            definitions[match.group(1).strip().lower()] = match.group(2).strip()
        for match in list(MD_LINK_RE.finditer(text)) + list(MD_IMAGE_RE.finditer(text)):
            _check_relative_target(path, root, _link_target(match.group(2)), errors, "link")
        for match in list(MD_REF_USE_RE.finditer(text)) + list(MD_REF_IMG_RE.finditer(text)):
            label = match.group(1).strip()
            ref = (match.group(2) or "").strip() or label
            key = ref.lower()
            if key not in definitions:
                fail(
                    f"{path.relative_to(root)} missing reference-style definition for [{ref}]",
                    errors,
                )
                continue
            _check_relative_target(path, root, _link_target(definitions[key]), errors, "reference")
        for ref, target in definitions.items():
            _check_relative_target(path, root, _link_target(target), errors, "reference definition")


def check_process_placeholders(errors: list[str], root: Path | None = None) -> None:
    root = root or ROOT
    process_dir = root / "docs" / "process"
    if not process_dir.is_dir():
        return
    for path in process_dir.glob("*.md"):
        text = path.read_text(encoding="utf-8")
        if is_forbidden_placeholder(text):
            fail(f"{path.relative_to(root)} contains a forbidden placeholder token", errors)


def check_security_channel(errors: list[str], root: Path | None = None) -> None:
    root = root or ROOT
    for rel in ("SECURITY.md", "GOVERNANCE.md", "CODE_OF_CONDUCT.md"):
        text = (root / rel).read_text(encoding="utf-8") if (root / rel).is_file() else ""
        if PRIVATE_SECURITY_URL not in text:
            fail(f"{rel} missing GitHub private vulnerability reporting URL", errors)
        if PUBLIC_SECURITY_FALLBACK_URL not in text and rel != "CODE_OF_CONDUCT.md":
            if "issues/new" not in text:
                fail(f"{rel} missing usable public fallback that does not request secrets", errors)
        lowered = text.lower()
        if "paste" in lowered and "secret" in lowered and "do not" not in lowered and "不要" not in text:
            fail(f"{rel} appears to request secret disclosure", errors)


def _is_placeholder_user(segment: str) -> bool:
    if segment in ALLOWED_HOME_USERS:
        return True
    if segment.startswith("<") and segment.endswith(">"):
        return True
    if segment.startswith("$"):
        return True
    return False


NON_PUBLISHABLE_CONTROL_FILES = {
    "control.json",
    "policy.frozen.json",
    "acceptance.frozen.json",
}


def _skipped_home_path(rel_parts: tuple[str, ...]) -> bool:
    if any(part in SKIP_MD_DIRS for part in rel_parts):
        return True
    if rel_parts and rel_parts[0] == "acceptance":
        return True
    # Workflow-control receipts are not product artifacts. Canonical docs stay scanned.
    if len(rel_parts) >= 2 and rel_parts[0] == "docs" and rel_parts[1] == "plan":
        name = rel_parts[-1]
        if name in NON_PUBLISHABLE_CONTROL_FILES:
            return True
        if name.startswith("receipt-") and name.endswith(".json"):
            return True
    return False


def publishable_text_files(root: Path) -> list[Path]:
    files = []
    for path in root.rglob("*"):
        if not path.is_file():
            continue
        rel_parts = path.relative_to(root).parts
        if _skipped_home_path(rel_parts):
            continue
        if path.suffix.lower() not in PUBLISHABLE_HOME_SUFFIXES:
            continue
        files.append(path)
    return files


def check_home_paths(errors: list[str], root: Path | None = None) -> None:
    root = root or ROOT
    for path in publishable_text_files(root):
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        rel = path.relative_to(root).as_posix()
        for match in HOME_UNIX_RE.finditer(text):
            user = match.group(2)
            if _is_placeholder_user(user):
                continue
            fail(f"{rel} contains a real absolute user home path /Users-or-home/{user}/", errors)
        for match in HOME_WIN_RE.finditer(text):
            user = match.group(2)
            if _is_placeholder_user(user):
                continue
            fail(f"{rel} contains a real absolute Windows user home path Users/{user}", errors)


def check_license(errors: list[str]) -> None:
    license_text = (ROOT / "LICENSE").read_text(encoding="utf-8") if (ROOT / "LICENSE").is_file() else ""
    if "Apache License" not in license_text or "Version 2.0" not in license_text:
        fail("LICENSE is not Apache-2.0", errors)
    for rel in ["NOTICE", "SECURITY.md", "GOVERNANCE.md", "CONTRIBUTING.md", "SUPPORT.md"]:
        text = (ROOT / rel).read_text(encoding="utf-8") if (ROOT / rel).is_file() else ""
        if "SBOM" not in text and rel in {"NOTICE", "CONTRIBUTING.md", "docs/process/release.md"}:
            continue
    provenance = ROOT / "docs/process/dependency-and-provenance.md"
    if provenance.is_file():
        text = provenance.read_text(encoding="utf-8")
        for token in ["SBOM", "CycloneDX", "SPDX", "Apache-2.0", "provenance"]:
            if token not in text:
                fail(f"dependency policy missing {token}", errors)


def check_gate_tables(errors: list[str]) -> None:
    """Every canonical gate listing must carry all six required gates, name and command."""
    for rel in GATE_TABLE_DOCS:
        path = ROOT / rel
        if not path.is_file():
            fail(f"missing gate-table document: {rel}", errors)
            continue
        text = path.read_text(encoding="utf-8")
        for name, command in REQUIRED_GATES:
            if name not in text:
                fail(f"{rel} does not list required gate {name}", errors)
            if command not in text:
                fail(f"{rel} does not list the command for gate {name}", errors)


def main() -> int:
    errors: list[str] = []
    check_exists(errors)
    check_index(errors)
    check_status_banners(errors)
    check_coverage(errors)
    check_gate_tables(errors)
    check_license(errors)
    check_markdown_links(errors)
    check_process_placeholders(errors)
    check_security_channel(errors)
    check_home_paths(errors)
    if errors:
        print("check_docs.py FAIL")
        for item in errors:
            print(f"- {item}")
        return 1
    print("check_docs.py PASS")
    print(f"root files: {len(ROOT_OSS_FILES)}")
    print(f"canonical docs: {len(CANONICAL_DOCS)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
