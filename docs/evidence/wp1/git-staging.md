# WP1 evidence — `git-staging`

**Covers:** plan WP1 spike "Git staging-ref adapter and protected-branch behaviour"; R-GW-5; catalogue D-13; invariant 19 (integrity promotion, protected-object subset).
**Baseline:** VM 110, git 2.47.3.
**Spike:** `spikes/git-staging/`. **Raw transcript:** `raw/git-staging.txt`. **Command:** `spikes/run.sh git-staging`.

Shape exercised: the session workspace clone has **no remote and no credential**; it produces a `git bundle` of its new commits (the object payload that crosses the gateway connection — descriptor transfer of the bundle is ADR-0002 D7 item 3, already verified). The gateway fetches the bundle into a quarantine bare repository, runs `git fsck --connectivity-only`, applies its ref policy, and pushes with a credential only the gateway holds. The Git host is a bare repository with a `pre-receive` hook standing in for the host's protected-branch rule (the R-GW-5 **[assumption]**).

## Results

| ID | Required result | Observed | Result |
|---|---|---|---|
| GS-1 | Session produces objects without any remote | bundle created; `git remote` empty | **PASS** |
| GS-2 | Gateway imports from bundle only and verifies connectivity | fetch + `fsck` ok; imported commit matches session `HEAD` | **PASS** |
| GS-3 | Push to `refs/agentbound/<session>/…` permitted | ref present on host at the session commit | **PASS** |
| GS-4 | Every non-staging or forged push refused **by the gateway** (before any transport) | 12/12 denied: `main`, other branch, other session's namespace, trace mismatch, `..` traversal, empty tail, embedded `:` refspec, `+` force marker, `.lock` suffix, whitespace, tag, `HEAD` | **PASS** |
| GS-5 | `main` unchanged after all attempts | unchanged | **PASS** |
| GS-6 | Host protected-branch rule composes even if the gateway is bypassed | forced push to `refs/heads/main` from the gateway repo refused by the host hook; `main` unchanged | **PASS** (validates the assumption's role, not the assumption) |
| GS-7 | Session UID cannot read the gateway credential | `EACCES` as uid 200042 | **PASS** |
| GS-8 | Non-fast-forward to the session's own staging ref refused without explicit force | rejected `(non-fast-forward)` | **PASS** — whether `force` is a distinct operation is a WP2 policy decision; the mechanism refuses by default |

## Notes

- **No findings.** R-GW-5's three refusal classes (other ref, other session, trace mismatch) are all enforceable by string policy in the gateway before any Git process runs; `git check-ref-format` is used as a second filter. The `+`-prefix and embedded-`:` cases matter because the adapter must never pass a session-supplied string into a refspec position; the gateway constructs `refs/incoming/<session>:<validated-target>` itself.
- Bundle import into a quarantine repository gives the gateway a place to `fsck` and to apply size/object-type limits before anything touches the host; it also means the session's objects are never fetched *from* the session by a Git transport, so no `upload-pack` runs in the session.
- The host-side rule is and remains an **[assumption]** (traceability matrix row 19): GS-6 shows what it protects against, not that any particular host enforces it.
