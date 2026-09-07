# 测试策略

> 状态：规范（尚未实施产品运行时）

## 三套 corpus

| 套 | 谁能看答案 | 用途 |
| --- | --- | --- |
| development | 实现者 | 日常 TDD 与回归 |
| sealed | RC 冻结后的独立 gate runner | 防过拟合 |
| live native-oracle | 临时采集的脱敏 evidence/recipe | 与真实 harness 对账 |

Case id 不重叠。实现不得对 sealed/live 写特判。Gate 报告必须分列，不得用 development 高分代替。

## 数量门槛（PRD §17.0 / §17.1）

- 每个 `required-supported` 静态坐标 ≥60 golden cases
- 每个已声明可重复 native oracle ≥12 oracle cases
- Doctor ≥250（≥125 clean、≥125 reachable issue）
- 每个可阻断规则 ≥20 正例和 ≥20 负例
- 可阻断规则在声明 grammar 内 precision = 1.00 才能让 CI exit 2

本 foundation 用**生成夹具**满足数量与分类。它们的 `live_tested` 均为 false。

## 正确性门禁

- item discovery F1 ≥0.95，任何 activation/cap/override 类不得低于 0.90
- claim tuple accuracy ≥0.95
- truncation、secret redaction、path containment、未知版本、partial coverage 分类 100%
- 分母只排除 not-applicable
- 工具不暴露的 surface 进入 unknown-honesty corpus

## 安全门禁

见 PRD §17.3。测试 secret 不得出现在日志、SQLite metadata、Receipt、sync、LLM、SARIF、崩溃报告、executor argv/env/stdio/temp/backup/Git/crash artifact。

## OS lanes

`acceptance/compatibility-matrix.yaml` 的全部 required lane 都要执行，禁止抽样推断。not-applicable 必须有官方支持证据。Ubuntu 24.04.4 live-server SHA-256 与 Windows 11 24H2 build `26100.9278` 已按官方源冻结；Windows ISO 内容哈希为 `digest-not-published-by-source`。

## UI

Doctor 截图与 `images/10-context-doctor-final.png` 对照构图；文案以 `03-screen-specs.md` 为准。程序化检查 18 个 family 名称。键盘与读序必须实操，不只跑自动 a11y。

## 本阶段实际可跑的测试

本阶段 13 条 required gate（名称 + 命令）必须一起跑，不得只跑其中一部分：

| 名称 | 命令 |
| --- | --- |
| docs-structure | `python3 scripts/check_docs.py` |
| acceptance-validation | `python3 scripts/check_acceptance.py --structure` |
| traceability-validation | `python3 scripts/check_acceptance.py --traceability` |
| corpus-validation | `python3 scripts/check_acceptance.py --corpus` |
| semantic-team-validation | `python3 scripts/check_semantic_team.py` |
| validator-negative-tests | `TMPDIR=/tmp python3 -m unittest discover -s tests/acceptance -p 'test_*.py'` |
| cargo-build | `cargo build --workspace` |
| cargo-test | `cargo test --workspace` |
| cargo-clippy | `cargo clippy --workspace --all-targets` |

```bash
python3 scripts/check_docs.py
python3 scripts/check_acceptance.py --structure
python3 scripts/check_acceptance.py --traceability
python3 scripts/check_acceptance.py --corpus
python3 scripts/check_semantic_team.py
TMPDIR=/tmp python3 -m unittest discover -s tests/acceptance -p 'test_*.py'
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
```

不要把这些脚本的绿当成 F1/oracle 已测。`check_semantic_team.py` 只证明语义对齐与 Team Context Standard 的生成夹具/合同，不证明产品运行时。

### 前端/UI required gate（本阶段新增，与原九条并列）

cwd 见下表。env：不强制 `CARGO_NET_OFFLINE`，不设 `CI`。fixture：`packages/ui/src/routes.ts`、`packages/ui-tokens/tokens.js`。lane：development。失败判据：缺 V01–V16 路由（含详情页）、C03 token 漂移、`tsc` 错误、Vite build 非 0。

| 名称 | cwd | 命令 | 版本 |
| --- | --- | --- | --- |
| ui-routes | 仓库根 | `python3 scripts/check_ui_routes.py` | Python 3.12 |
| ui-unit | `packages/ui` | `pnpm test`（`node --test tests/routes.test.mjs tests/i18n.test.mjs tests/mask.test.mjs`） | Node ≥22，pnpm 11.20.0 |
| ui-typecheck | `packages/ui` | `pnpm typecheck` | typescript 5.7.3 |
| ui-build | `packages/ui` | `pnpm build` | vite 6.0.11 |

未跑或失败不得记为通过。Tauri 桌面壳尚未创建，因此没有 Tauri required gate。

语义对齐夹具必须覆盖 ST1–ST8 正负例：CanonicalIntent 不是文件副本；四 harness native-equivalent 但字节不同；overlay/loss/round-trip；签名 TeamContextStandard；分层与 detect-only；exception fail-closed；leader 脱敏；以及 malformed/forged/partial bundle。byte/hash 相等不得作为 pass。
