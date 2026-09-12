//! A minimal JSON value, parser and canonical serializer.
//!
//! Canonical form is byte-compatible with the acceptance generator's
//! `canonical_json`, i.e. Python's
//! `json.dumps(obj, ensure_ascii=False, sort_keys=True, separators=(",", ":"))`,
//! **over the JSON subset this crate supports** — see the crate docs for the
//! exact boundary. Digests computed here must equal the ones frozen by that
//! generator, so the escaping and key-ordering rules below are a contract, not a
//! style choice.

use std::collections::BTreeMap;
use std::fmt::Write as _;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    /// A non-integer JSON number kept as the exact lexeme it was read with.
    /// Only [`parse_preserving_numbers`] produces it — the strict [`parse`]
    /// still refuses floats, so product documents never carry one — and it
    /// is written back verbatim. It is a *representation*, not a value: no
    /// arithmetic, `as_i64` is `None`, and digest equality with another
    /// implementation holds only when that implementation wrote the same
    /// text (see [`NumberLexeme::is_js_shortest_form`]).
    Number(NumberLexeme),
    Str(String),
    Array(Vec<Value>),
    /// `BTreeMap` keeps keys in code-point order, matching `sort_keys=True`.
    Object(BTreeMap<String, Value>),
}

/// The text of a JSON number that is not an integer, validated against the
/// JSON number grammar (RFC 8259 §6) when constructed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberLexeme(String);

impl NumberLexeme {
    /// Accept `text` if it is exactly one JSON number.
    pub fn new(text: &str) -> Option<NumberLexeme> {
        let bytes = text.as_bytes();
        let mut i = 0;
        if bytes.first() == Some(&b'-') {
            i += 1;
        }
        let int_start = i;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if i == int_start || (bytes[int_start] == b'0' && i - int_start > 1) {
            return None;
        }
        if bytes.get(i) == Some(&b'.') {
            i += 1;
            let frac = i;
            while bytes.get(i).is_some_and(u8::is_ascii_digit) {
                i += 1;
            }
            if i == frac {
                return None;
            }
        }
        if matches!(bytes.get(i), Some(b'e' | b'E')) {
            i += 1;
            if matches!(bytes.get(i), Some(b'+' | b'-')) {
                i += 1;
            }
            let exp = i;
            while bytes.get(i).is_some_and(u8::is_ascii_digit) {
                i += 1;
            }
            if i == exp {
                return None;
            }
        }
        if i != bytes.len() {
            return None;
        }
        Some(NumberLexeme(text.to_string()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether the lexeme has the syntactic shape `JSON.stringify` gives a
    /// finite double: no redundant fraction zeros, a normalized nonzero
    /// mantissa, and exponent notation only outside [1e-6, 1e21).
    /// This is a rejection filter, not a canonical writer or a proof of
    /// floating-point shortest-round-trip equivalence. Preserving an actual
    /// JS writer's lexeme retains its bytes; matching this shape alone does
    /// not establish which writer produced an arbitrary input.
    #[must_use]
    pub fn is_js_shortest_form(&self) -> bool {
        let text = self.0.as_str();
        let unsigned = text.strip_prefix('-').unwrap_or(text);
        let Ok(value) = text.parse::<f64>() else { return false; };
        if !value.is_finite() || value == 0.0 && text != "0" {
            return false;
        }
        let (mantissa, exponent) = match unsigned.split_once('e') {
            Some((m, e)) => (m, Some(e)),
            None => (unsigned, None),
        };
        if unsigned.contains('E') || mantissa.ends_with('0') && mantissa.contains('.') {
            return false;
        }
        let Some(exponent) = exponent else {
            return value == 0.0 || (1e-6..1e21).contains(&value.abs());
        };
        let integer = mantissa.split('.').next().unwrap_or("");
        if integer.len() != 1 || integer == "0" {
            return false;
        }
        let (sign, digits) = match exponent.as_bytes().first() {
            Some(b'+') => (1i32, &exponent[1..]),
            Some(b'-') => (-1i32, &exponent[1..]),
            _ => return false,
        };
        if digits.is_empty() || digits.starts_with('0') || mantissa.contains('.') && mantissa.ends_with('0') {
            return false;
        }
        let Ok(magnitude) = digits.parse::<i32>() else {
            return false;
        };
        // JS uses exponent form for |x| >= 1e21 and 0 < |x| < 1e-6 only.
        (sign > 0 && magnitude >= 21) || (sign < 0 && magnitude >= 7)
    }
}

impl Value {
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(map) => map.get(key),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_object(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Value::Object(map) => Some(map),
            _ => None,
        }
    }

    /// Walk a sequence of object keys. Missing keys or non-objects yield `None`.
    #[must_use]
    pub fn pointer(&self, keys: &[&str]) -> Option<&Value> {
        let mut cur = self;
        for key in keys {
            cur = cur.get(key)?;
        }
        Some(cur)
    }

    /// String values of an array, or the single string value, as a list.
    /// Frozen contracts use both shapes for the same field.
    #[must_use]
    pub fn as_str_list(&self) -> Vec<&str> {
        match self {
            Value::Str(s) => vec![s.as_str()],
            Value::Array(items) => items.iter().filter_map(Value::as_str).collect(),
            _ => Vec::new(),
        }
    }
}

fn escape_into(out: &mut String, text: &str) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            // ensure_ascii=False: every other character is emitted as itself.
            c => out.push(c),
        }
    }
    out.push('"');
}

