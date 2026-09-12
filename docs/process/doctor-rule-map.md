# Doctor 规则映射表

> 状态：规范（Doctor 项目内容规则已实施；规则语义以本表为准，度量以 `doctor-corpus` 门禁输出为准）。
> 目的：把冻结 Doctor 语料（`acceptance/corpus/development/doctor/doctor-corpus.jsonl`，707 行，20 个 `rule_id`；2026-09-09 contract revision 前为 589 行）的每条规则映射到**语义、输入合同、实现位置、阻断性**，并说明它与既有 `D-*` 规则的关系。规则实现在 `crates/ctxpect-doctor/src/rules.rs`；精确率/召回率由 `cargo test -p ctxpect-cli --test doctor_corpus` 逐规则打印并断言。

## 命名空间与别名

- **语料命名空间**（`secret_literal` 等 20 条）是 PRD §17.1 与语料共用的规则 id，本表逐条实现，作为**新规则**进入产品输出（`rule_namespace: "doctor-corpus"`）。
- **既有 `D-*` 规则**（`D-UNKNOWN-SURFACE`、`D-FACET-*`、`D-TRUNCATED`、`D-IGNORE-G4`、`D-HOME-NOT-GRANTED`、`D-UNSUPPORTED-VERSION`）来自 inspect Receipt，**不重命名**：finding id 已随 Receipt 发布，改名会改变已发布 id。
- **规则集之外的两条产品 finding**：`declaration_unreadable`（`rule_namespace: "doctor-declarations"`，非阻断：`.ctxpect/<name>` 存在但不是 JSON 对象，或根目录同名文件带了 `schema` 却不可解析）与 `doctor.suppression_invalid`（非阻断、`suspected`：suppression 条目缺字段、已过期、`evidence_digest` 不匹配或整份文件不可读）。两者不在语料里，不参与 precision/recall。
- 语义相同者只做别名：`D-TRUNCATED`（G3 聚合截断）与 `cap_truncation`（声明的 cap 截断）语义相同，`D-TRUNCATED` finding 带 `aliases: ["cap_truncation"]`。`D-UNSUPPORTED-VERSION` 是 harness 坐标版本，`version_incompatible` 是 adapter 声明版本，不是同一语义，不做别名。其余 `D-*` 无对应语料规则。

## 声明类 finding 的「声明校验」标注（C-F04，2026-09-12）

凡读取自有声明文件（`layout.json` / `inventory.json` / `plan.json` / `archive-manifest.json` / `hooks.json` / `budget.json` / `device-lock.json` / `provenance.json` / `adapter-version.json` / `placement.json`，含 `declaration_unreadable`）产生的 finding，输出中带 `declaration` 对象：

- `kind: "declaration-validation"` —— 该 finding 校验的是声明文本，不是被声明对象的真实行为；
- `source_file` —— 实际命中的声明文件路径；
- `producer: "unknown"` 且 `trusted_producer_connected: false` —— 十个声明文件**没有任何产品侧可信写入者**（见「诚实边界」），声明不能自证其来源；
- `declared_at` —— 仅当声明文件自己带了 `declared_at` 时原样引用（本身也是自声明字符串），否则为 `null`，不编造时间。

**`approved: true` 不是授权证据。** `unapproved_lossy_projection` 的触发条件不变（`drops` 非空且 `approved != true`，语料答案一行未动），但 finding 文案明确：plan.json 的 `approved` 是产品自声明，即使写了 `approved: true` 也不构成授权——授权证据只认 policy/exception 链（`ctxpect_policy` 的决策与例外记录）。同理，`inventory.json.present`、`budget.json.truncated` 等自有声明只作为「声明原文」被校验，不创造事实。

## `D-*` finding 的 treatment lock（C-F03，2026-09-12）

Doctor 的 treatment 是**有限静态修复**；某条 indeterminate 证据是否锁定它，按 policy 前置证据表（`ctxpect_policy::precondition`，`ActionClass::LimitedStaticFix`）判定，不再按 facet 名一刀切：

