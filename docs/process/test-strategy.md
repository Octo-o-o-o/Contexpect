# 测试策略

> 状态：规范（尚未实施产品运行时）

## 三套 corpus

| 套 | 谁能看答案 | 用途 |
| --- | --- | --- |
| development | 实现者 | 日常 TDD 与回归 |
| sealed | RC 冻结后的独立 gate runner | 防过拟合 |
| live native-oracle | 临时采集的脱敏 evidence/recipe | 与真实 harness 对账 |
| native-synthetic | 实现者 | 由 harness 自己的公开 API 在 pinned SHA 生成的合成会话日志与期望值（`acceptance/corpus/development/native/`，`live_tested: false`），只核对原生 importer 的前缀重建，不执行 harness、不扩大冻结 oracle |

Case id 不重叠。实现不得对 sealed/live 写特判。Gate 报告必须分列，不得用 development 高分代替：`corpus-conformance` 与 `adapter test` 共用的 runner 按 coordinate × capability 报 `total / implemented-pass / unknown-honesty-pass / fail / unimplemented` 五列，unknown-honesty 通过不计入 implemented，并在总计后单列 `NOT_EXECUTED sealed=… live=… oracle=… doctor=…`（数值来自 `corpus-manifest.json` 的 fixtures；`doctor` 是 development Doctor 语料，由 `doctor-corpus` 门禁执行而非此 runner）。

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

本阶段 16 条 required gate（名称 + 命令）必须一起跑，不得只跑其中一部分：

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
| corpus-conformance | `cargo test -p ctxpect-cli --test corpus_conformance` |
| doctor-corpus | `cargo test -p ctxpect-cli --test doctor_corpus` |
| native-conformance | `cargo test -p ctxpect-cli --test native_conformance` |

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
cargo test -p ctxpect-cli --test corpus_conformance
cargo test -p ctxpect-cli --test doctor_corpus
cargo test -p ctxpect-cli --test native_conformance
```

不要把这些脚本的绿当成 F1/oracle 已测。`check_semantic_team.py` 只证明语义对齐与 Team Context Standard 的生成夹具/合同，不证明产品运行时。

### 前端/UI required gate（本阶段新增，与其余十二条并列）

cwd 见下表。env：不强制 `CARGO_NET_OFFLINE`，不设 `CI`。fixture：`packages/ui/src/routes.ts`、`packages/ui-tokens/tokens.js`。lane：development。失败判据：缺 V01–V16 路由（含详情页）、C03 token 漂移、`tsc` 错误、Vite build 非 0。

| 名称 | cwd | 命令 | 版本 |
| --- | --- | --- | --- |
| ui-routes | 仓库根 | `python3 scripts/check_ui_routes.py` | Python 3.12 |
| ui-unit | `packages/ui` | `pnpm test`（`node --test tests/routes.test.mjs tests/i18n.test.mjs tests/mask.test.mjs tests/page-state.test.mjs tests/settings-form.test.mjs tests/contrast.test.mjs tests/generation.test.mjs tests/render.test.mjs`；`render.test.mjs` 先以 `vite build --ssr` 编译再用 `react-dom/server` 渲染各页契约声明适用的状态） | Node ≥22，pnpm 11.20.0 |
| ui-typecheck | `packages/ui` | `pnpm typecheck` | typescript 5.7.3 |
| ui-build | `packages/ui` | `pnpm build` | vite 6.0.11 |

CI 模板 `.github/workflows/gates.yml` 在 ubuntu-24.04 与 macos-15 上跑上表全部 required gate（前端四条只在 ubuntu）；远端结果未回来之前不算 lane 已验。

可选门禁（不进 required 表，[ADR 0006](../adr/0006-third-party-dependency-policy-and-estimator.md)）：

| 名称 | cwd | 命令 | 覆盖 |
| --- | --- | --- | --- |
| ui-e2e | `packages/ui` | `pnpm test:e2e` | Playwright 1.56.1 + Chromium headless shell，`global-setup` 构建 UI 到 `tests/.e2e-dist`、在临时项目/store 上起真实 `ctxpect daemon`（`--listen 127.0.0.1:0`）并经 API 种入两份原生会话与一个实验；用例覆盖 `/sessions/:id` 取数与选择、会话切换后旧错误不残留、**取消**（延迟 `/api/v1/receipts` 后点击取消 → `cancelled` 状态且响应不落页）、**离开会话后其迟到的响应被丢弃**（`page.route` 延迟 s-alpha 的请求证据，客户端跳到不存在的会话后错误横幅保留、表格不出现）、`/lab` 列表与明细的执行状态、`/checkup` 真实 inspect 往返。需联网安装浏览器与已构建二进制，因此本机实跑记录退出码，不作为离线 required gate |

未跑或失败不得记为通过。Tauri 桌面壳已存在于独立 workspace `apps/desktop/src-tauri`（macOS lane 已验，见 [交付状态](2026-09-08-delivery-status.md)），但**没有 Tauri required gate**：是否把 `cargo build --manifest-path apps/desktop/src-tauri/Cargo.toml` 纳入门禁取决于第三方依赖政策的决定（缺口分析 A4-(d)）。

语义对齐夹具必须覆盖 ST1–ST8 正负例：CanonicalIntent 不是文件副本；四 harness native-equivalent 但字节不同；overlay/loss/round-trip；签名 TeamContextStandard；分层与 detect-only；exception fail-closed；leader 脱敏；以及 malformed/forged/partial bundle。byte/hash 相等不得作为 pass。
