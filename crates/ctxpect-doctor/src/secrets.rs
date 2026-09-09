//! Secret-literal grammar shared by Doctor (`secret_literal`) and the
//! projection secret gate.
//!
//! The grammar is a closed list of credential *shapes*, not a list of
//! suspicious words: `API_KEY=documentation-only-placeholder` is a
//! placeholder, `ghp_` followed by a token body is a credential. A blocking
//! rule must not fire on the former, so nothing here matches a bare key name
//! or a bare vendor prefix.

/// A matched secret shape, by class. The literal is never returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretClass {
    PrivateKeyBlock,
    AwsAccessKeyId,
    AwsSecretAssignment,
    GitHubToken,
    SlackToken,
    OpenAiStyleKey,
    BearerAuthorization,
}

impl SecretClass {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            SecretClass::PrivateKeyBlock => "private-key-block",
            SecretClass::AwsAccessKeyId => "aws-access-key-id",
            SecretClass::AwsSecretAssignment => "aws-secret-assignment",
            SecretClass::GitHubToken => "github-token",
            SecretClass::SlackToken => "slack-token",
            SecretClass::OpenAiStyleKey => "openai-style-key",
            SecretClass::BearerAuthorization => "bearer-authorization",
        }
    }
}

fn is_token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'
}

/// Length of the token body starting at `from`, over `is_token_byte`.
fn token_run(bytes: &[u8], from: usize) -> usize {
    bytes[from..].iter().take_while(|b| is_token_byte(**b)).count()
}

/// Whether `prefix` occurs at a token boundary followed by at least
/// `min_body` token bytes.
fn prefixed_token(text: &str, prefix: &str, min_body: usize) -> bool {
    let bytes = text.as_bytes();
    let needle = prefix.as_bytes();
    let mut i = 0usize;
    while i + needle.len() <= bytes.len() {
        if bytes[i..].starts_with(needle) && (i == 0 || !is_token_byte(bytes[i - 1])) {
            let body = token_run(bytes, i + needle.len());
            if body >= min_body {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Bytes a credential body is made of. A value that starts with anything
/// else — `<your-key>`, `${ENV}`, `{{template}}`, `****` — is a placeholder
/// shape, not a secret shape.
fn is_value_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'/' | b'+' | b'=' | b'.')
}

/// Placeholder words that make an otherwise well-formed value documentation
/// rather than a credential (`YOUR_TOKEN_HERE`, `EXAMPLE_KEY`).
const PLACEHOLDER_WORDS: &[&str] = &["your", "placeholder", "example", "redacted", "changeme", "xxxx"];

fn looks_like_placeholder(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    PLACEHOLDER_WORDS.iter().any(|word| lower.contains(word))
}

/// Length of the credential-shaped run at `from`, or 0 for a placeholder.
fn value_run(text: &str, from: usize) -> usize {
    let bytes = text.as_bytes();
    let len = bytes[from..].iter().take_while(|b| is_value_byte(**b)).count();
    if len == 0 || looks_like_placeholder(&text[from..from + len]) {
        0
    } else {
        len
    }
}

/// Skip an optional quote, then whitespace. Returns the new index.
fn skip_quote_ws(bytes: &[u8], mut j: usize) -> usize {
    if j < bytes.len() && (bytes[j] == b'"' || bytes[j] == b'\'') {
        j += 1;
    }
    while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
        j += 1;
    }
    j
}

/// `KEY <ws>* [=:] <ws>* <value>` — in `.env`, YAML, TOML and JSON spellings
/// (`KEY="v"`, `"key": "v"`) — where value is at least `min` credential
/// bytes.
fn assignment_with_value(text: &str, key: &str, min: usize) -> bool {
    let lower = text.to_ascii_lowercase();
    let key = key.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut search = 0usize;
    while let Some(pos) = lower[search..].find(&key) {
        let at = search + pos;
        let mut j = skip_quote_ws(bytes, at + key.len());
        if j < bytes.len() && (bytes[j] == b'=' || bytes[j] == b':') {
            j += 1;
            while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
                j += 1;
            }
            j = skip_quote_ws(bytes, j);
            if value_run(&lower, j) >= min {
                return true;
            }
        }
        search = at + key.len();
    }
    false
}

fn aws_access_key_id(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i + 20 <= bytes.len() {
        if bytes[i..].starts_with(b"AKIA")
            && (i == 0 || !is_token_byte(bytes[i - 1]))
            && bytes[i + 4..i + 20]
                .iter()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
            && (i + 20 == bytes.len() || !is_token_byte(bytes[i + 20]))
        {
            return true;
        }
        i += 1;
    }
    false
}

/// `Authorization: Bearer <token>` as a header line or as a JSON pair
/// (`"authorization": "bearer <token>"`), token ≥ 16 credential bytes.
fn bearer_authorization(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut search = 0usize;
    while let Some(pos) = lower[search..].find("authorization") {
        let at = search + pos + "authorization".len();
        let mut j = skip_quote_ws(bytes, at);
        if j < bytes.len() && bytes[j] == b':' {
            j += 1;
            while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
                j += 1;
            }
            j = skip_quote_ws(bytes, j);
            if lower[j..].starts_with("bearer ") {
                let start = j + "bearer ".len();
                let start = skip_quote_ws(bytes, start);
                if value_run(&lower, start) >= 16 {
                    return true;
                }
            }
        }
        search = at;
    }
    false
}

/// A PEM private-key armor line: `-----BEGIN <words> PRIVATE KEY-----` on
/// one line. Prose that merely mentions both halves is not a key block.
fn private_key_block(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim();
        line.starts_with("-----BEGIN ") && line.ends_with("PRIVATE KEY-----")
    })
}