- `D-FACET-MODEL-VISIBLE` / `D-FACET-USE-EVIDENCE`（runtime-surface 证据）与 `D-FACET-OUTCOME-AFFECTING`（outcome，表中显式「仍可未知」列）indeterminate 时：finding 仍输出、`confirmation`/`severity` 为 `suspected`、`evidence_state: "indeterminate"`、计入 `counts.unknown`，但 `treatment.locked: false`、`lock_reason: "none"`——缺效果或运行面证据不阻止安全静态修复。`--fail-on confirmed` 因此对纯 facet-indeterminate 诊断不再 exit 2。
- 目标归属/权限/坐标类未知仍锁：`D-UNKNOWN-SURFACE`（`lock_reason` = 该 unknown 的 reason code，如 `permission_not_granted`）与 `D-UNSUPPORTED-VERSION`（`lock_reason: "unsupported_harness_version"`）保持 `treatment.locked: true`、`suspected`。它们不标 `confirmed`，所以也不单独触发 `--fail-on confirmed`。
- 语料命名空间 20 条规则的 finding 与 8 条阻断规则不受影响（下表与 `BLOCKING_RULES` 未动）。


## 阻断判定（`ci` 与 `doctor` 共用）

唯一判定函数：`ctxpect_doctor::blocking_exit(diagnosis, fail_on)`。

- finding 带 `blocking: true` → exit 2。只有 `BLOCKING_RULES` 里的规则产生 `blocking: true`。
- `--fail-on confirmed` 且 `counts.active_confirmed > 0` → exit 2（无 suppression 时 `active_confirmed == confirmed`）。
- 带 `suppressed: true` 的 finding 不计入以上两条（见「suppressions」）。
- 其余 → 0。`ci` 再与 inspect 出码、policy 出码合并（任一 3 → 3；否则任一 2 → 2；否则 0）。

**阻断资格 = 语料精确率 1.00。** `doctor-corpus` 门禁对 `BLOCKING_RULES` 逐条断言 precision == 1.00；任一规则出现假阳性即门禁失败，修法只能是把该规则移入 `NON_BLOCKING_RULES` 并在本表改「阻断」为「否」。不能反过来修改语料答案（红线 3）。

## 输入合同

规则输入是 `ScannedFile` 列表：由被动采集器 `ctxpect_collect::scan` 列出、经 `read_contained` 读取（≤ 1 MiB、非 withheld、非多硬链接）的常规文件，以及符号链接的原样 target。**不执行、不解压、不联网。**

「惰性声明」类规则读取项目内的声明文件（`layout.json`、`inventory.json`、`plan.json`、`archive-manifest.json`、`hooks.json`、`budget.json`、`device-lock.json`、`provenance.json`、`adapter-version.json`、`placement.json`）。一份*声明*了 `../etc/passwd` 条目的 manifest 就是遍历声明，不需要打开任何压缩包才能判定——这与 B16「真实解压安全」是两件事（缺口分析 §5）。

**声明文件的双轨识别（2026-09-09 contract revision）**：一个文件只在两种拼法之一下才是 Contexpect 声明——`.ctxpect/<name>`（无需 `schema` 字段），或根目录 `<name>` 且顶层 `"schema": "ctxpect-<name 去掉 .json>-v1"`（例如 `plan.json` → `ctxpect-plan-v1`）。根目录同名文件没有该 `schema` 就是别人的文件，不判读、不报；两种拼法同时存在时**都**判读，finding 的 `path` 是实际命中的那份。`crates/ctxpect-resolve` 的 Claude Code resolver 读 `budget.json` 时遵守同一规则（`native_paths_used` 与 evidence 记实际路径）。语料生成器 `scripts/contexpect_fixtures.py` 的 `_declarations` 是同一规则的参考实现。

**suppressions**：`.ctxpect/doctor-suppressions.json`（`schema: ctxpect-doctor-suppressions-v1`，`suppressions[]` 每条 `rule_id` / `path`（精确的项目相对路径）/ `owner` / `reason` / `expires_at`（store 时钟 `secs.msZ`、RFC 3339 或整数秒）；阻断规则必须带 `evidence_digest` = 被评审文件字节的 sha256（symlink 取 target 文本））。命中的 finding **仍然输出**，保留真实 `confirmation`，加 `suppressed: true` 与 `suppression{owner, reason, expires_at, evidence_digest}`；`counts` 保留事实计数 `confirmed` / `blocking`，另加 `active_confirmed` / `active_blocking` / `suppressed`，`blocking_exit` 只看 active。诊断带 `suppressions{file, present, file_digest, evaluated_at_secs, applied, unmatched, invalid, entries, scope}`。过期按评估时钟判定（默认墙钟；`doctor --as-of` 指定时以该日 UTC 正午为评估时刻，输出 `suppressions.evaluated_at_secs` 记录实际读数）；内容改一个字节即 `evidence_digest` 不匹配、条目失效并作为 `doctor.suppression_invalid` 报出。文件在 `.ctxpect/` 控制路径下，`apply` 例外写不了它。suppression 是项目级接受风险，不放宽 projection 的 secret 门、路径包含与被动扫描规则。

