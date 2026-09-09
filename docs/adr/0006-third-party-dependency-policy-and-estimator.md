# ADR 0006：第三方依赖政策、Effect Lab 估计器与存储偏离的结束条件

> 状态：已接受（2026-09-09，用户决定；估计器已实施，SQLite 继续延期）
> 修订：2026-09-09 交叉 review（Claude 子代理与 Codex `gpt-6-astra` 各自独立复算）指出 v1 的等价区间把不一致率当作已知值——11 对中 1 个不一致对即判"等价"。估计器按本 ADR 的换名规则升为 `paired-exact-binomial-v2`，决策一的等价条款按下文修订；其余结论不变。
> 日期：2026-09-09
> 补充 [ADR 0001](0001-rust-tauri-react-sqlite.md)（SQLite/FTS5 目标与 JSON ledger 偏离）与缺口分析 A4-(d)。不重开 0001 的技术栈结论。

## 背景

根 Cargo workspace 自始保持 **0 个第三方 crate**，16 条 required gate 离线通过。这条不变量卡住了两件事：Effect Lab 需要「冻结版本的成熟统计实现」（PRD F-15、[效果实验室指南](../guides/llm-advisor-and-effect-lab.md)），ADR 0001 冻结的 SQLite/FTS5 引擎需要 `rusqlite` 之类的绑定。2026-09-09 之前 Effect Lab 的产品路径没有任何估计器，合法的 runs 文档也只能 `inconclusive` + `effect.estimator_unavailable`。

前端 `packages/ui` 此前也没有浏览器交互测试（缺口分析 §5 明确不引入 Playwright），只有 SSR 渲染测试。

## 决策

### 一：Rust workspace 继续零第三方依赖；统计估计器在仓内实现并冻结版本

Effect Lab 的估计器不引入统计库，而是在 `crates/ctxpect-effect` 内实现一个**算法可写在一页纸上、可独立复算**的冻结程序 `paired-exact-binomial-v2`（`PairedExactBinomial`；首版 v1 见修订说明）：

- 方向：不一致对（discordant pairs）上的精确二项检验（McNemar 精确 / 符号检验），双侧 `p < alpha` 且方向一致才给 `supported-beneficial` / `supported-harmful`。
- 等价（v2）：配对差异 `d = (2q − 1)·r`，其中 `r = m/n` 是不一致率、`q = b/m` 是不一致对中 treatment 独胜的份额。对 `r`（`m` of `n`）与 `q`（`b` of `m`）**各**做精确 Clopper–Pearson 区间（各自单侧 `alpha`）；`d` 对 `(q, r)` 双线性，所以在两区间的四个角上取到范围。该范围严格落在 `(−margin_pp, +margin_pp)` 内才给 `supported-equivalent-within-margin`；无不一致对时 `r` 的上界是 `1 − alpha^(1/n)`，与 v1 相同。v1 只对 `q` 取区间、把 `r` 当作点估计，因此 11 对中 1 个不一致对得到 `[−8.18, 9.09]` 并判等价；v2 对同一输入给出 `[−36.4, 36.4]`，inconclusive。方向判定优先于等价判定；双侧 p 值定义为 `2·min(单侧)` 截到 1。
- 其余一律 `inconclusive`（`effect.estimator_inconclusive`）并输出 `detail`（配对数、不一致数、p 值、区间、margin）。
- v2 仍不实现 `multiplicity != none` 的校正（如实报 inconclusive）；power 只作为合同预注册值，不重算。合同侧同时收紧：`alpha ∈ (0, 0.05]`、`frozen_at` 必须可解析（否则 `effect.contract_invalid`）；可用配对数必须恰等于 `n_planned`（多收是 optional stopping，`effect.n_mismatch`）；同一 `(task_id, arm)` 两条 run → `effect.run_duplicate`；run 时间与冻结时间换算到同一时间轴比较，不可解析 → `effect.run_time_invalid`；已持久化实验的合同整体冻结（`effect.contract_locked`）。

选择它而不是统计库的理由：F-15 要求的是**冻结、可审计**的判据，不是丰富的模型；精确二项 + Clopper–Pearson 只依赖对数空间的二项 pmf 与二分求根，纯 `f64`/`std` 即可，版本号写在名字里，改动即换名。

### 二：SQLite/FTS5 偏离继续保留，结束条件写明

JSON ledger（ADR 0001 记录的偏离）继续作为唯一实现。引入 `rusqlite`（或任何第三方 crate）的条件：(a) 出现零依赖无法满足的功能需求（当前的 journal / index 修复 / 同机文件锁都已用 `std` 满足）；(b) 有离线 vendoring 方案（`cargo vendor` 进仓或本机 registry 镜像）保证 16 条门禁不联网；(c) 以 supersede 本 ADR 的新 ADR 记录，并同步 AGENTS.md 的依赖声明。未满足前，任何 `Cargo.lock` 出现 registry 条目都视为门禁失败。

### 三：前端允许 Playwright 作为 dev 依赖，交互 E2E 是可选门禁

`packages/ui` 引入 `@playwright/test`（冻结版本 1.56.1，浏览器为 Playwright 自带的 Chromium headless shell），新增 `ui-e2e` 门禁：`pnpm test:e2e`。它需要联网安装浏览器与已构建的 `ctxpect` 二进制，因此**不进 16 条 required gate**，直到 lockfile 冻结与 CI 环境具备为止；在本机它是交付前必须实跑并记录的可选门禁。缺口分析 §5「不为 C6 引入 Playwright」由本决定替代。

### 四：DSH 真实 runner adapter 暂不执行

本机没有可运行的 dsh 可执行文件与 provider 凭据；真实执行可能触发付费模型调用。保留合成语料闭环（`native-conformance`），真实 runner 的首次运行另行授权。

## 后果

- Effect Lab 的产品路径现在能到达四个判定，但只在精确检验支持时；小样本的「无差异」仍是 inconclusive。
- Rust 零依赖不变量不变；`packages/ui` 的 devDependencies 多一项，`pnpm-lock.yaml` 相应变化。
- `ui-e2e` 未通过或未运行时不得报交付完成，但它不是 `check_docs.py` 校验的 required 表的一部分。
