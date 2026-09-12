# CLI 参考

> 状态：规范（`ctxpect inspect` 开发切片以及 doctor/collect/receipt/diff/daemon 等命令已可构建；`--config`/`--privacy`/`--sarif` 等仍 fail-closed。完整产品运行时尚未实施。）
> 二进制 `ctxpect` 的 inspect 仍输出 `schema=dev-inspect-v0` / `receipt_kind=development-snapshot`。正式 Receipt 需 `migrate_dev_inspect_v0`。

机器字段默认英文。人类输出可本地化。JSON envelope 稳定、可版本化。

## 全局

```text
ctxpect [--json] [--offline] [--config <path>] [--project <dir>] [--cwd <dir>]
        [--harness <family>] [--surface <id>] [--version <ver>]
        [--privacy default|strict|metadata-only]
```

`--offline` 下核心 inspect/diff/doctor/receipt/history 必须可用。未知版本不能通过 `--force` 变成权威 pass。

## 命令树

| 命令 | 工作包 | 作用 |
| --- | --- | --- |
| `ctxpect inspect` | WP-02 | one-shot 探测 + development snapshot；`--store` 时显式迁移为正式 Receipt（`one-shot` kind）。持久化失败**不再被吞**：JSON 带 `persist_error {code, message}`、`formal_receipt_id: null`，exit 1（IO 错误类；快照本身仍输出） |
| `ctxpect collect` | WP-02 | 在目标环境采集清单；`--store <dir>` 时把清单作为 **`device-baseline` kind Receipt** 持久化（清单条目即其 evidence，`inventory_digest` 即 `snapshot_digest`），输出 `receipt_id` / `receipt_persisted`；无 `--store` 时 `receipt_id: null` |
| `ctxpect doctor` | WP-02 | 确定性 Doctor；`--sarif`（fail-closed）/ `--fail-on confirmed` / `--as-of YYYY-MM-DD`。`--fail-on` 只接受 `confirmed`，其它值 → `usage.invalid`（exit 1）而不是被读成"无阈值"。`--as-of` 显式指定 `stale` 规则与 suppression 过期判定对照的评估日期（默认系统当日；指定后 suppression 以该日 UTC 正午为评估时刻，输出带 `as_of` 与 `suppressions.evaluated_at_secs`）；不存在的日期（如 `2026-02-30`）→ `usage.invalid`（exit 1），不做静默截断。最终 exit 合并 inspect 出码（无 `AGENTS.md` 的项目 inspect 本身 exit 2），见 [doctor-rule-map](../process/doctor-rule-map.md)「阻断判定」 |
| `ctxpect diff` | WP-03 | 跨 harness/device/version/snapshot |
| `ctxpect receipt show\|verify\|export\|redact` | WP-03 | Receipt 生命周期。`verify` 的 MAC 覆盖 `manifest.digest + created_at + signed_at`（见「Receipt 签名覆盖什么」）；无 `signed_at` 的旧 Receipt → `ok: false` + `receipt.signature_legacy`，exit 3，不因兼容放行；`signed_at` 存在但不是字符串 → `receipt.signature_mismatch`（被改过的信封，不是旧格式）；tombstone → `ok: false` + `receipt.tombstoned`，exit 3。CLI 与 `POST /api/v1/receipts/:id/verify` 调用同一个 `verify_receipt_report`（R04） |
| `ctxpect inventory` | WP-02 | **本切片为 `collect` 的别名**（同一函数、同一输出，`command` 字段回显 `inventory`）；独立的「不含动态伪事件的资产目录」尚未实施 |
| `ctxpect preflight` | WP-02 | task probe；生成可审计 argv 数组 |
| `ctxpect launch` | WP-02 | 用 structured process API 启动 harness |
| `ctxpect import` | WP-05 | 原生诊断/日志。`--mapping deepseek-harness-cli` 走原生 DSH JSONL 解析（见「DSH 原生会话导入合同」）；其余 mapping 走通用 `events[]`。缺省 `session_id` 为文件内容摘要，同一文件导入两次得到同一记录；同一 id 下已有**不同**记录 → `store.session_exists`（先删再导或换 id，不静默替换） |
| `ctxpect sessions` | WP-05 | timeline；未暴露类型只报 capability unknown。`--session <id>` 返回含 `requests[]` 的记录；`--reason delete` 级联删除 sessions / insights，之后 `GET /api/v1/sessions/:id/requests` 也不存在；删除不存在的 id → `store.missing`（exit 1），不报 `deleted` |
| `ctxpect daemon start\|stop\|status` | WP-05 | 可选。`status` 以健康探测判断运行（`probe: reachable\|unreachable\|no_addr`）；`stop` 不发送信号，只移除 pid 文件并报 `pid_file_removed_only`（daemon 仍可达时 exit 3） |
| `ctxpect sync preview\|apply\|status` | WP-07 | transport + reconciliation 分列。`status` 返回 store 的同步状态文档（与 `GET /api/v1/sync` 同源），`preview` 打包并（有 `--dest` 时）预览应用，`apply` 应用；三者是三个操作 |
| `ctxpect intent validate\|show\|project\|preview` | WP-06 | 四个操作：`validate` 只校验结构不碰文件系统（非法则 exit 2 + `reason_code`）；`show` 渲染 CanonicalIntent；`project` 计算预览不持久化；`preview` 计算并**持久化**预览到 store `previews/<tx_id>`，输出 `apply_with` |
| `ctxpect apply --tx <id>` / `ctxpect rollback --id <tx>` / `ctxpect apply status` | WP-06 | 唯一 authority。`apply` **只接受** `intent preview` 持久化的 `--tx`，不重算预览；目标自预览后变化 → `projection.concurrent_hash`。预览绑定 **project**（记录含 `project_digest`，`tx_id` 由 project digest + intent + target 派生）：在另一项目 apply → `projection.preview_scope`；预览有消费状态 `state`（`previewed` → `applied` → `rolled-back`），已 applied 或已 rolled-back 的 tx 再 apply → `projection.tx_consumed`，rollback 后要重新 preview。`rollback` 先比对 `after_digest`，被改 → `projection.rollback_conflict`；apply 新建的文件回滚为删除；事务记录 `tx.json` 先于目标落盘（pending → committed → rolled-back）。`apply status` 只读列出仍为 `pending` 的事务并按目标当前 digest 判定 `committed-recovered` / `aborted` / `in-doubt`，无法解析的 `tx.json` 列为 `unreadable` 并同样计入 in-doubt，**不改写目标**（有 in-doubt 时 exit 3）。`tx_id` 每次 preview **唯一**（派生自 project、intent、target、两个 digest 与进程内 nonce）：再次 preview 得到新 id，已提交事务的 backup 不会被后来的 preview/apply 覆盖，按顺序 rollback 能回到最初字节；`rollback --id` 只接受 `tx_<16 hex>`，否则 `store.bad_id`，不拼路径；`tx.json` 记录 `project_digest`，另一项目的记录 → `projection.rollback_scope`。目标是 symlink（含悬空）、FIFO/socket、目录 → `projection.not_a_file`（指向项目内常规文件的 symlink 会被解析到真实文件）；`.git/`、`.ctxpect/` 或 store 内任何路径 → `projection.control_path`（apply 授权不得自写策略、例外与 git hook）。所有写入经**独占创建**的临时文件（pid + 计数命名）+ fsync + rename，预置的同名链接被拒绝而不是被跟随；事务目录及其父目录若是 symlink 一并拒绝 |
| `ctxpect standard validate\|publish\|preview\|adopt\|pin\|update\|status\|leave\|rollback\|revoke` | WP-07 | TeamContextStandard；git/file local-first |
| `ctxpect exception request\|approve\|reject\|revoke\|status` | WP-11 | 受控例外；过期 fail-closed。`request` 必填 `--action <mutation>` 与 `--expires-in <秒>`，可带 `--target`（默认 `*`）与 `--reason`；记录绑定 action / project digest / target（见「授权绑定」）。`--role`/`--actor` 仍是调用方自报、不构成授权；身份来自仓内已登记 principals（见「例外批准的身份源」） |
| `ctxpect assets [status\|list\|preview\|copy\|rollback\|sbom]` | WP-08 | catalog、资产复制 executor、lock 与 SBOM；无子命令时为概览。复制前先核验登记、许可证与内容摘要；被复制的字节与将进入 backup 的现有目标内容都过同一份 secret 门（`assets.contains_secrets`，在建 backup 之前拒绝）；`rollback --id` 只接受 `tx_<16 hex>` |
| `ctxpect advisor` | WP-09 | 显式同意、payload preview。`--adapter` 只接受 `none`（本地启发式）；其它值 → `advisor.adapter_unavailable`，不回显不存在的分析路径 |
| `ctxpect experiment [--runs <file>] [--id <exp>] [--store <dir>]` | WP-10 | Effect Lab。**无 `--runs` 即未执行**：`executed: false`、`reason_code: effect.runs_required`、`decision: null`，exit 3，不写 store。`--runs` 为 `ctxpect-effect-runs-v1` 文档（冻结合同 + 逐 run 结果，见「Effect Lab 的 runs 文档」）；协议偏离一律 `inconclusive` + 具体原因；估计经冻结的 `paired-exact-binomial-v2`（对不一致率与不一致对中的方向份额各取精确区间，[ADR 0006](../adr/0006-third-party-dependency-policy-and-estimator.md) 修订），小样本或区间超出 margin → `inconclusive` + `effect.estimator_inconclusive`；可用配对数少于 `n_planned` → `effect.n_insufficient`，多于 → `effect.n_mismatch`（固定样本合同不接受追加观测）；同一 `(task_id, arm)` 出现两条 run → `effect.run_duplicate`；时间不可解析或 `ended_at` 早于 `started_at` → `effect.run_time_invalid`（store 时钟读数与 RFC 3339 换算到同一时间轴比较）；`alpha` 须在 `(0, 0.05]`、`frozen_at` 须可解析，否则 `effect.contract_invalid`；已持久化实验的合同**整体**冻结，任一字段改变 → `effect.contract_locked`。`supported-*` → exit 0，其余 exit 3；结果只在 `experiment.persist` 授权下写入，n 锁定（`effect.n_locked`） |
| `ctxpect policy eval\|show` | WP-11 | 分层 context policy；detect-only 诚实性 |
| `ctxpect align status\|diff` | WP-06, WP-11 | `status` 列出可对齐的 Receipt；`diff` 要求 `--a/--b`（缺失 → `usage.invalid`），输出 `command: "align diff"`。不得把 byte-equality 当 pass |
| `ctxpect adapter test --adapter <family> [--from <repo-root>]\|list` | WP-11 | `list` 读静态 catalog；`test` **真实运行**该 family 的 development 静态语料行（与 `corpus-conformance` 门禁共用 `ctxpect_cli::conformance` runner），输出 `ran: true`、`executed_rows`、`implemented_rows`、五类计数（`implemented_pass` / `unknown_honesty_pass` / `fail` / `unimplemented` / `total`）、`corpus_digest`（所跑 jsonl 的 sha256 汇总）、`matrix_digest`、`resolver_version`、`live_oracle_executed: false`；`implemented_rows`（= implemented-pass + fail，honesty 通过**不计入**）为 0 → `decision: unimplemented`（`adapter.family_unimplemented`，exit 3），有失败 → `fail`（`adapter.rows_failed`，exit 2）；`--from` 下没有该 family 的语料 → `adapter.corpus_missing`（exit 1）；缺 `--adapter` 或未知 family → `usage.invalid`。`corpus_digest` 是对**所跑文件**现算的摘要，与 `corpus-manifest.json` 冻结值的一致性由 `corpus-validation` 门禁保证，`adapter test` 本身不核对清单。语料根默认当前目录 |
| `ctxpect ci` | WP-11 | 稳定 exit code 的 CI 入口：inspect 出码、Doctor 阻断判定（与 `doctor --fail-on` 共用 `blocking_exit`）、store 有效 policy（无 policy → 3 `policy.unknown`；required deny → 2）三者合并（任一 3 → 3，否则任一 2 → 2）。`ci` 不创建 store；有 store 时把所依据的观测持久化为 **`ci` kind Receipt** 并输出 `receipt_id` |
| `ctxpect store status\|repair` | WP-03 | `status` 只读报告 index 健康（缺失 → `store.index_missing`，损坏 → `store.index_corrupt`）、未完成 journal 记录（含文件名与解析错误）、审计日志健康（`ok` / `torn` / `unparseable` / `head-stale` / `anchor-invalid`）、同机 advisory lock 与 pending 事务判定；任一不健康即 exit 3。`repair` 先处置审计尾行（撕裂尾行移入 `audit/torn-<时刻>.bin`、锚点落后则重锚），再从 `receipts/` + `tombstones/` 确定性重建 `index.json`（与正常路径逐字节相同）并重放 journal；修不掉的仍 in-doubt 时 exit 3，而不是报 0。见「store 的可恢复性与同机锁」 |

