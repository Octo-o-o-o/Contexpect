# Doctor 规则映射表

> 状态：规范（Doctor 项目内容规则已实施；规则语义以本表为准，度量以 `doctor-corpus` 门禁输出为准）。
> 目的：把冻结 Doctor 语料（`acceptance/corpus/development/doctor/doctor-corpus.jsonl`，589 行，20 个 `rule_id`）的每条规则映射到**语义、输入合同、实现位置、阻断性**，并说明它与既有 `D-*` 规则的关系。规则实现在 `crates/ctxpect-doctor/src/rules.rs`；精确率/召回率由 `cargo test -p ctxpect-cli --test doctor_corpus` 逐规则打印并断言。

## 命名空间与别名

- **语料命名空间**（`secret_literal` 等 20 条）是 PRD §17.1 与语料共用的规则 id，本表逐条实现，作为**新规则**进入产品输出（`rule_namespace: "doctor-corpus"`）。
- **既有 `D-*` 规则**（`D-UNKNOWN-SURFACE`、`D-FACET-*`、`D-TRUNCATED`、`D-IGNORE-G4`、`D-HOME-NOT-GRANTED`、`D-UNSUPPORTED-VERSION`）来自 inspect Receipt，**不重命名**：finding id 已随 Receipt 发布，改名会改变已发布 id。
- 语义相同者只做别名：`D-TRUNCATED`（G3 聚合截断）与 `cap_truncation`（声明的 cap 截断）语义相同，`D-TRUNCATED` finding 带 `aliases: ["cap_truncation"]`。`D-UNSUPPORTED-VERSION` 是 harness 坐标版本，`version_incompatible` 是 adapter 声明版本，不是同一语义，不做别名。其余 `D-*` 无对应语料规则。

## 阻断判定（`ci` 与 `doctor` 共用）

唯一判定函数：`ctxpect_doctor::blocking_exit(diagnosis, fail_on)`。

- finding 带 `blocking: true` → exit 2。只有 `BLOCKING_RULES` 里的规则产生 `blocking: true`。
- `--fail-on confirmed` 且 `counts.confirmed > 0` → exit 2。
- 其余 → 0。`ci` 再与 inspect 出码、policy 出码合并（任一 3 → 3；否则任一 2 → 2；否则 0）。

**阻断资格 = 语料精确率 1.00。** `doctor-corpus` 门禁对 `BLOCKING_RULES` 逐条断言 precision == 1.00；任一规则出现假阳性即门禁失败，修法只能是把该规则移入 `NON_BLOCKING_RULES` 并在本表改「阻断」为「否」。不能反过来修改语料答案（红线 3）。

## 输入合同

规则输入是 `ScannedFile` 列表：由被动采集器 `ctxpect_collect::scan` 列出、经 `read_contained` 读取（≤ 1 MiB、非 withheld、非多硬链接）的常规文件，以及符号链接的原样 target。**不执行、不解压、不联网。**

「惰性声明」类规则读取项目内的声明文件（`layout.json`、`inventory.json`、`plan.json`、`archive-manifest.json`、`hooks.json`、`budget.json`、`device-lock.json`、`provenance.json`、`adapter-version.json`、`placement.json`）。一份*声明*了 `../etc/passwd` 条目的 manifest 就是遍历声明，不需要打开任何压缩包才能判定——这与 B16「真实解压安全」是两件事（缺口分析 §5）。

## 规则表

「实现」列：`rules.rs` = 已实现于 `crates/ctxpect-doctor/src/rules.rs`；来源列区分 **内容**（文件字节）、**采集**（collect 清单，如符号链接/摘要）、**声明**（上述 JSON 声明文件）。语料列 = 正例数 / 反例数（clean-lookalike）。

