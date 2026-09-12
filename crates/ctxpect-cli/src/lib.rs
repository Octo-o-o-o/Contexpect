//! `ctxpect` command-line entry.
//!
//! The inspect slice (`dev-inspect-v0` / `development-snapshot`) is unchanged.
//! Additional commands persist formal `ctxpect-receipt-v1` documents via an
//! explicit migration. Unimplemented CLI-reference flags (`--config`,
//! `--privacy`, `--allow-unknown`, `--force`, `--sarif`) still fail closed.
//!
//! # What the tests observe
//!
//! - The built `ctxpect` binary is started via `CARGO_BIN_EXE_ctxpect`.
//! - `--json` envelopes carry `schema=dev-inspect-v0` and
//!   `receipt_kind=development-snapshot`, with a `snapshot_digest` of the
//!   time-stripped canonical JSON.
//! - Exit 0 / 2 / 3 / 1 follow required present / absent / indeterminate /
//!   usage-or-IO.
//! - Unimplemented flags (`--config`, `--privacy`, `--allow-unknown`,
//!   `--force`, `--sarif`) are refused. Listed command flags that remain
//!   unimplemented fail with `usage.unimplemented`; tokens absent from the
//!   reference fail with `usage.invalid`.
//! - G1–G5 grammar, HOME/CODEX_HOME sentinel non-reads, explanation fields,
//!   and corpus expected_output / expected_claim are asserted from the
//!   binary's JSON, not from a Python answer sheet.
//! - Explicit `--codex-home` does not surface sibling session/auth/config
//!   sentinels. Root/cwd IO failures emit structured `error.code` values
//!   (`io.missing`, `io.not_a_directory`, `io.escapes_root`,
//!   `io.cwd_outside_project`) whose messages use only placeholders
//!   (`<project>`, `<codex-home>`, `<cwd>`, `<outside>`) and an OS error
//!   class; `snapshot_digest` is taken from that envelope. G3 explanations
//!   use the truncated file's root prefix. Aggregate bodies at or below
//!   32768 bytes do not emit truncated-after; a zero-byte file after a full
//!   cap does not emit truncated-after.
//! - Global JSON layers use `<codex-home>/`; project layers keep
//!   `<project>/...`. Same-layer unknown `AGENTS.override.md` does not let
//!   `AGENTS.md` be adopted (exit 3, overridden-by).
//! - A `.ctxpect-ignore` hit is not read; JSON/human omit its body and
//!   SHA-256. Absolute, parent-segment, and control-character ignore lines
//!   are skipped; warnings record only the 1-based line number.
//! - Coordinate flags (`--version`, `--harness`, `--surface`, `--os-lane`,
//!   `--require`) are rejected at parse with `usage.invalid` when they do
//!   not match the closed grammar; the original value is not echoed.
//!   `--version` accepts dotted digits (optional pre-release suffix) or a
//!   closed identifier (`unknown-honesty` honesty cells). `--os-lane`
//!   hyphen groups allow `_` because frozen lanes use `x86_64`.
//! - Every stdout/stderr JSON and human byte is passed through
//!   `redact_output` before write. `snapshot_digest` is taken from that
//!   redacted JSON. Residual absolute-path matches become `<abs>` and add
//!   `redaction.residual_absolute_path`. `<project>/rel` and
//!   `<codex-home>/rel` are not residual: the slash after `>` is a
//!   placeholder suffix, not a host path.
//! - On success and error writes alike (JSON envelope and human text),
//!   declared project / codex-home root forms and `HOME` share one
//!   path-boundary rule: a match must sit at the start of the string or
//!   after `"`, `'`, whitespace, `(`, `=`, `:`, and must end at the end of
//!   the string or `/`, `"`, `'`, whitespace, `)`, `,`. Longer root forms
//!   are applied first. `HOME` still requires a `/`-prefixed value of
//!   length > 1. There is no earlier unbounded HOME or root-form substring
//!   rewrite on usage or IO messages: they reach the same `redact_output`
//!   boundary. `--codex-home /tmp` does not rewrite `<project>/target/tmp`;
//!   a project root form does not rewrite the same bytes as an ordinary
//!   path suffix; `HOME=/tmp` does not rewrite `<project>/target/tmp`;
//!   `HOME=instructions` does not rewrite `required` or `capability_id`;
//!   `HOME=project` does not rewrite `--project <dir>`; `HOME=inspect` does
//!   not rewrite the missing-command token `inspect`; `HOME=require` does
//!   not rewrite `--require`. A host root or HOME path that appears as
//!   such a prefix (success paths, error text, or leftover evidence) is
//!   still replaced. Placeholder tokens (`<project>`, `<codex-home>`,
//!   `<home>`, `<abs>`) are not scanned again.
//! - Symlink escape, dangling symlink, FIFO, and multiply-linked AGENTS.md
//!   yield required indeterminate (exit 3), not absent/present.
//!
//! # What they do not observe
//!
//! - Fallback filenames, Codex `config.toml`, or `project_doc_max_bytes`
//!   overrides.
//! - glob / negation `.ctxpect-ignore` syntax.
//! - Native oracle reconciliation against `codex debug prompt-input`.
//! - Other harness families as authoritative resolvers.
//! - Other capabilities (skills, plugins, MCP, commands, …).
//! - ubuntu / windows lanes or desktop / cloud surfaces as live scans; those
//!   coordinates emit honesty cells only.
//! - `--config`, `--privacy`, `--allow-unknown`, `--force`, and `--sarif`.
//! - Pretty-printed JSON; `--json` is canonical (sorted keys, no extra
//!   whitespace).
//! - Socket or device files as a separate CLI case; FIFO covers not-regular.
//! - A full-tree collect scan of the project; named instruction files are
//!   classified instead.
//! - Non-UTF-8 argv bytes as a redaction case; those tokens fail as invalid
//!   UTF-8 without echoing the bytes.
//! - Windows drive-letter residual matches as a live OS-lane run; the
//!   pattern is applied, but this slice is not executed on Windows.
//! - `HOME` values that are not `/`-prefixed Unix paths of length > 1;
//!   those are left unchanged rather than treated as host roots.
//! - Matching `HOME` or a root form inside an already-emitted placeholder
//!   token (`<project>`, `<codex-home>`, `<home>`, `<abs>`), or matching
//!   those forms as an ordinary path suffix without a declared boundary.

