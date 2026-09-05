# 语义对齐与 Team Context Standard

> 状态：规范（尚未实施产品运行时）
> 本文冻结 CanonicalIntent、harness-native projection、TeamContextStandard 与分层执行的机器可读合同。产品运行时尚未实现。
> PRD 给出产品要求；长 JSON/字段表以本文为准，避免并行 Intent/Policy/Receipt 体系。

Acceptance cutoff：`2026-09-04T23:59:59+08:00`。

机器可读合同与夹具由 `python3 scripts/generate_acceptance.py` 生成，并由 `python3 scripts/check_semantic_team.py` 校验。夹具 `live_tested: false`，许可 Apache-2.0。

## 非目标

- 禁止把同一份 MD/config **字节复制**到每个 harness 并称为对齐。
- 禁止把 text/hash 相等当作语义等价。相等只证明文件相同。
- 禁止第二套 Intent/Policy/Receipt 对象。`Intent` **就是** `CanonicalIntent`。
- 禁止第二套 `ProjectionAuthority`。每个 target coordinate 仍只有一个 authority。
- 禁止编造 native oracle。可重复 oracle 仍只有 Codex `debug prompt-input` 与 Grok `inspect --json`。
- 禁止新增 F-19 或 WP-13。本规范扩展 F-09/WP-06、F-10/WP-07、F-18/WP-11。

## ST1 CanonicalIntent

`CanonicalIntent` 捕获稳定语义，而不是文件副本。必填字段：

| 字段 | 含义 |
| --- | --- |
| `id` | 稳定标识，`intent.*` |
| `meaning` | 非空 `goal` / `constraints` / `non_goals` |
| `scope` | `{kind,target,paths}` |
| `precedence` | `{rank,policy,shadows_lower}` |
| `activation` | `{trigger,mode}` |
| `required_capability` | 能力 taxonomy 项 |
| `dependencies` | 依赖 intent/capability |
| `permission_security_boundary` | 指令 ≠ sandbox/deny policy |
| `lifecycle` | 生命周期 |
| `owner` | 所有者 |
| `desired_outcome` | 期望的 native 效果 |
| `authority` | `{id,kind}` |
| `provenance` | `{source,recorded_at,digest_sha256}` |

跨 harness 对齐必须走 native projection。不得要求 Codex `AGENTS.md`、Claude `CLAUDE.md`、Cursor `.cursor/rules` 与 Grok 项目指令字节相同。

## ST2 流水线与 native projection

确定性顺序，不得调换：

1. canonical intent
2. target coordinate（family × version × surface × OS lane）
3. capability negotiation
4. authority selection（现有唯一 `ProjectionAuthority`）
5. harness-native projection
6. preview / apply / rollback
7. target resolver / oracle
8. reconciliation Receipt

`FAMILIES` 中每一个 family 都必须使用该 family 的 native path、syntax、scope、precedence、lifecycle 与受支持 primitive。本冻结的静态路径见 `FAMILY_INPUT_SPECS` 与 `acceptance/projection-matrix.yaml`。

四锚点项目指令示例（不是唯一资产，但是 ST2 夹具使用的 native 目标）：

| Family | Native path | Syntax |
| --- | --- | --- |
| Codex | `AGENTS.md` nested chain | markdown-nested-agents-chain |
| Claude Code | `CLAUDE.md` + `.claude/rules/**` | markdown-plus-path-scoped-rules |
| Cursor | `.cursor/rules/**` | cursor-mdc-project-rules |
| Grok Build | `AGENTS.md` | grok-project-instructions |

Codex 与 Grok 可以共享 path glob，但 native syntax/正文不得相同，也不得因此被判等价。OpenCode、Kimi Code、ZCode、DeepSeek Harness、Coze 及其他已声明 family 同样走各自 native primitive；无独立 primitive 时必须 loss/Unknown，不得静默删除。

投影结果枚举与核对状态正交，**不得**把 `verified` 当作 projection outcome：

`exact | native-equivalent | transformed | lossless-native-overlay | lossy | unsupported | unknown`

- `exact`：native 产物表达所需语义，无映射变化。
- `native-equivalent`：不同 path/syntax/primitive，所需语义保留。text/hash 不相等是预期，不是 drift。
- `transformed`：已文档化的 native 映射保留所需语义。
- `lossy`：省略、削弱或近似。必须进入 loss report。无有效 covering exception 时不得 `verified`。
- `unsupported`：目标无法表达所需语义。不得静默丢弃，不得 `verified`。
- `unknown`：能力、native 结果或 oracle 证据不足。不得静默丢弃，不得 `verified`。

核对状态仍仅为：`verified | structural-only | indeterminate | failed`。

没有 captured native result 时，Receipt 不得从 hash 相等推出 `verified`。Claude/Cursor 等无声明可重复 oracle 的 family 使用 static resolver + loss/Unknown 诚实性。

## ST3 Overlay、loss、drift、round-trip

Harness-specific native overlay 必须显式 `owner`、`reason`、`scope` 和 **独立 digest**。overlay 不得在没有有效 exception 时削弱 required team rule。

Loss 类别（封闭）：`omitted | weakened | approximated | duplicated | harness-only`。

