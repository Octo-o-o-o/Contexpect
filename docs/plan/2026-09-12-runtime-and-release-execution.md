# 原生运行、加密同步、Effect runner 与发行验收

> 状态：实施中。接续已提交的交接收口；本页不代表完整产品验收通过。

## 范围与验收

按既有 WP-07 / WP-10 / WP-12 合同实施，不缩减 F-01–F-18 或冻结矩阵。

| 工作包 | 本次执行与可判定验收 | 保留的独立条件 |
| --- | --- | --- |
| Native | 隔离的合成项目调用实际 `grok inspect --json` / `codex debug prompt-input`，记录版本、退出码和可观察字段；版本漂移单列 | 不把目录清单解释为已消费正文；冻结版本之外不升级 required 结论 |
| E2EE | age 格式 + SSH 签名 adapter；双 profile 加解密、篡改、未知 signer、撤销、replay、分叉、并发状态测试 | 私钥由外部工具管理；不得复用本地 Receipt HMAC 冒充远端信任 |
| Effect | versioned 外部 runner adapter，冻结合同、进程结果、超时、预算和证据回读，实际子进程运行与统计入口闭环 | fixture gate 与真实模型效果分列；不自造模型观测 |
| Release | 从 canonical matrix 逐格汇总证据，缺证据 fail-closed；安装/恢复/删除与浏览器验收；长 soak 可恢复运行 | 三条实际 OS lane、72h、8 人及独立 readback 不由本机模拟替代 |

本次获准实施、运行和模拟验收；不自动发布正式版本、不改用户原生配置、不读取用户会话与凭据。

## 当前环境核对

Grok 实装版本为 1.0.13；系统 Codex 为 0.153.3。本轮额外取官方 0.147.0 到隔离工具目录（未替换系统安装），以冻结 harness 版本跑实际 oracle。实际 macOS 27.0 build 26A428 与冻结 build 26A5425a 不同，因此 OS coordinate 不报 required pass。隔离 profile 不继承用户会话或登录材料。age 1.3.2 的官方 darwin-arm64 发布包已按官方 SHA-256 核对，工具保存在忽略的本地任务目录，不安装全局配置。

## 已运行的本地验收

- 加密：`scripts/check_secure_sync.py` 29 项通过，真实 age/SSHSIG、双临时 profile；包括未来 recipient 撤销、签名/密文篡改、replay/fork/rollback、过期或变更预览、拒绝后密文 head 不变。原生投影仍未应用。
- Effect：`scripts/check_effect_runner.py` 14 项通过。真实本地 runner 子进程覆盖四种 decision、漂移、超时/crash、预算、不重复执行与错误协议强制 inconclusive；模型调用 0 次。
- Native：产品 capture 的两种工具各完成采集与 pin 漂移拒绝，共 4 检查。冻结 oracle 34 条输入均实际执行退出 0；其中 Codex 18 条历史 expected-shape 为对象，实际 0.147.0 输出数组，只有 Grok 16 条 raw-shape 匹配。未重写冻结 golden，未将 process success 当作 34 条语义验收通过。产品 adapter 使用实际数组格式，输出脱敏 observation，不生成 native Claim 或模型消费证明。
- Daemon：`scripts/check_daemon_soak.py --seconds 250 --changes 1000` 实际 308.174 秒，观察 1,000 次稳定指令变化与 1,001 条含初始基线的通知；重复进程拒绝、同内容/重启去重、产品 stop 通过。首次压力运行暴露 macOS 接收 socket 继承非阻塞导致 408，修复后该轮通过。`soak_72h_complete: false`。
- 安装：`scripts/package_cli_candidate.py` 生成 unsigned 本机 CLI tar 与 SHA-256，在临时目录检查安装、同候选并存/回退、删除所拥有前缀后保留原生文件和 store。它不是正式 updater，不证明跨版本迁移、签名或桌面安装。
- UI：新增真实 daemon 的 periodic Monitor Receipt 跳转与 Sync 边界提示截图；完整浏览器用例 40/40、UI unit 60/60 通过；16 条 required gate 已分别运行并在修正后通过，提交后还会记录干净 HEAD 的整轮结果。独立 Tauri workspace 离线 dev build 与 CLI release build 通过，未签名、未安装到用户正式应用目录。

## 复现入口与未完成条件

`python3 scripts/release_inventory.py --output <local-report.json>` 从冻结表全量枚举证据槽位；未提供证据返回 3。输入证据须绑定候选与工件 digest。验证文件存在/摘要不等于认证 reviewer 或语义结论；`full_product_accepted` 保持 false。缺槽位数量不是源码缺陷数量。

测试密钥、真实工具、原始运行日志与包保留在忽略的本地任务目录，不发布私人路径或密钥。上述 smoke 脚本均接收 `--bin`；同步脚本另需显式 age/age-keygen，native 脚本需显式 Codex/Grok。

完整工作包仍需：同步原生落地及跨设备语义核对、组织密钥/信任流程、Git provider；真实模型 Effect 的 harness/model/调用预算与外部 gate 证据；完整冻结三 OS build、签名安装/升级/卸载与恢复、72h、50 sessions/四周期、8 人六任务 QA、独立 readback。模拟人工操作只覆盖软件可复现路径，不能代签人的评价。历史状态文档保留历史语境。