fn write_value(out: &mut String, value: &Value) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Int(n) => {
            let _ = write!(out, "{n}");
        }
        Value::Number(lexeme) => out.push_str(lexeme.as_str()),
        Value::Str(s) => escape_into(out, s),
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_value(out, item);
            }
            out.push(']');
        }
        Value::Object(map) => {
            out.push('{');
            for (index, (key, item)) in map.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                escape_into(out, key);
                out.push(':');
                write_value(out, item);
            }
            out.push('}');
        }
    }
}

/// Object constructor used by Receipt/API crates. Keys are sorted by `BTreeMap`.
#[must_use]
pub fn object<K: Into<String>>(pairs: impl IntoIterator<Item = (K, Value)>) -> Value {
    Value::Object(pairs.into_iter().map(|(k, v)| (k.into(), v)).collect())
}

/// Array constructor.
#[must_use]
pub fn array(items: impl IntoIterator<Item = Value>) -> Value {
    Value::Array(items.into_iter().collect())
}

/// String constructor.
#[must_use]
pub fn string(text: impl Into<String>) -> Value {
    Value::Str(text.into())
}

/// Optional string: `None` becomes JSON null.
#[must_use]
pub fn opt_string(text: Option<impl Into<String>>) -> Value {
    match text {
        Some(text) => string(text),
        None => Value::Null,
    }
}

const TIME_KEYS: &[&str] = &[
    "generated_at",
    "timestamp",
    "inspected_at",
    "created_at",
    "emitted_at",
    "now",
    "time",
    "deleted_at",
    "applied_at",
    "imported_at",
];

/// Drop time fields so a digest is stable across clock readings.
#[must_use]
pub fn strip_time_fields(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = BTreeMap::new();
            for (key, child) in map {
                if TIME_KEYS.contains(&key.as_str()) {
                    continue;
                }
                out.insert(key.clone(), strip_time_fields(child));
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(strip_time_fields).collect()),
        other => other.clone(),
    }
}

