//! age + SSHSIG transport. Keys and trust are local profile inputs, never bundle inputs.
use crate::tool_process::{self, Scratch};
use ctxpect_schema::{Value, array, canonical_json, object, parse, sha256_text, string};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    time::Duration,
};

type Result<T> = std::result::Result<T, &'static str>;
pub const SCHEMA: &str = "ctxpect-age-ssh-envelope-v1";
fn text<'a>(v: &'a Value, k: &str) -> Result<&'a str> {
    v.get(k)
        .and_then(Value::as_str)
        .ok_or("sync.profile_invalid")
}
fn integer(v: &Value, k: &str) -> Result<i64> {
    v.get(k)
        .and_then(Value::as_i64)
        .ok_or("sync.envelope_invalid")
}
fn strings(v: &Value, k: &str) -> Result<Vec<String>> {
    v.get(k)
        .and_then(Value::as_array)
        .ok_or("sync.profile_invalid")?
        .iter()
        .map(|v| v.as_str().map(str::to_owned).ok_or("sync.profile_invalid"))
        .collect()
}
fn short_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 80
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        && !matches!(s, "." | "..")
}
fn now() -> i64 {
    crate::dispatch::now_unix()
}
pub fn read_json(path: &Path) -> Result<Value> {
    let bytes = tool_process::read_small(path).map_err(|_| "sync.input_unreadable")?;
    parse(std::str::from_utf8(&bytes).map_err(|_| "sync.input_invalid")?)
        .map_err(|_| "sync.input_invalid")
}
pub struct Profile {
    value: Value,
    pub group: String,
    pub digest: String,
    age: PathBuf,
    ssh: PathBuf,
}
impl Profile {
    pub fn load(path: &Path) -> Result<Self> {
        let value = read_json(path)?;
        if text(&value, "schema")? != "ctxpect-age-ssh-profile-v1"
            || !short_id(text(&value, "group")?)
        {
            return Err("sync.profile_invalid");
        }
        if integer(&value, "trust_valid_until")? <= now() {
            return Err("sync.trust_stale");
        }
        if integer(&value, "epoch")? < 1 {
            return Err("sync.profile_invalid");
        }
        let recipients = strings(&value, "recipients")?;
        if recipients.is_empty()
            || recipients.len() > 32
            || recipients.windows(2).any(|p| p[0] >= p[1])
            || recipients.iter().any(|r| {
                !r.starts_with("age1")
                    || r.len() != 62
                    || !r
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
            })
        {
            return Err("sync.recipients_invalid");
        }
        let signers = value
            .get("signers")
            .and_then(Value::as_object)
            .ok_or("sync.profile_invalid")?;
        if signers.is_empty()
            || signers.iter().any(|(id, recipient)| {
                !short_id(id)
                    || !recipient
                        .as_str()
                        .is_some_and(|r| recipients.iter().any(|a| a == r))
            })
        {
            return Err("sync.signers_invalid");
        }
        let age = tool_process::pinned_tool(&value, "age").map_err(|_| "sync.tool_drift")?;
        let ssh = tool_process::pinned_tool(&value, "ssh_keygen").map_err(|_| "sync.tool_drift")?;
        let work = Scratch::new().map_err(|_| "sync.io")?;
        let version = tool_process::run(
            &age,
            &["--version".into()],
            &[],
            &work.0,
            Duration::from_secs(5),
        )
        .map_err(|_| "sync.tool_drift")?;
        if version.code != Some(0)
            || version.timed_out
            || std::str::from_utf8(&version.stdout).map(str::trim) != Ok("v1.3.2")
        {
            return Err("sync.age_version_mismatch");
        }
        let digest = sha256_text(&canonical_json(&value));
        Ok(Self {
            group: text(&value, "group")?.into(),
            value,
            digest,
            age,
            ssh,
        })
    }
    fn path(&self, key: &str) -> Result<String> {
        let p = text(&self.value, key)?;
        if !Path::new(p).is_absolute() {
            return Err("sync.profile_invalid");
        }
        Ok(p.into())
    }
    fn call(&self, tool: &Path, args: Vec<String>, input: &[u8], cwd: &Path) -> Result<Vec<u8>> {
        let out = tool_process::run(tool, &args, input, cwd, Duration::from_secs(20))
            .map_err(|_| "sync.tool_failed")?;
        if out.timed_out {
            return Err("sync.tool_timeout");
        }
        if out.code != Some(0) {
            return Err("sync.tool_rejected");
        }
        Ok(out.stdout)
    }
}

