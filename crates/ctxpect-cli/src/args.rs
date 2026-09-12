//! Hand-written argv parser. No clap; unimplemented CLI-reference flags fail closed.

use std::ffi::OsString;
use std::path::PathBuf;

use ctxpect_resolve::{DEFAULT_ANCHOR, INSTRUCTIONS};

/// Closed lists from `docs/guides/cli-reference.md` (global box, command tree,
/// and the command-specific flags named there). Tokens on these lists that this
/// slice does not implement fail with `usage.unimplemented`. Tokens that are
/// not on the lists fail with `usage.invalid`.
const IMPLEMENTED_COMMAND: &str = "inspect";

const IMPLEMENTED_COMMANDS: &[&str] = &[
    "inspect", "collect", "doctor", "diff", "receipt", "inventory", "preflight", "launch",
    "import", "sessions", "daemon", "sync", "intent", "apply", "rollback", "standard",
    "exception", "assets", "advisor", "experiment", "policy", "align", "adapter", "ci",
    "store",
];

const LISTED_GLOBAL_FLAGS: &[&str] = &[
    "json", "offline", "config", "project", "cwd", "harness", "surface", "version", "privacy",
];

/// Global flags named in the CLI-reference prose but not in the top box.
const LISTED_GLOBAL_FLAGS_PROSE: &[&str] = &["allow-unknown", "force"];

/// Command-specific flags named in the CLI-reference command tree (`doctor`).
const LISTED_COMMAND_FLAGS: &[&str] = &["sarif"];

/// Slice-only inspect flags; not a reason to treat other unknown tokens as listed.
const SLICE_INSPECT_FLAGS: &[&str] = &["codex-home", "require", "os-lane"];

const LISTED_COMMANDS: &[&str] = &[
    "inspect",
    "collect",
    "doctor",
    "diff",
    "receipt",
    "inventory",
    "preflight",
    "launch",
    "import",
    "sessions",
    "daemon",
    "sync",
    "intent",
    "apply",
    "rollback",
    "standard",
    "exception",
    "assets",
    "advisor",
    "experiment",
    "policy",
    "align",
    "adapter",
    "ci",
    "store",
];

/// Flags the command tree lists but this slice does not act on. They are
/// refused (`usage.unimplemented`) rather than parsed and ignored: a flag
/// that is accepted and does nothing reads as a feature that exists.
/// `--locale` / `--kind` were once parsed into fields no code read.
const UNIMPLEMENTED_FLAGS: &[&str] = &["config", "privacy", "allow-unknown", "force", "locale", "kind"];

const UNIMPLEMENTED_COMMANDS: &[&str] = &[];

#[derive(Debug, Clone)]
pub struct InspectArgs {
    pub json: bool,
    pub offline: bool,
    pub project: PathBuf,
    pub cwd: Option<PathBuf>,
    pub harness: String,
    pub surface: String,
    pub version: String,
    pub version_explicit: bool,
    pub codex_home: Option<PathBuf>,
    pub require: Vec<String>,
    pub os_lane: String,
    pub store: Option<PathBuf>,
}

/// Parsed CLI after inspect-or-product dispatch.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Cli {
    Inspect(InspectArgs),
    Product(Box<ProductArgs>),
}