/// `sk-` keys: a body of at least 20 token bytes that mixes letters and
/// digits. A word run like `sk-learn-compatible-transformers` is prose.
fn openai_style_key(text: &str) -> bool {
    let bytes = text.as_bytes();
    let needle = b"sk-";
    let mut i = 0usize;
    while i + needle.len() <= bytes.len() {
        if bytes[i..].starts_with(needle) && (i == 0 || !is_token_byte(bytes[i - 1])) {
            let body = &bytes[i + needle.len()..];
            let len = body.iter().take_while(|b| is_token_byte(**b)).count();
            let run = &body[..len];
            if len >= 20
                && run.iter().any(u8::is_ascii_digit)
                && run.iter().any(u8::is_ascii_alphabetic)
            {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Classify the first secret shape found in `text`, if any.
#[must_use]
pub fn secret_literal(text: &str) -> Option<SecretClass> {
    if private_key_block(text) {
        return Some(SecretClass::PrivateKeyBlock);
    }
    if aws_access_key_id(text) {
        return Some(SecretClass::AwsAccessKeyId);
    }
    if assignment_with_value(text, "aws_secret_access_key", 16) {
        return Some(SecretClass::AwsSecretAssignment);
    }
    if prefixed_token(text, "ghp_", 20)
        || prefixed_token(text, "github_pat_", 20)
        || prefixed_token(text, "gho_", 20)
        || prefixed_token(text, "ghs_", 20)
    {
        return Some(SecretClass::GitHubToken);
    }
    if prefixed_token(text, "xoxb-", 10)
        || prefixed_token(text, "xoxp-", 10)
        || prefixed_token(text, "xoxa-", 10)
        || prefixed_token(text, "xoxr-", 10)
        || prefixed_token(text, "xoxs-", 10)
    {
        return Some(SecretClass::SlackToken);
    }
    if openai_style_key(text) {
        return Some(SecretClass::OpenAiStyleKey);
    }
    if bearer_authorization(text) {
        return Some(SecretClass::BearerAuthorization);
    }
    None
}

/// Convenience: whether any secret shape is present.
#[must_use]
pub fn contains_secret(text: &str) -> bool {
    secret_literal(text).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_shapes_match_and_placeholders_do_not() {
        assert_eq!(
            secret_literal("TOKEN=ghp_fixture_not_a_real_secret_00\n"),
            Some(SecretClass::GitHubToken)
        );
        assert_eq!(
            secret_literal("key sk-abcdefghijklmnopqrstuvwxyz0123"),
            Some(SecretClass::OpenAiStyleKey)
        );
        assert_eq!(
            secret_literal("-----BEGIN RSA PRIVATE KEY-----\nabc\n-----END RSA PRIVATE KEY-----"),
            Some(SecretClass::PrivateKeyBlock)
        );
        assert_eq!(secret_literal("id AKIAABCDEFGHIJKLMNOP end"), Some(SecretClass::AwsAccessKeyId));
        assert_eq!(
            secret_literal("Authorization: Bearer 0123456789abcdef0123"),
            Some(SecretClass::BearerAuthorization)
        );
        assert_eq!(
            secret_literal("xoxb-1234567890-abcdefghij"),
            Some(SecretClass::SlackToken)
        );

        // Look-alikes: a key *name*, a short prefix, a placeholder value.
        assert_eq!(secret_literal("API_KEY=documentation-only-placeholder"), None);
        assert_eq!(secret_literal("use sk- prefixes for keys"), None);
        assert_eq!(secret_literal("ghp_ tokens start like this"), None);
        assert_eq!(secret_literal("## always\nUse pytest for tests.\n"), None);
        assert_eq!(secret_literal("aws_secret_access_key = <redacted>"), None);
        assert_eq!(secret_literal("Authorization: Bearer <token>"), None);
    }

    #[test]
    fn quoted_json_and_placeholder_spellings_are_classified_the_same_as_bare_ones() {
        let body = "a".repeat(40);
        // Quoted and JSON spellings of a real-shaped value are credentials.
        assert!(secret_literal(&format!("aws_secret_access_key = \"{body}\"")).is_some());
        assert!(secret_literal(&format!("\"aws_secret_access_key\": \"{body}\"")).is_some());
        assert!(secret_literal(&format!("{{\"Authorization\": \"Bearer {body}\"}}")).is_some());
        assert!(secret_literal(&format!("headers:\n  Authorization: 'Bearer {body}'")).is_some());
        // Placeholder shapes are documentation, however they are quoted.
        for text in [
            "Authorization: Bearer <your-access-token>",
            "Authorization: Bearer $ACCESS_TOKEN_VALUE",
            "Authorization: Bearer YOUR_TOKEN_HERE_1234",
            "Authorization: Bearer {{access_token}}",
            "aws_secret_access_key = ${AWS_SECRET_ACCESS_KEY}",
            "AWS_SECRET_ACCESS_KEY=<your-secret-access-key>",
            "aws_secret_access_key = ****************",
            "aws_secret_access_key = \"EXAMPLE_KEY_DO_NOT_USE_000\"",
            "Use sk-learn-compatible-transformers-with-fit here",
            "class=\"sk-circle-bounce-animation-loader\"",
            "-----BEGIN PGP SIGNED MESSAGE-----\nthe words PRIVATE KEY----- in prose",
            "| `aws_secret_access_key=<≥16>` | key shape |",
        ] {
            assert_eq!(secret_literal(text), None, "{text}");
        }
        // A real armor line still is one.
        assert!(secret_literal("-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXkt\n").is_some());
    }
}
