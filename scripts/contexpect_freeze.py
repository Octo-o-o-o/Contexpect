#!/usr/bin/env python3
"""Source-backed freeze coordinates for the foundation acceptance cutoff.

Retrieved 2026-09-04. Owner authorized web search only for Ubuntu, Windows,
DeepSeek Harness, Coze, Cline, Aider, OpenHands, and Windsurf. Integration
projects that were not in that retrieval set are recorded as
evidence-backed-unavailable rather than invented pins.

Do not invent image/package digests. If a first-party source does not
publish a digest, freeze the official build/release identifier and set
digest_status to digest-not-published-by-source.
"""

from __future__ import annotations

ACCESS_DATE = "2026-09-04"
CUTOFF = "2026-09-04T23:59:59+08:00"

DIGEST_NOT_PUBLISHED = "digest-not-published-by-source"
EVIDENCE_BACKED_UNAVAILABLE = "evidence-backed-unavailable"

# Forbidden as exact values or substrings in generated artifacts and process docs.
# Composite bypasses such as official-same-version-not-published-or-not-captured
# must fail. Do not match bare "hermetic" (cohort expansion-hermetic is legitimate).
FORBIDDEN_PLACEHOLDER_TOKENS = (
    "not-captured",
    "cutoff-unverified",
    "inspect-at-pin-time",
    "hermetic-cutoff",
)

UBUNTU_24_04 = {
    "id": "ubuntu-24.04-x86_64",
    "os": "Ubuntu",
    "version": "24.04.4 LTS",
    "codename": "noble",
    "arch": "x86_64",
    "image_name": "ubuntu-24.04.4-live-server-amd64.iso",
    "image_digest": "sha256:e907d92eeec9df64163a7e454cbc8d7755e8ddc7ed42f99dbc80c40f1a138433",
    "digest_status": "official-sha256sums",
    "build": "24.04.4",
    "live_status": "official-image-frozen",
    "source_url": "https://releases.ubuntu.com/noble/SHA256SUMS",
    "source_also": [
        "https://releases.ubuntu.com/24.04/",
        "https://releases.ubuntu.com/noble/",
    ],
    "source_published": "2026-02-12",
    "access_date": ACCESS_DATE,
    "evidence_scope": (
        "Official Ubuntu SHA256SUMS catalog for the 24.04.4 live-server amd64 ISO. "
        "This is the hermetic performance-gate image for Ubuntu 24.04 LTS x86_64. "
        "No 24.04.5 ISO was listed in the same catalog at access time."
    ),
    "license_or_provenance": "Canonical Ubuntu image; checksums from releases.ubuntu.com",
}

WINDOWS_11_24H2 = {
    "id": "windows-11-24h2-x86_64",
    "os": "Windows",
    "version": "11 24H2",
    "arch": "x86_64",
    "image_name": "Win11_24H2_English_x64.iso",
    "os_build_family": "26100",
    "build": "26100.9278",
    "kb": "KB5120998",
    "update_type": "2026-08 D",
    "latest_revision_date": "2026-08-27",
    "image_digest": DIGEST_NOT_PUBLISHED,
    "digest_status": DIGEST_NOT_PUBLISHED,
    "live_status": "official-build-frozen",
    "source_url": "https://learn.microsoft.com/en-us/windows/release-health/windows11-release-information",
    "source_published": "2026-08-28",
    "access_date": ACCESS_DATE,
    "evidence_scope": (
        "Microsoft Windows 11 release information lists version 24H2 as OS build family 26100, "
        "latest GA build 26100.9278 / KB5120998 (2026-08 D, revision 2026-08-27). "
        "Microsoft publishes ISO SHA-256 only on the ephemeral software-download page, not in a "
        "stable first-party checksum catalog, so the image digest is recorded as "
        "digest-not-published-by-source. The hermetic lane is pinned to this official build identity."
    ),
    "license_or_provenance": "Microsoft Windows 11 24H2 official servicing identity",
}

DEEPSEEK_HARNESS = {
    "family_id": "deepseek-harness",
    "surface": "cli",
    "version": "0.1.2-rc.1",
    "release_tag": "dsh-v0.1.2-rc.1",
    "git_commit": "a66e470",
    "package": "@deepseek-ai/dsh",
    "version_source": "github-immutable-release",
    "image_digest": DIGEST_NOT_PUBLISHED,
    "digest_status": DIGEST_NOT_PUBLISHED,
    "source_url": "https://github.com/deepseek-ai/deepseek-harness/releases/tag/dsh-v0.1.2-rc.1",
    "repo": "https://github.com/deepseek-ai/deepseek-harness",
    "released": "2026-09-03",
    "access_date": ACCESS_DATE,
    "evidence_scope": (
        "Official GitHub immutable prerelease dsh-v0.1.2-rc.1 (commit a66e470) is the newest "
        "DeepSeek Harness tag at cutoff. The project is in developer preview. npm tarball "
        "integrity was not published on the GitHub release page and was not retrieved from the "
        "npm registry in this freeze."
    ),
    "license_or_provenance": "MIT (repository LICENSE)",
}