#[derive(Debug, Clone)]
pub struct ProductArgs {
    pub command: String,
    pub subcommand: Option<String>,
    pub json: bool,
    pub offline: bool,
    pub project: Option<PathBuf>,
    pub cwd: Option<PathBuf>,
    pub harness: String,
    pub surface: String,
    pub version: String,
    pub version_explicit: bool,
    pub codex_home: Option<PathBuf>,
    pub require: Vec<String>,
    pub os_lane: String,
    pub store: Option<PathBuf>,
    pub listen: Option<String>,
    pub ui_root: Option<PathBuf>,
    pub task: Option<String>,
    pub files: Vec<String>,
    pub receipt: Option<String>,
    pub left: Option<String>,
    pub right: Option<String>,
    pub preflight_id: Option<String>,
    pub from: Option<PathBuf>,
    pub fail_on: Option<String>,
    /// Explicit `stale`-rule reference date for `doctor` / `ci`
    /// (`--as-of YYYY-MM-DD`, parsed and validated at parse time); also the
    /// clock suppression expiry is judged against for that run.
    pub as_of: Option<(i64, u32, u32)>,
    pub home: Option<PathBuf>,
    pub id: Option<String>,
    pub reason: Option<String>,
    pub actor: Option<String>,
    pub role: Option<String>,
    /// Enrolled principal id. Identity is the id *plus* the enrolled secret,
    /// which arrives through the environment and never through argv.
    pub principal: Option<String>,
    pub dest: Option<PathBuf>,
    pub mapping: Option<String>,
    pub session: Option<String>,
    pub target: Option<String>,
    pub desired: Option<String>,
    pub authority: Option<String>,
    pub export: Option<PathBuf>,
    pub profile: Option<String>,
    pub adapter: Option<String>,
    pub n: Option<i64>,
    pub execute: bool,
    pub oneshot: bool,
    pub text: Option<String>,
    /// Mutation action an exception is requested for / a policy query is about.
    pub action: Option<String>,
    /// Exception lifetime in seconds; required by `exception request`.
    pub expires_in: Option<i64>,
    /// A persisted preview transaction id; required by `apply`.
    pub tx: Option<String>,
    /// A `ctxpect-effect-runs-v1` document for `experiment`.
    pub runs: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct UsageError {
    pub code: &'static str,
    pub message: String,
    pub command: Option<String>,
}

pub fn parse_cli<I, S>(args: I) -> Result<Cli, UsageError>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut tokens = Vec::new();
    for (index, raw) in args.into_iter().enumerate() {
        let os = raw.into();
        let Some(text) = os.to_str() else {
            return Err(invalid(
                format!("argument {index} is not valid UTF-8"),
                None,
            ));
        };
        if index == 0 {
            continue;
        }
        tokens.push(text.to_string());
    }

    let mut json = false;
    let mut offline = false;
    let mut project = None;
    let mut cwd = None;
    let mut harness = None;
    let mut surface = None;
    let mut version = None;
    let mut version_explicit = false;
    let mut codex_home = None;
    let mut require: Vec<String> = Vec::new();
    let mut os_lane = None;
    let mut store = None;
    let mut listen = None;
    let mut ui_root = None;
    let mut task = None;
    let mut files: Vec<String> = Vec::new();
    let mut receipt = None;
    let mut left = None;
    let mut right = None;
    let mut preflight_id = None;
    let mut from = None;
    let mut fail_on = None;
    let mut as_of = None;
    let mut home = None;
    let mut id = None;
    let mut reason = None;
    let mut actor = None;
    let mut role = None;
    let mut principal = None;
    let mut dest = None;
    let mut mapping = None;
    let mut session = None;
    let mut target = None;
    let mut desired = None;
    let mut authority = None;
    let mut export = None;
    let mut profile = None;
    let mut adapter = None;
    let mut n = None;
    let mut execute = false;
    let mut oneshot = false;
    let mut text = None;
    let mut action = None;
    let mut expires_in = None;
    let mut tx = None;
    let mut runs = None;
    let mut command = None;
    let mut subcommand = None;

