//! Strict JSON parsing and RFC 8785 (JCS) canonicalization.
//!
//! Manifest schema §2.3–2.4 and §4: the parser rejects duplicate member names,
//! invalid UTF-8, non-finite numbers, trailing bytes, excess depth and size.
//! Canonical form: members sorted by UTF-16 code units, no whitespace,
//! ES6 number serialization (Phase 1 admits integers ≤ 2^53−1 only, so the
//! integer path is exact), and JCS string escaping.

use std::collections::BTreeMap;
use std::fmt;

/// Ordered JSON value; object members are kept in a map keyed by name so a
/// duplicate is detected at parse time and canonical ordering is by construction.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    /// Phase 1 admits only integers exactly representable as f64 (≤ 2^53−1).
    Int(i64),
    Str(String),
    Arr(Vec<Value>),
    Obj(BTreeMap<Utf16Key, Value>),
}

/// Object key ordered by UTF-16 code units (RFC 8785 §3.2.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Utf16Key(pub String);
impl PartialOrd for Utf16Key {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(o)) }
}
impl Ord for Utf16Key {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering { self.0.encode_utf16().cmp(o.0.encode_utf16()) }
}
impl From<&str> for Utf16Key { fn from(s: &str) -> Self { Utf16Key(s.to_string()) } }

#[derive(Debug, Clone, PartialEq)]
pub enum JsonError {
    InvalidUtf8,
    TooLarge(usize),
    TooDeep(usize),
    DuplicateMember(String),
    TrailingBytes,
    UnexpectedEof,
    Unexpected(usize, String),
    NonIntegerNumber,
    IntegerOutOfRange,
    ControlCharacter,
    BadEscape,
}
impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{self:?}") }
}
impl std::error::Error for JsonError {}

pub struct Limits { pub max_bytes: usize, pub max_depth: usize }
/// Manifest schema §2.4: request ≤ 16 KiB, depth ≤ 4 including root.
pub const REQUEST_LIMITS: Limits = Limits { max_bytes: 16 * 1024, max_depth: 4 };
/// Signed objects: generous but bounded.
pub const MANIFEST_LIMITS: Limits = Limits { max_bytes: 256 * 1024, max_depth: 8 };
pub const MAX_SAFE_INT: i64 = 9_007_199_254_740_991;

pub fn parse(bytes: &[u8], lim: &Limits) -> Result<Value, JsonError> {
    if bytes.len() > lim.max_bytes { return Err(JsonError::TooLarge(bytes.len())); }
    let s = std::str::from_utf8(bytes).map_err(|_| JsonError::InvalidUtf8)?;
    let mut p = Parser { s: s.as_bytes(), i: 0, depth: 0, max_depth: lim.max_depth };
    p.ws();
    let v = p.value()?;
    p.ws();
    if p.i != p.s.len() { return Err(JsonError::TrailingBytes); }
    Ok(v)
}

struct Parser<'a> { s: &'a [u8], i: usize, depth: usize, max_depth: usize }