/// Canonical JSON text: sorted keys, no insignificant whitespace, non-ASCII kept literal.
#[must_use]
pub fn canonical_json(value: &Value) -> String {
    let mut out = String::new();
    write_value(&mut out, value);
    out
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseError {
    pub offset: usize,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid JSON at byte {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for ParseError {}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
    /// Keep non-integer numbers as [`Value::Number`] instead of refusing them.
    preserve_numbers: bool,
    /// Current nesting of arrays and objects. Recursion is bounded by
    /// [`MAX_DEPTH`]: a document of ten thousand `[` would otherwise
    /// overflow the stack and abort the process, which no error path can
    /// catch.
    depth: usize,
}

/// Deepest nesting the parser accepts. Product documents nest a handful of
/// levels; a session log's largest structures stay well under this.
pub const MAX_DEPTH: usize = 128;

impl<'a> Parser<'a> {
    fn err<T>(&self, message: &str) -> Result<T, ParseError> {
        Err(ParseError {
            offset: self.pos,
            message: message.to_string(),
        })
    }

    fn skip_ws(&mut self) {
        while let Some(b) = self.bytes.get(self.pos) {
            if matches!(b, b' ' | b'\t' | b'\n' | b'\r') {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn expect(&mut self, byte: u8) -> Result<(), ParseError> {
        if self.peek() == Some(byte) {
            self.pos += 1;
            Ok(())
        } else {
            self.err(&format!("expected `{}`", byte as char))
        }
    }

    fn literal(&mut self, word: &str, value: Value) -> Result<Value, ParseError> {
        if self.bytes[self.pos..].starts_with(word.as_bytes()) {
            self.pos += word.len();
            Ok(value)
        } else {
            self.err("unknown literal")
        }
    }

    fn parse_string(&mut self) -> Result<String, ParseError> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let Some(byte) = self.peek() else {
                return self.err("unterminated string");
            };
            match byte {
                b'"' => {
                    self.pos += 1;
                    return Ok(out);
                }
                b'\\' => {
                    self.pos += 1;
                    let Some(esc) = self.peek() else {
                        return self.err("unterminated escape");
                    };
                    self.pos += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{08}'),
                        b'f' => out.push('\u{0c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let code = self.parse_hex4()?;
                            // Surrogate pair: a high surrogate must be followed by a low one.
                            if (0xD800..0xDC00).contains(&code) {
                                if self.peek() != Some(b'\\') {
                                    return self.err("lone high surrogate");
                                }
                                self.pos += 1;
                                self.expect(b'u')?;
                                let low = self.parse_hex4()?;
                                if !(0xDC00..0xE000).contains(&low) {
                                    return self.err("invalid low surrogate");
                                }
                                let combined =
                                    0x1_0000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                                match char::from_u32(combined) {
                                    Some(c) => out.push(c),
                                    None => return self.err("invalid surrogate pair"),
                                }
                            } else {
                                match char::from_u32(code) {
                                    Some(c) => out.push(c),
                                    None => return self.err("invalid code point"),
                                }
                            }
                        }
                        _ => return self.err("unknown escape"),
                    }
                }
                // Python's json.loads is strict: a raw control character inside a
                // string is invalid, it must be escaped. Accepting it here would
                // let a document parse for us and fail for the generator.
                0x00..=0x1f => return self.err("unescaped control character in string"),
                _ => {
                    let start = self.pos;
                    while let Some(b) = self.peek() {
                        if b == b'"' || b == b'\\' || b < 0x20 {
                            break;
                        }
                        self.pos += 1;
                    }
                    match std::str::from_utf8(&self.bytes[start..self.pos]) {
                        Ok(chunk) => out.push_str(chunk),
                        Err(_) => return self.err("invalid UTF-8"),
                    }
                }
            }
        }
    }

    fn parse_hex4(&mut self) -> Result<u32, ParseError> {
        if self.pos + 4 > self.bytes.len() {
            return self.err("truncated \\u escape");
        }
        let slice = &self.bytes[self.pos..self.pos + 4];
        let Ok(text) = std::str::from_utf8(slice) else {
            return self.err("invalid \\u escape");
        };
        if !text.bytes().all(|b| b.is_ascii_hexdigit()) {
            // `u32::from_str_radix` would accept a leading `+`; JSON does not.
            return self.err("invalid \\u escape");
        }
        let Ok(code) = u32::from_str_radix(text, 16) else {
            return self.err("invalid \\u escape");
        };
        self.pos += 4;
        Ok(code)
    }

    fn parse_number(&mut self) -> Result<Value, ParseError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        // JSON forbids leading zeros (`01`); Python rejects them and so must we,
        // or a document parses for one side and not the other.
        let int_start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        let int_len = self.pos - int_start;
        if int_len == 0 {
            return self.err("expected a digit");
        }
        if int_len > 1 && self.bytes[int_start] == b'0' {
            return self.err("leading zero in number");
        }
        // Floats are not supported: see the crate docs. Reject them rather than
        // emit a value whose canonical form would differ from the generator's.
        // The number-preserving parser scans the fraction and exponent and
        // keeps the exact lexeme instead.
        if matches!(self.peek(), Some(b'.' | b'e' | b'E')) {
            if !self.preserve_numbers {
                return self.err("floating-point numbers are not supported");
            }
            if self.peek() == Some(b'.') {
                self.pos += 1;
                let frac = self.pos;
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.pos += 1;
                }
                if self.pos == frac {
                    return self.err("expected a digit after the decimal point");
                }
            }
            if matches!(self.peek(), Some(b'e' | b'E')) {
                self.pos += 1;
                if matches!(self.peek(), Some(b'+' | b'-')) {
                    self.pos += 1;
                }
                let exp = self.pos;
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.pos += 1;
                }
                if self.pos == exp {
                    return self.err("expected a digit in the exponent");
                }
            }
        }
        let Ok(text) = std::str::from_utf8(&self.bytes[start..self.pos]) else {
            return self.err("invalid number");
        };
        match text.parse::<i64>() {
            Ok(n) => Ok(Value::Int(n)),
            Err(_) if self.preserve_numbers && (text.contains('.') || text.contains(['e', 'E'])) => {
                NumberLexeme::new(text)
                    .map(Value::Number)
                    .ok_or_else(|| ParseError { message: "invalid number".into(), offset: start })
            }
            Err(_) => self.err("integer out of range"),
        }
    }

    fn enter(&mut self) -> Result<(), ParseError> {
        if self.depth >= MAX_DEPTH {
            return self.err("nesting deeper than the supported maximum");
        }
        self.depth += 1;
        Ok(())
    }

    fn parse_value(&mut self) -> Result<Value, ParseError> {
        self.skip_ws();
        match self.peek() {
            None => self.err("unexpected end of input"),
            Some(b'n') => self.literal("null", Value::Null),
            Some(b't') => self.literal("true", Value::Bool(true)),
            Some(b'f') => self.literal("false", Value::Bool(false)),
            Some(b'"') => Ok(Value::Str(self.parse_string()?)),
            Some(b'[') => {
                self.pos += 1;
                self.enter()?;
                let mut items = Vec::new();
                self.skip_ws();
                if self.peek() == Some(b']') {
                    self.pos += 1;
                    self.depth -= 1;
                    return Ok(Value::Array(items));
                }
                loop {
                    items.push(self.parse_value()?);
                    self.skip_ws();
                    match self.peek() {
                        Some(b',') => self.pos += 1,
                        Some(b']') => {
                            self.pos += 1;
                            self.depth -= 1;
                            return Ok(Value::Array(items));
                        }
                        _ => return self.err("expected `,` or `]`"),
                    }
                }
            }
            Some(b'{') => {
                self.pos += 1;
                self.enter()?;
                let mut map = BTreeMap::new();
                self.skip_ws();
                if self.peek() == Some(b'}') {
                    self.pos += 1;
                    self.depth -= 1;
                    return Ok(Value::Object(map));
                }
                loop {
                    self.skip_ws();
                    let key = self.parse_string()?;
                    self.skip_ws();
                    self.expect(b':')?;
                    let value = self.parse_value()?;
                    map.insert(key, value);
                    self.skip_ws();
                    match self.peek() {
                        Some(b',') => self.pos += 1,
                        Some(b'}') => {
                            self.pos += 1;
                            self.depth -= 1;
                            return Ok(Value::Object(map));
                        }
                        _ => return self.err("expected `,` or `}`"),
                    }
                }
            }
            Some(b'-' | b'0'..=b'9') => self.parse_number(),
            Some(_) => self.err("unexpected character"),
        }
    }
}