Drift 类别（封闭）：`benign-native-representation | unapproved-semantic-drift | approved-exception | unknown-evidence`。

Reconciliation Receipt 必须绑定下列 digest：intent、plan、authority、transaction、native artifact、resolver/oracle、policy、exception。

## ST4 TeamContextStandard

可移植签名 bundle。Git/file 分发必须 local-first。加密云 registry/sync 只是 **可选传输**，不是语义依赖。bundle **不得**含 secret 值。

必填字段：`stable_id`、`semantic_version`、`revision`、`publisher`、`compatibility_floors`、`target_harness_coordinates`、`canonical_intent_set`、`policy_rules`、`release_notes`、`migration`、`rollback`、`content_digest_manifest`、`signatures`、`expiry`、`channel`。

生命周期命令与状态：`validate`、`publish`、`preview`、`adopt`/`pin`、`update`、`status`、`leave`、`rollback`、`revoke`。

`channel` 封闭：`git-file-local | file-local | encrypted-cloud-optional-transport`。

## ST5 分层与诚实执行

权威从高到低：`organization > team > project > role > personal`。

执行模式：`recommended | required | prohibited`。

执行诚实性：`enforceable | detect-only | non-enforceable`。无托管可执行通道时必须报告 `detect-only` / `non-enforceable`，不得声称 compliance。

Tighten-only：低层可以增加限制，不得静默放宽高层 required/prohibited。Personal 只能扩展或覆盖 **recommended**。

成员有效状态必须 **按 harness** 经同一 native projection + reconciliation 流水线计算。

## ST6 受控例外

状态：`request | approval | rejection | revocation | expiry`。

字段：requester、approver、reason、精确 intent/policy ID、member/device/project/harness scope、use/time limits、timestamps、signature、audit chain。

离线 stale/expired exception 对 required policy **fail-closed**，同时仍允许安全 inspection/export。Rollback 必须保留无关个人文件。

## ST7 隐私与 drift

Leader 视图 **可以** 默认获得：standard version、compatibility、compliance state、semantic drift class、loss/Unknown、exception metadata、device freshness、redacted evidence references。

Leader 视图 **默认不得** 获得：private prompt text、secrets、unrelated personal context、full session history。

任何 report/upload 之前必须有成员可见 preview 与 disclosure。

必须覆盖：staged rollout、deterministic bundles、multi-device conflicts、last-known-good、rollback、key/signature rotation。

## 封闭 schema（生成器与门禁用同一份）

Projection outcomes、loss/drift/enforcement 枚举与 `CanonicalIntent` / `TeamContextStandard` 字段集由 `scripts/contexpect_semantic_team.py` 冻结，并写入 `acceptance/semantic-team-contract.yaml`。

```text
CanonicalIntent:
  id, meaning{goal,constraints,non_goals},
  scope{kind,target,paths}, precedence{rank,policy,shadows_lower},
  activation{trigger,mode}, required_capability, dependencies,
  permission_security_boundary, lifecycle, owner, desired_outcome,
  authority{id,kind}, provenance{source,recorded_at,digest_sha256}

TeamContextStandard:
  stable_id, semantic_version, revision,
  publisher{id,display_name,signing_identity,role,authorization,key_id,trust_state},
  members[{id,role,key_id,authorization,trust_state}],
  trust{publisher_key_id,state,verified_key_id},
  lineage{current_floor,previous_revision,previous_digest,history},
  compatibility_floors, target_harness_coordinates,
  canonical_intent_set, policy_rules, release_notes,
  migration, rollback{safe,preserves_unrelated_personal_files,last_known_good_revision,transaction},
  content_digest_manifest, signatures, expiry, channel,
  transport{kind,local_first,encrypted_cloud_is_optional},
  lifecycle_metadata{state,published_at,expires_at}
```

合成签名使用离线 HMAC fixture key（`hmac-sha256-synthetic`），不是用户 token，也不是 live signer。

## CLI / UI / 数据

命令与路由扩展现有树，见 [cli-reference](../guides/cli-reference.md) 与 [desktop-ui](../guides/desktop-ui.md)。Exit 仍为 `0/2/3/1`；命令特定错误放在 JSON envelope 的 `error.code`，不发明第二套 exit 宗教。

实体与 SQLite 表扩展 PRD §10，不替换。本阶段不执行 migration。

验收：`acceptance/semantic-team-contract.yaml` + `acceptance/semantic-team/` + `scripts/check_semantic_team.py`。文档-only 声明不算对齐。

本阶段六条 required gate（名称 + 命令）：

| 名称 | 命令 |
| --- | --- |
| docs-structure | `python3 scripts/check_docs.py` |
| acceptance-validation | `python3 scripts/check_acceptance.py --structure` |
| traceability-validation | `python3 scripts/check_acceptance.py --traceability` |
| corpus-validation | `python3 scripts/check_acceptance.py --corpus` |
| semantic-team-validation | `python3 scripts/check_semantic_team.py` |
| validator-negative-tests | `TMPDIR=/tmp python3 -m unittest discover -s tests/acceptance -p 'test_*.py'` |
