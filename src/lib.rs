//! Bundu Cell 0.1.0 draft: a prototype validator. It exists to stress-test the draft, not to
//! be the reference implementation.
//!
//! ```sh
//! cargo run -- check FILE.cell    # validate one cell
//! cargo run -- render FILE.cell   # print its text with facts filled in
//! ```
//!
//! A cell is a JSON object held as a [`serde_json::Value`]. Every function here takes one,
//! so a cell read from a file and a cell read from a database row are checked the same way.

#![forbid(unsafe_code)]

mod jcs;

use std::fmt;

use base64::Engine as _;
use base64::alphabet::URL_SAFE;
use base64::engine::DecodePaddingMode;
use base64::engine::general_purpose::{GeneralPurpose, GeneralPurposeConfig, URL_SAFE_NO_PAD};
use ed25519_dalek::{Signature, Signer as _, Verifier as _};
pub use ed25519_dalek::{SigningKey, VerifyingKey};
pub use jcs::number;
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use sha2::{Digest as _, Sha256};
use unicode_normalization::is_nfc;

/// What can go wrong outside validation: reading a cell, its text, or making a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The input is not JSON, or breaks I-JSON (RFC 7493), such as a duplicate member.
    Parse(String),
    /// `text` is neither a string nor a language map of strings.
    TextShape,
    /// The system gave no randomness for a new key.
    Random(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Parse(m) => f.write_str(m),
            Error::TextShape => f.write_str("text must be a string or a language map"),
            Error::Random(m) => write!(f, "no randomness for a key: {m}"),
        }
    }
}

impl std::error::Error for Error {}

/// RFC 8785 (JCS) canonical form. Numbers use the ECMAScript form, which JCS adopts, so an
/// integer beyond 2^53 is rounded to the nearest double, as JCS requires.
pub fn canonicalize(v: &Value) -> String {
    let mut out = String::new();
    jcs::write(v, true, &mut out);
    out
}

/// `JSON.stringify(v)`: no whitespace, members in their own order, ECMAScript numbers.
pub fn stringify(v: &Value) -> String {
    let mut out = String::new();
    jcs::write(v, false, &mut out);
    out
}

const ENVELOPE: [&str; 2] = ["cell:hash", "proof"];

/// The content: the cell without its hash and proof. A value that is not an object has
/// no members, so its content is the empty object.
pub fn content(cell: &Value) -> Value {
    Value::Object(without(cell, &ENVELOPE))
}

fn without(cell: &Value, drop: &[&str]) -> Map<String, Value> {
    match cell {
        Value::Object(m) => m
            .iter()
            .filter(|(k, _)| !drop.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        _ => Map::new(),
    }
}

fn b64url(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

/// `sha256-` and the unpadded base64url SHA-256 of the content's canonical form.
pub fn hash(cell: &Value) -> String {
    let digest = Sha256::digest(canonicalize(&content(cell)).as_bytes());
    format!("sha256-{}", b64url(&digest))
}

/// Parses JSON as I-JSON (RFC 7493): a duplicate member name is refused, where a plain
/// parser would silently keep one of the two.
pub fn parse(text: &str) -> Result<Value, Error> {
    serde_json::from_str::<Strict>(text)
        .map(|s| s.0)
        .map_err(|e| Error::Parse(e.to_string()))
}

/// A JSON value read with every object checked for duplicate member names.
struct Strict(Value);

impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(StrictVisitor).map(Strict)
    }
}

struct StrictVisitor;

impl<'de> Visitor<'de> for StrictVisitor {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value")
    }
    fn visit_bool<E>(self, b: bool) -> Result<Value, E> {
        Ok(Value::Bool(b))
    }
    fn visit_i64<E>(self, n: i64) -> Result<Value, E> {
        Ok(Value::Number(n.into()))
    }
    fn visit_u64<E>(self, n: u64) -> Result<Value, E> {
        Ok(Value::Number(n.into()))
    }
    fn visit_f64<E: de::Error>(self, n: f64) -> Result<Value, E> {
        Number::from_f64(n)
            .map(Value::Number)
            .ok_or_else(|| E::custom("JCS: not a finite number"))
    }
    fn visit_str<E>(self, s: &str) -> Result<Value, E> {
        Ok(Value::String(s.to_owned()))
    }
    fn visit_string<E>(self, s: String) -> Result<Value, E> {
        Ok(Value::String(s))
    }
    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut out = Vec::new();
        while let Some(Strict(v)) = seq.next_element()? {
            out.push(v);
        }
        Ok(Value::Array(out))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut out = Map::new();
        while let Some(k) = map.next_key::<String>()? {
            if out.contains_key(&k) {
                return Err(de::Error::custom(format!(
                    "I-JSON: duplicate member \"{k}\""
                )));
            }
            let Strict(v) = map.next_value()?;
            out.insert(k, v);
        }
        Ok(Value::Object(out))
    }
}