    let mut iter = tokens.into_iter().peekable();
    while let Some(token) = iter.next() {
        if token == "--" {
            return Err(invalid(
                "`--` is not used in this development slice".to_string(),
                command.clone(),
            ));
        }
        if let Some(flag) = token.strip_prefix("--") {
            let (name, inline) = match flag.split_once('=') {
                Some((name, value)) => (name, Some(value.to_string())),
                None => (flag, None),
            };
            if is_listed_unimplemented_flag(name) {
                consume_optional_value(inline, &mut iter);
                return Err(unimplemented(name, command.clone()));
            }
            match name {
                "json" => {
                    reject_inline(name, inline, command.as_deref())?;
                    json = true;
                }
                "offline" => {
                    reject_inline(name, inline, command.as_deref())?;
                    offline = true;
                }
                "project" => {
                    project = Some(need_value(
                        "project",
                        inline,
                        &mut iter,
                        command.as_deref(),
                    )?)
                }
                "cwd" => cwd = Some(need_value("cwd", inline, &mut iter, command.as_deref())?),
                "harness" => {
                    let value = need_value("harness", inline, &mut iter, command.as_deref())?;
                    validate_ident("harness", &value, command.as_deref())?;
                    harness = Some(value);
                }
                "surface" => {
                    let value = need_value("surface", inline, &mut iter, command.as_deref())?;
                    validate_ident("surface", &value, command.as_deref())?;
                    surface = Some(value);
                }
                "version" => {
                    let value = need_value("version", inline, &mut iter, command.as_deref())?;
                    validate_version(&value, command.as_deref())?;
                    version = Some(value);
                    version_explicit = true;
                }
                "codex-home" => {
                    codex_home = Some(need_value(
                        "codex-home",
                        inline,
                        &mut iter,
                        command.as_deref(),
                    )?)
                }
                "require" => {
                    let value = need_value("require", inline, &mut iter, command.as_deref())?;
                    validate_ident("require", &value, command.as_deref())?;
                    require.push(value);
                }
                "os-lane" => {
                    let value = need_value("os-lane", inline, &mut iter, command.as_deref())?;
                    validate_os_lane(&value, command.as_deref())?;
                    os_lane = Some(value);
                }
                "store" => {
                    store = Some(need_value("store", inline, &mut iter, command.as_deref())?);
                }
                "listen" => {
                    listen = Some(need_value("listen", inline, &mut iter, command.as_deref())?);
                }
                "ui-root" => {
                    ui_root = Some(need_value("ui-root", inline, &mut iter, command.as_deref())?);
                }
                "task" => {
                    task = Some(need_value("task", inline, &mut iter, command.as_deref())?);
                }
                "files" => {
                    files.push(need_value("files", inline, &mut iter, command.as_deref())?);
                }
                "receipt" => {
                    receipt = Some(need_value("receipt", inline, &mut iter, command.as_deref())?);
                }
                "a" => {
                    left = Some(need_value("a", inline, &mut iter, command.as_deref())?);
                }
                "b" => {
                    right = Some(need_value("b", inline, &mut iter, command.as_deref())?);
                }
                "preflight-id" => {
                    preflight_id = Some(need_value(
                        "preflight-id",
                        inline,
                        &mut iter,
                        command.as_deref(),
                    )?);
                }
                "from" => {
                    from = Some(need_value("from", inline, &mut iter, command.as_deref())?);
                }
                "fail-on" => {
                    let value = need_value("fail-on", inline, &mut iter, command.as_deref())?;
                    // The only threshold the Doctor implements. A misspelling
                    // must not read as "no threshold" and turn a gate green.
                    if value != "confirmed" {
                        return Err(invalid(
                            format!("`--fail-on` accepts only `confirmed`, not `{value}`"),
                            command.clone(),
                        ));
                    }
                    fail_on = Some(value);
                }
                "as-of" => {
                    let value = need_value("as-of", inline, &mut iter, command.as_deref())?;
                    // A date that does not exist must fail closed; silently
                    // clamping it would judge `stale` against a date the user
                    // never asked for. The invalid value is not echoed.
                    as_of = Some(ctxpect_doctor::parse_civil_date(&value).ok_or_else(|| {
                        invalid(
                            "`--as-of` must be a calendar date `YYYY-MM-DD`".to_string(),
                            command.clone(),
                        )
                    })?);
                }
                "home" => {
                    home = Some(need_value("home", inline, &mut iter, command.as_deref())?);
                }
                "id" => {
                    id = Some(need_value("id", inline, &mut iter, command.as_deref())?);
                }
                "reason" => {
                    reason = Some(need_value("reason", inline, &mut iter, command.as_deref())?);
                }
                "actor" => {
                    actor = Some(need_value("actor", inline, &mut iter, command.as_deref())?);
                }
                "role" => {
                    role = Some(need_value("role", inline, &mut iter, command.as_deref())?);
                }
                "principal" => {
                    principal =
                        Some(need_value("principal", inline, &mut iter, command.as_deref())?);
                }
                "dest" => {
                    dest = Some(need_value("dest", inline, &mut iter, command.as_deref())?);
                }
                "mapping" => {
                    mapping = Some(need_value("mapping", inline, &mut iter, command.as_deref())?);
                }
                "session" => {
                    session = Some(need_value("session", inline, &mut iter, command.as_deref())?);
                }
                "target" => {
                    target = Some(need_value("target", inline, &mut iter, command.as_deref())?);
                }
                "desired" => {
                    desired = Some(need_value("desired", inline, &mut iter, command.as_deref())?);
                }
                "authority" => {
                    authority = Some(need_value("authority", inline, &mut iter, command.as_deref())?);
                }
                "export" => {
                    export = Some(need_value("export", inline, &mut iter, command.as_deref())?);
                }
                "profile" => {
                    profile = Some(need_value("profile", inline, &mut iter, command.as_deref())?);
                }
                "adapter" => {
                    adapter = Some(need_value("adapter", inline, &mut iter, command.as_deref())?);
                }
                "n" => {
                    let value = need_value("n", inline, &mut iter, command.as_deref())?;
                    n = Some(value.parse::<i64>().map_err(|_| {
                        invalid("`--n` must be an integer".to_string(), command.clone())
                    })?);
                }
                "text" => {
                    text = Some(need_value("text", inline, &mut iter, command.as_deref())?);
                }
                "action" => {
                    let value = need_value("action", inline, &mut iter, command.as_deref())?;
                    validate_action(&value, command.as_deref())?;
                    action = Some(value);
                }
                "expires-in" => {
                    let value = need_value("expires-in", inline, &mut iter, command.as_deref())?;
                    let parsed = value.parse::<i64>().ok().filter(|n| *n >= 1);
                    expires_in = Some(parsed.ok_or_else(|| {
                        invalid(
                            "`--expires-in` must be a positive number of seconds".to_string(),
                            command.clone(),
                        )
                    })?);
                }
                "tx" => {
                    tx = Some(need_value("tx", inline, &mut iter, command.as_deref())?);
                }
                "runs" => {
                    runs = Some(need_value("runs", inline, &mut iter, command.as_deref())?);
                }
                "execute" => {
                    reject_inline(name, inline, command.as_deref())?;
                    execute = true;
                }
                "oneshot" => {
                    reject_inline(name, inline, command.as_deref())?;
                    oneshot = true;
                }
                other => {
                    if is_listed_unimplemented_flag(other) || listed_but_unimplemented_global(other)
                    {
                        return Err(unimplemented(other, command.clone()));
                    }
                    return Err(invalid(
                        format!("unknown flag --{}", display_token(other)),
                        command.clone(),
                    ));
                }
            }
            continue;
        }
        if command.is_none() {
            command = Some(token);
            continue;
        }
        if let Some(cmd) = command.as_deref()
            && listed_subcommands(cmd).contains(&token.as_str())
        {
            if subcommand.is_some() {
                return Err(invalid(
                    format!("unexpected argument `{}`", display_token(&token)),
                    command.clone(),
                ));
            }
            subcommand = Some(token);
            continue;
        }
        return Err(invalid(
            format!("unexpected argument `{}`", display_token(&token)),
            command.clone(),
        ));
    }