fn validate_payload(payload: &Value) -> Result<()> {
    let map = payload.as_object().ok_or("sync.payload_invalid")?;
    if map.len() != 2 || text(payload, "schema")? != "ctxpect-sync-assets-v1" {
        return Err("sync.payload_invalid");
    }
    let assets = payload
        .get("assets")
        .and_then(Value::as_array)
        .ok_or("sync.payload_invalid")?;
    if assets.is_empty() || assets.len() > 256 {
        return Err("sync.payload_invalid");
    }
    let mut ids = BTreeSet::new();
    for asset in assets {
        if asset.as_object().is_none_or(|m| m.len() != 4) {
            return Err("sync.payload_invalid");
        }
        let id = text(asset, "id")?;
        let body = text(asset, "content")?;
        if !short_id(id)
            || !ids.insert(id)
            || text(asset, "classification")? != "device-group"
            || text(asset, "kind")? != "instruction"
        {
            return Err("sync.asset_not_allowed");
        }
        if ctxpect_doctor::contains_secret(body) {
            return Err("sync.secret_in_bundle");
        }
    }
    Ok(())
}
fn head_digest(head: Option<&Value>) -> String {
    head.map(|v| sha256_text(&canonical_json(v)))
        .unwrap_or_default()
}
pub fn seal(profile: &Profile, payload: &Value, head: Option<&Value>) -> Result<Value> {
    validate_payload(payload)?;
    let sender = text(&profile.value, "sender")?;
    if profile.value.pointer(&["signers", sender]).is_none() {
        return Err("sync.signer_not_member");
    }
    let generation = match head {
        Some(h) => integer(h, "generation")?
            .checked_add(1)
            .ok_or("sync.generation_overflow")?,
        None => 1,
    };
    let scratch = Scratch::new().map_err(|_| "sync.io")?;
    let mut args = vec!["--encrypt".into(), "--armor".into()];
    for r in strings(&profile.value, "recipients")? {
        args.extend(["--recipient".into(), r]);
    }
    let cipher = profile.call(
        &profile.age,
        args,
        canonical_json(payload).as_bytes(),
        &scratch.0,
    )?;
    let ciphertext = String::from_utf8(cipher).map_err(|_| "sync.cipher_invalid")?;
    let envelope = object([
        ("schema", string(SCHEMA)),
        ("group", string(&profile.group)),
        ("generation", Value::Int(generation)),
        ("parent", string(head_digest(head))),
        ("epoch", Value::Int(integer(&profile.value, "epoch")?)),
        (
            "recipients",
            profile
                .value
                .get("recipients")
                .cloned()
                .ok_or("sync.profile_invalid")?,
        ),
        ("sender", string(sender)),
        ("ciphertext_digest", string(sha256_text(&ciphertext))),
        ("ciphertext", string(ciphertext)),
    ]);
    let bytes = canonical_json(&envelope);
    let signature = profile.call(
        &profile.ssh,
        vec![
            "-Y".into(),
            "sign".into(),
            "-f".into(),
            profile.path("signing_key")?,
            "-n".into(),
            "ctxpect-sync-v1".into(),
        ],
        bytes.as_bytes(),
        &scratch.0,
    )?;
    Ok(object([
        ("envelope", envelope),
        (
            "signature",
            string(String::from_utf8(signature).map_err(|_| "sync.signature_invalid")?),
        ),
    ]))
}

/// Verify authenticity before decryption. Returned plaintext stays in local memory.
pub fn open(profile: &Profile, document: &Value) -> Result<Value> {
    if document.as_object().is_none_or(|m| m.len() != 2) {
        return Err("sync.envelope_invalid");
    }
    let envelope = document.get("envelope").ok_or("sync.envelope_invalid")?;
    if envelope.as_object().is_none_or(|m| m.len() != 9)
        || text(envelope, "schema")? != SCHEMA
        || text(envelope, "group")? != profile.group
    {
        return Err("sync.envelope_invalid");
    }
    if integer(envelope, "generation")? < 1
        || integer(envelope, "epoch")? != integer(&profile.value, "epoch")?
        || envelope.get("recipients") != profile.value.get("recipients")
    {
        return Err("sync.recipient_epoch_mismatch");
    }
    let parent = text(envelope, "parent")?;
    if !(parent.is_empty() || (parent.len() == 64 && parent.bytes().all(|b| b.is_ascii_hexdigit())))
    {
        return Err("sync.envelope_invalid");
    }
    let sender = text(envelope, "sender")?;
    if profile.value.pointer(&["signers", sender]).is_none() {
        return Err("sync.signer_not_member");
    }
    let cipher = text(envelope, "ciphertext")?;
    if text(envelope, "ciphertext_digest")? != sha256_text(cipher) {
        return Err("sync.cipher_digest_mismatch");
    }
    let scratch = Scratch::new().map_err(|_| "sync.io")?;
    let signature = scratch.0.join("signature");
    tool_process::private_write(&signature, text(document, "signature")?.as_bytes())
        .map_err(|_| "sync.io")?;
    profile
        .call(
            &profile.ssh,
            vec![
                "-Y".into(),
                "verify".into(),
                "-f".into(),
                profile.path("allowed_signers")?,
                "-r".into(),
                profile.path("revoked_signers")?,
                "-I".into(),
                sender.into(),
                "-n".into(),
                "ctxpect-sync-v1".into(),
                "-s".into(),
                signature.to_string_lossy().into_owned(),
            ],
            canonical_json(envelope).as_bytes(),
            &scratch.0,
        )
        .map_err(|_| "sync.signature_untrusted")?;
    let plaintext = profile
        .call(
            &profile.age,
            vec![
                "--decrypt".into(),
                "--identity".into(),
                profile.path("identity")?,
            ],
            cipher.as_bytes(),
            &scratch.0,
        )
        .map_err(|_| "sync.decrypt_failed")?;
    let payload = parse(std::str::from_utf8(&plaintext).map_err(|_| "sync.payload_invalid")?)
        .map_err(|_| "sync.payload_invalid")?;
    validate_payload(&payload)?;
    Ok(payload)
}