## 规则表

「实现」列：`rules.rs` = 已实现于 `crates/ctxpect-doctor/src/rules.rs`；来源列区分 **内容**（文件字节）、**采集**（collect 清单，如符号链接/摘要）、**声明**（上述 JSON 声明文件）。语料列 = 正例数 / 反例数（clean-lookalike），含 2026-09-09 revision 新增的行：每条规则 2–3 个**判定条件不同**的正例与反例变体（`…:variant:NN`），声明类规则另有「根目录同名文件无 `schema`」反例（`…:negative:noschema:00`）、「`.ctxpect/` 拼法」正例（`…:positive:dotctxpect:00`），阻断的声明类规则再加「根目录干净声明 + `.ctxpect/` 违规声明」正例（`…:positive:conflict:00`，finding 落在 `.ctxpect/` 路径）。

| rule_id | 语义 | 来源 | 输入合同（触发条件） | 反例边界 | 阻断 | 实现 | 语料 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `secret_literal` | 项目文本含凭据**形状** | 内容 | `ctxpect_doctor::secret_literal`：PEM 单行 armor（`-----BEGIN … PRIVATE KEY-----` 在同一行）、`AKIA` + 16 大写/数字、`aws_secret_access_key` 赋值（`=`/`:`，键与值可带引号，值 ≥16 凭据字节 `[A-Za-z0-9_/+=.-]`）、`ghp_/gho_/ghs_/github_pat_` + ≥20、`xox[abprs]-` + ≥10、`sk-` + ≥20 且**含数字与字母**、`Authorization: Bearer`（header 行或 JSON 键值，token ≥16 凭据字节） | 键名（`API_KEY=placeholder`）、裸前缀、占位形状（`<your-key>`、`${ENV}`、`{{tpl}}`、`****`、含 your/placeholder/example/redacted/changeme/xxxx 的值）、`sk-learn-…` 之类无数字的词串、只在散文里提到 `PRIVATE KEY-----` 的行都不算；`password=…`、数据库 URL 口令、裸 hex token 不在文法内 | 是 | `rules.rs` / `secrets.rs` | 23 / 23 |
| `hidden_unicode` | 零宽/不可见/双向控制字符隐藏文本 | 内容 | U+200B、U+2060、U+FEFF（非首字节 BOM）、U+00AD、U+034F、U+061C、U+180E、U+2061–2064、U+202A–E、U+2066–9、U+206A–F、tag 块 U+E0000–E007F；U+200C/U+200D（ZWNJ/ZWJ）只在**前后都是 ASCII** 时算隐藏 | 首字节 BOM 不算；emoji ZWJ 序列（👩‍💻）、波斯语 ZWNJ、天城文连字里的 joiner 不算 | 是 | `rules.rs` | 23 / 22 |
| `path_containment_escape` | 指令 `read:` 路径逃出根 | 内容 | 行首 `read: <path>`（大小写不敏感，允许 `- ` / `* ` / `+ ` 列表前缀），path 绝对或按所在目录词法解析后越过根 | `read: ./inside/key` 不算；`@../x`、URL 编码形式不在文法内 | 是 | `rules.rs` | 23 / 22 |
| `symlink_escape` | 符号链接目标离开工作区 | 采集 + 声明 | (a) 采集到的 symlink，target 词法逃出根；(b) `layout.json.symlinks[].to` 同判；产品入口传入项目根（`project_findings_in`），绝对 target 词法落在根内的不算 | 指向根内文件（相对或绝对）不算；`~`、Windows 盘符 target 不判 | 是 | `rules.rs` | 25 / 23 |
| `required_asset_missing` | 声明必需的资产缺失 | 声明 | `inventory.json.required` 中既不在 `present` 也不在磁盘的项 | 在 present 或磁盘上存在不算 | 是 | `rules.rs` | 25 / 23 |
| `unapproved_lossy_projection` | 有损投影未获批准（声明校验：plan.json 的 `approved` 是自声明，授权证据只认 policy/exception 链） | 声明 | `plan.json.drops` 非空且 `approved != true`（缺失即未批准） | `drops: []` 或 `approved: true` | 是 | `rules.rs` | 25 / 23 |
| `archive_traversal` | 归档条目逃出目的地 | 声明 | `archive-manifest.json.entries[]` 含 `..` 段或绝对路径 | 普通相对文件名 | 是 | `rules.rs` | 25 / 23 |
| `passive_scan_exec` | 被动扫描被声明执行 hook | 声明 | `hooks.json.on_scan` 含非空命令 | `on_scan: []` | 是 | `rules.rs` | 25 / 23 |
| `duplicate` | 指令正文字节相同 | 采集（摘要） | ≥2 个 `.md` 内容摘要相同；报告组内排序最后的路径 | 正文不同 | 否 | `rules.rs` | 10 / 6 |
| `conflict` | 指令文件间包管理器指令冲突 | 内容 | `.md` 中 `must be (npm\|pnpm\|yarn\|bun)` 指令出现 ≥2 种；报告首个与基线不同的文件 | 同一种管理器 | 否 | `rules.rs` | 10 / 6 |
| `stale` | frontmatter `updated:` 过旧 | 内容 | 日期比评估日 `as_of` 早 365 天以上；`as_of` 是显式输入：CLI 默认系统当日、`--as-of YYYY-MM-DD` 覆盖，corpus 门禁与测试固定注入 acceptance cutoff（2026-09-04，即 `STALE_CUTOFF`） | 一年内 | 否 | `rules.rs` | 10 / 6 |
| `oversized_resident` | 常驻资产超过声明 cap | 声明 | `budget.json`：实际字节 > `max_bytes` 且未声明 `truncated` | 未超 | 否 | `rules.rs` | 11 / 7 |
| `cap_truncation` | 指令被 cap 截断 | 声明 | `budget.json`：实际字节 > `max_bytes` 且 `truncated: true`，或 `truncated: true` | `truncated: false` 且未超 | 否 | `rules.rs`（别名 `D-TRUNCATED`） | 11 / 7 |
| `bad_frontmatter` | frontmatter 不是合法 key/value YAML | 内容 | `---` 块未闭合、行非 `key: value`、`[`/`{`/引号未闭合 | 合法块 | 否 | `rules.rs` | 10 / 6 |
| `gitignore_mismatch` | 被忽略的路径仍存在且被指令引用 | 内容 | `.ctxpect-gitignore` / `.gitignore` 条目：文件存在且任一 `.md` 引用其名 | 条目不存在为文件 | 否 | `rules.rs` | 10 / 6 |
| `single_device_only` | 资产锁定单设备且不同步 | 声明 | `device-lock.json.sync == false` 且 `device_id` 非空、非 `*` | `device_id: "*"` 或 `sync: true` | 否 | `rules.rs` | 11 / 7 |
| `unknown_source` | 导入资产无来源 | 声明 | `provenance.json.source` 缺失/空；路径取 `path` 或 `imported.md` | 有来源 | 否 | `rules.rs` | 11 / 7 |
| `version_incompatible` | adapter 声明版本不匹配 | 声明 | `adapter-version.json.required != actual` | 相等 | 否 | `rules.rs` | 11 / 7 |
| `undiscoverable_path` | 声明的指令路径不可发现 | 声明 | `layout.json.files[].path` 以 `..` 或 `/` 开头 | 相对根内路径 | 否 | `rules.rs` | 11 / 7 |
| `placement_recommendation` | 资产不在推荐路径 | 声明 | `placement.json.path != recommended` 且 `path` 存在 | 相等或不存在 | 否 | `rules.rs` | 11 / 7 |

