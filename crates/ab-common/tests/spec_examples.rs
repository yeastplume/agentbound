//! The manifest schema §6 illustrative pair must validate and correspond.
use ab_common::json::{parse, MANIFEST_LIMITS};
use ab_common::schema::*;
use ab_common::sig::object_digest;

fn load(n: &str) -> ab_common::json::Value {
    parse(&std::fs::read(format!("{}/tests/fixtures/{n}", env!("CARGO_MANIFEST_DIR"))).unwrap(), &MANIFEST_LIMITS).unwrap()
}
#[test]
fn spec_pair_validates() {
    let m = load("manifest-example.json"); let b = load("binding-example.json");
    let mv = validate_manifest(&m).unwrap(); let bv = validate_binding(&b).unwrap();
    assert_eq!(mv.topology, "local-socket"); assert_eq!(bv.uid, 200001);
    // the example binding carries a placeholder digest, so correspondence check 2 must fail on the real digest...
    let real = object_digest(&m);
    assert_eq!(correspond(&mv, &bv, &real).unwrap_err().rule, "correspondence-2");
    // ...and pass when the verified digest is the one the binding names
    correspond(&mv, &bv, bv.manifest_digest).unwrap();
}
#[test]
fn smuggled_and_unknown_members_rejected() {
    let mut m = load("manifest-example.json");
    m.set("uid", ab_common::json::Value::Int(0));
    assert_eq!(validate_manifest(&m).unwrap_err().rule, "unknown-member");
    let mut b = load("binding-example.json");
    b.get("execution_identity").unwrap(); // present
    let mut ei = b.get("execution_identity").unwrap().clone(); ei.set("mac_context", ab_common::json::Value::s("u:r:t"));
    b.set("execution_identity", ei);
    assert_eq!(validate_binding(&b).unwrap_err().rule, "mac-context-not-null");
}
#[test]
fn topology_none_forbids_gateway_content() {
    let mut m = load("manifest-example.json");
    let mut gw = m.get("gateway").unwrap().clone(); gw.set("channel_topology", ab_common::json::Value::s("none")); m.set("gateway", gw);
    assert_eq!(validate_manifest(&m).unwrap_err().rule, "topology-none-with-gateway-content");
}
#[test]
fn continue_degraded_only_where_permitted() {
    let mut m = load("manifest-example.json");
    let mut rv = m.get("revocation").unwrap().clone(); rv.set("task_cancelled", ab_common::json::Value::s("continue-degraded")); m.set("revocation", rv);
    assert_eq!(validate_manifest(&m).unwrap_err().rule, "continue-degraded-not-permitted");
}