pub fn check_chain(document: &Value, head: Option<&Value>) -> Result<()> {
    let incoming = document.get("envelope").ok_or("sync.envelope_invalid")?;
    if let Some(current) = head {
        if sha256_text(&canonical_json(incoming)) == head_digest(Some(current)) {
            return Err("sync.replay");
        }
        if integer(incoming, "generation")? == integer(current, "generation")?
            && text(incoming, "parent")? == text(current, "parent")?
        {
            return Err("sync.fork");
        }
        if integer(incoming, "generation")? <= integer(current, "generation")? {
            return Err("sync.rollback");
        }
    }
    let expected = head
        .map(|h| integer(h, "generation"))
        .transpose()?
        .unwrap_or(0)
        .checked_add(1)
        .ok_or("sync.generation_overflow")?;
    if integer(incoming, "generation")? != expected
        || text(incoming, "parent")? != head_digest(head)
    {
        return Err("sync.parent_mismatch");
    }
    Ok(())
}

pub fn preview_summary(payload: &Value, document: &Value) -> Value {
    let assets = payload
        .get("assets")
        .and_then(Value::as_array)
        .unwrap_or(&[]);
    let summaries = assets.iter().map(|a| {
        object([
            ("id", a.get("id").cloned().unwrap_or(Value::Null)),
            (
                "content_bytes",
                Value::Int(
                    a.get("content")
                        .and_then(Value::as_str)
                        .map_or(0, |s| s.len()) as i64,
                ),
            ),
        ])
    });
    object([
        ("transport", string("verified-envelope")),
        ("semantic", string("structural-only")),
        ("runtime_verification", string("not-observed")),
        (
            "envelope_digest",
            string(sha256_text(&canonical_json(document))),
        ),
        ("assets", array(summaries)),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    fn doc(generation: i64, parent: &str, body: &str) -> Value {
        object([(
            "envelope",
            object([
                ("generation", Value::Int(generation)),
                ("parent", string(parent)),
                ("ciphertext", string(body)),
            ]),
        )])
    }
    #[test]
    fn same_parent_fork_is_not_replay_and_chain_cannot_skip() {
        let first = doc(1, "", "a");
        let head = first.get("envelope").unwrap();
        assert!(check_chain(&first, None).is_ok());
        assert_eq!(check_chain(&first, Some(head)).unwrap_err(), "sync.replay");
        assert_eq!(
            check_chain(&doc(1, "", "b"), Some(head)).unwrap_err(),
            "sync.fork"
        );
        assert_eq!(
            check_chain(&doc(3, &head_digest(Some(head)), "b"), Some(head)).unwrap_err(),
            "sync.parent_mismatch"
        );
        assert!(check_chain(&doc(2, &head_digest(Some(head)), "b"), Some(head)).is_ok());
    }
    #[test]
    fn metadata_only_summary_contains_neither_body_nor_plaintext_digest() {
        let payload=parse(r#"{"schema":"ctxpect-sync-assets-v1","assets":[{"id":"i","kind":"instruction","classification":"device-group","content":"sensitive low entropy"}]}"#).unwrap();
        validate_payload(&payload).unwrap();
        let summary = canonical_json(&preview_summary(&payload, &doc(1, "", "encrypted")));
        assert!(!summary.contains("sensitive low entropy"));
        assert!(!summary.contains(&sha256_text("sensitive low entropy")));
        let mut secret = payload.clone();
        if let Value::Object(map) = &mut secret {
            map.insert("unexpected".into(), string("body"));
        }
        assert_eq!(
            validate_payload(&secret).unwrap_err(),
            "sync.payload_invalid"
        );
    }
}