20 条规则全部有实现；`doctor-corpus` 门禁逐条 precision 1.00 / recall 1.00，125 条 clean 无 finding，707 行 `expected_exit` 与 `blocking_exit` 全部一致。

**Contract revision（2026-09-09）**：原 589 行的**答案一行未改**（`expected_findings` / `expected_output` / `expected_exit` / `class` / `blocking` 与 43ee2fe 逐 id 相同，脚本核对），只有声明类输入文件多了 `schema` 字段（280 个输入文件的差异仅此一键，脚本核对）；新增 118 行（29 行双轨识别 + 89 行内容变体）。理由：声明类文件名此前是无命名空间的自有约定，真实项目里任意同名 `plan.json` 会被判读，加命名空间必然改输入；按 PRD §17.0 用 `scripts/generate_acceptance.py` 出 revision 而不是手改语料。

## 度量与语料的诚实边界（2026-09-09 交叉 review）

- **语料信息量**：revision 前，20 条规则中 19 条各只有 **1 个独立正例 + 1 个独立反例**（其余行逐字复制，只有 `envelope.json` 不同）。revision 后每条规则有 3–4 个内容不同的正例与 3 个内容不同的反例（变体差在判定条件：列表前缀、引号写法、边界相等、目标缺失、`.ctxpect/` 拼法等），仍全部落在 Python 参考解释器与 Rust 规则**共同**的文法内——它仍是回归护栏，不是对文法泛化能力的度量；解释器文法更窄的部分（例如 `hidden_unicode` 只认 U+200B、`stale` 只认 2019 年）不能借语料区分，靠 `secrets.rs` / `rules.rs` 单测覆盖。
- **`expected_exit` 的层次**：语料的 `expected_exit` 与 `ctxpect_doctor::blocking_exit(diagnosis, fail_on)` 函数逐行一致；产品命令 `doctor` / `ci` 的最终 exit 还**合并 inspect 出码**——40 条非阻断正例的输入目录没有 `AGENTS.md`，inspect 的 `policy_result.verdict = fail` 使 `ctxpect doctor` 对它们 exit 2。这不是规则误报，是同一命令承载两个判定；接手者不要把「非阻断规则永不 exit 2」读成产品命令级的保证。
- **声明文件是 Contexpect 自有约定**：十个声明文件目前**没有任何产品侧写入者**（只有语料生成器写它们；产品侧只有 Claude Code resolver 读 `budget.json`）。2026-09-09 起按「输入合同」的双轨识别读取：根目录同名文件不带 `schema` 不再被判读，Cursor 真实 hook 位置 `.cursor/hooks.json` 仍不在范围。
- **`symlink_escape` 的绝对路径**：`doctor` / `ci` / API 经 `project_findings_in(files, Some(root))` 传入项目根，`ln -s "$(pwd)/x" y` 这类指向根内的绝对链接不再误报；无根调用（`project_findings`）仍把绝对 target 一律判逃逸；`~`、Windows 盘符 target 不判。
- **对本仓库自身**：`ctxpect doctor --project <本仓库>` exit 2——语料与测试里的 fixture token、文档里的 `Authorization: Bearer` 示例按「fixture token fail closed」策略命中。suppress 机制已实施（见「输入合同」），但本仓库**没有**为自己写 suppressions：这些命中是语料与文档的事实，逐文件绑定 `evidence_digest` 会随每次改动失效，`ctxpect ci` 因此仍不在本仓库自己的门禁里。
- **文本解码**：不是合法 UTF-8 的字节被替换后仍参与内容规则（`String::from_utf8_lossy`），与 projection 的 secret 门一致；此前单个 Latin-1 字节会让整个文件跳过全部内容规则而无任何 finding。>1 MiB 与多硬链接的文件仍按 collect 规则不读，且**不报**——rule-map 只写了「≤1 MiB」。