    let command = match command {
        Some(name) => name,
        None => {
            return Err(invalid(
                "missing command; this slice implements `inspect`".to_string(),
                None,
            ));
        }
    };
    if command == IMPLEMENTED_COMMAND {
        if subcommand.is_some() {
            return Err(invalid(
                "`inspect` does not take a subcommand".to_string(),
                Some(command),
            ));
        }
        let project = match project {
            Some(path) if !path.is_empty() => PathBuf::from(path),
            _ => {
                return Err(invalid(
                    "`inspect` requires `--project <dir>`".to_string(),
                    Some(command),
                ));
            }
        };

        let mut required = Vec::new();
        for item in require {
            if !required.iter().any(|have| have == &item) {
                required.push(item);
            }
        }
        if required.is_empty() {
            required.push(INSTRUCTIONS.to_string());
        }

        return Ok(Cli::Inspect(InspectArgs {
            json,
            offline,
            project,
            cwd: cwd.map(PathBuf::from),
            harness: harness.unwrap_or_else(|| DEFAULT_ANCHOR.harness.to_string()),
            surface: surface.unwrap_or_else(|| DEFAULT_ANCHOR.surface.to_string()),
            version: version.unwrap_or_else(|| DEFAULT_ANCHOR.version.to_string()),
            version_explicit,
            codex_home: codex_home.map(PathBuf::from),
            require: required,
            os_lane: os_lane.unwrap_or_else(|| DEFAULT_ANCHOR.os_lane.to_string()),
            store: store.map(PathBuf::from),
        }));
    }