## Exit codes

| Code | 含义 |
| --- | --- |
| 0 | policy pass：所有 required checks 确定通过 |
| 2 | 确定性 drift/finding failure（含可阻断 Doctor 规则） |
| 3 | required indeterminate / 未知版本 / 过期 policy / 无法刷新 |
| 1 | 用法/IO/内部错误 |

Unknown 不得自动映射为 0。组织可以把 indeterminate 降为非阻断，但必须写入 Receipt，且不改变 claim。

命令特定错误写入 JSON envelope 的 `error.code`（例如 `standard.unsigned`、`exception.expired`、`principal.secret_mismatch`、`exception.approver_role_required`、`store.busy`、`store.index_corrupt`、`projection.preview_scope`、`projection.tx_consumed`、`projection.control_path`、`projection.not_a_file`、`store.audit_truncated`、`effect.contract_locked`），不新增第五种 exit code。`align` 把 byte/hash 相等当作语义通过时必须非 0。`inspect --store` 的持久化失败属 IO 错误类（exit 1），但快照仍完整输出并带 `persist_error`；`store status` / `apply status` 的 in-doubt 与 `experiment` 的未执行 / inconclusive 属 indeterminate（exit 3）。

`policy show`（无 `--from`）读取与 `apply` 相同的 store policy / exception 集合，并对一个 **scope**（`--action`，默认 `apply`；`--target`，默认 `AGENTS.md`）调用同一 `authorize_mutation` 判定，输出带 `scope` 与 `policy_source_fresh`。无 layers 时 `verdict=unknown`、`reason_code=policy.unknown`，不是 pass。`policy eval --from <file>` 只评价该文件，不替代 store 判定。