impl<'a> Parser<'a> {
    fn ws(&mut self) { while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\t' | b'\n' | b'\r') { self.i += 1; } }
    fn peek(&self) -> Option<u8> { self.s.get(self.i).copied() }
    fn expect(&mut self, lit: &[u8]) -> Result<(), JsonError> {
        if self.s[self.i..].starts_with(lit) { self.i += lit.len(); Ok(()) } else { Err(self.err("literal")) }
    }
    fn err(&self, what: &str) -> JsonError { JsonError::Unexpected(self.i, what.to_string()) }
    fn value(&mut self) -> Result<Value, JsonError> {
        match self.peek().ok_or(JsonError::UnexpectedEof)? {
            b'{' => self.object(),
            b'[' => self.array(),
            b'"' => Ok(Value::Str(self.string()?)),
            b't' => { self.expect(b"true")?; Ok(Value::Bool(true)) }
            b'f' => { self.expect(b"false")?; Ok(Value::Bool(false)) }
            b'n' => { self.expect(b"null")?; Ok(Value::Null) }
            b'-' | b'0'..=b'9' => self.number(),
            _ => Err(self.err("value")),
        }
    }
    fn enter(&mut self) -> Result<(), JsonError> {
        self.depth += 1;
        if self.depth > self.max_depth { return Err(JsonError::TooDeep(self.depth)); }
        Ok(())
    }
    fn object(&mut self) -> Result<Value, JsonError> {
        self.enter()?; self.i += 1; self.ws();
        let mut m = BTreeMap::new();
        if self.peek() == Some(b'}') { self.i += 1; self.depth -= 1; return Ok(Value::Obj(m)); }
        loop {
            self.ws();
            if self.peek() != Some(b'"') { return Err(self.err("member name")); }
            let k = self.string()?;
            self.ws();
            if self.peek() != Some(b':') { return Err(self.err("colon")); }
            self.i += 1; self.ws();
            let v = self.value()?;
            if m.insert(Utf16Key(k.clone()), v).is_some() { return Err(JsonError::DuplicateMember(k)); }
            self.ws();
            match self.peek() {
                Some(b',') => { self.i += 1; }
                Some(b'}') => { self.i += 1; self.depth -= 1; return Ok(Value::Obj(m)); }
                _ => return Err(self.err("comma or brace")),
            }
        }
    }
    fn array(&mut self) -> Result<Value, JsonError> {
        self.enter()?; self.i += 1; self.ws();
        let mut a = Vec::new();
        if self.peek() == Some(b']') { self.i += 1; self.depth -= 1; return Ok(Value::Arr(a)); }
        loop {
            self.ws();
            a.push(self.value()?);
            self.ws();
            match self.peek() {
                Some(b',') => { self.i += 1; }
                Some(b']') => { self.i += 1; self.depth -= 1; return Ok(Value::Arr(a)); }
                _ => return Err(self.err("comma or bracket")),
            }
        }
    }
    fn number(&mut self) -> Result<Value, JsonError> {
        let start = self.i;
        if self.peek() == Some(b'-') { self.i += 1; }
        let ds = self.i;
        while matches!(self.peek(), Some(b'0'..=b'9')) { self.i += 1; }
        if self.i == ds { return Err(self.err("digit")); }
        if self.s[ds] == b'0' && self.i - ds > 1 { return Err(self.err("leading zero")); }
        if matches!(self.peek(), Some(b'.') | Some(b'e') | Some(b'E')) { return Err(JsonError::NonIntegerNumber); }
        let txt = std::str::from_utf8(&self.s[start..self.i]).unwrap();
        let n: i64 = txt.parse().map_err(|_| JsonError::IntegerOutOfRange)?;
        if n.abs() > MAX_SAFE_INT { return Err(JsonError::IntegerOutOfRange); }
        Ok(Value::Int(n))
    }
    fn string(&mut self) -> Result<String, JsonError> {
        self.i += 1;
        let mut out = String::new();
        loop {
            let c = self.peek().ok_or(JsonError::UnexpectedEof)?;
            match c {
                b'"' => { self.i += 1; return Ok(out); }
                b'\\' => {
                    self.i += 1;
                    let e = self.peek().ok_or(JsonError::UnexpectedEof)?; self.i += 1;
                    match e {
                        b'"' => out.push('"'), b'\\' => out.push('\\'), b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'), b'f' => out.push('\u{c}'), b'n' => out.push('\n'),
                        b'r' => out.push('\r'), b't' => out.push('\t'),
                        b'u' => {
                            let cu = self.hex4()?;
                            if (0xD800..0xDC00).contains(&cu) {
                                self.expect(b"\\u").map_err(|_| JsonError::BadEscape)?;
                                let lo = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&lo) { return Err(JsonError::BadEscape); }
                                let cp = 0x10000 + ((cu as u32 - 0xD800) << 10) + (lo as u32 - 0xDC00);
                                out.push(char::from_u32(cp).ok_or(JsonError::BadEscape)?);
                            } else if (0xDC00..0xE000).contains(&cu) {
                                return Err(JsonError::BadEscape);
                            } else {
                                out.push(char::from_u32(cu as u32).ok_or(JsonError::BadEscape)?);
                            }
                        }
                        _ => return Err(JsonError::BadEscape),
                    }
                }
                0..=0x1f => return Err(JsonError::ControlCharacter),
                _ => {
                    // copy one UTF-8 scalar
                    let rest = std::str::from_utf8(&self.s[self.i..]).map_err(|_| JsonError::InvalidUtf8)?;
                    let ch = rest.chars().next().unwrap();
                    out.push(ch); self.i += ch.len_utf8();
                }
            }
        }
    }
    fn hex4(&mut self) -> Result<u16, JsonError> {
        if self.i + 4 > self.s.len() { return Err(JsonError::UnexpectedEof); }
        let h = std::str::from_utf8(&self.s[self.i..self.i + 4]).map_err(|_| JsonError::BadEscape)?;
        self.i += 4;
        u16::from_str_radix(h, 16).map_err(|_| JsonError::BadEscape)
    }
}

/// RFC 8785 canonical serialization.
pub fn canonical(v: &Value) -> Vec<u8> { let mut o = Vec::new(); write(v, &mut o); o }

fn write(v: &Value, o: &mut Vec<u8>) {
    match v {
        Value::Null => o.extend_from_slice(b"null"),
        Value::Bool(b) => o.extend_from_slice(if *b { b"true" } else { b"false" }),
        Value::Int(n) => o.extend_from_slice(n.to_string().as_bytes()),
        Value::Str(s) => write_str(s, o),
        Value::Arr(a) => {
            o.push(b'[');
            for (i, x) in a.iter().enumerate() { if i > 0 { o.push(b','); } write(x, o); }
            o.push(b']');
        }
        Value::Obj(m) => {
            o.push(b'{');
            for (i, (k, x)) in m.iter().enumerate() {
                if i > 0 { o.push(b','); }
                write_str(&k.0, o); o.push(b':'); write(x, o);
            }
            o.push(b'}');
        }
    }
}