mod args;
mod catalog;
pub mod conformance;
mod dispatch;
mod http;
mod inspect;
mod jsonutil;
mod redact;
mod secure_sync;
mod effect_runner;
mod native_oracle;
mod tool_process;

pub use args::{Cli, InspectArgs, ProductArgs, UsageError, parse_args, parse_cli};
pub use ctxpect_schema::{Value, canonical_json, parse};
pub use ctxpect_store::Store;
pub use dispatch::{persist_inspect, project_doctor_findings, project_scope_digest, scan_project_for_doctor};
pub use inspect::{error_envelope, error_human, inspect, InspectFailure, InspectReport};
pub use jsonutil::{strip_time_fields, with_snapshot_digest};
pub use redact::{RedactOutcome, RedactRoots, redact_json_envelope, redact_output};

use inspect::InspectFailure as Failure;
use std::ffi::OsString;
use std::io::{self, Write as _};

/// Usage text for `--help` / `-h`.
///
/// This is a development slice: it names only what this binary actually
/// implements. Every other token from the CLI reference still fails closed in
/// `parse_args`; help does not widen the accepted grammar.
const USAGE: &str = "\
ctxpect — AI coding context 核对与控制工具

用法:
  ctxpect inspect [选项]
  ctxpect doctor --project <dir> [--store <dir>] [--fail-on confirmed] [--as-of YYYY-MM-DD]
  ctxpect collect --project <dir>
  ctxpect receipt show|verify|export|redact --store <dir> --receipt <id>
  ctxpect daemon start --project <dir> --store <dir> --listen 127.0.0.1:7420
  ctxpect --help

inspect 是两个 anchor 的 instructions 静态切片：Codex CLI 0.147.0 与 Claude Code CLI
2.1.259（均 cli / macos-27-arm64）；其余坐标与 capability 如实报 Unknown。
开发快照 schema=dev-inspect-v0 / receipt_kind=development-snapshot。正式 Receipt
必须经过 migrate_dev_inspect_v0，不能靠改名升级。

mutation 合同（详见 docs/guides/cli-reference.md「授权绑定」）:
  ctxpect intent preview --project <dir> --store <dir> --target <rel> --desired <text>
  ctxpect apply --tx <id> --project <dir> --store <dir>
  ctxpect exception request --action <mutation> --expires-in <秒> [--target <t>] [--reason <r>]

doctor 选项:
  --fail-on confirmed 确认级 finding 也置 exit 2
  --as-of YYYY-MM-DD  stale 规则与 suppression 过期判定对照的评估日期（默认系统当日；
                      指定后 suppression 以该日 UTC 正午为评估时刻）。非法日期拒绝（exit 1）。

inspect 选项:
  --project <dir>     要检查的项目根（必填）
  --cwd <rel>         项目内的工作目录，默认为项目根
  --codex-home <dir>  显式授权的 Codex home，用于读取全局 AGENTS.md。不传则按 G5
                      报 permission_not_granted；不会去读 $HOME 或 $CODEX_HOME
  --require <cap>     要求的 capability，可重复，默认 instructions
  --version <ver>     工具版本坐标，默认 0.147.0。这是 user-attested 坐标，
                      不是对本机已安装该版本的断言
  --harness <name>    harness 坐标，默认 codex
  --surface <name>    surface 坐标，默认 cli
  --os-lane <lane>    OS lane 坐标，默认 macos-27-arm64
  --store <dir>       可选：把快照迁移为正式 Receipt 写入本地 ledger
  --json              输出机器可读 JSON
  --offline           声明离线；本切片本来就不联网
  --help, -h          显示本帮助