## 授权绑定：action / project / target

每次 mutation 都经唯一 authority `authorize_store_apply(action, target)`。它把当前 **action**（下表）、**project digest**（项目规范路径的 sha256；无 `--project` 时为 store 根的 sha256）与 **target** 交给 `authorize_mutation`，只有同时匹配的**已批准、未过期、policy 来源新鲜**的例外才放行。为项目 A 的 `apply` 批准的例外不能授权项目 B，也不能授权 A 的 `sessions.import`；例外的 `target` 可以是具体目标或 `*`，action 与 project 没有通配。`*` 也不覆盖**控制路径**：`.git/`、`.ctxpect/` 与 store 所在的任何路径都不能是 projection 目标（`projection.control_path`），否则一条 `apply` 例外就能自写 rollback 例外或 git hook。缺少 scope 字段的旧记录不覆盖任何 mutation（`exception.scope_missing`）。

| action | 触发命令 / 端点 | target |
| --- | --- | --- |
| `apply` / `rollback` | `apply --tx` / `rollback --id`、`POST /api/v1/apply|rollback` | 项目相对路径 |
| `assets.copy` / `assets.rollback` | `assets copy|rollback`、`POST /api/v1/assets/:id/copy|rollback` | asset id / tx id |
| `sessions.import` / `sessions.delete` | `import`、`sessions --reason delete`、`POST /api/v1/sessions/import` | session id |
| `standard.publish|validate|adopt|pin|update|leave|rollback|revoke` | `standard <sub>` | standard id |
| `sync.apply` | `sync apply`、`POST /api/v1/sync/apply` | bundle id |
| `receipt.redact` / `receipt.delete` | `receipt redact`、`POST /api/v1/receipts/:id/delete` | receipt id |
| `preflight.intent` | `preflight` | preflight id |
| `advisor.persist` / `experiment.persist` | `advisor --store`、`experiment --store`、`POST /api/v1/lab` | candidate / experiment id |
| `settings.put` | `PUT|POST /api/v1/settings` | `settings` |

