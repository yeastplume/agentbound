// Source provenance, embedded at build time (independent WP3.1 validation, finding 4: every earlier register reported the commit
// from a hand-written file that was never updated, so binaries built from later source claimed an earlier commit).
//
// `crates/build.sh` writes `crates/SOURCE-PROVENANCE` on the build host from the *sending* checkout: the full commit, and whether the
// working tree differed from it (`dirty`) — a dirty build can never be attributed to a reviewed commit and is labelled as such. The
// file is regenerated on every sync, so it can't go stale the way `COMMIT` did. If it is absent the build is `unknown`, never a guess.
fn main() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../SOURCE-PROVENANCE");
    println!("cargo:rerun-if-changed={}", p.display());
    let s = std::fs::read_to_string(&p).unwrap_or_default();
    let get = |k: &str| s.lines().find_map(|l| l.strip_prefix(&format!("{k}="))).unwrap_or("unknown").trim().to_string();
    println!("cargo:rustc-env=AB_SOURCE_COMMIT={}", get("commit"));
    println!("cargo:rustc-env=AB_SOURCE_DIRTY={}", get("dirty"));
    println!("cargo:rustc-env=AB_SOURCE_TREE={}", get("tree"));
}