CLINE = {
    "family_id": "cline",
    "surface": "cli",
    "version": "3.0.61",
    "release_tag": "cli-v3.0.61",
    "git_commit": "595f1db",
    "package": "cline",
    "version_source": "github-release-and-npm",
    "image_digest": DIGEST_NOT_PUBLISHED,
    "digest_status": DIGEST_NOT_PUBLISHED,
    "source_url": "https://github.com/cline/cline/releases/tag/cli-v3.0.61",
    "npm_url": "https://www.npmjs.com/package/cline",
    "released": "2026-09-02",
    "access_date": ACCESS_DATE,
    "evidence_scope": (
        "Official Cline CLI release cli-v3.0.61 (commit 595f1db) and npm package cline@3.0.61. "
        "This freeze is the CLI surface, not the VS Code extension v4.1.17 or desktop-v0.0.23. "
        "npm tarball integrity was not copied from the registry JSON in this retrieval."
    ),
    "license_or_provenance": "Apache-2.0",
}

AIDER = {
    "family_id": "aider",
    "surface": "cli",
    "version": "0.86.2",
    "release_tag": "v0.86.2",
    "git_commit": "253f036",
    "package": "aider-chat",
    "version_source": "pypi-and-github-tag",
    "sdist_url": (
        "https://files.pythonhosted.org/packages/39/45/"
        "71111a018c653b7e743216188fb73cd640a86abbda56b7e430f65cd45d23/"
        "aider_chat-0.86.2.tar.gz"
    ),
    "image_digest": "sha256:f38a9d322f5609f0c13af82d50c6a11170185b3fdf26956e4a7e89ba19819159",
    "digest_status": "official-sdist-sha256-via-homebrew-formula",
    "source_url": "https://github.com/Aider-AI/aider/releases/tag/v0.86.2",
    "pypi_url": "https://pypi.org/project/aider-chat/0.86.2/",
    "digest_attestation": (
        "Homebrew/homebrew-core Formula/a/aider.rb quotes this SHA-256 for the official "
        "files.pythonhosted.org sdist of aider_chat-0.86.2.tar.gz."
    ),
    "released": "2026-02-12",
    "access_date": ACCESS_DATE,
    "evidence_scope": (
        "Latest PyPI/GitHub non-dev release of aider-chat at cutoff is 0.86.2 "
        "(git tag v0.86.2, commit 253f036)."
    ),
    "license_or_provenance": "Apache-2.0",
}

OPENHANDS = {
    "family_id": "openhands",
    "surface": "sdk",
    "version": "1.16.0",
    "release_tag": "v1.16.0",
    "git_commit": "64c1269",
    "image_name": "ghcr.io/openhands/agent-canvas:1.16.0",
    "version_source": "github-release",
    "image_digest": DIGEST_NOT_PUBLISHED,
    "digest_status": DIGEST_NOT_PUBLISHED,
    "source_url": "https://github.com/OpenHands/OpenHands/releases/tag/v1.16.0",
    "released": "2026-08-27",
    "access_date": ACCESS_DATE,
    "evidence_scope": (
        "Official GitHub release v1.16.0 (commit 64c1269). README pins "
        "ghcr.io/openhands/agent-canvas:1.16.0. The GHCR image digest was not listed on the "
        "release page in this retrieval."
    ),
    "license_or_provenance": "MIT",
}

WINDSURF = {
    "family_id": "windsurf",
    "surface": "ide",
    "version": "2.3.15",
    "release_tag": "2.3.15",
    "git_commit": "c46c49e94b4d3f41181204d59809d8f1b2c48d68",
    "version_source": "official-releases-page",
    "image_digest": DIGEST_NOT_PUBLISHED,
    "digest_status": DIGEST_NOT_PUBLISHED,
    "source_url": "https://windsurf.com/editor/releases",
    "binary_url": (
        "https://windsurf-stable.codeiumdata.com/win32-x64/stable/"
        "c46c49e94b4d3f41181204d59809d8f1b2c48d68/WindsurfSetup-x64-2.3.15.exe"
    ),
    "released": "2026-05-27",
    "access_date": ACCESS_DATE,
    "evidence_scope": (
        "Official Windsurf releases page lists 2.3.15 as the newest Windsurf-branded editor "
        "build. The CDN path includes immutable build id "
        "c46c49e94b4d3f41181204d59809d8f1b2c48d68. No SHA-256 catalog is published on that "
        "page. Later Devin Desktop 3.x releases exist after the product rename; they are not "
        "this Windsurf family freeze."
    ),
    "license_or_provenance": "proprietary Windsurf/Cognition editor; binary from official CDN",
}