**新鲜度**：`offline_fresh` 不再写死为 `true`，而是由 policy 来源文件（store `policies/active.json` 等，或项目 `.ctxpect/policy.json`）的修改时间计算：超过 `POLICY_FRESHNESS_SECS`（30 天）即 `exception.offline_stale`，例外不放行。`exception status` 输出 `state`、`grants`、`reason_code`（`exception.expired` / `exception.offline_stale` / …）、`action`、`target`、`expires_at`。

**mutation 的 secret 门**：`intent preview` / `apply` 对 desired 内容与将进入 backup 的现有内容跑同一份凭据形状识别（`ctxpect_doctor::secret_literal`），命中即 `projection.contains_secrets`，不写预览、不建 backup。`standard publish` 用同一识别（`standard.contains_secrets`）。

**导入是 metadata-only**：`import` 只保存每条事件的类型、长度与摘要，不保存正文预览（`bodies_stored: false`）；未知事件类型不回显；`--mapping` 必须是 `acceptance/field-to-claim/*.yaml` 之一（默认 `generic-json`），否则 `import.mapping_unknown`。

**连续性密钥的随机源**：store 首次生成 `keys/continuity.key` 时只从平台安全随机源读取；不可用时 `store.random_unavailable`，不退化为时间哈希。

## store 的可恢复性与同机锁

store 的三写（Receipt 文件、`index.json`、审计链）不是原子的，因此每次 `put_receipt` / `delete_receipt` 先在 `store/journal/<op>-<id>.json` 落一条 write-ahead 记录（操作类型、receipt_id、阶段），成功后删除。`Store::open` **只在拿到同机锁时**重放未完成的 journal（另一存活进程持锁时，那些记录属于它正在进行的写入，不是崩溃遗留，原样保留并由 `status` 报告）：Receipt 文件已存在的补齐 index 行与审计条目（幂等，不产生重复行 / 重复条目）；文件不存在的记为 rolled-back 并删除记录；无法判定的（典型：index 损坏）保留为 **in-doubt**，由 `store status` 读出、由 `store repair` 解决。`index.json` 只为空 store 铸造：已有 Receipt/tombstone 而 index 缺失 → `store.index_missing`，从不读成空 ledger。`index.json` 读不出或不是数组 → `store.index_corrupt`，从不被当作空 ledger；`repair` 从 `receipts/` + `tombstones/` 重建（tombstone 为此保留 `created_at` 与 `coordinate`），index 按 `(created_at, receipt_id)` 规范排序，因此重建结果与正常路径逐字节相同。已存在的 Receipt 再次 `put` 仍 first-write-wins，但会补齐缺失的 index / 审计。

**同机 advisory lock**：store 的每个写入方法（`put_receipt` / `delete_receipt` / `put_named` / `delete_named` / `put_settings` / `put_snapshot` / `audit` / `repair`）自己取锁（进程内可重入），`authorize_store_apply` 与 `persist_inspect` 在外层持锁到 mutation 结束；`apply` / `rollback` 在读取预览状态**之前**取锁，不给另一进程在"读到 previewed"与"授权"之间消费同一事务的窗口。锁是对 `store/lock` 取 OS 文件锁（std `File::try_lock`，`flock` 语义），持有到该次 mutation 结束；持有者退出或崩溃时内核自动释放，因此不存在需要接管的 stale 锁，文件内容（pid、时间）只是信息。另一存活进程持锁 → `store.busy`；`store status` 的 `lock.holder` 报 `this-process` / `other-process` / `none`（以实际探锁结果为准，不以文件内容为准）。同一进程内可重入。这是**同机 advisory lock**：不是强制锁，不跨主机或网络文件系统；CLI 与 daemon 在同一 store 上的并发 apply 只有一个能成功，其余得到 `store.busy` / `projection.tx_consumed` / `projection.concurrent_hash` 之一。

