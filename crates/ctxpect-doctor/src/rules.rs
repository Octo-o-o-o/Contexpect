//! Project-content Doctor rules: the twenty rules of the frozen Doctor corpus
//! (`acceptance/corpus/development/doctor/doctor-corpus.jsonl`), evaluated
//! over a scanned project.
//!
//! The rule ids are the corpus namespace (`secret_literal`, …). The older
//! `D-*` ids produced by [`crate::diagnose`] from an inspect Receipt are not
//! renamed; where a `D-*` rule has the same meaning as a corpus rule the
//! `D-*` finding carries an `aliases` field (see `docs/process/doctor-rule-map.md`).
//!
//! # Inputs
//!
//! Rules read a flat list of [`ScannedFile`]: project-relative path, bytes
//! (when the collector read them), and for a symlink its target as written.
//! Some rules read **declaration files** a project may carry
//! (`layout.json`, `inventory.json`, `plan.json`, `archive-manifest.json`,
//! `hooks.json`, `budget.json`, `device-lock.json`, `provenance.json`,
//! `adapter-version.json`, `placement.json`). Reading a declaration needs no
//! extraction, execution or network: a manifest that *declares* an entry
//! `../etc/passwd` is a traversal declaration whether or not any archive is
//! ever opened. Those rules are therefore lazy declaration rules, not archive
//! or hook executors.
//!
//! # Blocking
//!
//! [`BLOCKING_RULES`] lists the rules whose finding blocks `ctxpect ci` and
//! `ctxpect doctor`. A rule is blocking only while its precision on the
//! frozen corpus is 1.00; `crates/ctxpect-cli/tests/doctor_corpus.rs` measures
//! that and fails if a listed rule ever produces a false positive there.

use crate::secrets::secret_literal;
use ctxpect_schema::{array, object, parse, sha256_hex, string, Value};
use std::collections::BTreeMap;

/// One file as the collector saw it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedFile {
    /// Project-relative path with `/` separators.
    pub path: String,
    /// Content, when the collector read it. `None` for withheld, unreadable
    /// or non-regular entries.
    pub bytes: Option<Vec<u8>>,
    /// For a symlink, the target as written.
    pub link_target: Option<String>,
    pub is_symlink: bool,
}

/// Rules whose finding blocks. Each has precision 1.00 on the frozen Doctor
/// corpus; the corpus runner enforces that and this list must shrink if a
/// rule ever produces a false positive there.
pub const BLOCKING_RULES: &[&str] = &[
    "secret_literal",
    "symlink_escape",
    "hidden_unicode",
    "path_containment_escape",
    "required_asset_missing",
    "unapproved_lossy_projection",
    "archive_traversal",
    "passive_scan_exec",
];

/// Non-blocking rules. Their findings are reported, never exit 2.
pub const NON_BLOCKING_RULES: &[&str] = &[
    "duplicate",
    "conflict",
    "stale",
    "oversized_resident",
    "cap_truncation",
    "bad_frontmatter",
    "gitignore_mismatch",
    "single_device_only",
    "unknown_source",
    "version_incompatible",
    "undiscoverable_path",
    "placement_recommendation",
];

/// The frozen corpus reference date for the `stale` rule (PRD invariant 6,
/// the acceptance cutoff). The rule itself takes an explicit `as_of`; corpus
/// and fixture runs pass this constant so golden output stays reproducible,
/// while a production run passes the wall-clock date (or the CLI `--as-of`
/// override). An old date only proves the file crossed a maintenance
/// threshold, not that its content is wrong.
pub const STALE_CUTOFF: (i64, u32, u32) = (2026, 9, 4);

#[must_use]
pub fn is_blocking_rule(rule_id: &str) -> bool {
    BLOCKING_RULES.contains(&rule_id)
}

/// C-F04 (2026-09-12): a finding produced by reading a self-declared file
/// (`budget.json` / `inventory.json` / `plan.json` / …) is a
/// **declaration-validation**, not an observed fact. The declaration files
/// have no product-side writer wired to a trusted producer, so `producer` is
/// `"unknown"` and `trusted_producer_connected` is `false` — a declaration
/// that says `approved: true` or `present: [...]` is the product's own
/// statement about itself, not authorization or presence evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    /// The declaration file the finding was read from.
    pub source_file: String,
    /// Who produced the declaration. Always `"unknown"` in this slice: no
    /// trusted producer is connected to any declaration file.
    pub producer: String,
    /// The declaration's own `declared_at`, when it carries one — itself a
    /// self-declared string, not a trusted timestamp.
    pub declared_at: Option<String>,
}

impl Declaration {
    /// Read the source metadata a declaration file offers. The producer is
    /// never taken from the file: a declaration cannot attest its own origin.
    fn of(source_file: &str, declaration: Option<&Value>) -> Self {
        Declaration {
            source_file: source_file.to_string(),
            producer: "unknown".to_string(),
            declared_at: declaration
                .and_then(|value| value.get("declared_at"))
                .and_then(Value::as_str)
                .map(str::to_string),
        }
    }
}

/// A project finding before it is rendered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFinding {
    pub rule_id: &'static str,
    pub path: String,
    pub message: String,
    /// Set when the finding validates a declaration file's claims (C-F04).
    pub declaration: Option<Declaration>,
}

impl ProjectFinding {
    fn new(rule_id: &'static str, path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            rule_id,
            path: path.into(),
            message: message.into(),
            declaration: None,
        }
    }

    /// A finding read from a declaration file: it validates what the file
    /// declares, carrying the declaration's source metadata with it.
    fn declared(
        rule_id: &'static str,
        path: impl Into<String>,
        message: impl Into<String>,
        source_file: &str,
        declaration: &Value,
    ) -> Self {
        Self {
            declaration: Some(Declaration::of(source_file, Some(declaration))),
            ..Self::new(rule_id, path, message)
        }
    }
}