exit code:
  0  required capability 全部 present
  1  错误：用法非法、IO 失败等
  2  required capability 确定为 absent
  3  required capability 不确定（indeterminate / unknown）

被动只读：不执行 harness、hook、MCP 或 plugin，不读取真实私人 home、session 或
凭据正文。model-visible、use-evidence、outcome-affecting 一律报 indeterminate，
需要 native runtime snapshot 才能确定。

例外生命周期身份:
  --principal <id>    仓内 .ctxpect/principals.json 里已登记的 principal。
                      密钥经环境变量 CTXPECT_PRINCIPAL_SECRET 传入，不走 argv。
                      --role / --actor 是调用方自报，不构成授权。

完整命令合同见 docs/guides/cli-reference.md
";

/// Parse argv, run inspect, write stdout/stderr, return the process exit code.
pub fn run<I, S>(args: I) -> i32
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let collected: Vec<OsString> = args.into_iter().map(Into::into).collect();
    if collected
        .iter()
        .skip(1)
        .any(|item| item == "--help" || item == "-h")
    {
        print!("{USAGE}");
        return 0;
    }
    let want_json = collected.iter().any(|item| item == "--json");
    let argv_roots = RedactRoots::from_argv(&collected);
    match parse_cli(collected) {
        Err(error) => emit_error(&Failure::Usage(error), want_json, &argv_roots),
        Ok(Cli::Inspect(parsed)) => {
            let roots = RedactRoots::from_inspect_args(&parsed);
            let store_path = parsed.store.clone();
            let project_path = parsed.project.clone();
            match inspect(parsed) {
                Ok(mut report) => {
                    if let Some(path) = store_path {
                        // `--store` asks for a formal Receipt. A persistence
                        // failure is reported in the envelope and as exit 1
                        // (IO error class), never swallowed into a clean
                        // snapshot with no Receipt.
                        let persisted = ctxpect_store::Store::open(&path)
                            .map_err(|err| (err.code, err.message))
                            .and_then(|store| {
                                dispatch::persist_inspect_in(&store, &report.envelope, "one-shot", Some(project_path.as_path()))
                                    .map_err(|err| (err.code(), err.message()))
                            });
                        if let ctxpect_schema::Value::Object(map) = &mut report.envelope {
                            match persisted {
                                Ok(receipt) => {
                                    map.insert(
                                        "formal_receipt_id".into(),
                                        ctxpect_schema::string(
                                            receipt
                                                .get("receipt_id")
                                                .and_then(ctxpect_schema::Value::as_str)
                                                .unwrap_or(""),
                                        ),
                                    );
                                    map.insert("persist_error".into(), ctxpect_schema::Value::Null);
                                }
                                Err((code, message)) => {
                                    map.insert("formal_receipt_id".into(), ctxpect_schema::Value::Null);
                                    map.insert(
                                        "persist_error".into(),
                                        ctxpect_schema::object([
                                            ("code", ctxpect_schema::string(code)),
                                            ("message", ctxpect_schema::string(message)),
                                        ]),
                                    );
                                    report.exit_code = 1;
                                }
                            }
                        }
                    }
                    emit_ok(report, &roots)
                }
                Err(failure) => emit_error(&failure, want_json, &roots),
            }
        }
        Ok(Cli::Product(args)) => {
            let roots = dispatch::redact_roots_from_product(&args);
            let json = args.json;
            match dispatch::run_product(*args) {
                Ok(report) => {
                    let envelope = redact_json_envelope(report.envelope, &roots);
                    let _ = writeln!(io::stdout().lock(), "{}", canonical_json(&envelope));
                    report.exit_code
                }
                Err(failure) => emit_error(&failure, json, &roots),
            }
        }
    }
}

fn emit_ok(report: InspectReport, roots: &RedactRoots) -> i32 {
    let mut stdout = io::stdout().lock();
    if report.json {
        let envelope = redact_json_envelope(report.envelope, roots);
        let _ = writeln!(stdout, "{}", canonical_json(&envelope));
    } else {
        let outcome = redact_output(&report.human, roots);
        let mut text = outcome.text;
        if outcome.residual && !text.contains("redaction.residual_absolute_path") {
            if !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str("\nWarnings\n- redaction.residual_absolute_path\n");
        }
        let _ = write!(stdout, "{text}");
        if !text.ends_with('\n') {
            let _ = writeln!(stdout);
        }
    }
    report.exit_code
}

fn emit_error(failure: &Failure, want_json: bool, roots: &RedactRoots) -> i32 {
    if want_json {
        let envelope = redact_json_envelope(error_envelope(failure), roots);
        let _ = writeln!(io::stdout().lock(), "{}", canonical_json(&envelope));
    } else {
        let outcome = redact_output(&error_human(failure), roots);
        eprint!("{}", outcome.text);
    }
    1
}