    if IMPLEMENTED_COMMANDS.contains(&command.as_str()) {
        if listed_subcommands(&command).is_empty() && subcommand.is_some() {
            return Err(invalid(
                format!("`{command}` does not take a subcommand"),
                Some(command),
            ));
        }
        if !listed_subcommands(&command).is_empty()
            && subcommand.is_none()
            && !subcommand_optional(&command)
        {
            return Err(invalid(
                format!(
                    "`{command}` requires a subcommand ({})",
                    listed_subcommands(&command).join("|")
                ),
                Some(command),
            ));
        }
        let mut required = Vec::new();
        for item in require {
            if !required.iter().any(|have| have == &item) {
                required.push(item);
            }
        }
        return Ok(Cli::Product(Box::new(ProductArgs {
            command,
            subcommand,
            json,
            offline,
            project: project.filter(|p| !p.is_empty()).map(PathBuf::from),
            cwd: cwd.map(PathBuf::from),
            harness: harness.unwrap_or_else(|| DEFAULT_ANCHOR.harness.to_string()),
            surface: surface.unwrap_or_else(|| DEFAULT_ANCHOR.surface.to_string()),
            version: version.unwrap_or_else(|| DEFAULT_ANCHOR.version.to_string()),
            version_explicit,
            codex_home: codex_home.map(PathBuf::from),
            require: required,
            os_lane: os_lane.unwrap_or_else(|| DEFAULT_ANCHOR.os_lane.to_string()),
            store: store.map(PathBuf::from),
            listen,
            ui_root: ui_root.map(PathBuf::from),
            task,
            files,
            receipt,
            left,
            right,
            preflight_id,
            from: from.map(PathBuf::from),
            fail_on,
            as_of,
            home: home.map(PathBuf::from),
            id,
            reason,
            actor,
            role,
            principal,
            dest: dest.map(PathBuf::from),
            mapping,
            session,
            target,
            desired,
            authority,
            export: export.map(PathBuf::from),
            profile,
            adapter,
            n,
            execute,
            oneshot,
            text,
            action,
            expires_in,
            tx,
            runs: runs.map(PathBuf::from),
        })));
    }

    if LISTED_COMMANDS.contains(&command.as_str()) {
        return Err(unimplemented(&command, Some(command.clone())));
    }
    Err(invalid(
        format!("unknown command `{}`", display_token(&command)),
        Some(command),
    ))
}

pub fn parse_args<I, S>(args: I) -> Result<InspectArgs, UsageError>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    match parse_cli(args)? {
        Cli::Inspect(parsed) => Ok(parsed),
        Cli::Product(other) => Err(invalid(
            format!(
                "parse_args only accepts inspect; got `{}`",
                display_token(&other.command)
            ),
            Some(other.command),
        )),
    }
}

/// Commands whose bare form is meaningful, so a subcommand is optional.
///
/// `ctxpect assets` on its own is the catalog overview and was valid before
/// the copy executor existed; adding subcommands must not retire it.
fn subcommand_optional(command: &str) -> bool {
    // `apply --tx <id>` is the apply itself; `apply status` is the read-only
    // judgement of pending transactions.
    command == "assets" || command == "apply"
}

fn listed_subcommands(command: &str) -> &'static [&'static str] {
    match command {
        "receipt" => &["show", "verify", "export", "redact"],
        "daemon" => &["start", "stop", "status"],
        "sync" => &["preview", "apply", "status"],
        "intent" => &["validate", "show", "project", "preview"],
        "standard" => &[
            "validate", "publish", "preview", "adopt", "pin", "update", "status", "leave",
            "rollback", "revoke",
        ],
        "exception" => &["request", "approve", "reject", "revoke", "status"],
        "policy" => &["eval", "show"],
        "align" => &["status", "diff"],
        "adapter" => &["test", "list"],
        "assets" => &["status", "list", "preview", "copy", "rollback", "sbom"],
        "apply" => &["status"],
        "store" => &["status", "repair"],
        _ => &[],
    }
}

fn is_listed_unimplemented_flag(name: &str) -> bool {
    UNIMPLEMENTED_FLAGS.contains(&name)
        || LISTED_COMMAND_FLAGS.contains(&name)
        || LISTED_GLOBAL_FLAGS_PROSE.contains(&name)
}

fn listed_but_unimplemented_global(name: &str) -> bool {
    LISTED_GLOBAL_FLAGS.contains(&name)
        && !matches!(
            name,
            "json" | "offline" | "project" | "cwd" | "harness" | "surface" | "version"
        )
        && !SLICE_INSPECT_FLAGS.contains(&name)
}