/// Whitespace as ECMAScript's `\s` matches it (ECMA-262 WhiteSpace and LineTerminator).
fn js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// One `{{ path }}` reference in a text: its byte range and the path inside.
struct Ref<'a> {
    start: usize,
    end: usize,
    path: &'a str,
}

/// Every reference in `t`, left to right, matching `\{\{\s*([^{}\s]+)\s*\}\}`.
fn refs(t: &str) -> Vec<Ref<'_>> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < t.len() {
        if let Some(r) = ref_at(t, i) {
            i = r.end;
            out.push(r);
        } else {
            i += t[i..].chars().next().map_or(1, char::len_utf8);
        }
    }
    out
}

fn ref_at(t: &str, start: usize) -> Option<Ref<'_>> {
    if !t[start..].starts_with("{{") {
        return None;
    }
    let skip_space = |from: usize| {
        from + t[from..]
            .char_indices()
            .find(|(_, c)| !js_space(*c))
            .map_or(t.len() - from, |(n, _)| n)
    };
    let mut j = skip_space(start + 2);
    let p0 = j;
    j += t[j..]
        .char_indices()
        .find(|(_, c)| *c == '{' || *c == '}' || js_space(*c))
        .map_or(t.len() - j, |(n, _)| n);
    if j == p0 {
        return None;
    }
    let path = &t[p0..j];
    j = skip_space(j);
    t[j..].starts_with("}}").then_some(Ref {
        start,
        end: j + 2,
        path,
    })
}

/// Resolves a dotted path; a segment of ASCII digits indexes an array.
pub fn resolve<'a>(cell: &'a Value, path: &str) -> Option<&'a Value> {
    let mut v = cell;
    for seg in path.split('.') {
        v = match v {
            Value::Object(m) => m.get(seg)?,
            Value::Array(a) if !seg.is_empty() && seg.bytes().all(|b| b.is_ascii_digit()) => {
                a.get(seg.parse::<usize>().ok()?)?
            }
            _ => return None,
        };
    }
    Some(v)
}

/// The text in every language, in the cell's order: a string is the one text under the
/// empty tag, and a JSON-LD language map gives one text per tag. No `text` gives none.
pub fn texts(cell: &Value) -> Result<Vec<(String, String)>, Error> {
    match cell.get("text") {
        None => Ok(Vec::new()),
        Some(Value::String(t)) => Ok(vec![(String::new(), t.clone())]),
        Some(Value::Object(m)) => m
            .iter()
            .map(|(lang, t)| match t {
                Value::String(t) => Ok((lang.clone(), t.clone())),
                _ => Err(Error::TextShape),
            })
            .collect(),
        Some(_) => Err(Error::TextShape),
    }
}

/// How a resolved value reads in prose: a string as itself, any other scalar in its
/// ECMAScript form, an object or array as compact JSON.
fn show(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        _ => stringify(v),
    }
}

/// Fills references in one pass: a value that itself contains `{{...}}` is not expanded
/// again. `lang` picks a text from a language map; if it has none, the first text is used.
/// A reference that names no field is left as written (`validate` refuses such a cell).
pub fn render(cell: &Value, lang: &str) -> Result<String, Error> {
    let all = texts(cell)?;
    let t = all
        .iter()
        .find(|(l, _)| l == lang)
        .or_else(|| all.first())
        .map_or("", |(_, t)| t.as_str());
    let mut out = String::with_capacity(t.len());
    let mut last = 0;
    for r in refs(t) {
        out.push_str(&t[last..r.start]);
        match resolve(cell, r.path) {
            Some(v) => out.push_str(&show(v)),
            None => out.push_str(&t[r.start..r.end]),
        }
        last = r.end;
    }
    out.push_str(&t[last..]);
    Ok(out)
}

/// Length in UTF-16 code units, as a JavaScript string counts it.
fn js_len(s: &str) -> usize {
    s.encode_utf16().count()
}

/// Scalar facts worth looking for in prose: strings of 4+ characters, and numbers.
fn facts(v: &Value, path: &str, out: &mut Vec<(String, String)>) {
    let mut visit = |k: &str, x: &Value| {
        if k.starts_with('@') || k.starts_with("cell:") || k == "text" || k == "proof" {
            return;
        }
        let p = if path.is_empty() {
            k.to_owned()
        } else {
            format!("{path}.{k}")
        };
        facts(x, &p, out);
    };
    match v {
        Value::Object(m) => m.iter().for_each(|(k, x)| visit(k, x)),
        Value::Array(a) => a
            .iter()
            .enumerate()
            .for_each(|(i, x)| visit(&i.to_string(), x)),
        Value::Number(_) => out.push((path.to_owned(), stringify(v))),
        Value::String(s) if js_len(s) >= 4 => out.push((path.to_owned(), s.clone())),
        _ => {}
    }
}

/// What `validate` found: errors refuse the cell, warnings flag it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Strict equality of two members as JavaScript's `===` sees them: two missing members
/// are equal, numbers compare as doubles, and an object or array equals nothing.
fn same(a: Option<&Value>, b: Option<&Value>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(Value::Null), Some(Value::Null)) => true,
        (Some(Value::Bool(x)), Some(Value::Bool(y))) => x == y,
        (Some(Value::String(x)), Some(Value::String(y))) => x == y,
        (Some(Value::Number(x)), Some(Value::Number(y))) => x.as_f64() == y.as_f64(),
        _ => false,
    }
}

