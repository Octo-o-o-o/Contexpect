# 用户指南

> 状态：规范（完整产品运行时尚未实施）
> `ctxpect inspect` / `doctor` / localhost UI 已可在开发树构建。安装包、全 OS WebView 与人工可用性门禁尚未实施。

## 三种节奏，同一产品

| 模式 | 何时 | 做什么 |
| --- | --- | --- |
| One-shot / 偶尔诊断 | 行为异常、升级工具、换仓库 | `ctxpect inspect` 或打开 Doctor |
| Periodic / 定期保养 | 每周/月/季度 | scheduler 或 CI 生成差量报告 |
| Continuous / 长期开启 | 文件、版本、session、设备变化 | 可选 daemon，安静提醒 |

不需要账号。daemon 可选。第一次 one-shot 不要求初始化数据库。

## 只读桌面主链（当前可复制）

在隔离临时项目上（不要指向真实 home）：

```bash
# cwd：仓库根
export CARGO_NET_OFFLINE=true
cargo build -p ctxpect-cli
CORPUS=acceptance/corpus/development/static/inputs
PROJ="$CORPUS/dev__static__codex__0.147.0__cli__macos-27-arm64__instructions__positive"
STORE=/tmp/ctxpect-demo-store
./target/debug/ctxpect inspect --offline --json --project "$PROJ" --store "$STORE"
./target/debug/ctxpect doctor --json --project "$PROJ" --store "$STORE"
python3 scripts/check_ui_routes.py
# UI（需要 Node）：pnpm --dir packages/ui build
# ./target/debug/ctxpect daemon start --project "$PROJ" --store "$STORE" --listen 127.0.0.1:7420 --ui-root packages/ui/dist
```

忽略示例请用 `__instructions__negative`（含 `.ctxpect-ignore` / G4），不要把 `__ignore__04` 当成 instructions 排除示例。默认不扫描 HOME。未知版本 fail-closed（exit 3）。

## 第一次打开（计划行为）

1. 产品解释 local-first 与数据边界
2. 选择项目目录；可选授权扫描个人配置目录
3. 探测 18 个 family 的 executable / App / residue / auth / connector，不登录
4. 为每个 harness/surface 生成 Expected Receipt；不可见层为 Unknown
5. 主界面展示工具差异、设备依赖、budget、高置信问题
6. 可选导入原生 runtime snapshot

首次优先回答三个问题：工具差异、设备依赖、当前风险。

## One-shot

```text
ctxpect inspect
ctxpect doctor
ctxpect receipt export --redact default
```

在当前目录自动探测。终端给紧凑结果，并可打开本地 UI。导出的 Receipt 默认脱敏，可交给同事或 issue。

## 任务前

选择项目、cwd、harness、surface、任务描述。查看常驻/条件/渐进/运行时/不透明层。确认 profile、MCP 与敏感边界。生成 preflight Receipt。可启动 harness 或复制严格 quoting 的启动参数。Secret 不出现在命令预览里。

## 定期

使用系统 cron/launchd/Task Scheduler 或 CI。报告只列新增/恶化/已解决。月度 context debt：过时、重复、在已观察窗口未激活、过大、无来源、版本不兼容；同时展示未导入历史和不可观察 surface。

## 连续

daemon 默认只听 metadata/hash/mtime/版本和明确授权的 runtime event。相同状态不重复通知。资源占用可见且有上限。可按项目暂停。用户能看到“为什么提醒”和来源 snapshot。

## Doctor 工作流

问诊 → 证据链 → 必要时收集原生证据 → Care Plan preview → 唯一 authority 应用 → rollback point → Recheck Receipt。

Indeterminate 时 Treatment 锁定。不要在模型可见性未知时直接改配置。

设计参考：`docs/gpt-img-2-design/20260904-1200-contexpect-doctor/`。文案与状态以 spec 为准。

## 换设备

设备 A 选择非秘密资产 → 加密 bundle 或 Git manifest → 设备 B preview → 原子应用 → 语义 Receipt 核对。成功同步不等于 verified。

## 团队标准

负责人 `ctxpect standard publish` 一份签名的 Team Context Standard。成员先 preview/disclosure，再 adopt/pin。对齐看的是 CanonicalIntent 的 native projection，不是把同一份 MD 拷到每个工具。Care Plan 仍负责 preview/apply/rollback。本阶段这些命令已有阶段实现（签名/验签、采纳状态、持久化预览与事务化 apply/rollback，见 [cli-reference](cli-reference.md)）；revision 历史、Git provider 与可到达 `verified` 的语义对账尚未实施（见 [交付状态](../process/2026-09-08-delivery-status.md)）。

## 效果评估

只有冻结 ExperimentContract 的对照实验才能给出 supported-* decision。单次前后对比不是因果证据。

## 隐私

Privacy mode 隐藏路径、用户名、repo、prompt 片段和 server URL。默认不上传正文。删除历史会失效派生分析。
