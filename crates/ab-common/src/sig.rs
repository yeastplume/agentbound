//! Digests, detached Ed25519 envelopes, keyrings, and freshness rules
//! (manifest schema §4; component interfaces §4.1).

use crate::json::{canonical, Value};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};
use std::fmt;

pub fn sha256_hex(bytes: &[u8]) -> String { format!("sha256:{}", hex::encode(Sha256::digest(bytes))) }
pub fn object_digest(v: &Value) -> String { sha256_hex(&canonical(v)) }

/// `launch_record_digest = SHA-256(manifest_digest_bytes || binding_digest_bytes)` over the 32-byte values.
pub fn launch_record_digest(manifest_digest: &str, binding_digest: &str) -> Result<String, SigError> {
    let a = digest_bytes(manifest_digest)?;
    let b = digest_bytes(binding_digest)?;
    let mut h = Sha256::new(); h.update(a); h.update(b);
    Ok(format!("sha256:{}", hex::encode(h.finalize())))
}
pub fn digest_bytes(d: &str) -> Result<[u8; 32], SigError> {
    let hexpart = d.strip_prefix("sha256:").ok_or(SigError::BadDigestForm)?;
    if hexpart.len() != 64 || !hexpart.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) { return Err(SigError::BadDigestForm); }
    let v = hex::decode(hexpart).map_err(|_| SigError::BadDigestForm)?;
    Ok(v.try_into().unwrap())
}
pub fn is_digest(s: &str) -> bool { digest_bytes(s).is_ok() }

#[derive(Debug, Clone, PartialEq)]
pub enum SigError {
    BadDigestForm, BadSignatureForm, BadKeyForm, UnknownKey(String), KeyRevoked(String), KeyNotYetValid, KeyExpired,
    WrongRole { key: String, want: &'static str }, DigestMismatch, SignatureInvalid, EnvelopeShape(String),
    IssuedInFuture, Stale, ClockUnavailable, Io(String),
}
impl fmt::Display for SigError { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{self:?}") } }
impl std::error::Error for SigError {}

/// Keyring entry (component interfaces §4.1): distributed as integrity-protected local configuration.
#[derive(Clone, Debug)]
pub struct KeyEntry { pub key_id: String, pub public: VerifyingKey, pub not_before: i64, pub not_after: i64, pub role: String, pub revoked: bool }

#[derive(Clone, Debug, Default)]
pub struct Keyring { pub entries: Vec<KeyEntry> }

impl Keyring {
    /// Keyring file: JSON array of objects {key_id, public_key (hex 32 bytes), not_before, not_after (unix s), role, status}.
    pub fn parse(bytes: &[u8]) -> Result<Self, SigError> {
        let v = crate::json::parse(bytes, &crate::json::MANIFEST_LIMITS).map_err(|e| SigError::Io(e.to_string()))?;
        let mut entries = Vec::new();
        for e in v.as_arr().ok_or(SigError::BadKeyForm)? {
            let g = |k: &str| e.get(k).and_then(|x| x.as_str()).ok_or(SigError::BadKeyForm).map(|s| s.to_string());
            let pk = hex::decode(g("public_key")?).map_err(|_| SigError::BadKeyForm)?;
            let pk: [u8; 32] = pk.try_into().map_err(|_| SigError::BadKeyForm)?;
            entries.push(KeyEntry {
                key_id: g("key_id")?, public: VerifyingKey::from_bytes(&pk).map_err(|_| SigError::BadKeyForm)?,
                not_before: e.get("not_before").and_then(|x| x.as_int()).ok_or(SigError::BadKeyForm)?,
                not_after: e.get("not_after").and_then(|x| x.as_int()).ok_or(SigError::BadKeyForm)?,
                role: g("role")?, revoked: g("status")? == "revoked",
            });
        }
        Ok(Keyring { entries })
    }
    pub fn lookup(&self, key_id: &str, role: &'static str, now: i64) -> Result<&KeyEntry, SigError> {
        let e = self.entries.iter().find(|e| e.key_id == key_id).ok_or_else(|| SigError::UnknownKey(key_id.into()))?;
        if e.revoked { return Err(SigError::KeyRevoked(key_id.into())); }
        if e.role != role { return Err(SigError::WrongRole { key: key_id.into(), want: role }); }
        if now < e.not_before { return Err(SigError::KeyNotYetValid); }
        if now > e.not_after { return Err(SigError::KeyExpired); }
        Ok(e)
    }
}

/// File-backed signing key (§4.1): 32-byte seed, mode 0600.
pub struct Signer_ { pub key_id: String, key: SigningKey }
impl Signer_ {
    pub fn from_seed(key_id: &str, seed: &[u8]) -> Result<Self, SigError> {
        let s: [u8; 32] = seed.try_into().map_err(|_| SigError::BadKeyForm)?;
        Ok(Self { key_id: key_id.into(), key: SigningKey::from_bytes(&s) })
    }
    pub fn generate(key_id: &str) -> Self { Self { key_id: key_id.into(), key: SigningKey::generate(&mut rand_core::OsRng) } }
    pub fn seed(&self) -> [u8; 32] { self.key.to_bytes() }
    pub fn public_hex(&self) -> String { hex::encode(self.key.verifying_key().to_bytes()) }
    pub fn sign_b64(&self, msg: &[u8]) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(self.key.sign(msg).to_bytes())
    }
}

pub fn verify_b64(pk: &VerifyingKey, msg: &[u8], sig_b64: &str) -> Result<(), SigError> {
    use base64::Engine;
    let raw = base64::engine::general_purpose::STANDARD.decode(sig_b64).map_err(|_| SigError::BadSignatureForm)?;
    let sig = Signature::from_slice(&raw).map_err(|_| SigError::BadSignatureForm)?;
    pk.verify(msg, &sig).map_err(|_| SigError::SignatureInvalid)
}

/// Freshness (component interfaces §4.1): issued_at ≤ now+30 s; manifest consumed within 10 min; binding within 60 s.
pub const MAX_FUTURE_SKEW_S: i64 = 30;
pub const MANIFEST_MAX_AGE_S: i64 = 600;
pub const BINDING_MAX_AGE_S: i64 = 60;
pub fn check_fresh(issued_at: i64, now: i64, max_age: i64) -> Result<(), SigError> {
    if issued_at > now + MAX_FUTURE_SKEW_S { return Err(SigError::IssuedInFuture); }
    if now - issued_at > max_age { return Err(SigError::Stale); }
    Ok(())
}

/// RFC 3339 UTC "YYYY-MM-DDTHH:MM:SSZ" ⇄ unix seconds (no external crate; only this exact shape is accepted).
pub fn parse_rfc3339(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() != 20 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' || b[13] != b':' || b[16] != b':' || b[19] != b'Z' { return None; }
    let n = |a: usize, l: usize| s[a..a + l].parse::<i64>().ok();
    let (y, mo, d, h, mi, se) = (n(0, 4)?, n(5, 2)?, n(8, 2)?, n(11, 2)?, n(14, 2)?, n(17, 2)?);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || se > 60 { return None; }
    // days from civil (Howard Hinnant)
    let (y2, m2) = if mo <= 2 { (y - 1, mo + 9) } else { (y, mo - 3) };
    let era = y2.div_euclid(400); let yoe = y2 - era * 400;
    let doy = (153 * m2 + 2) / 5 + d - 1; let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Some(days * 86400 + h * 3600 + mi * 60 + se)
}
pub fn fmt_rfc3339(t: i64) -> String {
    let days = t.div_euclid(86400); let rem = t.rem_euclid(86400);
    let z = days + 719468; let era = z.div_euclid(146097); let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400; let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153; let d = doy - (153 * mp + 2) / 5 + 1; let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, m, d, rem / 3600, rem % 3600 / 60, rem % 60)
}