fn unimplemented(name: &str, command: Option<String>) -> UsageError {
    UsageError {
        code: "usage.unimplemented",
        message: format!(
            "`{}` is listed in the CLI contract but is not implemented in this development slice",
            display_token(name)
        ),
        command: sanitize_command(command),
    }
}

fn invalid(message: String, command: Option<String>) -> UsageError {
    UsageError {
        code: "usage.invalid",
        message,
        command: sanitize_command(command),
    }
}

fn is_safe_token(value: &str) -> bool {
    let len = value.len();
    (1..=32).contains(&len)
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

fn display_token(value: &str) -> String {
    if value.split_whitespace().any(|part| part != value) {
        return value
            .split_whitespace()
            .map(|part| {
                if is_safe_token(part) {
                    part.to_string()
                } else {
                    "<argument>".to_string()
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
    }
    if is_safe_token(value) {
        value.to_string()
    } else {
        "<argument>".to_string()
    }
}

fn sanitize_command(command: Option<String>) -> Option<String> {
    command.map(|value| display_token(&value))
}

fn is_ident(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() || bytes.len() > 32 {
        return false;
    }
    bytes[0].is_ascii_lowercase()
        && bytes[1..]
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'-')
}

fn is_os_lane(value: &str) -> bool {
    if value.is_empty() || value.len() > 40 {
        return false;
    }
    let mut parts = value.split('-');
    let Some(first) = parts.next() else {
        return false;
    };
    if first.is_empty()
        || !first
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    {
        return false;
    }
    let mut rest = 0usize;
    for part in parts {
        rest += 1;
        if rest > 4
            || part.is_empty()
            || !part
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'_')
        {
            return false;
        }
    }
    (1..=4).contains(&rest)
}

fn is_semver_coordinate(value: &str) -> bool {
    if value.is_empty() || value.len() > 40 {
        return false;
    }
    let (core, suffix) = match value.find(['-', '+']) {
        Some(index) => {
            let sign = value.as_bytes()[index];
            if sign != b'-' && sign != b'+' {
                return false;
            }
            (&value[..index], Some(&value[index + 1..]))
        }
        None => (value, None),
    };
    if let Some(suffix) = suffix {
        if suffix.is_empty() || suffix.len() > 32 {
            return false;
        }
        if !suffix
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.')
        {
            return false;
        }
    }
    let mut parts = 0usize;
    for part in core.split('.') {
        parts += 1;
        if parts > 4 || part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
    }
    (1..=4).contains(&parts)
}

fn is_version(value: &str) -> bool {
    if value.len() > 40 {
        return false;
    }
    is_semver_coordinate(value) || is_ident(value)
}

fn validate_ident(name: &str, value: &str, command: Option<&str>) -> Result<(), UsageError> {
    if is_ident(value) {
        Ok(())
    } else {
        Err(invalid(
            format!(
                "`--{name}` must match a closed identifier (lowercase letter followed by lowercase letters, digits or hyphen; max 32 characters)"
            ),
            command.map(str::to_string),
        ))
    }
}

/// Mutation actions are dotted lowercase identifiers (`apply`, `assets.copy`).
fn validate_action(value: &str, command: Option<&str>) -> Result<(), UsageError> {
    let ok = !value.is_empty()
        && value.len() <= 40
        && value
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_lowercase() || b == b'-'));
    if ok {
        Ok(())
    } else {
        Err(invalid(
            "`--action` must be a dotted lowercase identifier such as `apply` or `assets.copy`"
                .to_string(),
            command.map(str::to_string),
        ))
    }
}

fn validate_os_lane(value: &str, command: Option<&str>) -> Result<(), UsageError> {
    if is_os_lane(value) {
        Ok(())
    } else {
        Err(invalid(
            "`--os-lane` must match a closed OS lane (lowercase identifiers separated by hyphens; max 40 characters)".to_string(),
            command.map(str::to_string),
        ))
    }
}

fn validate_version(value: &str, command: Option<&str>) -> Result<(), UsageError> {
    if is_version(value) {
        Ok(())
    } else {
        Err(invalid(
            "`--version` must match a version coordinate (dotted digits with optional pre-release suffix, or a closed identifier; max 40 characters)".to_string(),
            command.map(str::to_string),
        ))
    }
}

fn reject_inline(
    name: &str,
    inline: Option<String>,
    command: Option<&str>,
) -> Result<(), UsageError> {
    if inline.is_some() {
        return Err(invalid(
            format!("`--{name}` does not take a value"),
            command.map(str::to_string),
        ));
    }
    Ok(())
}