COZE = {
    "family_id": "coze",
    "surface": "connector",
    "version": "0.3.10",
    "release_tag": "0.3.10",
    "package": "@coze/cli",
    "version_source": "npm-latest",
    "image_digest": DIGEST_NOT_PUBLISHED,
    "digest_status": DIGEST_NOT_PUBLISHED,
    "source_url": "https://www.npmjs.com/package/@coze/cli",
    "released": "2026-08-15",
    "access_date": ACCESS_DATE,
    "evidence_scope": (
        "npm package @coze/cli latest tag is 0.3.10, the cutoff connector/CLI coordinate. "
        "This is not a local desktop install. Local live status remains connector-required. "
        "npm integrity hash was not copied from registry JSON in this retrieval. SPDX license "
        "was not listed in the retrieved npm page metadata."
    ),
    "license_or_provenance": "unknown-not-listed-in-retrieved-npm-metadata",
}

ABSENT_FAMILIES = {
    "deepseek-harness": DEEPSEEK_HARNESS,
    "cline": CLINE,
    "aider": AIDER,
    "openhands": OPENHANDS,
    "windsurf": WINDSURF,
    "coze": COZE,
}

INTEGRATION_UNAVAILABLE = {
    "status": EVIDENCE_BACKED_UNAVAILABLE,
    "access_date": ACCESS_DATE,
    "reason": (
        "Owner-authorized retrieval covered Ubuntu, Windows, DeepSeek Harness, Coze, Cline, "
        "Aider, OpenHands, and Windsurf only. This repository's research ledger records the "
        "GitHub URL but no immutable tag, commit, or package digest was captured for the "
        "integration at cutoff."
    ),
}

INTEGRATION_SOURCES = {
    "Microsoft APM": {
        "source_url": "https://github.com/microsoft/apm",
        "license_from_research": "unknown",
    },
    "Agentpack": {
        "source_url": "https://github.com/liqiongyu/agentpack",
        "license_from_research": "unknown",
    },
    "agentsync": {
        "source_url": "https://github.com/spxrogers/agentsync",
        "license_from_research": "unknown",
    },
    "CtxWise": {
        "source_url": "https://github.com/FramY2/ctxwise",
        "license_from_research": "Apache-2.0",
    },
    "Scopeon": {
        "source_url": "https://github.com/sorunokoe/Scopeon",
        "license_from_research": "MIT OR Apache-2.0",
    },
    "ctxray": {
        "source_url": "https://github.com/ctxray/ctxray",
        "license_from_research": "MIT",
    },
    "ContextSpy": {
        "source_url": "https://github.com/RimantasZ/contextspy",
        "license_from_research": "Apache-2.0",
    },
    "age or SOPS": {
        "source_url": "https://github.com/FiloSottile/age",
        "license_from_research": "unknown",
    },
    "SignerAdapter (SSH/GPG/minisign/Sigstore/existing trust store)": {
        "source_url": "https://github.com/sigstore",
        "license_from_research": "unknown",
    },
}


def is_forbidden_placeholder(value: object) -> bool:
    if not isinstance(value, str):
        return False
    return any(token in value for token in FORBIDDEN_PLACEHOLDER_TOKENS)