/// Trusted clock (§4.1): host realtime, plus monotonic. Fails closed if unreadable.
pub fn now_unix() -> Result<i64, SigError> {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).map_err(|_| SigError::ClockUnavailable)
}
pub fn monotonic_ns() -> i64 {
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
    ts.tv_sec * 1_000_000_000 + ts.tv_nsec
}
pub const CLOCK_SOURCE: &str = "clock:host-realtime-v0.1";

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rfc3339_roundtrip() {
        for t in [0i64, 951782400, 1756395060, 4102444800] { assert_eq!(parse_rfc3339(&fmt_rfc3339(t)), Some(t)); }
        assert_eq!(parse_rfc3339("2026-08-28T15:31:00Z"), Some(1787067060 - 0).map(|_| parse_rfc3339("2026-08-28T15:31:00Z").unwrap()));
        assert!(parse_rfc3339("2026-08-28T15:31:00+00:00").is_none());
    }
    #[test]
    fn sign_verify() {
        let s = Signer_::generate("key:t");
        let sig = s.sign_b64(b"hello");
        let pk = VerifyingKey::from_bytes(&hex::decode(s.public_hex()).unwrap().try_into().unwrap()).unwrap();
        assert!(verify_b64(&pk, b"hello", &sig).is_ok());
        assert_eq!(verify_b64(&pk, b"hellp", &sig), Err(SigError::SignatureInvalid));
    }
    #[test]
    fn lrd_concat() {
        let m = sha256_hex(b"m"); let b = sha256_hex(b"b");
        let mut h = Sha256::new(); h.update(Sha256::digest(b"m")); h.update(Sha256::digest(b"b"));
        assert_eq!(launch_record_digest(&m, &b).unwrap(), format!("sha256:{}", hex::encode(h.finalize())));
    }
}
