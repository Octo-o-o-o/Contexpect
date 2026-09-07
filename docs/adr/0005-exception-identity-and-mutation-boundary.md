# ADR 0005：例外授权的身份源与 mutation 边界

> 状态：已接受（已实施）
> 日期：2026-09-07

## 背景

`authorize_store_apply` 是 store mutation 的唯一 authority，它要求一条存活的已批准例外。但 `exception request` 此前也走同一道门：创建第一条例外需要先有一条已批准的例外，`approve` / `reject` / `revoke` 因此不可达，整个 V13/L06 通道自锁。

前一轮为了消除 `--role` 自证（调用方自报角色不是授权证据，R06），把批准路径整体 fail-closed 成 `exception.identity_source_uncovered`。这消除了伪授权，但没有给出真授权，通道保持不可用。

解开自锁必须先定义**初始授权来源**。

## 决策一：身份源是仓内已登记 principals

身份 = **项目内受版本控制的 principals 登记** + **调用方实际持有的登记密钥**。两者缺一不成立。

登记位于 `<project>/.ctxpect/principals.json`（`schema: ctxpect-principals-v1`），随项目进版本控制，因此「谁可以申请、谁可以批准」与代码在同一处评审。

`key_digest = HMAC-SHA256(secret, "ctxpect-principal-enrollment-v1:" + principal_id)`。摘要绑定 `principal_id`，一份登记不能被挪用给另一个 principal。密钥只经环境变量 `CTXPECT_PRINCIPAL_SECRET` 传入，不入库、不进 argv（argv 在进程列表可见，也会进入 redaction roots）。

### 被否决的替代方案

| 方案 | 否决理由 |
| --- | --- |
| 绑定 OS 用户 | 实现最小，但「OS 用户 = 身份」是弱断言：多人共用机器即失效，且不可跨设备、不可用于团队。 |
| 外部 IdP（OIDC/SAML） | 唯一真正的团队级身份根，但引入第三方依赖与网络，直接违反「Rust workspace 零第三方 crate、13 条 gate 离线通过」的硬约束。本切片只能落成「显式未实现」。 |

### 已知限制

登记摘要随项目公开，因此 `secret` 必须是高熵随机材料；低熵口令可由公开摘要离线暴力破解。这一限制写在 `docs/guides/cli-reference.md` 的命令合同里，不靠约定俗成。

这是**本地**身份根，不是组织身份根。每份决策记录带 `identity_kind = "enrolled-local-principal"` 与 `org_identity = false`，输出不会把持有本地密钥说成组织身份。

### 授权链

- `request` 的授权来自 principals 登记，**不来自已批准例外**——这正是自锁的解法。
- `approve` / `reject` / `revoke` 要求 `approver` 角色。
- 默认四眼：请求者不批准自己的请求，除非登记里显式写了 `"self_approval": true`。
- 状态机终态即终态：`requested → approved|rejected`、`approved → revoked`；已 `rejected` / `revoked` 不能被复活。
- `--role` / `--actor` 仍被接受为参数，但不参与任何授权判定，也没有参数能让它们参与。

### daemon API 不承载此通道

`POST /api/v1/exceptions` 返回 `api.identity_required`。HTTP 请求不能安全携带登记密钥，daemon 因此无法建立所需身份。它明确失败，而不是造一条 requester 为字面量 `user` 的例外——诚实的降级优于虚假的能力。

## 决策二：mutation 门禁的边界

`authorize_store_apply` 保护的是**改变受保护状态或授予后续能力的写入**，不是所有落盘。

| 类别 | 例子 | 门禁 | 审计 |
| --- | --- | --- | --- |
| 受保护状态 / 能力 | standard 发布与删除、adoption、intent（`apply` / `rollback` / `preflight`）、advisor 候选、experiment 结果、`POST /api/v1/lab` | 必须 | 必须 |
| 只增观测记录 | Receipt append（`inspect` / `doctor --store` / `preflight` 的 Receipt 部分） | 不设 | 必须 |

Receipt append 不进 mutation 门禁的理由：它只增不改（`put_receipt` 首写即定，重复 persist 不覆盖），删除走 tombstone 且不可用同 id 重建，本身已内建审计，且不授予任何后续能力。把它纳入门禁会让「诊断并保存一次记录」在默认状态下必须先走完整例外审批，而 Doctor 是产品默认主入口。

这条边界与 `2026-09-06` 可视化闭环补充方案第三方评审 review-3 §9 第 1 条的字面清单有出入：该清单把 `doctor --store` 列入补授权范围。此处显式记录该分歧与理由，不静默绕过；若后续判定 Receipt append 也应受门禁，改动落在 `persist_inspect` 一处。

## 后果

- `exception.identity_source_uncovered` 不再是产品的终态，V13/L06 通道可用。
- 所有 store 写入进审计链：`put_named` / `delete_named` 补上了此前完全缺失的审计，`delete_named` 返回是否真的删除了东西，破坏性删除不再谎报成功。
- 新增 reason code：`principal.registry_absent` / `principal.registry_schema` / `principal.registry_malformed` / `principal.not_enrolled` / `principal.secret_absent` / `principal.secret_mismatch` / `principal.enrollment_incomplete` / `principal.role_unknown` / `principal.absent`、`exception.approver_role_required` / `exception.requester_role_required` / `exception.self_approval_not_enrolled` / `exception.transition_invalid` / `exception.already_exists`、`api.identity_required`。
- 没有引入第三方 crate，13 条 required gate 仍离线通过。
