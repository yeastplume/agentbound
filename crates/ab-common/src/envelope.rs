//! Signature envelopes (manifest schema §4; component interfaces §4.1).
//! Policy envelope: exactly `authorization_id, authorization_manifest_digest, issued_at, key_id, signature, timestamp_source`.
//! Constructor envelope: exactly `allocation_id, authorization_id, authorization_manifest_digest, boot_id, host_id, issued_at, key_id, launch_binding_digest, signature`.
//! Signatures bind the object AND all envelope metadata except `signature`,
//! using a domain-separated v0.2 transcript. Legacy object-only signatures are
//! rejected: accepting them would reintroduce unsigned issuance timestamps.

use crate::json::{canonical, Value};
use crate::sig::*;

#[derive(Debug)]
pub struct Verified { pub digest: String, pub issued_at: i64, pub key_id: String }

fn members(v: &Value, want: &[&str]) -> Result<(), SigError> {
    let m = v.as_obj().ok_or_else(|| SigError::EnvelopeShape("object".into()))?;
    if m.len() != want.len() { return Err(SigError::EnvelopeShape(format!("expected exactly {} members", want.len()))); }
    for k in want { if v.get(k).and_then(|x| x.as_str()).is_none() { return Err(SigError::EnvelopeShape(format!("missing/invalid {k}"))); } }
    Ok(())
}
fn s<'a>(v: &'a Value, k: &str) -> &'a str { v.get(k).unwrap().as_str().unwrap() }

fn signing_bytes(object: &Value, envelope: &Value, role: &str) -> Vec<u8> {
    let mut metadata = envelope.clone();
    if let Value::Obj(ref mut members) = metadata { members.remove(&"signature".into()); }
    canonical(&Value::obj(vec![
        ("domain", Value::s(&format!("agentbound.signature.{role}.v0.2"))),
        ("envelope", metadata), ("object", object.clone()),
    ]))
}

pub fn policy_envelope(signer: &Signer_, manifest: &Value, authorization_id: &str, now: i64) -> Value {
    let mut env = Value::obj(vec![("authorization_id", Value::s(authorization_id)), ("authorization_manifest_digest", Value::s(&object_digest(manifest))), ("issued_at", Value::s(&fmt_rfc3339(now))),
        ("key_id", Value::s(&signer.key_id)), ("timestamp_source", Value::s(CLOCK_SOURCE))]);
    env.set("signature", Value::s(&signer.sign_b64(&signing_bytes(manifest, &env, "policy"))));
    env
}
pub fn constructor_envelope(signer: &Signer_, binding: &Value, manifest_digest: &str, authorization_id: &str, allocation_id: &str, host_id: &str, boot_id: &str, now: i64) -> Value {
    let mut env = Value::obj(vec![("allocation_id", Value::s(allocation_id)), ("authorization_id", Value::s(authorization_id)), ("authorization_manifest_digest", Value::s(manifest_digest)), ("boot_id", Value::s(boot_id)), ("host_id", Value::s(host_id)),
        ("issued_at", Value::s(&fmt_rfc3339(now))), ("key_id", Value::s(&signer.key_id)), ("launch_binding_digest", Value::s(&object_digest(binding)))]);
    env.set("signature", Value::s(&signer.sign_b64(&signing_bytes(binding, &env, "launch"))));
    env
}

