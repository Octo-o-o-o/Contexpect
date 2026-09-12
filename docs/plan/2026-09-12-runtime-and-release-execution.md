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


## 真实模型实验 E-LUNA-01（2026-09-12）

按 owner 指定的 Codex `gpt-5.6-luna/max` 与 16 次调用预算，完成 8 组配对、32 道合成合同判断题。Codex native CLI 0.153.3 是本轮明确采用的开发期坐标，不修改冻结 native oracle 的 0.147.0 pin。无需第二个模型，未调用 Grok。

开跑前冻结 `tests/effect/contract-context-suite-v1.json` 的题目、选项、golden、合同上下文、source revision 与 digest。控制组不提供额外合同摘录，处理组只增加对应摘录；两组问题、模型、effort、schema、判分器一致。主指标为每个 task 的四题全部答对，逐题分数只作描述。顺序按 task 交替 control-first / treatment-first；没有在观察结果后补样本。n=8 的 power 声明仅基于“独立配对、benefit-only discordance=0.85”的大效应设计假设（0.894787），不代表对一般 10pp 差异或真实开发任务具备该检验力。

| 指标 | Control | Treatment |
| --- | --- | --- |
| 完成并通过 native 身份/协议核验 | 8/8 | 8/8 |
| 主指标：整组四题全对 | 6/8 | 8/8 |
| 逐题正确数（描述性） | 30/32 | 32/32 |

配对 discordant 为 treatment-only 2、control-only 0；两侧精确检验 `p=0.50`，产品返回 `inconclusive / effect.estimator_inconclusive`，exit 3。exit 3 在此表示实验已完成但统计结论不确定，不是 16 次执行失败；没有支持 beneficial、harmful 或 equivalent，也没有 causal Claim。两个 control 错题是：把静态匹配当作 model-visible，以及把 `--as-of` suppression 的 UTC 正午评估误选为当地零点。

16 次新会话均从自身 native `turn_context` 核对为 Luna/max、read-only、approval never；从各自 input 记录精确回读冻结 prompt；没有工具事件、超时、锁定字段漂移或重复调用。另一次宿主确定性回读将每条 gate 文件 hash、协议 stdout hash、产品 executor receipt、checkpoint run、逐题答案及配对 p 值逐项核对一致。这是宿主回读，不称独立 reviewer GREEN。聚合 usage 为 input 274,906、output 3,295（其中 reasoning 2,735，已包含在 output 内）；provider 内部重试次数不由 CLI 暴露，不将 16 次 CLI invocation 当作可验证的精确 provider request 数。

可发布的逐 run 脱敏摘要见 [E-LUNA-01 结果](../process/2026-09-12-effect-luna-max-result.json)。原始本地 profile、request、preregistration、checkpoint 和 gate 记录留在忽略的任务目录；不发布 HOME、proxy 或 native session id。后续再跑必须新建合同与预算，不能为追求显著性续接本实验。

### 复现工具

`scripts/codex_effect_runner.py` 是 command-v1 的外部 Codex adapter；`scripts/prepare_codex_effect.py` 只准备独立临时 store、固定数据集副本、profile、launcher pin 和请求，不调用模型。例：

```bash
python3 scripts/prepare_codex_effect.py --codex /absolute/path/to/native-codex --model gpt-5.6-luna --effort max --output /absolute/path/to/new-experiment
```

必须传原生可执行文件而不是解析到其它版本的 shell wrapper。准备器仅在新建的实验 store 写两项本地执行 grant，不修改用户既有 policy 或读取 auth 文件。获得该轮实际调用授权后运行：

```bash
ctxpect experiment --execute --adapter command-v1 --from /absolute/path/to/new-experiment/request.json --project /absolute/path/to/new-experiment/project --store /absolute/path/to/new-experiment/store --json
```

adapter 通过已有 Codex 登录启动真实模型；只保存判分标签、usage、元数据及摘要，不自行保存原始模型响应。Codex 自身保留本次新建的合成会话。首个 native 完成/身份/隔离故障会留下 stop 记录，后续 invocation 不再启动模型；要修复或重做必须显式开启新的实验，不能覆盖已冻结目录。

E-LUNA-01 验证的是这个固定合同判断坐标的真实 runner，尚不等于真实 coding-task benchmark、全部模型或 WP-10 的完整验收。

## 真实项目展示核验与修订（2026-09-13）

针对真实仓库浏览器审计，本轮修正了二进制正文误判、目录状态串扰、证据范围与加载状态表达、记录时间显示，以及浏览器 inspect 未继承 daemon 已授权全局根的问题。根 README、AGENTS 与文档入口同步当前阶段实现状态，历史验收报告保留。

复核既有合同后，撤回“可按测试目录直接排除凭据命中”的初步方向：测试 token 仍 fail-closed。Doctor 默认展示全项目审计，以只读筛选区分 Receipt 缺口、关联指令文件和其他项目文件；测试/历史路径仅作提示，不授予信任、不更改 CI。PNG 等已识别二进制容器不进入文本规则，但 Markdown 伪装前缀、普通文本错误扩展名与异常 UTF-8 仍有反例检查。普通 collect 内容摘要语义不变。

真实仓库本轮 CLI Doctor 一次测得 10.847 秒，返回 exit 2（存在阻断，符合审计结果）；浏览器通过新构建 daemon 读到相同 127 confirmed、5 suspected、5 unknown、56 active_blocking，共 132 行，其中 Receipt 相关 6 行、关联指令文件命中 0 行、其他项目文件 126 行。此为该时点项目与未授权全局根的本地快照，不是稳定 benchmark，也不是 132 个当前上下文缺陷。PNG 误报消失；仓库内容变化后总数可变化。

浏览器实查 Doctor 筛选及不变的总计、Inspector 六 facet 与未测预算、Checkup 静态 pass 边界、Receipt 列表与详情的本地时间、集成逐项状态、资产和同步配置说明。另在显式授权全局根的临时 daemon 中，通过页面检查确认全局与项目 AGENTS 两层静态采用；该授权不读取私人会话或凭据文件，声明的规则坐标仍不代表实际安装版本。当前仍缺运行期 installed/model-visible/use-evidence/outcome-affecting 的对应证据，不能把静态采用提升为真实消费证明。

加载期间保持加载状态直到诊断完成；单次 HTTP 请求 30 秒超时会明确报告 api.timeout，不把超时当作 Unknown，也不宣称服务器已停止。扫描性能改善仅来自避免读取无需分析的大文件；未声称全部真实项目都在固定时间内完成。

本轮 16 项 required gate 全部通过，UI unit 63/63、完整 Chromium E2E 43/43 通过（含真实 30 秒超时与诊断完成前持续 loading 的回归）。源码、产物与本地服务分别核验；原验收服务通过所有权 stop 更新，15 个 store 文件在重启前逐一校验备份，原历史保留，并继续使用原地址。以上为宿主自检与浏览器实查，不称独立 reviewer GREEN，也不替代冻结全系统发行验收。