/// Checks a cell against the five rules in SPEC.md ("Why it can't diverge") and, given the
/// version before it, the version chain.
pub fn validate(cell: &Value, previous: Option<&Value>) -> Report {
    let mut r = Report::default();
    for k in ["@context", "@type", "@id", "cell:version", "cell:hash"] {
        if cell.get(k).is_none() {
            r.errors.push(format!("missing {k}"));
        }
    }
    let all = texts(cell).unwrap_or_else(|e| {
        r.errors.push(e.to_string());
        Vec::new()
    });
    let mut known = Vec::new();
    facts(&content(cell), "", &mut known);
    for (lang, t) in &all {
        let at = if lang.is_empty() {
            String::new()
        } else {
            format!(" ({lang})")
        };
        if !is_nfc(t) {
            r.errors.push(format!("text{at} is not NFC"));
        }
        let found = refs(t);
        for x in &found {
            if resolve(cell, x.path).is_none() {
                r.errors
                    .push(format!("text{at}: {{{{{}}}}} names no field", x.path));
            }
        }
        // Rule 1, heuristically: a fact's value written out in prose instead of referenced.
        let mut prose = String::with_capacity(t.len());
        let mut last = 0;
        for x in &found {
            prose.push_str(&t[last..x.start]);
            last = x.end;
        }
        prose.push_str(&t[last..]);
        for (p, v) in &known {
            if prose.contains(v.as_str()) {
                r.warnings.push(format!(
                    "text{at} restates {p} (\"{v}\"); write {{{{{p}}}}}"
                ));
            }
        }
    }
    if let Some(h) = cell.get("cell:hash") {
        if h.as_str() != Some(hash(cell).as_str()) {
            r.errors.push("cell:hash does not match the content".into());
        }
    }
    if let Some(prev) = previous {
        if !same(cell.get("@id"), prev.get("@id")) {
            r.errors.push("a new version keeps the @id".into());
        }
        let next = prev
            .get("cell:version")
            .and_then(Value::as_f64)
            .map(|v| v + 1.0);
        let this = cell.get("cell:version").and_then(Value::as_f64);
        if next.is_none() || this != next {
            r.errors.push("cell:version must go up by one".into());
        }
        if !same(cell.get("cell:previous"), prev.get("cell:hash")) {
            r.errors
                .push("cell:previous must be the hash of the version before".into());
        }
    }
    r
}

/// Seal: set the hash, then sign the canonical content plus the hash. Prototype proof only:
/// it follows the shape of a VC Data Integrity `eddsa-jcs-2022` proof, not its algorithm.
pub fn seal(cell: &Value, key: &SigningKey) -> Value {
    let mut out = without(cell, &ENVELOPE);
    let h = hash(&Value::Object(out.clone()));
    out.insert("cell:hash".into(), Value::String(h));
    let sig = key.sign(canonicalize(&Value::Object(out.clone())).as_bytes());
    let mut proof = Map::new();
    proof.insert("type".into(), "DataIntegrityProof".into());
    proof.insert("cryptosuite".into(), "eddsa-jcs-2022".into());
    proof.insert(
        "proofValue".into(),
        format!("u{}", b64url(&sig.to_bytes())).into(),
    );
    out.insert("proof".into(), Value::Object(proof));
    Value::Object(out)
}

/// Base64url, padded or not, as Node's `Buffer.from(s, "base64url")` reads it.
const B64URL_ANY: GeneralPurpose = GeneralPurpose::new(
    &URL_SAFE,
    GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

/// Checks the proof against the cell without its proof (the hash included). A missing or
/// malformed proof is a failed check, not an error.
pub fn verify_proof(cell: &Value, key: &VerifyingKey) -> bool {
    let Some(proof) = cell.get("proof").filter(|p| truthy(p)) else {
        return false;
    };
    let Some(value) = proof.get("proofValue").and_then(Value::as_str) else {
        return false;
    };
    // The first character is the multibase prefix (`u`, base64url).
    let mut chars = value.chars();
    chars.next();
    let Ok(bytes) = B64URL_ANY.decode(chars.as_str()) else {
        return false;
    };
    let Ok(sig) = Signature::from_slice(&bytes) else {
        return false;
    };
    let rest = Value::Object(without(cell, &["proof"]));
    key.verify(canonicalize(&rest).as_bytes(), &sig).is_ok()
}

/// JavaScript truthiness of a JSON value.
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// A new Ed25519 key pair from the system's randomness.
pub fn keys() -> Result<(SigningKey, VerifyingKey), Error> {
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed).map_err(|e| Error::Random(e.to_string()))?;
    let signing = SigningKey::from_bytes(&seed);
    let verifying = signing.verifying_key();
    Ok((signing, verifying))
}