fn need_value(
    name: &str,
    inline: Option<String>,
    iter: &mut std::iter::Peekable<std::vec::IntoIter<String>>,
    command: Option<&str>,
) -> Result<String, UsageError> {
    if let Some(value) = inline {
        if value.is_empty() {
            return Err(invalid(
                format!("`--{name}` requires a value"),
                command.map(str::to_string),
            ));
        }
        return Ok(value);
    }
    match iter.next() {
        Some(value) if !value.starts_with("--") && !value.is_empty() => Ok(value),
        _ => Err(invalid(
            format!("`--{name}` requires a value"),
            command.map(str::to_string),
        )),
    }
}

fn consume_optional_value(
    inline: Option<String>,
    iter: &mut std::iter::Peekable<std::vec::IntoIter<String>>,
) {
    if inline.is_some() {
        return;
    }
    if iter.peek().is_some_and(|next| {
        !next.starts_with("--")
            && !UNIMPLEMENTED_COMMANDS.contains(&next.as_str())
            && next.as_str() != "inspect"
    }) {
        let _ = iter.next();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<InspectArgs, UsageError> {
        let mut full = vec!["ctxpect"];
        full.extend_from_slice(args);
        parse_args(full)
    }

    #[test]
    fn coordinate_validators_accept_frozen_and_honesty_tokens() {
        assert!(is_version("0.147.0"));
        assert!(is_version("0.153.3-beta.1"));
        assert!(is_version("unknown-honesty"));
        assert!(is_ident("codex"));
        assert!(is_ident("cli"));
        assert!(is_ident("instructions"));
        assert!(is_ident("desktop"));
        assert!(is_os_lane("macos-27-arm64"));
        assert!(is_os_lane("ubuntu-24.04-x86_64"));
        assert!(is_os_lane("windows-11-24h2-x86_64"));
        assert!(parse(&["inspect", "--project", ".", "--version", "0.147.0"]).is_ok());
        assert!(
            parse(&[
                "inspect",
                "--project",
                ".",
                "--os-lane",
                "ubuntu-24.04-x86_64"
            ])
            .is_ok()
        );
        assert!(parse(&["inspect", "--project", ".", "--require", "instructions"]).is_ok());
    }

    #[test]
    fn coordinate_validators_reject_path_like_values_without_echo() {
        for (flag, value) in [
            ("version", "/etc/passwd"),
            ("harness", "/etc/passwd"),
            ("surface", "/etc/passwd"),
            ("os-lane", "/etc/passwd"),
            ("require", "/etc/passwd"),
            ("version", "../x"),
            ("harness", "../x"),
            ("surface", "../x"),
            ("os-lane", "../x"),
            ("require", "../x"),
        ] {
            let flag_arg = format!("--{flag}");
            let err = parse(&["inspect", "--project", ".", &flag_arg, value]).expect_err(flag);
            assert_eq!(err.code, "usage.invalid", "{flag} {value}");
            assert!(!err.message.contains(value), "{flag} {}", err.message);
            assert!(!err.message.contains("/etc"), "{}", err.message);
            assert!(!err.message.contains(".."), "{}", err.message);
        }
    }

    #[test]
    fn as_of_accepts_a_real_date_and_rejects_impossible_ones_without_echo() {
        let Cli::Product(parsed) =
            parse_cli(["ctxpect", "doctor", "--project", ".", "--as-of", "2026-09-04"]).unwrap()
        else {
            panic!("doctor parses as a product command")
        };
        assert_eq!(parsed.as_of, Some((2026, 9, 4)));
        for bad in ["2026-13-01", "2026-02-30", "2025-02-29", "not-a-date", "2026-09-04T00:00:00Z"] {
            let err = parse_cli(["ctxpect", "doctor", "--project", ".", "--as-of", bad]).expect_err(bad);
            assert_eq!(err.code, "usage.invalid", "{bad}");
            assert!(!err.message.contains(bad), "{bad}");
        }
    }

    #[test]
    fn unsafe_command_token_is_not_echoed() {
        let err = parse(&["/etc/passwd", "--json"]).expect_err("command");
        assert_eq!(err.code, "usage.invalid");
        assert!(!err.message.contains("/etc"));
        assert_eq!(err.command.as_deref(), Some("<argument>"));
    }
}