/// Parse a complete JSON document. Trailing content is an error.
pub fn parse(text: &str) -> Result<Value, ParseError> {
    parse_with(text, false)
}

/// [`parse`], but a non-integer number becomes [`Value::Number`] holding its
/// exact lexeme instead of an error. For readers of *foreign* documents (a
/// harness's session log) whose digests must reproduce the foreign writer's
/// bytes; product documents keep the strict parser.
pub fn parse_preserving_numbers(text: &str) -> Result<Value, ParseError> {
    parse_with(text, true)
}

fn parse_with(text: &str, preserve_numbers: bool) -> Result<Value, ParseError> {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        pos: 0,
        depth: 0,
        preserve_numbers,
    };
    let value = parser.parse_value()?;
    parser.skip_ws();
    if parser.pos != parser.bytes.len() {
        return parser.err("trailing content after JSON value");
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(pairs: &[(&str, Value)]) -> Value {
        Value::Object(
            pairs
                .iter()
                .map(|(k, v)| ((*k).to_string(), v.clone()))
                .collect(),
        )
    }

    #[test]
    fn sorts_keys_and_omits_whitespace() {
        let value = obj(&[
            ("b", Value::Int(1)),
            ("a", Value::Int(2)),
            ("C", Value::Int(3)),
        ]);
        assert_eq!(canonical_json(&value), r#"{"C":3,"a":2,"b":1}"#);
    }

    #[test]
    fn keeps_non_ascii_literal() {
        let value = obj(&[("k", Value::Str("语义对齐".into()))]);
        assert_eq!(canonical_json(&value), "{\"k\":\"语义对齐\"}");
    }

    #[test]
    fn escapes_control_characters() {
        let value = Value::Str("a\nb\tc\"d\\e\u{01}".into());
        assert_eq!(canonical_json(&value), r#""a\nb\tc\"d\\e\u0001""#);
    }

    #[test]
    fn round_trips_nested_documents() {
        let text = r#"{"z":[1,{"y":null,"x":true}],"a":"值"}"#;
        let parsed = parse(text).expect("parse");
        assert_eq!(canonical_json(&parsed), r#"{"a":"值","z":[1,{"x":true,"y":null}]}"#);
    }

    #[test]
    fn parses_escapes_and_surrogates() {
        let parsed = parse(r#""\u00e9\ud83d\ude00\/""#).expect("parse");
        assert_eq!(parsed, Value::Str("é😀/".into()));
    }

    #[test]
    fn rejects_malformed_input() {
        for bad in [
            "{",
            "[1,]",
            "{\"a\"}",
            "tru",
            "\"unterminated",
            "{} extra",
            "[1 2]",
            "\"\\ud800\"",
            "\"raw \u{1} control\"",
            "\"raw \n newline\"",
        ] {
            assert!(parse(bad).is_err(), "expected {bad:?} to be rejected");
        }
    }

    #[test]
    fn js_number_shape_refuses_non_normalized_and_non_finite_forms() {
        for number in ["0.5", "-0.5", "0.000001", "1e+21", "1.25e-7", "5e-324"] {
            assert!(NumberLexeme::new(number).unwrap().is_js_shortest_form(), "{number}");
        }
        for number in ["10e+21", "0.1e-7", "0e+21", "-0.0", "1e+309", "1e-400", "0.0000001", "1e-6", "1e+20", "1e-07", "1.0"] {
            assert!(!NumberLexeme::new(number).unwrap().is_js_shortest_form(), "{number}");
        }
    }

    #[test]
    fn rejects_floating_point_literals() {
        // Matching Python's shortest-round-trip repr needs a Ryu-class algorithm;
        // until that exists, a float must fail loudly rather than produce a
        // canonical form that silently differs from the generator's.
        for bad in [
            "1.0", "0.5", "1e5", "1E5", "-2.5", "1.", "0.e5", "[1.5]", "{\"k\":1.0}",
            "1e16", "5e-324",
        ] {
            assert!(
                parse(bad).is_err(),
                "expected {bad:?} to be rejected as floating point"
            );
        }
        // Integers are unaffected.
        assert_eq!(parse("1").expect("int"), Value::Int(1));
        assert_eq!(parse("-0").expect("int"), Value::Int(0));
        assert_eq!(canonical_json(&Value::Int(1)), "1");

        // The rejection must come from the float guard itself, naming the reason.
        // Without this the trailing-content check would mask the guard's absence
        // and the test would pass either way.
        for bad in ["1.0", "[1.5]", "{\"k\":2.5}", "1e5"] {
            let err = parse(bad).expect_err("must reject");
            assert!(
                err.message.contains("floating-point"),
                "{bad:?} was rejected as {:?}, not as a float", err.message
            );
        }
    }
}