fn write_str(s: &str, o: &mut Vec<u8>) {
    o.push(b'"');
    for c in s.chars() {
        match c {
            '"' => o.extend_from_slice(b"\\\""),
            '\\' => o.extend_from_slice(b"\\\\"),
            '\u{8}' => o.extend_from_slice(b"\\b"),
            '\u{c}' => o.extend_from_slice(b"\\f"),
            '\n' => o.extend_from_slice(b"\\n"),
            '\r' => o.extend_from_slice(b"\\r"),
            '\t' => o.extend_from_slice(b"\\t"),
            c if (c as u32) < 0x20 => o.extend_from_slice(format!("\\u{:04x}", c as u32).as_bytes()),
            c => { let mut b = [0u8; 4]; o.extend_from_slice(c.encode_utf8(&mut b).as_bytes()); }
        }
    }
    o.push(b'"');
}

/// Parse bytes that a transport claims are canonical: reject if re-encoding differs (§4).
pub fn parse_canonical(bytes: &[u8], lim: &Limits) -> Result<Value, JsonError> {
    let v = parse(bytes, lim)?;
    if canonical(&v) != bytes { return Err(JsonError::Unexpected(0, "non-canonical".into())); }
    Ok(v)
}

// ---- accessors used by schema code ----
impl Value {
    pub fn as_obj(&self) -> Option<&BTreeMap<Utf16Key, Value>> { if let Value::Obj(m) = self { Some(m) } else { None } }
    pub fn as_arr(&self) -> Option<&Vec<Value>> { if let Value::Arr(a) = self { Some(a) } else { None } }
    pub fn as_str(&self) -> Option<&str> { if let Value::Str(s) = self { Some(s) } else { None } }
    pub fn as_int(&self) -> Option<i64> { if let Value::Int(n) = self { Some(*n) } else { None } }
    pub fn as_bool(&self) -> Option<bool> { if let Value::Bool(b) = self { Some(*b) } else { None } }
    pub fn is_null(&self) -> bool { matches!(self, Value::Null) }
    pub fn get(&self, k: &str) -> Option<&Value> { self.as_obj()?.get(&Utf16Key(k.to_string())) }
    pub fn obj(pairs: Vec<(&str, Value)>) -> Value {
        Value::Obj(pairs.into_iter().map(|(k, v)| (Utf16Key(k.to_string()), v)).collect())
    }
    pub fn s(x: &str) -> Value { Value::Str(x.to_string()) }
    pub fn arr_of_str(xs: &[&str]) -> Value { Value::Arr(xs.iter().map(|x| Value::s(x)).collect()) }
    pub fn set(&mut self, k: &str, v: Value) { if let Value::Obj(m) = self { m.insert(Utf16Key(k.to_string()), v); } }
    pub fn pretty(&self) -> String { String::from_utf8(canonical(self)).unwrap() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_rejected() {
        assert_eq!(parse(br#"{"a":1,"a":2}"#, &MANIFEST_LIMITS), Err(JsonError::DuplicateMember("a".into())));
    }
    #[test]
    fn canonical_order_utf16() {
        let v = parse(r#"{"b":1,"a":2,"\u20ac":3,"aa":4}"#.as_bytes(), &MANIFEST_LIMITS).unwrap();
        assert_eq!(canonical(&v), "{\"a\":2,\"aa\":4,\"b\":1,\"€\":3}".as_bytes());
    }
    #[test]
    fn rfc8785_escapes() {
        let v = parse(br#"{"x":"\u0001\t\"\\/\u00e9"}"#, &MANIFEST_LIMITS).unwrap();
        assert_eq!(canonical(&v), "{\"x\":\"\\u0001\\t\\\"\\\\/é\"}".as_bytes());
    }
    #[test]
    fn floats_and_big_ints_rejected() {
        assert_eq!(parse(b"1.5", &MANIFEST_LIMITS), Err(JsonError::NonIntegerNumber));
        assert_eq!(parse(b"9007199254740992", &MANIFEST_LIMITS), Err(JsonError::IntegerOutOfRange));
    }
    #[test]
    fn depth_and_trailing() {
        assert!(matches!(parse(b"[[[[1]]]]", &REQUEST_LIMITS), Ok(_)));
        assert!(matches!(parse(b"[[[[[1]]]]]", &REQUEST_LIMITS), Err(JsonError::TooDeep(_))));
        assert_eq!(parse(b"{} x", &MANIFEST_LIMITS), Err(JsonError::TrailingBytes));
    }
}