**审计链锚点**：`audit/head.json` 记录最后一条链接条目的 `seq` 与 `mac`，并用 continuity key 对二者做 MAC。`audit_chain()` 先核链接，再核锚点：尾部条目被删除（链接前缀本身仍完整）→ `audit.truncated`；锚点缺失 → `audit.head_missing`；锚点落后（append 与锚点写入之间崩溃）→ `audit.head_stale`（`repair` 重锚）；锚点 MAC 不符 → `audit.head_mismatch`。日志被截断或锚点无效时 **store 拒绝再写**（`store.audit_truncated` / `store.audit_anchor`），包括 `repair` 自身的审计条目，因此截断不会被下一次写入掩盖。撕裂的尾行（崩溃中的 append）→ 写入被拒（`store.audit_torn`），`store status` 报 `torn`，`repair` 把尾行移入 `audit/torn-<时刻>.bin` 后截回最后一整行。边界不变：持有 store 密钥与完整本地访问权者仍可整体重写（删除 `head.json` 后 `repair` 会按当前链尾重锚，这在同一边界内）。`keys/continuity.key` 以 0600 独占创建；再次读取时若对组/其他用户可读 → `store.key_permissions`，拒绝使用可能已泄露的密钥。

**pending 事务**：`apply status`（或 `store status --project`）列出 `store/apply/*/tx.json` 中仍为 `pending` 的事务，按目标当前 digest 判定 `committed-recovered`（目标已是 apply 后的字节）、`aborted`（仍是 apply 前的字节 / 新建文件不存在）或 `in-doubt`（两者皆非）；判定**只读**，不改写目标也不改写记录，`rollback --id` 才关闭记录。

## Receipt 签名覆盖什么

`docs/schemas/ctxpect-receipt-v1.schema.json` 的 `x-display-only` 声明哪些字段是 display-only：`created_at`、`signature/signed_at`（tombstone 复制原 Receipt 的签名、永远不能作为已签 Receipt 验签，因此 `tombstone/deleted_at` 既不进摘要也不进 MAC，不在清单里）。`manifest.digest` 按**这份路径清单**剥离（不再按 `time` / `now` 这类键名递归剥离，名为 `time` 的业务字段留在摘要里），因此同内容不同时钟读数的两份 Receipt 共享 `manifest.digest`。本地连续性 MAC 覆盖 `manifest.digest + created_at + signed_at`，篡改任一时间即 `receipt.signature_mismatch`。`signed_at` 与 `created_at` 都是**本机时钟读数，不是可信时间戳**；本地 MAC 只证明同一 store 连续性，不是组织签名（`org_identity: false`）。签名规则变更前写入的 Receipt 没有 `signed_at`，`verify` 返回 `receipt.signature_legacy`（exit 3），既不当有效也不当被篡改。`stable_digest` / redaction 用的 `strip_time_fields` 保留其用途（快照稳定摘要、脱敏），与 Receipt 身份无关。

## Effect Lab 的 runs 文档

`ctxpect experiment --runs <file>` 与 `POST /api/v1/lab {"runs": {...}}` 接受 `ctxpect-effect-runs-v1`：

```json
{"schema":"ctxpect-effect-runs-v1",
 "contract":{"schema":"experiment-contract-v1","experiment_id":"e1","primary_outcome":"task-pass","margin_pp":10,
   "pairing":"paired-by-task","n_planned":4,"alpha":"0.05","power":"0.80","multiplicity":"none","itt":"count-as-fail",
   "locked":{"code_digest":"…","model":"…","harness":"…","tool_availability_digest":"…"},
   "frozen_at":"<store 时钟或 RFC 3339>","invalidation":["harness update"]},
 "runs":[{"run_id":"c-0","arm":"control","task_id":"t0","outcome":"fail","code_digest":"…","model":"…","harness":"…",
   "tool_availability_digest":"…","started_at":"…","ended_at":"…","receipt_id":null}]}
```