/// Verify a policy envelope over `manifest`: shape, digest equality, key (role `policy`), signature, freshness.
pub fn verify_policy(keyring: &Keyring, manifest: &Value, env: &Value, authorization_id: &str, now: i64) -> Result<Verified, SigError> {
    members(env, &["authorization_id", "authorization_manifest_digest", "issued_at", "key_id", "signature", "timestamp_source"])?;
    if s(env, "timestamp_source") != CLOCK_SOURCE { return Err(SigError::EnvelopeShape("timestamp source".into())); }
    common(keyring, manifest, env, "authorization_manifest_digest", "policy", authorization_id, now, MANIFEST_MAX_AGE_S)
}
/// Verify a constructor envelope over `binding`; also checks the manifest digest and host/boot binding it names.
pub fn verify_constructor(keyring: &Keyring, binding: &Value, env: &Value, authorization_id: &str, manifest_digest: &str, host_id: &str, boot_id: &str, now: i64) -> Result<Verified, SigError> {
    members(env, &["allocation_id", "authorization_id", "authorization_manifest_digest", "boot_id", "host_id", "issued_at", "key_id", "launch_binding_digest", "signature"])?;
    if s(env, "authorization_manifest_digest") != manifest_digest { return Err(SigError::DigestMismatch); }
    if s(env, "host_id") != host_id || s(env, "boot_id") != boot_id { return Err(SigError::EnvelopeShape("host/boot binding differs".into())); }
    common(keyring, binding, env, "launch_binding_digest", "launch", authorization_id, now, BINDING_MAX_AGE_S)
}
fn common(keyring: &Keyring, obj: &Value, env: &Value, digest_member: &str, role: &'static str, authorization_id: &str, now: i64, max_age: i64) -> Result<Verified, SigError> {
    if s(env, "authorization_id") != authorization_id { return Err(SigError::EnvelopeShape("authorization_id differs".into())); }
    let digest = object_digest(obj);
    if s(env, digest_member) != digest { return Err(SigError::DigestMismatch); }
    let issued_at = parse_rfc3339(s(env, "issued_at")).ok_or_else(|| SigError::EnvelopeShape("issued_at".into()))?;
    check_fresh(issued_at, now, max_age)?;
    let key = keyring.lookup(s(env, "key_id"), role, now)?;
    verify_b64(&key.public, &signing_bytes(obj, env, role), s(env, "signature"))?;
    Ok(Verified { digest, issued_at, key_id: key.key_id.clone() })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ring(sig: &Signer_, role: &str) -> Keyring {
        Keyring::parse(format!(r#"[{{"key_id":"{}","public_key":"{}","not_before":0,"not_after":9999999999,"role":"{role}","status":"active"}}]"#, sig.key_id, sig.public_hex()).as_bytes()).unwrap()
    }
    #[test]
    fn changing_unsigned_issuance_cannot_refresh_a_stale_signature() {
        let sg = Signer_::generate("key:p1");
        let m = Value::obj(vec![("a", Value::Int(1))]);
        let now = 1_800_000_000;
        let mut env = policy_envelope(&sg, &m, "authz:1", now);
        env.set("issued_at", Value::s(&fmt_rfc3339(now + 601)));
        assert!(verify_policy(&ring(&sg, "policy"), &m, &env, "authz:1", now + 601).is_err(), "changing the timestamp must invalidate the signature");
    }

    #[test]
    fn constructor_metadata_and_legacy_signatures_are_not_accepted() {
        let sg = Signer_::generate("key:l1");
        let binding = Value::obj(vec![("a", Value::Int(1))]);
        let now = 1_800_000_000;
        let env = constructor_envelope(&sg, &binding, "sha256:manifest", "authz:1", "allocation:1", "host:1", "boot:1", now);
        let kr = ring(&sg, "launch");
        let verify = |e: &Value| verify_constructor(&kr, &binding, e, "authz:1", "sha256:manifest", "host:1", "boot:1", now + 5);
        assert!(verify(&env).is_ok());
        for (key, value) in [("issued_at", fmt_rfc3339(now + 1)), ("allocation_id", "allocation:2".to_string())] {
            let mut changed = env.clone(); changed.set(key, Value::s(&value));
            assert!(verify(&changed).is_err(), "unsigned change to {key} accepted");
        }
        let mut legacy = env.clone();
        legacy.set("signature", Value::s(&sg.sign_b64(&canonical(&binding))));
        assert!(verify(&legacy).is_err(), "legacy object-only signatures must not downgrade the contract");
    }

    #[test]
    fn policy_metadata_is_authenticated() {
        let sg = Signer_::generate("key:p1"); let object = Value::Null; let now = 1_800_000_000;
        let mut env = policy_envelope(&sg, &object, "authz:1", now);
        env.set("authorization_id", Value::s("authz:2"));
        assert!(verify_policy(&ring(&sg, "policy"), &object, &env, "authz:2", now).is_err());
        let mut env = policy_envelope(&sg, &object, "authz:1", now);
        env.set("timestamp_source", Value::s("untrusted-clock"));
        assert!(verify_policy(&ring(&sg, "policy"), &object, &env, "authz:1", now).is_err());
    }

    #[test]
    fn policy_roundtrip_and_failures() {
        let sg = Signer_::generate("key:p1"); let m = Value::obj(vec![("a", Value::Int(1))]); let now = 1_800_000_000;
        let env = policy_envelope(&sg, &m, "authz:1", now);
        assert!(verify_policy(&ring(&sg, "policy"), &m, &env, "authz:1", now + 5).is_ok());
        assert_eq!(verify_policy(&ring(&sg, "policy"), &m, &env, "authz:1", now + 601).unwrap_err(), SigError::Stale);
        assert_eq!(verify_policy(&ring(&sg, "policy"), &m, &env, "authz:1", now - 31).unwrap_err(), SigError::IssuedInFuture);
        assert!(matches!(verify_policy(&ring(&sg, "launch"), &m, &env, "authz:1", now).unwrap_err(), SigError::WrongRole { .. }));
        let m2 = Value::obj(vec![("a", Value::Int(2))]);
        assert_eq!(verify_policy(&ring(&sg, "policy"), &m2, &env, "authz:1", now).unwrap_err(), SigError::DigestMismatch);
        let mut env2 = env.clone(); env2.set("extra", Value::Null);
        assert!(matches!(verify_policy(&ring(&sg, "policy"), &m, &env2, "authz:1", now).unwrap_err(), SigError::EnvelopeShape(_)));
    }
}