/// The file's text for the content rules. Bytes that are not valid UTF-8
/// are replaced, not skipped: a single stray byte must not switch every
/// content rule off for the whole file (the projection gate reads the same
/// way).
fn text_of(file: &ScannedFile) -> Option<std::borrow::Cow<'_, str>> {
    file.bytes.as_deref().map(String::from_utf8_lossy)
}

/// The `schema` a root-level declaration file must carry to be read as one:
/// `ctxpect-<name>-v1` for `<name>.json`. A same-named file without it is
/// somebody else's file (a project's own `plan.json`), not a declaration.
fn declaration_schema(name: &str) -> String {
    format!("ctxpect-{}-v1", name.trim_end_matches(".json"))
}

/// Path of the namespaced form of a declaration: `.ctxpect/<name>`.
fn namespaced(name: &str) -> String {
    format!(".ctxpect/{name}")
}

/// Every readable declaration named `name`, with the path it was read from.
///
/// Two spellings are declarations: `.ctxpect/<name>` (the namespace is the
/// claim), and root `<name>` only when its top-level `schema` is
/// `ctxpect-<name>-v1`. Both may exist; both are evaluated — a clean root
/// file never hides a violating namespaced one or the other way round. A
/// namespaced file that does not parse, is not an object or carries another
/// schema is reported by [`declaration_problems`], not silently absent.
fn declarations(files: &[ScannedFile], name: &str) -> Vec<(String, Value)> {
    let schema = declaration_schema(name);
    let mut out = Vec::new();
    for file in files {
        let namespaced_path = file.path == namespaced(name);
        if !(file.path == name || namespaced_path) {
            continue;
        }
        let Some(text) = text_of(file) else { continue };
        let Ok(value) = parse(&text) else { continue };
        if value.as_object().is_none() {
            continue;
        }
        let declared = value.get("schema").and_then(Value::as_str);
        let accepted = match declared {
            Some(s) => s == schema,
            None => namespaced_path,
        };
        if accepted {
            out.push((file.path.clone(), value));
        }
    }
    out
}

/// The declaration file names this crate reads.
const DECLARATION_NAMES: &[&str] = &[
    "layout.json",
    "inventory.json",
    "plan.json",
    "archive-manifest.json",
    "hooks.json",
    "budget.json",
    "device-lock.json",
    "provenance.json",
    "adapter-version.json",
    "placement.json",
];

/// `.ctxpect/<name>` files that cannot be read as the declaration their
/// name claims: not JSON, not an object, or another `schema`. Reported as
/// `declaration_unreadable` (non-blocking, outside the corpus rule set) so
/// a broken declaration is never mistaken for "no declaration".
fn declaration_problems(files: &[ScannedFile]) -> Vec<ProjectFinding> {
    let mut out = Vec::new();
    for name in DECLARATION_NAMES {
        let path = namespaced(name);
        let Some(file) = files.iter().find(|file| file.path == path) else { continue };
        let Some(text) = text_of(file) else {
            out.push(ProjectFinding {
                declaration: Some(Declaration::of(&path, None)),
                ..ProjectFinding::new("declaration_unreadable", &path, "namespaced declaration could not be read")
            });
            continue;
        };
        let problem = match parse(&text) {
            Err(_) => Some("namespaced declaration is not valid JSON"),
            Ok(value) if value.as_object().is_none() => Some("namespaced declaration is not a JSON object"),
            Ok(value) => match value.get("schema").and_then(Value::as_str) {
                Some(s) if s != declaration_schema(name) => Some("namespaced declaration carries another schema"),
                _ => None,
            },
        };
        if let Some(problem) = problem {
            out.push(ProjectFinding {
                declaration: Some(Declaration::of(&path, None)),
                ..ProjectFinding::new("declaration_unreadable", &path, problem)
            });
        }
    }
    out
}

fn has_file(files: &[ScannedFile], path: &str) -> bool {
    files.iter().any(|file| file.path == path && file.bytes.is_some())
}

fn is_markdown(path: &str) -> bool {
    path.ends_with(".md")
}

/// Whether a path (as written) leaves its root lexically: absolute, or more
/// `..` segments than segments before them.
fn escapes_lexically(base_dir: &str, target: &str) -> bool {
    let target = target.replace('\\', "/");
    if target.starts_with('/') {
        return true;
    }
    let mut depth: i64 = base_dir
        .split('/')
        .filter(|seg| !seg.is_empty() && *seg != ".")
        .count() as i64;
    for segment in target.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                depth -= 1;
                if depth < 0 {
                    return true;
                }
            }
            _ => depth += 1,
        }
    }
    false
}

fn parent_dir(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(dir, _)| dir)
}

fn has_parent_segment(path: &str) -> bool {
    path.replace('\\', "/").split('/').any(|segment| segment == "..")
}

/// Code points that hide text from a reader while remaining in what a
/// model receives: zero-width, bidirectional controls, invisible operators,
/// the soft hyphen and the tag block (ASCII smuggling).
fn hidden_char(ch: char) -> bool {
    matches!(
        ch,
        '\u{00AD}' | '\u{034F}' | '\u{061C}' | '\u{180E}'
            | '\u{200B}' | '\u{2060}' | '\u{FEFF}'
            | '\u{2061}'..='\u{2064}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2066}'..='\u{2069}'
            | '\u{206A}'..='\u{206F}'
            | '\u{E0000}'..='\u{E007F}'
    )
}

