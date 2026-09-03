# WP1 evidence — `sloc-arms`

**Covers:** open-question register item **VM-2** (cross-arm SLOC comparability); ADR-0003 "Trusted-code size" accounting; requirements §12 SLOC accounting rules (five figures + sixth).
**Tool pinned by this spike:** `tokei 13.0.0-alpha.8` (JSON output, `code` lines only — blanks and comments excluded). Firecracker **v1.16.1** source at `2038188f`; dependency closure from the upstream `Cargo.lock` via `cargo tree -e normal` (build- and dev-dependencies excluded); third-party crate sources from the crates.io registry checkout.
**Spike:** `spikes/sloc-arms/` (`run.sh`, `tk.py`). **Raw transcript:** `raw/sloc-arms.txt`. **Command:** `spikes/run.sh sloc-arms`.

## Measurements (VM arm, per shipped binary)

| Figure | `firecracker` | `jailer` | Method |
|---|---|---|---|
| 1 Direct (workspace crates linked: `firecracker`, `vmm`, `utils`, `acpi_tables` / `jailer`, `utils`; `src/` only) | **77 904** (Rust 77 861, Shell 43) | **3 494** | tokei over each crate's `src/` |
| 3 Transitive third-party (whole crate directories, 67 / 14 crates) | **2 819 798** | **279 790** | tokei over each resolved registry crate |
| 3 of which Rust | 1 429 364 | 272 013 | |
| 5 Unsafe-by-default language in the closure | **1 288 000** ≈ (GNU asm 881 671, Assembly 79 471, C 109 205, C headers 108 335, C++ 109 294) | **none** | tokei language split |
| 4 Configuration (shipped seccomp filters, `resources/seccomp`) | JSON 2 831 | — | tokei |
| — `unsafe` occurrences in workspace `src/` (indicative, not a SLOC figure) | 411 | — | grep |

The VM arm ships both binaries per session, so the arm's figures are the sums.

### What dominates Figure 3/5

`aws-lc-sys 0.41.0` alone is 1 798 731 lines: the **AWS-LC** C/assembly cryptographic library, vendored in the crate with per-platform generated assembly (`generated-src/linux-x86_64` alone is 167 602 lines of GNU assembly; nine other platform directories are also present in the source tree but never compiled on x86-64 Linux). It enters via `vmm → aws-lc-rs 1.17.0`, used only for `rand` in `virtio-rng`, `vmgenid`, and the aarch64 FDT. Next: `linux-raw-sys 0.12.1` (478 956, generated bindings), `libc 0.2.186` (112 662, largely generated per-platform modules).

### Linux-arm stand-in

The same procedure over `spikes/identity-store` (Rust, `libc` + `rusqlite` + `sha2`) yields direct 182, 20 third-party crates; `rusqlite` with `bundled` pulls the SQLite amalgamation (C) into Figure 5 exactly as `aws-lc-sys` does on the VM arm. The attribution method is identical on both arms; the only arm-specific input is the `Cargo.lock`.

## Disposition of VM-2

**The pinned tool attributes transitive dependencies consistently across arms; comparability does not fail on attribution.** Three accounting rules are needed, and they apply symmetrically:

1. **Compiled-vs-present.** Whole-crate counting overstates by including sources never compiled for the target (aws-lc-sys's nine other platform directories; libc's non-Linux modules). The accounting rule MUST state whether Figure 3 counts *source present in the resolved crate* (reproducible, tool-only, what this spike did) or *source compiled for the target* (requires build introspection — e.g. `cargo build --build-plan`/`-Zbuild-std`-style file lists or `cc` invocation capture for `-sys` crates). Recommendation: report the *present* figure as the reproducible primary and the *compiled* figure as a secondary where obtainable; state this in ADR-0003.
2. **Generated code (Figure 2) in third-party crates** cannot be separated by the tool alone; upstream commits bindgen output as ordinary source (`kvm-bindings`, `linux-raw-sys`, `libc`, aws-lc `generated-src`). Figure 2 therefore needs a manual, published allowlist of generated paths per arm. Same rule for both arms.
3. **Feature-gated dependencies.** `cargo tree -e normal` with default features was used; the daemon's actual Firecracker build features and the Linux arm's crate features MUST be pinned in the accounting so the closure is reproducible.

**Finding F-8 (ADR-0003, accounting).** The VM arm's trusted closure contains ≈1.29 M lines of C/C++/assembly (AWS-LC) that the Linux arm has no counterpart to, and the arm's Figure 3 is an order of magnitude larger than the jailer's or any plausible Linux-arm daemon closure. This is exactly what the per-arm disclosure is for; it is not a comparability failure, but ADR-0003 should (a) record the three accounting rules above, (b) name tokei 13.0.0-alpha.8 (or a chosen release) as the pinned tool, and (c) note that Firecracker's `aws-lc-rs` dependency exists solely for randomness and that a Firecracker build without `virtio-rng`/`vmgenid` — or an upstream feature to use `getrandom` — would remove it; the Phase 1 control arm ships upstream binaries, so this is disclosure, not a change.

The register's failure branch ("per-arm disclosure only, excluded from the decision rule") is already the rule; VM-2 confirms the exclusion is **not** triggered by attribution inconsistency, but the numbers themselves argue for keeping code size out of the cross-arm score, as ADR-0003 already does.