合同字段即 PRD F-15（唯一 primary outcome、equivalence margin、配对、样本量、alpha/power、multiplicity、ITT 规则、混杂锁定、失效条件），缺一即 `effect.contract_invalid`（exit 1）；run 缺字段或 `arm` / `outcome` 越界 → `effect.run_malformed`。协议检查各有原因码并全部导致 `inconclusive`：`effect.n_insufficient`、`effect.arm_unbalanced`、`effect.pair_incomplete`、`effect.confounder_drift`（code / model / harness / tool availability 与锁定值不一致）、`effect.run_precedes_freeze`、`effect.run_duplicate`；`timeout` / `crash` / `refusal` / `missing` 按 `itt` 计入（`count-as-fail` 或 `exclude-pair`）并列在 `deviations[]`。估计是 `Estimator` trait，产品实现为仓内冻结的 `paired-exact-binomial-v2`（ADR 0006：不一致对精确二项检验定方向；等价用不一致率 `m/n` 与方向份额 `b/m` 各自的精确 Clopper–Pearson 区间在四个角上取差异范围——v1 把 `m/n` 当已知值，11 对里 1 个不一致对就被读成"等价"，因此换名；方向判定优先于等价判定；不支持的判据 `effect.estimator_inconclusive` 并输出 `estimator.detail`）；`supported-*` 只在精确检验支持时出现。结果的 `runs[]` 只回填逐 run 摘要（id、arm、task、outcome、计入方式、时间、receipt_id），不含正文。

## DSH 原生会话导入合同

`--mapping deepseek-harness-cli` 只接受 `@deepseek-ai/dsh-session-persistence-jsonl` 在 pinned SHA（`acceptance/corpus/development/native/deepseek-harness-cli-0.1.2-rc.1/meta.json` 的 `dsh_sha`）下写出的**未压缩、未打包** JSONL：header 行（`type: "session"`，`version: 0`）+ 逐事件行。其它编码需要 Zstandard 解码器或 packed-row codec，零依赖政策下不读，也不部分读：zstd 帧 → `import.encoding_unsupported`（reason 写明 zstd），`text-chunks` / `reasoning-chunks` / `tool-call-chunks` 行 → `import.encoding_unsupported`（reason 写明 packed）；header `version` 不是 0 → `import.format_version_unsupported`。

**每个 step 一个请求**（DSH 自己的 request-reconstruction 定理，`packages/core/agent-loop/tests/request-reconstruction.spec.ts`）：请求锚点是 `step/start`（`requests[].seq`），消息序列是该边界处的派生 surface（`surface.ts` `foldSurface` + `deriveEventMessage`），header 是该 step **首个 provider 输出之前**日志前缀里最新的 `request/header` 快照（`request-header.ts` `canonicalHeader`；记录为 `header_seq` 与 `header_logged_in_step`），只保存 digest。DSH 只在 initial / resume、header 变化或 series 开始时才追加 `request/header`（`agent.ts:505-518`），因此真实日志里多数 step 没有自己的 header 却仍是一次请求——此前按「每个 `request/header` 一个请求」重建会把这些 step 误报为 `request_prefix_incomplete` 并少算请求（2026-09-09 交叉 review 发现，夹具已加入一个无 header 的 step 3 覆盖）。`request/header` 只证明**已准备**（`agent.ts:508-517` 在派发前追加它）；**派发证据**是该 step 内的 `assistant/chunk` 或 `assistant/message`（`agent.ts:364` 打开 provider 流、`:368` 逐 chunk 追加），记录为 `dispatch_evidence: <seq>`，否则为 `null`，对应 claim 的 `model-visible` 为 indeterminate（`runtime_snapshot_missing`）。前缀里根本没有 header 的 step 以 `header_digest: null` 列出、不发 claim，若它还有 provider 输出则记 `request_prefix_incomplete`。usage 只按 provider 报告展示（`assistant/message.usage`），累计 input 不是占用，`request/context.contextWindow` 是容量不是已用。

精确的 partial / Unknown：`seq_discontinuity`（记录 expected/found，重建在断口停止）、`import_parse_failed`（词表外且无 `ignorable` 的事件类型，只记 `type_len`，从该事件起拒绝重建，与 DSH `validateStoredEvents` 一致；`ignorable: true` 的跳过并记 `import_event_ignorable_skipped`）、`request_prefix_incomplete`（该 step 有模型输出却无 header）、`provenance_incomplete`（replacement 的 `sourceEventSeqs` 指向不存在的 seq 或未覆盖被遮蔽节点）、尾部 `TOOL_OUTCOME_UNKNOWN` / `TOOL_NOT_STARTED`（`repair.ts` `interruptedTurnClosers` 会合成的 closers 只列出、不追加）。持久化记录 metadata-only：`timeline[]` 只有类型 / seq / 长度 / digest / surface 放置，`requests[]` 只有 digest、计数、区间、证据 seq、`reason`（仅 DSH 词表 `initial|resume|change|series`，其它只记长度）与 provider 报告的 usage **整数**字段（非整数落 `null`），header 的 `createdAt` / `delegationDepth` 同样只保留整数，`bodies_stored: false`；输入侧：JSON 嵌套深度上限 128（超出 → `import_parse_failed`，不再栈溢出），`sourceEventSeqs` 的区间在展开前按当前 seq 设上界（与 DSH `decodeSeqRanges(value, record.seq)` 一致，`[[0, 2^63]]` 立即 `provenance_incomplete`），含浮点字面量的行按本解析器边界报 `import_parse_failed`（详见 `crates/ctxpect-schema/BOUNDARY.md`；DSH header 可携带 `temperature`，这是已知未覆盖项）；evidence 条目带 `evidence_id = sha256(input digest ":" seq)`、采集方式 `import` 与 importer 版本，其余 C19 字段为 `unknown`。该语料是合成的（`live_tested: false`），只核对 importer 的前缀重建，不扩大冻结 oracle。