## 诚实边界

- 声明类规则判定的是**声明**，不是被声明对象的真实行为：`archive_traversal` 不证明解压安全，`passive_scan_exec` 不证明 hook 未被别处执行。真实归档/执行安全合同独立实施（缺口分析 B16）。2026-09-12 起（C-F04）这类 finding 在输出中带 `declaration{kind: "declaration-validation", source_file, producer: "unknown", trusted_producer_connected: false, declared_at}`，显式表达「该声明未接通可信生产者」。
- `conflict` 只识别语料冻结的 `must be <manager>` 指令形式；其它形式的矛盾不在本规则内，不报 Unknown 也不报通过——它们不是本规则的输入。
- `stale` 对照显式 `as_of` 判定（C-F05）：crate 内不读时钟，corpus/测试注入固定 `STALE_CUTOFF`（2026-09-04）保持 golden 可复现，CLI 生产路径用系统当日或 `--as-of` 覆盖；finding 文案写出实际对照的日期。日期久只证明越过 365 天维护阈值，不是「文件内容错误」的语义判断。
- 所有 20 条规则的 finding 均为 `confirmation: confirmed`（对字节确定性判定）；Unknown 不是 severity（C03）。
- `doctor` 与 `ci` 的项目规则扫描读取项目内文件（经 collect 的排除清单：`.env`、密钥文件等不读），不读取 `$HOME`。