| rule_id | 语义 | 来源 | 输入合同（触发条件） | 反例边界 | 阻断 | 实现 | 语料 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `secret_literal` | 项目文本含凭据**形状** | 内容 | `ctxpect_doctor::secret_literal`：PEM 单行 armor（`-----BEGIN … PRIVATE KEY-----` 在同一行）、`AKIA` + 16 大写/数字、`aws_secret_access_key` 赋值（`=`/`:`，键与值可带引号，值 ≥16 凭据字节 `[A-Za-z0-9_/+=.-]`）、`ghp_/gho_/ghs_/github_pat_` + ≥20、`xox[abprs]-` + ≥10、`sk-` + ≥20 且**含数字与字母**、`Authorization: Bearer`（header 行或 JSON 键值，token ≥16 凭据字节） | 键名（`API_KEY=placeholder`）、裸前缀、占位形状（`<your-key>`、`${ENV}`、`{{tpl}}`、`****`、含 your/placeholder/example/redacted/changeme/xxxx 的值）、`sk-learn-…` 之类无数字的词串、只在散文里提到 `PRIVATE KEY-----` 的行都不算；`password=…`、数据库 URL 口令、裸 hex token 不在文法内 | 是 | `rules.rs` / `secrets.rs` | 20 / 20 |
| `hidden_unicode` | 零宽/不可见/双向控制字符隐藏文本 | 内容 | U+200B、U+2060、U+FEFF（非首字节 BOM）、U+00AD、U+034F、U+061C、U+180E、U+2061–2064、U+202A–E、U+2066–9、U+206A–F、tag 块 U+E0000–E007F；U+200C/U+200D（ZWNJ/ZWJ）只在**前后都是 ASCII** 时算隐藏 | 首字节 BOM 不算；emoji ZWJ 序列（👩‍💻）、波斯语 ZWNJ、天城文连字里的 joiner 不算 | 是 | `rules.rs` | 20 / 20 |
| `path_containment_escape` | 指令 `read:` 路径逃出根 | 内容 | 行首 `read: <path>`，path 绝对或按所在目录词法解析后越过根 | `read: ./inside/key` 不算 | 是 | `rules.rs` | 20 / 20 |
| `symlink_escape` | 符号链接目标离开工作区 | 采集 + 声明 | (a) 采集到的 symlink，target 词法逃出根；(b) `layout.json.symlinks[].to` 同判 | 指向根内文件不算 | 是 | `rules.rs` | 20 / 20 |
| `required_asset_missing` | 声明必需的资产缺失 | 声明 | `inventory.json.required` 中既不在 `present` 也不在磁盘的项 | 在 present 或磁盘上存在不算 | 是 | `rules.rs` | 20 / 20 |
| `unapproved_lossy_projection` | 有损投影未获批准 | 声明 | `plan.json.drops` 非空且 `approved != true`（缺失即未批准） | `drops: []` 或 `approved: true` | 是 | `rules.rs` | 20 / 20 |
| `archive_traversal` | 归档条目逃出目的地 | 声明 | `archive-manifest.json.entries[]` 含 `..` 段或绝对路径 | 普通相对文件名 | 是 | `rules.rs` | 20 / 20 |
| `passive_scan_exec` | 被动扫描被声明执行 hook | 声明 | `hooks.json.on_scan` 含非空命令 | `on_scan: []` | 是 | `rules.rs` | 20 / 20 |
| `duplicate` | 指令正文字节相同 | 采集（摘要） | ≥2 个 `.md` 内容摘要相同；报告组内排序最后的路径 | 正文不同 | 否 | `rules.rs` | 12 / 4 |
| `conflict` | 指令文件间包管理器指令冲突 | 内容 | `.md` 中 `must be (npm\|pnpm\|yarn\|bun)` 指令出现 ≥2 种；报告首个与基线不同的文件 | 同一种管理器 | 否 | `rules.rs` | 8 / 4 |
| `stale` | frontmatter `updated:` 过旧 | 内容 | 日期比 acceptance cutoff（2026-09-04）早 365 天以上 | 一年内 | 否 | `rules.rs` | 8 / 4 |
| `oversized_resident` | 常驻资产超过声明 cap | 声明 | `budget.json`：实际字节 > `max_bytes` 且未声明 `truncated` | 未超 | 否 | `rules.rs` | 8 / 4 |
| `cap_truncation` | 指令被 cap 截断 | 声明 | `budget.json`：实际字节 > `max_bytes` 且 `truncated: true`，或 `truncated: true` | `truncated: false` 且未超 | 否 | `rules.rs`（别名 `D-TRUNCATED`） | 8 / 4 |
| `bad_frontmatter` | frontmatter 不是合法 key/value YAML | 内容 | `---` 块未闭合、行非 `key: value`、`[`/`{`/引号未闭合 | 合法块 | 否 | `rules.rs` | 8 / 4 |
| `gitignore_mismatch` | 被忽略的路径仍存在且被指令引用 | 内容 | `.ctxpect-gitignore` / `.gitignore` 条目：文件存在且任一 `.md` 引用其名 | 条目不存在为文件 | 否 | `rules.rs` | 8 / 4 |
| `single_device_only` | 资产锁定单设备且不同步 | 声明 | `device-lock.json.sync == false` 且 `device_id` 非空、非 `*` | `device_id: "*"` 或 `sync: true` | 否 | `rules.rs` | 8 / 4 |
| `unknown_source` | 导入资产无来源 | 声明 | `provenance.json.source` 缺失/空；路径取 `path` 或 `imported.md` | 有来源 | 否 | `rules.rs` | 8 / 4 |
| `version_incompatible` | adapter 声明版本不匹配 | 声明 | `adapter-version.json.required != actual` | 相等 | 否 | `rules.rs` | 8 / 4 |
| `undiscoverable_path` | 声明的指令路径不可发现 | 声明 | `layout.json.files[].path` 以 `..` 或 `/` 开头 | 相对根内路径 | 否 | `rules.rs` | 8 / 4 |
| `placement_recommendation` | 资产不在推荐路径 | 声明 | `placement.json.path != recommended` 且 `path` 存在 | 相等或不存在 | 否 | `rules.rs` | 8 / 4 |