/// ZWNJ / ZWJ are ordinary text in Persian, Devanagari and emoji sequences;
/// they hide something only between plain ASCII, where nothing joins.
fn joiner_is_hidden(prev: Option<char>, next: Option<char>) -> bool {
    let plain = |c: Option<char>| c.is_none_or(|c| c.is_ascii());
    plain(prev) && plain(next)
}

/// Positions of hidden code points in `text`, contextual joiners included.
fn hidden_positions(text: &str) -> Vec<usize> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut out = Vec::new();
    for (i, (offset, ch)) in chars.iter().enumerate() {
        let hidden = match ch {
            '\u{200C}' | '\u{200D}' => joiner_is_hidden(
                i.checked_sub(1).map(|p| chars[p].1),
                chars.get(i + 1).map(|n| n.1),
            ),
            other => hidden_char(*other),
        };
        if hidden {
            out.push(*offset);
        }
    }
    out
}

/// Frontmatter block lines, if the file starts with `---`.
fn frontmatter(text: &str) -> Option<(Vec<&str>, bool)> {
    let mut lines = text.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    let mut body = Vec::new();
    for line in lines {
        if line.trim() == "---" {
            return Some((body, true));
        }
        body.push(line);
    }
    Some((body, false))
}

fn frontmatter_line_valid(line: &str) -> bool {
    let trimmed = line.trim_end();
    if trimmed.trim().is_empty() || trimmed.trim_start().starts_with('#') {
        return true;
    }
    // Continuation / list items belong to the previous key.
    if trimmed.starts_with(' ') || trimmed.starts_with('\t') || trimmed.trim_start().starts_with("- ") {
        return true;
    }
    let Some((key, value)) = trimmed.split_once(':') else {
        return false;
    };
    let key = key.trim();
    if key.is_empty()
        || !key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return false;
    }
    let value = value.trim();
    if let Some(rest) = value.strip_prefix('[') {
        return rest.ends_with(']');
    }
    if let Some(rest) = value.strip_prefix('{') {
        return rest.ends_with('}');
    }
    if let Some(rest) = value.strip_prefix('"') {
        return rest.ends_with('"') && !rest.is_empty();
    }
    if let Some(rest) = value.strip_prefix('\'') {
        return rest.ends_with('\'') && !rest.is_empty();
    }
    true
}

fn parse_date(text: &str) -> Option<(i64, u32, u32)> {
    let mut parts = text.trim().splitn(3, '-');
    let year = parts.next()?.parse::<i64>().ok()?;
    let month = parts.next()?.parse::<u32>().ok()?;
    let day_part = parts.next()?;
    let day = day_part
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse::<u32>()
        .ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some((year, month, day))
}

/// Days since the Unix epoch for a civil date (Howard Hinnant's
/// days-from-civil, proleptic Gregorian calendar).
#[must_use]
pub fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = i64::from(month);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Days in `month` of `year` in the proleptic Gregorian calendar.
fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    }
}

/// A strict `YYYY-MM-DD` calendar date: every digit fixed in place and the
/// day real for its month (`2026-02-30` is not a date). Callers that take a
/// civil date from a user (CLI `--as-of`) go through this, never through the
/// lenient frontmatter parser above.
#[must_use]
pub fn parse_civil_date(text: &str) -> Option<(i64, u32, u32)> {
    let bytes = text.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return None;
    }
    let num = |from: usize, to: usize| -> Option<i64> {
        let slice = text.get(from..to)?;
        if slice.bytes().all(|b| b.is_ascii_digit()) { slice.parse().ok() } else { None }
    };
    let year = num(0, 4)?;
    let month = u32::try_from(num(5, 7)?).ok()?;
    let day = u32::try_from(num(8, 10)?).ok()?;
    if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
        return None;
    }
    Some((year, month, day))
}

/// The inverse of [`days_from_civil`]: the civil date `days` after the
/// Unix epoch (Hinnant's civil-from-days).
#[must_use]
pub fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}

fn package_manager(text: &str) -> Option<&'static str> {
    let lower = text.to_ascii_lowercase();
    for manager in ["pnpm", "npm", "yarn", "bun"] {
        // `must be <manager>` is the directive form the corpus freezes; a
        // mention of a manager elsewhere is not a directive.
        if lower.contains(&format!("must be {manager}")) {
            return Some(match manager {
                "pnpm" => "pnpm",
                "npm" => "npm",
                "yarn" => "yarn",
                _ => "bun",
            });
        }
    }
    None
}

fn str_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Evaluate every project rule without knowing the project's absolute
/// location: an absolute symlink target is then always read as leaving the
/// workspace. Callers that know the root use [`project_findings_in`].
///
/// `as_of` is the civil date the `stale` rule is judged against; it is an
/// explicit input, never read from the clock inside this crate.
#[must_use]
pub fn project_findings(files: &[ScannedFile], as_of: (i64, u32, u32)) -> Vec<ProjectFinding> {
    project_findings_in(files, None, as_of)
}