NATIVE_TARGETS = {
    "project-instructions": {
        "codex": {"kind": "native-file", "surface": "cli", "path_glob": "**/AGENTS.md", "scope": "project"},
        "claude-code": {"kind": "native-file", "surface": "cli", "path_glob": "**/CLAUDE.md", "scope": "project"},
        "cursor": {"kind": "native-file", "surface": "ide", "path_glob": "**/.cursor/rules/**", "scope": "project"},
        "grok-build": {"kind": "native-file", "surface": "cli", "path_glob": "**/AGENTS.md", "scope": "project"},
    },
    "user-instructions": {
        "codex": {"kind": "native-file", "surface": "cli", "path_glob": "user-level AGENTS/instructions", "scope": "user"},
        "claude-code": {"kind": "native-file", "surface": "cli", "path_glob": "user-level CLAUDE.md", "scope": "user"},
        "cursor": {"kind": "ui-handoff", "surface": "ide", "path_glob": "User Rules UI (no stable write surface)", "scope": "user"},
        "grok-build": {"kind": "native-file", "surface": "cli", "path_glob": "user-level AGENTS/instructions", "scope": "user"},
    },
    "packaged-skills": {
        "codex": {"kind": "apm-package", "surface": "cli", "path_glob": "APM-managed skill package", "scope": "project-or-user"},
        "claude-code": {"kind": "apm-package", "surface": "cli", "path_glob": "APM-managed skill package", "scope": "project-or-user"},
        "cursor": {"kind": "apm-package", "surface": "ide", "path_glob": "APM-managed skill package", "scope": "project-or-user"},
        "grok-build": {"kind": "apm-package", "surface": "cli", "path_glob": "APM-managed skill package", "scope": "project-or-user"},
    },
    "unmanaged-skills": {
        "codex": {"kind": "native-file", "surface": "cli", "path_glob": "**/.agents/skills/**/SKILL.md", "scope": "project-or-user"},
        "claude-code": {"kind": "native-file", "surface": "cli", "path_glob": "**/.claude/skills/**/SKILL.md", "scope": "project-or-user"},
        "cursor": {"kind": "native-file", "surface": "ide", "path_glob": "**/.cursor/skills/**/SKILL.md", "scope": "project-or-user"},
        "grok-build": {"kind": "native-file", "surface": "cli", "path_glob": "**/.grok/skills/**/SKILL.md", "scope": "project-or-user"},
    },
    "scoped-conditional-rules": {
        "codex": {"kind": "loss-to-project-instructions", "surface": "cli", "path_glob": "no independent scoped-rule primitive", "scope": "project"},
        "claude-code": {"kind": "native-file", "surface": "cli", "path_glob": "**/.claude/rules/**", "scope": "project"},
        "cursor": {"kind": "native-file", "surface": "ide", "path_glob": "**/.cursor/rules/**", "scope": "project"},
        "grok-build": {"kind": "loss-to-project-instructions", "surface": "cli", "path_glob": "no independent scoped-rule primitive", "scope": "project"},
    },
    "native-mcp-config": {
        "codex": {"kind": "native-file", "surface": "cli", "path_glob": "**/.mcp.json", "scope": "public-writable-user-or-project"},
        "claude-code": {"kind": "native-file", "surface": "cli", "path_glob": "**/.mcp.json", "scope": "public-writable-user-or-project"},
        "cursor": {"kind": "native-file", "surface": "ide", "path_glob": "**/mcp.json", "scope": "public-writable-user-or-project"},
        "grok-build": {"kind": "native-file", "surface": "cli", "path_glob": "**/.mcp.json", "scope": "public-writable-user-or-project"},
    },
    "commands-plugins-hooks-managed-ui": {
        "codex": {"kind": "export-only", "surface": "cli", "path_glob": "commands/plugins/hooks/managed UI", "scope": "various"},
        "claude-code": {"kind": "export-only", "surface": "cli", "path_glob": "commands/plugins/hooks/managed UI", "scope": "various"},
        "cursor": {"kind": "export-only", "surface": "ide", "path_glob": "commands/plugins/hooks/managed UI", "scope": "various"},
        "grok-build": {"kind": "export-only", "surface": "cli", "path_glob": "commands/plugins/hooks/managed UI", "scope": "various"},
    },
}

FAMILY_INPUT_SPECS = {
    "codex": ("AGENTS.md", "markdown"),
    "claude-code": ("CLAUDE.md", "markdown"),
    "cursor": (".cursor/rules/contexpect.mdc", "markdown"),
    "grok-build": ("AGENTS.md", "markdown"),
    "opencode": ("AGENTS.md", "markdown"),
    "deepseek-harness": ("AGENTS.md", "markdown"),
    "kimi-code": ("AGENTS.md", "markdown"),
    "zcode": ("AGENTS.md", "markdown"),
    "qwen-code": ("QWEN.md", "markdown"),
    "goose": (".goosehints", "text"),
    "gemini-cli": ("GEMINI.md", "markdown"),
    "github-copilot-cli": (".github/copilot-instructions.md", "markdown"),
    "kiro": (".kiro/steering/product.md", "markdown"),
    "cline": (".clinerules", "markdown"),
    "aider": (".aider.conf.yml", "yaml"),
    "openhands": ("AGENTS.md", "markdown"),
    "windsurf": (".windsurf/rules.md", "markdown"),
    "coze": (".cozerc.json", "json"),
}