20 条规则全部有实现；`doctor-corpus` 门禁逐条 precision 1.00 / recall 1.00，125 条 clean 无 finding，589 行 `expected_exit` 与 `blocking_exit` 全部一致。

## 度量与语料的诚实边界（2026-09-09 交叉 review）

- **语料信息量**：按 `input_path` 下文件内容去重后，20 条规则中 19 条各只有 **1 个独立正例 + 1 个独立反例**（其余 19/7 条是逐字复制，只有 `envelope.json` 不同），`secret_literal` 的 20 个正例只差末尾两位数字，125 条 clean 是同一内容。「precision/recall 1.00」在计数上满足 PRD §17.1 的 ≥20，在信息量上等于每条规则两个布尔断言。它是回归护栏，不是对文法泛化能力的度量；语料外的正反例由 `secrets.rs` / `rules.rs` 单测覆盖（占位符、引号写法、ZWJ emoji、Latin-1 字节等），冻结答案不改。
- **`expected_exit` 的层次**：语料的 `expected_exit` 与 `ctxpect_doctor::blocking_exit(diagnosis, fail_on)` 函数逐行一致；产品命令 `doctor` / `ci` 的最终 exit 还**合并 inspect 出码**——40 条非阻断正例的输入目录没有 `AGENTS.md`，inspect 的 `policy_result.verdict = fail` 使 `ctxpect doctor` 对它们 exit 2。这不是规则误报，是同一命令承载两个判定；接手者不要把「非阻断规则永不 exit 2」读成产品命令级的保证。
- **声明文件是 Contexpect 自有约定**：`layout.json`、`inventory.json`、`plan.json`、`archive-manifest.json`、`hooks.json`、`budget.json`、`device-lock.json`、`provenance.json`、`adapter-version.json`、`placement.json` 目前**没有任何写入者**（只有语料生成器写它们；产品侧只有 Claude Code resolver 读 `budget.json`），也没有命名空间或 `schema` 字段保护。真实项目里同名文件会按这套约定被判读（例如根目录任意 `plan.json` 带 `drops` 且缺 `approved` 即阻断）；Cursor 真实 hook 位置 `.cursor/hooks.json` 不在范围。给声明加命名空间会改变冻结语料的答案，留待 contract revision。
- **`symlink_escape` 的绝对路径**：`ScannedFile` 不带根的绝对路径，绝对 target 一律判逃逸，`ln -s "$(pwd)/x" y` 这类指向根内的绝对链接是**已知假阳性**（阻断规则）；`~`、Windows 盘符 target 不判。
- **对本仓库自身**：`ctxpect doctor --project <本仓库>` exit 2——语料与测试里的 fixture token、文档里的 `Authorization: Bearer` 示例按「fixture token fail closed」策略命中。当前没有带理由/期限的 suppress 机制，因此本仓库不能把 `ctxpect ci` 加进自己的门禁；这是产品缺口，不是门禁绿的反例。
- **文本解码**：不是合法 UTF-8 的字节被替换后仍参与内容规则（`String::from_utf8_lossy`），与 projection 的 secret 门一致；此前单个 Latin-1 字节会让整个文件跳过全部内容规则而无任何 finding。>1 MiB 与多硬链接的文件仍按 collect 规则不读，且**不报**——rule-map 只写了「≤1 MiB」。

## 诚实边界

- 声明类规则判定的是**声明**，不是被声明对象的真实行为：`archive_traversal` 不证明解压安全，`passive_scan_exec` 不证明 hook 未被别处执行。真实归档/执行安全合同独立实施（缺口分析 B16）。
- `conflict` 只识别语料冻结的 `must be <manager>` 指令形式；其它形式的矛盾不在本规则内，不报 Unknown 也不报通过——它们不是本规则的输入。
- `stale` 以 cutoff 为参照而非当前时钟，因此结果可复现；它不是「文件内容过时」的语义判断。
- 所有 20 条规则的 finding 均为 `confirmation: confirmed`（对字节确定性判定）；Unknown 不是 severity（C03）。
- `doctor` 与 `ci` 的项目规则扫描读取项目内文件（经 collect 的排除清单：`.env`、密钥文件等不读），不读取 `$HOME`。