## 例外批准的身份源：仓内已登记 principals

> 选型理由、被否决的替代方案与 mutation 门禁边界见 [ADR 0005](../adr/0005-exception-identity-and-mutation-boundary.md)。

`ctxpect exception approve|reject|revoke` 不把调用方自报的 `--role` / `--actor` 当作授权证据。身份的唯一来源是**项目内受版本控制的 principals 登记**，配合调用方实际持有的登记密钥。

### 登记文件

`<project>/.ctxpect/principals.json`，随项目进版本控制，因此「谁可以申请、谁可以批准」与代码在同一处评审：

```json
{
  "schema": "ctxpect-principals-v1",
  "principals": [
    {
      "principal_id": "alice",
      "roles": ["requester"],
      "key_id": "k-alice",
      "key_digest": "<enrollment digest>"
    },
    {
      "principal_id": "carol",
      "roles": ["approver"],
      "key_id": "k-carol",
      "key_digest": "<enrollment digest>",
      "self_approval": false
    }
  ]
}
```

`roles` 只接受 `requester` 与 `approver`；出现其它取值时整条登记被拒（`principal.role_unknown`），不静默忽略。

### 登记摘要与密钥

`key_digest = HMAC-SHA256(secret, "ctxpect-principal-enrollment-v1:" + principal_id)`。摘要绑定到 `principal_id`，因此一份登记不能被挪用给另一个 principal。

密钥本身**不入库、不进 argv**，只经环境变量 `CTXPECT_PRINCIPAL_SECRET` 传入：argv 在进程列表里可见，也会进入 redaction roots，两处都不该出现 secret。

**登记摘要随项目公开**，因此 `secret` 必须是高熵随机材料；低熵口令可以由公开摘要离线暴力破解。

### 身份与授权

```bash
CTXPECT_PRINCIPAL_SECRET=<secret> ctxpect exception request \
  --project <dir> --store <dir> --id ex-1 --principal alice \
  --action apply --expires-in 3600
```

- `request` 的授权来自 principals 登记，**不来自已批准例外**。否则创建第一条例外需要先有一条已批准例外，整条通道自锁。
- `approve` / `reject` / `revoke` 要求 `approver` 角色。
- 默认四眼：请求者不批准自己的请求，除非该 principal 在登记里显式写了 `"self_approval": true`。
- 状态机终态即终态：`requested → approved|rejected`、`approved → revoked`；已 `rejected`/`revoked` 的例外不能被复活（`exception.transition_invalid`）。
- 决策写入 `decided_by`，其中 `identity_kind = "enrolled-local-principal"`、`org_identity = false`。持有本地登记密钥不是组织身份，输出不会把它说成组织身份。

### daemon API 不提供此通道

`POST /api/v1/exceptions` 返回 `api.identity_required`。HTTP 请求不能安全携带登记密钥，因此 daemon 无法建立所需身份；它明确失败，而不是造一条 requester 为字面量 `user` 的例外。

## 资源限制哪些真的生效

`settings.resource_limits` 的两个字段处理方式不同，因为一个能执行、一个不能：

| 字段 | 状态 |
| --- | --- |
| `scan_files` | **生效，但只在 `collect --store <dir>` 时读取**：限额来自该 store 的 settings；不传 `--store` 时 `collect` 不读任何 settings，遍历无上限（`file_limit: null`）。达到限额时输出带 `complete: false` 与 `reason_code: scan.file_limit_reached`。被截断的清单是**未知的**清单而不是更小的清单，因此它的 `inventory_digest` 与完整扫描不同，不能被当作完整 manifest 读。 |
| `daemon_rss_mb` | **不执行**。进程无法可移植地限制自身常驻内存——那需要 cgroups 或 setrlimit，本切片都不用。`GET /api/v1/settings/schema` 的 `unenforced` 字段公布这一点，Settings 界面据此在该字段旁标注「本切片不执行此项」。 |

两者此前都只是被存储与校验、从不被读取：配置它们没有任何效果。把能做的做了，做不到的说清楚，比让两个字段都看起来像可用开关要好。

## 资产复制 executor

`ctxpect assets preview|copy|rollback` 是资产的唯一复制 executor。它**不是包管理器**：不解析依赖、不下载、不理解版本区间。它只做一件事——把一份**仓内登记已声明**的文件复制到项目内声明的落点，作为可回滚的事务，并记录落地了什么。

### 资产登记

`<project>/.ctxpect/assets.json`（`schema: ctxpect-assets-v1`），随项目进版本控制：

```json
{
  "schema": "ctxpect-assets-v1",
  "assets": [
    {
      "asset_id": "skill-a",
      "origin": "project:vendor/skill-a.md",
      "license": "MIT",
      "digest": "<sha256 of the source bytes>",
      "target_rel": ".ctxpect/skills/skill-a.md"
    }
  ]
}
```

### 供应链门禁