/// Evaluate every project rule. With `root`, an absolute symlink target that
/// lies inside the root (`ln -s "$(pwd)/x" y`) is not an escape. `as_of` is
/// the civil date `stale` is judged against (see [`project_findings`]).
#[must_use]
pub fn project_findings_in(files: &[ScannedFile], root: Option<&std::path::Path>, as_of: (i64, u32, u32)) -> Vec<ProjectFinding> {
    let inside_root = |target: &str| {
        root.is_some_and(|root| {
            let target = std::path::Path::new(target);
            if !target.is_absolute() {
                return false;
            }
            // Lexical: `..` segments are resolved without touching the disk.
            let mut normalized = std::path::PathBuf::new();
            for component in target.components() {
                match component {
                    std::path::Component::ParentDir => {
                        normalized.pop();
                    }
                    std::path::Component::CurDir => {}
                    other => normalized.push(other.as_os_str()),
                }
            }
            normalized.starts_with(root)
        })
    };
    let mut out: Vec<ProjectFinding> = Vec::new();
    let mut seen: Vec<(&'static str, String)> = Vec::new();
    let mut add = |finding: ProjectFinding| {
        let key = (finding.rule_id, finding.path.clone());
        if !seen.contains(&key) {
            seen.push(key);
            out.push(finding);
        }
    };

    // --- content rules over every readable text file ---
    for file in files {
        let Some(text) = text_of(file) else {
            continue;
        };
        let text: &str = &text;
        if let Some(class) = secret_literal(text) {
            add(ProjectFinding::new(
                "secret_literal",
                &file.path,
                format!("credential shape `{}` in project text; a secret literal fails closed", class.as_str()),
            ));
        }
        if hidden_positions(text)
            .into_iter()
            .any(|offset| !(offset == 0 && text.starts_with('\u{FEFF}')))
        {
            add(ProjectFinding::new(
                "hidden_unicode",
                &file.path,
                "zero-width, invisible or bidirectional control characters hide text from a reader",
            ));
        }
        for line in text.lines() {
            // `read:` in any case, optionally as a list item (`- read: …`).
            let trimmed = line.trim_start();
            let trimmed = ["- ", "* ", "+ "]
                .iter()
                .find_map(|marker| trimmed.strip_prefix(marker))
                .map_or(trimmed, str::trim_start);
            let lowered = trimmed.get(..5).map(str::to_ascii_lowercase);
            if lowered.as_deref() == Some("read:")
                && let Some(rest) = trimmed.get(5..)
            {
                let target = rest.split_whitespace().next().unwrap_or("");
                if !target.is_empty() && escapes_lexically(parent_dir(&file.path), target) {
                    add(ProjectFinding::new(
                        "path_containment_escape",
                        &file.path,
                        "an instruction `read:` path escapes the declared root",
                    ));
                }
            }
        }
    }

    // --- symlink_escape: real links, then declared links ---
    for file in files {
        if file.is_symlink
            && let Some(target) = &file.link_target
            && escapes_lexically(parent_dir(&file.path), target)
            && !inside_root(target)
        {
            add(ProjectFinding::new(
                "symlink_escape",
                &file.path,
                "symlink target leaves the workspace",
            ));
        }
    }
    for (decl_path, layout) in declarations(files, "layout.json") {
        if let Some(links) = layout.get("symlinks").and_then(Value::as_array) {
            for link in links {
                let from = link.get("from").and_then(Value::as_str).unwrap_or("");
                let to = link.get("to").and_then(Value::as_str).unwrap_or("");
                if !to.is_empty() && escapes_lexically(parent_dir(from), to) && !inside_root(to) {
                    add(ProjectFinding::declared(
                        "symlink_escape",
                        &decl_path,
                        "declared symlink target leaves the workspace",
                        &decl_path,
                        &layout,
                    ));
                }
            }
        }
        if let Some(items) = layout.get("files").and_then(Value::as_array) {
            for item in items {
                let path = item.get("path").and_then(Value::as_str).unwrap_or("");
                if path.starts_with("..") || path.starts_with('/') {
                    add(ProjectFinding::declared(
                        "undiscoverable_path",
                        &decl_path,
                        "declared instruction path is outside the discoverable tree",
                        &decl_path,
                        &layout,
                    ));
                }
            }
        }
    }
    for problem in declaration_problems(files) {
        add(problem);
    }

    // --- lazy declaration rules ---
    for (decl_path, inventory) in declarations(files, "inventory.json") {
        let required = str_list(inventory.get("required"));
        let present = str_list(inventory.get("present"));
        let missing = required
            .iter()
            .any(|item| !present.contains(item) && !has_file(files, item));
        if missing {
            add(ProjectFinding::declared(
                "required_asset_missing",
                &decl_path,
                "a declared required asset is neither present nor on disk",
                &decl_path,
                &inventory,
            ));
        }
    }
    for (decl_path, plan) in declarations(files, "plan.json") {
        let drops = str_list(plan.get("drops"));
        let approved = plan.get("approved").and_then(Value::as_bool) == Some(true);
        if !drops.is_empty() && !approved {
            // C-F04: plan.json's `approved` is a product self-declaration, not
            // authorization evidence — authorization evidence is only the
            // policy/exception chain. This finding validates the declaration;
            // it does not and cannot read an approval out of it.
            add(ProjectFinding::declared(
                "unapproved_lossy_projection",
                &decl_path,
                "plan.json declares drops with no `approved: true` declaration; even `approved: true` here would be a product self-declaration, not authorization evidence (only the policy/exception chain authorizes)",
                &decl_path,
                &plan,
            ));
        }
    }
    for (decl_path, archive) in declarations(files, "archive-manifest.json") {
        for entry in str_list(archive.get("entries")) {
            if has_parent_segment(&entry) || entry.starts_with('/') {
                add(ProjectFinding::declared(
                    "archive_traversal",
                    &decl_path,
                    "a declared archive entry escapes its destination",
                    &decl_path,
                    &archive,
                ));
            }
        }
    }
    for (decl_path, hooks) in declarations(files, "hooks.json") {
        if str_list(hooks.get("on_scan"))
            .iter()
            .any(|cmd| !cmd.trim().is_empty())
        {
            add(ProjectFinding::declared(
                "passive_scan_exec",
                &decl_path,
                "a scan hook is declared; passive scanning executes nothing",
                &decl_path,
                &hooks,
            ));
        }
    }

    // --- duplicate: identical instruction bodies ---
    let mut by_digest: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for file in files {
        if is_markdown(&file.path)
            && let Some(bytes) = &file.bytes
        {
            by_digest
                .entry(sha256_hex(bytes))
                .or_default()
                .push(file.path.as_str());
        }
    }
    for group in by_digest.values() {
        if group.len() > 1 {
            let mut sorted = group.clone();
            sorted.sort_unstable();
            add(ProjectFinding::new(
                "duplicate",
                sorted[sorted.len() - 1],
                "instruction body is byte-identical to another instruction file",
            ));
        }
    }

    // --- conflict: contradictory package-manager directives ---
    let mut managers: Vec<(&str, &'static str)> = Vec::new();
    for file in files {
        if is_markdown(&file.path)
            && let Some(text) = text_of(file)
            && let Some(manager) = package_manager(&text)
        {
            managers.push((file.path.as_str(), manager));
        }
    }
    managers.sort_unstable();
    if let Some((_, baseline)) = managers.first()
        && let Some((path, _)) = managers.iter().find(|(_, manager)| manager != baseline)
    {
        add(ProjectFinding::new(
            "conflict",
            *path,
            "instruction files disagree on the package manager",
        ));
    }

    // --- stale / bad_frontmatter: frontmatter-driven ---
    let as_of_days = days_from_civil(as_of.0, as_of.1, as_of.2);
    for file in files {
        if !is_markdown(&file.path) {
            continue;
        }
        let Some(text) = text_of(file) else {
            continue;
        };
        let Some((lines, closed)) = frontmatter(&text) else {
            continue;
        };
        if !closed || lines.iter().any(|line| !frontmatter_line_valid(line)) {
            add(ProjectFinding::new(
                "bad_frontmatter",
                &file.path,
                "frontmatter does not parse as key/value YAML",
            ));
        }
        for line in &lines {
            if let Some(value) = line.trim_start().strip_prefix("updated:")
                && let Some((y, m, d)) = parse_date(value)
                && as_of_days - days_from_civil(y, m, d) > 365
            {
                add(ProjectFinding::new(
                    "stale",
                    &file.path,
                    format!(
                        "frontmatter `updated:` is more than 365 days before the as-of date {:04}-{:02}-{:02}",
                        as_of.0, as_of.1, as_of.2
                    ),
                ));
            }
        }
    }

    // --- budget.json: cap_truncation / oversized_resident ---
    for (decl_path, budget) in declarations(files, "budget.json") {
        let path = budget.get("path").and_then(Value::as_str).unwrap_or("").to_string();
        let max_bytes = budget.get("max_bytes").and_then(Value::as_i64);
        let truncated = budget.get("truncated").and_then(Value::as_bool) == Some(true);
        let actual = files
            .iter()
            .find(|file| file.path == path)
            .and_then(|file| file.bytes.as_ref())
            .map(|bytes| i64::try_from(bytes.len()).unwrap_or(i64::MAX))
            .or_else(|| budget.get("actual_bytes").and_then(Value::as_i64));
        match (max_bytes, actual) {
            (Some(max), Some(actual)) if actual > max => {
                if truncated {
                    add(ProjectFinding::declared(
                        "cap_truncation",
                        if path.is_empty() { decl_path.clone() } else { path.clone() },
                        "instruction bytes exceed the declared cap and are truncated",
                        &decl_path,
                        &budget,
                    ));
                } else {
                    add(ProjectFinding::declared(
                        "oversized_resident",
                        if path.is_empty() { decl_path.clone() } else { path.clone() },
                        "resident asset exceeds its declared byte cap",
                        &decl_path,
                        &budget,
                    ));
                }
            }
            _ if truncated => add(ProjectFinding::declared(
                "cap_truncation",
                if path.is_empty() { "AGENTS.md".to_string() } else { path.clone() },
                "instruction is declared truncated by a cap",
                &decl_path,
                &budget,
            )),
            _ => {}
        }
    }

    // --- gitignore_mismatch ---
    let ignore_text = files
        .iter()
        .find(|file| file.path == ".ctxpect-gitignore")
        .or_else(|| files.iter().find(|file| file.path == ".gitignore"))
        .and_then(text_of);
    if let Some(ignore_text) = ignore_text {
        for ignored in ignore_text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
        {
            let referenced = files.iter().any(|file| {
                is_markdown(&file.path)
                    && text_of(file).is_some_and(|text| text.contains(ignored))
            });
            if has_file(files, ignored) && referenced {
                add(ProjectFinding::new(
                    "gitignore_mismatch",
                    ignored,
                    "an ignored path is present and still referenced by instructions",
                ));
            }
        }
    }

    // --- device / provenance / adapter version / placement declarations ---
    for (decl_path, lock) in declarations(files, "device-lock.json") {
        let single = lock.get("sync").and_then(Value::as_bool) == Some(false)
            && lock
                .get("device_id")
                .and_then(Value::as_str)
                .is_some_and(|id| !id.is_empty() && id != "*");
        if single {
            add(ProjectFinding::declared(
                "single_device_only",
                &decl_path,
                "asset is locked to one device and excluded from sync",
                &decl_path,
                &lock,
            ));
        }
    }
    for (decl_path, provenance) in declarations(files, "provenance.json") {
        let source_missing = provenance
            .get("source")
            .is_none_or(|source| source.as_str().is_none_or(str::is_empty));
        if source_missing {
            let path = provenance
                .get("path")
                .and_then(Value::as_str)
                .filter(|p| !p.is_empty())
                .unwrap_or("imported.md");
            add(ProjectFinding::declared(
                "unknown_source",
                path,
                "imported asset declares no source provenance",
                &decl_path,
                &provenance,
            ));
        }
    }
    for (decl_path, adapter) in declarations(files, "adapter-version.json") {
        if adapter.get("required") != adapter.get("actual") {
            add(ProjectFinding::declared(
                "version_incompatible",
                &decl_path,
                "declared adapter version does not match the required one",
                &decl_path,
                &adapter,
            ));
        }
    }
    for (decl_path, placement) in declarations(files, "placement.json") {
        let path = placement.get("path").and_then(Value::as_str).unwrap_or("");
        let recommended = placement.get("recommended").and_then(Value::as_str).unwrap_or("");
        if !path.is_empty() && !recommended.is_empty() && path != recommended && has_file(files, path) {
            add(ProjectFinding::declared(
                "placement_recommendation",
                path,
                "asset sits on a non-recommended path",
                &decl_path,
                &placement,
            ));
        }
    }

    out
}

/// Render project findings in the Doctor finding shape, with `blocking` and
/// `path` fields. Same finding id derivation as Receipt-derived findings.
#[must_use]
pub fn render_project_findings(findings: &[ProjectFinding]) -> Vec<Value> {
    findings
        .iter()
        .map(|item| {
            let blocking = is_blocking_rule(item.rule_id);
            let id = format!(
                "f_{}",
                &ctxpect_schema::sha256_text(&format!("{}|{}|{}", item.rule_id, item.message, item.path))[..12]
            );
            let mut rendered = object([
                ("finding_id", string(id)),
                ("rule_id", string(item.rule_id)),
                (
                    "rule_namespace",
                    string(if item.rule_id == "declaration_unreadable" { "doctor-declarations" } else { "doctor-corpus" }),
                ),
                ("title", string(&item.message)),
                ("path", string(&item.path)),
                ("blocking", Value::Bool(blocking)),
                // Content rules are deterministic over bytes on disk.
                ("confirmation", string("confirmed")),
                ("severity", string("confirmed")),
                ("unknown_is_severity", Value::Bool(false)),
                ("affected_surfaces", array([string(&item.path)])),
                ("evidence_state", string("static-resolution")),
                ("impact", string(if blocking { "blocking" } else { "advisory" })),
                ("first_seen", string("current-receipt")),
                ("reason_code", string(item.rule_id)),
                ("facet", string("project-content")),
                (
                    "treatment",
                    object([
                        ("locked", Value::Bool(false)),
                        ("lock_reason", string("none")),
                        ("unlocks_via_export", Value::Bool(false)),
                        ("unlocks_via_advisor", Value::Bool(false)),
                        ("unlocks_via_user_attestation_alone", Value::Bool(false)),
                    ]),
                ),
                (
                    "next_evidence",
                    array([object([
                        ("action", string("Inspect the named path; the rule is deterministic over its bytes.")),
                        ("kind", string("evidence")),
                    ])]),
                ),
                (
                    "placement",
                    object([
                        ("authority", string("unknown")),
                        ("target", string(&item.path)),
                        ("loss", string("unknown")),
                    ]),
                ),
            ]);
            // C-F04: a finding read from a declaration file says so, and says
            // that the declaration's producer was never connected — the
            // finding validates the declaration text, not its origin.
            if let Some(declaration) = &item.declaration
                && let Value::Object(map) = &mut rendered
            {
                map.insert(
                    "declaration".to_string(),
                    object([
                        ("kind", string("declaration-validation")),
                        ("source_file", string(&declaration.source_file)),
                        ("producer", string(&declaration.producer)),
                        ("trusted_producer_connected", Value::Bool(false)),
                        (
                            "declared_at",
                            match &declaration.declared_at {
                                Some(at) => string(at),
                                None => Value::Null,
                            },
                        ),
                    ]),
                );
            }
            rendered
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, text: &str) -> ScannedFile {
        ScannedFile {
            path: path.to_string(),
            bytes: Some(text.as_bytes().to_vec()),
            link_target: None,
            is_symlink: false,
        }
    }

    /// Tests that do not exercise the `stale` boundary run against the frozen
    /// corpus reference date so they stay reproducible.
    fn rules(files: &[ScannedFile]) -> Vec<(&'static str, String)> {
        rules_as_of(files, STALE_CUTOFF)
    }

    fn rules_as_of(files: &[ScannedFile], as_of: (i64, u32, u32)) -> Vec<(&'static str, String)> {
        project_findings(files, as_of)
            .into_iter()
            .map(|f| (f.rule_id, f.path))
            .collect()
    }

    #[test]
    fn joiners_in_scripts_and_emoji_are_text_but_tag_characters_and_soft_hyphens_hide() {
        // ZWJ / ZWNJ carrying an emoji sequence, Persian or Devanagari text.
        for text in ["Team: 👩\u{200D}💻 ships it\n", "می\u{200C}خواهم\n", "क्\u{200D}ष\n", "🏳\u{FE0F}\u{200D}🌈\n"] {
            assert!(rules(&[file("AGENTS.md", text)]).is_empty(), "{text:?}");
        }
        // The same joiner between plain ASCII letters joins nothing: hidden.
        assert_eq!(
            rules(&[file("AGENTS.md", "over\u{200D}ride\n")]),
            vec![("hidden_unicode", "AGENTS.md".to_string())]
        );
        // Tag characters (ASCII smuggling), a soft hyphen, invisible operators.
        for text in ["AGENTS\u{E0041}\u{E0047}\n", "over\u{00AD}ride\n", "a\u{2062}b\n", "a\u{206C}b\n", "a\u{061C}b\n"] {
            assert_eq!(rules(&[file("AGENTS.md", text)]), vec![("hidden_unicode", "AGENTS.md".to_string())], "{text:?}");
        }
    }

    #[test]
    fn a_stray_non_utf8_byte_does_not_switch_the_content_rules_off() {
        let mut bytes = b"caf\xe9\nTOKEN=ghp_fixture_not_a_real_secret_00\nread: ../outside\n".to_vec();
        bytes.push(0xFF);
        let scanned = ScannedFile { path: "AGENTS.md".into(), bytes: Some(bytes), link_target: None, is_symlink: false };
        let found = rules(&[scanned]);
        assert!(found.contains(&("secret_literal", "AGENTS.md".to_string())), "{found:?}");
        assert!(found.contains(&("path_containment_escape", "AGENTS.md".to_string())), "{found:?}");
    }

    #[test]
    fn a_declaration_is_read_from_its_namespace_or_from_a_root_file_that_names_its_schema() {
        // Root file without the schema: somebody else's plan.json, not ours.
        assert!(rules(&[file("plan.json", r#"{"drops":["scoped-rule"]}"#)]).is_empty());
        // The namespaced form needs no schema field.
        assert_eq!(
            rules(&[file(".ctxpect/plan.json", r#"{"drops":["scoped-rule"]}"#)]),
            vec![("unapproved_lossy_projection", ".ctxpect/plan.json".to_string())]
        );
        // Both present: both are judged; a clean root file hides nothing.
        let both = rules(&[
            file("hooks.json", r#"{"schema":"ctxpect-hooks-v1","on_scan":[]}"#),
            file(".ctxpect/hooks.json", r#"{"on_scan":["curl http://example.invalid"]}"#),
        ]);
        assert_eq!(both, vec![("passive_scan_exec", ".ctxpect/hooks.json".to_string())]);
        // A broken namespaced declaration is reported, never treated as absent.
        for (body, why) in [("{not json", "json"), ("[]", "object"), (r#"{"schema":"ctxpect-plan-v2"}"#, "schema")] {
            let found = rules(&[file(".ctxpect/plan.json", body)]);
            assert_eq!(found, vec![("declaration_unreadable", ".ctxpect/plan.json".to_string())], "{why}");
        }
        assert!(!is_blocking_rule("declaration_unreadable"));
    }

    #[test]
    fn lexical_escape_is_about_depth_not_the_presence_of_dots() {
        assert!(escapes_lexically("", "../x"));
        assert!(!escapes_lexically("a/b", "../x"));
        assert!(escapes_lexically("a", "../../x"));
        assert!(escapes_lexically("", "/etc/passwd"));
        assert!(!escapes_lexically("", "docs/../AGENTS.md"));
    }

    #[test]
    fn blocking_rules_fire_on_their_grammar_and_not_on_look_alikes() {
        assert_eq!(
            rules(&[file("AGENTS.md", "TOKEN=ghp_fixture_not_a_real_secret_00\n")]),
            vec![("secret_literal", "AGENTS.md".to_string())]
        );
        assert!(rules(&[file("AGENTS.md", "API_KEY=documentation-only-placeholder\n")]).is_empty());
        assert_eq!(
            rules(&[file("AGENTS.md", "INVIS\u{200B}IBLE\n")]),
            vec![("hidden_unicode", "AGENTS.md".to_string())]
        );
        assert_eq!(
            rules(&[file("AGENTS.md", "read: ../outside/key\n")]),
            vec![("path_containment_escape", "AGENTS.md".to_string())]
        );
        assert!(rules(&[file("AGENTS.md", "read: ./inside/key\n")]).is_empty());
        let link = ScannedFile {
            path: "escape.md".into(),
            bytes: None,
            link_target: Some("../../outside/secret".into()),
            is_symlink: true,
        };
        assert_eq!(rules(&[link]), vec![("symlink_escape", "escape.md".to_string())]);
        assert_eq!(
            rules(&[file("hooks.json", r#"{"schema":"ctxpect-hooks-v1","on_scan":["curl http://example.invalid"]}"#)]),
            vec![("passive_scan_exec", "hooks.json".to_string())]
        );
        assert!(rules(&[file("hooks.json", r#"{"schema":"ctxpect-hooks-v1","on_scan":[]}"#)]).is_empty());
        assert_eq!(
            rules(&[file("plan.json", r#"{"schema":"ctxpect-plan-v1","drops":["scoped-rule"]}"#)]),
            vec![("unapproved_lossy_projection", "plan.json".to_string())],
            "a missing approval is not an approval"
        );
    }

    #[test]
    fn frontmatter_and_dates_are_parsed_not_pattern_matched() {
        assert!(frontmatter_line_valid("name: ok"));
        assert!(frontmatter_line_valid("globs: \"src/**\""));
        assert!(!frontmatter_line_valid("name: [unterminated"));
        assert!(!frontmatter_line_valid("no colon here"));
        assert_eq!(parse_date("2019-01-01"), Some((2019, 1, 1)));
        assert_eq!(parse_date("2026-09-01"), Some((2026, 9, 1)));
        assert_eq!(days_from_civil(2026, 9, 4) - days_from_civil(2025, 9, 4), 365);
        assert_eq!(
            rules(&[file("AGENTS.md", "---\nupdated: 2019-01-01\n---\nbody\n")]),
            vec![("stale", "AGENTS.md".to_string())]
        );
        assert!(rules(&[file("AGENTS.md", "---\nupdated: 2026-09-01\n---\nbody\n")]).is_empty());
        assert_eq!(
            rules(&[file("SKILL.md", "---\nname: [unterminated\n---\nbody\n")]),
            vec![("bad_frontmatter", "SKILL.md".to_string())]
        );
    }

    #[test]
    fn stale_is_judged_against_the_explicit_as_of_not_a_hidden_clock() {
        // `updated: 2025-09-10` crosses the 365-day threshold between
        // as_of 2026-09-04 (359 days: fresh) and 2026-09-11 (366 days: stale).
        let docs = [file("AGENTS.md", "---\nupdated: 2025-09-10\n---\nbody\n")];
        assert!(rules_as_of(&docs, (2026, 9, 4)).is_empty());
        assert_eq!(
            rules_as_of(&docs, (2026, 9, 11)),
            vec![("stale", "AGENTS.md".to_string())]
        );
        // The finding names the date it was judged against, so a re-analysis
        // of old evidence cannot be mistaken for a fresh observation.
        let found = project_findings(&docs, (2026, 9, 11));
        assert_eq!(
            found.first().map(|f| f.message.as_str()),
            Some("frontmatter `updated:` is more than 365 days before the as-of date 2026-09-11")
        );
        // The frozen corpus reference date reproduces the golden rows.
        assert_eq!(
            rules_as_of(&[file("OLD.md", "---\nupdated: 2019-01-01\n---\nold\n")], STALE_CUTOFF),
            vec![("stale", "OLD.md".to_string())]
        );
    }

    #[test]
    fn civil_date_conversion_round_trips_and_strict_parse_rejects_impossible_dates() {
        for date in [(1970, 1, 1), (2026, 9, 4), (2026, 9, 11), (2000, 2, 29), (2024, 2, 29)] {
            let days = days_from_civil(date.0, date.1, date.2);
            assert_eq!(civil_from_days(days), date, "{date:?}");
        }
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(parse_civil_date("2026-09-04"), Some((2026, 9, 4)));
        assert_eq!(parse_civil_date("2024-02-29"), Some((2024, 2, 29)));
        for bad in ["2026-13-01", "2026-02-30", "2025-02-29", "2026-9-4", "2026/09/04", "abcd-09-04", "2026-09-04 ", "2026-09-04T00:00:00Z"] {
            assert_eq!(parse_civil_date(bad), None, "{bad}");
        }
    }

    #[test]
    fn duplicate_and_conflict_name_the_second_file() {
        assert_eq!(
            rules(&[file("AGENTS.md", "same\n"), file("AGENTS.copy.md", "same\n")]),
            vec![("duplicate", "AGENTS.md".to_string())]
        );
        assert_eq!(
            rules(&[
                file("AGENTS.md", "Package manager must be npm.\n"),
                file("CLAUDE.md", "Package manager must be pnpm.\n"),
            ]),
            vec![("conflict", "CLAUDE.md".to_string())]
        );
        assert!(rules(&[
            file("AGENTS.md", "Package manager must be npm.\n"),
            file("OTHER.md", "Package manager must be npm.\n"),
        ])
        .iter()
        .all(|(rule, _)| *rule != "conflict"));
    }

    /// C-F04: a finding read from a self-declared file is marked as
    /// declaration-validation, names the source file, records that no trusted
    /// producer is connected, and never reads `approved: true` as
    /// authorization evidence. Content findings carry no such marker.
    #[test]
    fn declaration_findings_carry_declaration_validation_metadata() {
        let found = project_findings(
            &[file(
                ".ctxpect/plan.json",
                r#"{"drops":["scoped-rule"],"declared_at":"2026-09-01T00:00:00Z"}"#,
            )],
            STALE_CUTOFF,
        );
        assert_eq!(found.len(), 1, "{found:?}");
        let declaration = found[0].declaration.as_ref().expect("declaration metadata");
        assert_eq!(declaration.source_file, ".ctxpect/plan.json");
        assert_eq!(declaration.producer, "unknown");
        assert_eq!(declaration.declared_at.as_deref(), Some("2026-09-01T00:00:00Z"));
        assert!(
            found[0].message.contains("self-declaration, not authorization evidence"),
            "{}",
            found[0].message
        );

        let rendered = render_project_findings(&found);
        let declaration = rendered[0].get("declaration").expect("declaration in output");
        assert_eq!(
            declaration.get("kind").and_then(Value::as_str),
            Some("declaration-validation")
        );
        assert_eq!(
            declaration.get("trusted_producer_connected").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            declaration.get("declared_at").and_then(Value::as_str),
            Some("2026-09-01T00:00:00Z")
        );

        // A declaration with no declared_at reports null, not an invented time.
        let found = project_findings(&[file(".ctxpect/plan.json", r#"{"drops":["x"]}"#)], STALE_CUTOFF);
        let rendered = render_project_findings(&found);
        assert_eq!(
            rendered[0].pointer(&["declaration", "declared_at"]),
            Some(&Value::Null)
        );

        // Content findings are not declaration-validation.
        let content = project_findings(&[file("AGENTS.md", "over\u{200D}ride\n")], STALE_CUTOFF);
        assert_eq!(content.len(), 1);
        assert!(content[0].declaration.is_none());
        assert!(render_project_findings(&content)[0].get("declaration").is_none());
    }
}
