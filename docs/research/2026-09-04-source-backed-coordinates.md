# Source-backed freeze coordinates

> 状态：规范（尚未实施产品运行时）
> Access date: `2026-09-04`
> Cutoff: `2026-09-04T23:59:59+08:00`

本文件记录 owner 授权检索后写入验收合同的第一方坐标。禁止把本机未观察到的 native oracle 写成已捕获。若官方源没有发布 digest，冻结官方 build/release 标识并标注 `digest-not-published-by-source`。

## OS lanes

| Lane | Official identity | Digest / build | Source | Evidence scope | License / provenance |
| --- | --- | --- | --- | --- | --- |
| Ubuntu 24.04 LTS x86_64 | `ubuntu-24.04.4-live-server-amd64.iso` | `sha256:e907d92eeec9df64163a7e454cbc8d7755e8ddc7ed42f99dbc80c40f1a138433` | https://releases.ubuntu.com/noble/SHA256SUMS (2026-02-12) | Hermetic performance-gate image; 24.04.5 was not listed in that catalog at access time | Canonical Ubuntu image |
| Windows 11 24H2 x86_64 | Windows 11, version 24H2, OS build family 26100 | Build `26100.9278` / `KB5120998` (2026-08 D, revision 2026-08-27); ISO digest `digest-not-published-by-source` | https://learn.microsoft.com/en-us/windows/release-health/windows11-release-information | Official servicing identity for the 24H2 hermetic lane. Microsoft does not publish a stable ISO SHA-256 catalog | Microsoft Windows 11 24H2 |
| macOS 27.0 arm64 | build `26A5425a` | `os-build:26A5425a` | `docs/research/2026-09-04-local-environment-receipt.md` | Research capture machine, not the performance minimum | Apple macOS |

## Locally absent families

| Family | Frozen coordinate | Immutable id | Source | Evidence scope | License / provenance |
| --- | --- | --- | --- | --- | --- |
| DeepSeek Harness CLI | `0.1.2-rc.1` | GitHub tag `dsh-v0.1.2-rc.1`, commit `a66e470` | https://github.com/deepseek-ai/deepseek-harness/releases/tag/dsh-v0.1.2-rc.1 | Newest official immutable GitHub release at cutoff. Live lane remains `not-installed`. npm tarball digest `digest-not-published-by-source` | MIT |
| Cline CLI | `3.0.61` | tag `cli-v3.0.61`, commit `595f1db`, npm `cline@3.0.61` | https://github.com/cline/cline/releases/tag/cli-v3.0.61 | CLI surface only; not VS Code `v4.1.17` or desktop. Live lane `not-installed`. npm integrity `digest-not-published-by-source` | Apache-2.0 |
| Aider CLI | `0.86.2` | tag `v0.86.2`, commit `253f036`, PyPI `aider-chat==0.86.2` | https://github.com/Aider-AI/aider/releases/tag/v0.86.2 | Latest non-dev PyPI/GitHub release at cutoff. sdist SHA-256 `f38a9d322f5609f0c13af82d50c6a11170185b3fdf26956e4a7e89ba19819159` quoted from Homebrew core for the official pythonhosted tarball | Apache-2.0 |
| OpenHands SDK | `1.16.0` | tag `v1.16.0`, commit `64c1269`, image tag `ghcr.io/openhands/agent-canvas:1.16.0` | https://github.com/OpenHands/OpenHands/releases/tag/v1.16.0 | Official GitHub release. GHCR digest `digest-not-published-by-source`. Live lane `not-installed` | MIT |
| Windsurf IDE | `2.3.15` | build `c46c49e94b4d3f41181204d59809d8f1b2c48d68` | https://windsurf.com/editor/releases | Newest Windsurf-branded editor on the official releases page. No SHA-256 catalog. Later Devin Desktop 3.x is a successor name, not this freeze. Live lane `not-installed` | proprietary editor binary from official CDN |
| Coze connector | `@coze/cli@0.3.10` | npm latest `0.3.10` (published 2026-08-15) | https://www.npmjs.com/package/@coze/cli | Connector/CLI coordinate, not a local desktop install. Live lane remains `connector-required`. npm integrity and SPDX were not listed in the retrieved npm metadata | `unknown-not-listed-in-retrieved-npm-metadata` |

## Integration contracts

Owner-authorized retrieval did not cover APM, Agentpack, agentsync, CtxWise, Scopeon, ctxray, ContextSpy, age/SOPS, or SignerAdapter. Those pins are `evidence-backed-unavailable` with GitHub URLs from the research ledger and no invented tag/digest.

## Non-claims

- These coordinates are not live native oracle results.
- Ubuntu/Windows harness installs remain `unknown` on those OS lanes; only the OS image/build is frozen.
- Generated fixtures that use these versions stay `live_tested: false`.