复制前必须通过全部核验，任何一条不过都在 `preview` 阶段拒绝，因此**不会留下半个文件**：

| 情况 | reason code |
| --- | --- |
| asset_id 不在登记里 | `assets.not_registered` |
| 没有声明许可证 | `assets.license_unknown` |
| 没有声明来源 | `assets.origin_unknown` |
| 来源不是 `project:<rel>` | `assets.origin_unsupported` |
| 来源或落点含 `..` / 绝对路径 | `assets.path_escapes` |
| 源文件实际内容与登记摘要不符 | `assets.digest_mismatch` |

最后一条是冒名的防线：名字与许可证都对得上，但字节不是登记的那份，就不是那个资产。

**L07 的覆盖状态**：executor 存在之后，「无许可证仍执行复制」成为可构造的真实反例，已有对应负例测试。另外两条仍不可构造，且不做模拟——「重写 APM evaluator」需要先有一个可被重写的评估器，而 APM 在此不被重新评估；「双重 authority」需要先存在第二个 copy executor。两者保持未覆盖。

### 事务与并发

`copy` 经唯一 authority `authorize_store_apply`，并在写入前重新核对两件事：源字节仍与核验时一致（否则 `assets.source_changed`），落点自 preview 以来未被他人改动（否则 `assets.concurrent_hash`，并发编辑得以保留）。复制成功后写入 `store` 的 lock 条目、审计记录，以及一份 **post-Receipt**——复制改变了项目，之后的状态要被观测而不是假定。

`rollback` 恢复先前字节；若这次复制是新建文件，则删除它。**回滚同样产生 post-Receipt**：撤销也改变了项目，若不观测，最后一条记录描述的会是撤销之前、也就是已经不成立的那个状态。`apply` 与 `rollback` 的 post-Receipt id 因此可以对照——被观测的文件恢复原状时，id 也回到原值，回滚的效果由此可核验而不只是被声称。

### lock 与 SBOM

`ctxpect assets sbom` 从 lock 生成物料清单，含每个组件的来源、许可证与 sha256，并统计**没有许可证的组件数**而不是把它们从列表里略去。

该文档显式声明 `spdx_or_cyclonedx: "not-emitted"`：输出 SPDX 或 CycloneDX 意味着遵循其规范，一份仅仅顶着那个名字的近似文档会误导对合规性的判断。`scope` 同样写明它只覆盖本 executor 复制的资产，**不是** Cargo 或 npm 的依赖图。

## `standard` 各子命令的实际语义

`preview` / `adopt` / `pin` / `update` / `status` 是五个不同的操作，不是同一次读取的五个别名。采纳状态写在 store 的 `adoptions/<standard_id>.json`（`schema: ctxpect-adoption-v1`）。

| 子命令 | 写状态 | 前置条件 | 说明 |
| --- | --- | --- | --- |
| `status` | 否 | 无 | 读 standard 与本项目采纳状态；缺失即报 `absent` |
| `preview` | 否 | standard 存在且验签通过 | 报告 `adopt` 将写入什么（`would_write`），并标 `writes_state: false` |
| `adopt` | 是 | 未采纳 | 建立采纳，记录 `source_digest`；重复采纳报 `standard.already_adopted` |
| `pin` | 是 | 已采纳 | 固定到当前 `manifest_digest`，`state` 变 `pinned` |
| `update` | 是 | 已采纳且 standard 已变化 | 跟进到当前版本，返回 `from_digest`/`to_digest`；无变化报 `standard.already_current` |
| `leave` / `rollback` / `revoke` | 是 | standard 存在 | 破坏性删除；不存在时报 `standard.absent`，不谎报删除成功 |

`adopt` / `pin` / `update` / `publish` / `validate` / `leave` / `rollback` / `revoke` 都经过唯一 authority `authorize_store_apply`，并写入审计链。

## JSON envelope（计划）

```text
{
  "schema_version": 1,
  "command": "inspect",
  "exit_code": 3,
  "receipt_id": "...",
  "unknown": [{"reason_code": "not_installed", "family": "cline"}],
  "findings": [],
  "warnings": []
}
```

时间字段不参与 canonical digest。

## 安全

- 启动命令以参数数组生成，禁止字符串拼接后交给 shell
- 复制为 shell 文本时按目标 shell 严格 quoting
- secret 只通过 secret manager / 环境引用在执行边界注入
- `ctxpect collect` over SSH 调用系统 SSH，校验 known_hosts，不保存密码

## 当前可运行的范围

`ctxpect` 二进制可构建并实现命令树里列出的命令（`--config`/`--privacy`/`--allow-unknown`/`--force`/`--sarif` 与 `launch --execute` 仍 fail-closed）。静态 resolver 只覆盖两个 anchor 的 `instructions`（Codex 0.147.0、Claude Code 2.1.259，见 `docs/adapters/grammar/`）；其余坐标与 capability 如实报 Unknown。逐项状态见 [交付状态](../process/2026-09-08-delivery-status.md)。`python3 scripts/check_*.py` 是文档/合同门禁，不是产品命令，不要把它们别名为 `ctxpect`。
