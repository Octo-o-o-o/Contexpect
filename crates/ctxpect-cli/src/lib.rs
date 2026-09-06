//! `ctxpect` command-line entry for the Codex instructions inspect slice.
//!
//! The binary is `ctxpect`. This development slice implements `inspect` and
//! the flags named in the stage contract. Other CLI-reference commands and
//! flags fail closed with `error.code=usage.unimplemented`.
//!
//! # What the tests observe
//!
//! - The built `ctxpect` binary is started via `CARGO_BIN_EXE_ctxpect`.
//! - `--json` envelopes carry `schema=dev-inspect-v0` and
//!   `receipt_kind=development-snapshot`, with a `snapshot_digest` of the
//!   time-stripped canonical JSON.
//! - Exit 0 / 2 / 3 / 1 follow required present / absent / indeterminate /
//!   usage-or-IO.
//! - Unimplemented flags and commands (`--config`, `--privacy`,
//!   `--allow-unknown`, `--force`, `doctor`, `collect`, …) are refused.
//!   Nested CLI-reference forms (`receipt show`, `sync preview`) and listed
//!   command flags (`doctor --sarif`) fail with `usage.unimplemented`; tokens
//!   absent from the reference fail with `usage.invalid`.
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
//! - `--help`, `--config`, privacy modes, doctor, collect, or any WP-03+
//!   command as implemented behaviour (they are refused).
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
mod inspect;
mod jsonutil;
mod redact;

pub use args::{InspectArgs, UsageError, parse_args};
pub use ctxpect_schema::{Value, canonical_json, parse};
pub use inspect::{InspectFailure, InspectReport, error_envelope, error_human, inspect};
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
ctxpect — AI coding context 核对与控制工具（开发切片）

用法:
  ctxpect inspect [选项]
  ctxpect --help

本切片只实现 inspect。anchor 固定为 Codex CLI 0.147.0 / cli / macos-27-arm64，
capability 为 instructions。CLI 参考里的其余子命令与参数尚未实施，传入时会以
usage.unimplemented 或 usage.invalid 明确拒绝，不会被静默忽略。

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
    match parse_args(collected) {
        Err(error) => emit_error(&Failure::Usage(error), want_json, &argv_roots),
        Ok(parsed) => {
            let roots = RedactRoots::from_inspect_args(&parsed);
            match inspect(parsed) {
                Ok(report) => emit_ok(report, &roots),
                Err(failure) => emit_error(&failure, want_json, &roots),
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
