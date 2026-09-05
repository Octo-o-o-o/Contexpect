# Reference hardware and dataset freeze

> 状态：规范（尚未实施产品运行时）
> cutoff: `2026-09-04T23:59:59+08:00`

本文件冻结性能测试对照条件。它不是一份已跑分报告。Ubuntu 官方 live-server ISO SHA-256 与 Windows 11 24H2 官方 build 已按 cutoff 前第一方目录固定。Windows ISO 内容 SHA-256 未被 Microsoft 放入稳定 checksum catalog，因此记录 `digest-not-published-by-source`，禁止编造。

## Research capture machine (not the performance minimum)

| Field | Value | Honesty |
| --- | --- | --- |
| OS lane | macOS 27.0 build 26A5425a arm64 | Captured in `docs/research/2026-09-04-local-environment-receipt.md` |
| CPU | Apple M5 Pro | Captured |
| Memory | 51539607552 bytes | Captured |
| Logical CPUs | 18 | Captured |
| Filesystem | APFS | implied by macOS lane; not independently hashed |
| Role | research / documentation capture | Must not replace the three performance gate lanes |

Do not copy absolute home directories into other publishable files.

## Performance gate lanes (PRD §14.2)

These lanes are required. Results are not recorded in this stage.

| Lane id | Hardware / OS | Filesystem | Frozen image / build | Status |
| --- | --- | --- | --- | --- |
| perf-macos-m2 | Apple M2 / 16GB / NVMe / macOS | APFS | research receipt is not this lane | not-run |
| perf-ubuntu-24.04 | Ubuntu 24.04 VM 4 vCPU / 8GB | ext4 | `ubuntu-24.04.4-live-server-amd64.iso` `sha256:e907d92eeec9df64163a7e454cbc8d7755e8ddc7ed42f99dbc80c40f1a138433` | not-run; official image frozen |
| perf-windows-11-24h2 | Windows 11 4 core / 16GB | NTFS | Windows 11 24H2 build `26100.9278` / `KB5120998`; ISO digest `digest-not-published-by-source` | not-run; official build frozen |

Source facts: `docs/research/2026-09-04-source-backed-coordinates.md`.

Gate method: record exact hardware, dataset digest, cold cache and warm cache five times each; use the median; any single run above 2× target fails.

## Dataset generator freeze

| Dataset | Generator contract | Digest |
| --- | --- | --- |
| medium-repo | 100000 ordinary files, 1000 candidate context assets, 100 symlinks, 20 nested roots | `generator:medium-repo:v1`; content digest awaits generator implementation in WP-02+ |
| doctor-corpus | see `acceptance/corpus/development/doctor/` | file digests recorded in `acceptance/corpus-manifest.json` |
| static-golden | 60 generated cases per required-supported coordinate | file digests recorded in `acceptance/corpus-manifest.json` |

Generated fixtures are Apache-2.0 synthetic data. They are not live native observations.

## Cache conditions

- Cold: drop filesystem cache according to OS procedure before each of five runs.
- Warm: immediately repeat the same scan without deleting the local snapshot DB.
- Daemon idle window: 30 minutes, 1 second sample, importer idle.

## Explicit non-claims

- This stage did not run the 5s cold / 500ms incremental scan.
- Research-machine specs are not the pass bar.
- Windows ISO bytes were not hashed locally; the frozen identity is the official 24H2 build number.
