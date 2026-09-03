# ADR-0001: Per-session execution identity is distinct from the durable principal identity

**Status:** Accepted (revised)  
**Date:** 28 August 2026  
**Applies to:** all deployment profiles; normative for the Unix-governed profile  
**Related:** technical report §1.2, §4.1, Invariant 17; Phase 1 plan Gate 2

## Revision history

- **Accepted (revised)** — Decision as frozen at WP0.
- **Editorial** — Decision 2 restructured into sub-bullets under docs/STYLE.md; no content change.

## Context

The architecture identifies an organizational agent with a durable security principal. Concurrent sessions of the same agent must not interfere with each other (Invariant 17). Early drafts allowed the durable principal's host UID to be the identity under which session processes run. They listed PID/IPC namespaces, private procfs, `hidepid`, and Yama `ptrace_scope` as possible ways to isolate same-UID sessions.

That approach does not work. Two processes with the same UID pass ordinary discretionary access checks and signal permission checks against each other. PID namespaces hide identifiers but do not change authorization for a process that obtains a usable PID or pidfd. `hidepid` and `ptrace_scope` distinguish UIDs, not sessions. They are configurable mitigations rather than boundaries. A single leaked descriptor, shared `/run` path, host PID, broker socket, or supplementary-group permission defeats same-UID isolation.

Host UIDs carry little meaning in fleet deployments. Remote services authorize workload identity and gateway policy, not UIDs. Stable per-agent host accounts add provisioning, reconciliation, collision, and reuse burden without providing the required isolation.

## Decision

1. The **durable agent principal** is a policy, ownership, and audit identity with a stable global identifier and an accountable owner. It may be projected into a stable host UID or represented by a storage service for the purpose of owning durable state. In the Unix-governed profile, that identity **does not execute session code**. Higher-assurance profiles may realize the same separation differently (item 5). The invariant is that the kernel or substrate enforcing the session boundary distinguishes the identity that owns durable state from the identity under which a session acts.

2. Every session runs under a **per-session, uniquely allocated execution identity with verified reclamation and reuse quarantine**: a local UID with its own supplementary groups and, in MAC profiles, its own type or category set.
   - *Uniqueness and concurrency.* An identity is never shared between concurrent sessions. Linux UIDs are finite, so "non-reusable" is shorthand for reclamation followed by quarantine.
   - *Reclamation condition.* An identity is reclaimed only when a verifiable condition holds across a **declared managed reclamation domain**: no live process, no owned file or IPC object, and no outstanding grant within that domain.
   - *Managed-domain boundary.* The domain is all session namespaces and mounted filesystems; host paths registered in the manifest; session runtime and workspace stores; broker and storage-service grants; and known IPC namespaces and cgroup state. It is not "every reachable filesystem": detached mounts, removable volumes, backups, snapshots, and restored files can carry numeric ownership outside the allocator's view.
   - *Export rule.* Anything exported beyond the managed domain **must not rely on the numeric UID for durable authorization**. Persistent records and exported objects use the global principal and session identifiers.
   - *Audit disambiguation.* Audit records disambiguate reuse by pairing the execution UID with launch-record, boot, and session identifiers. Retention of audit history does not by itself block reclamation.

3. Durable state activated for a session is reached through **per-session grants**—bind mounts, ACL entries, inherited descriptors, or a storage broker operating on the session's behalf—not by running as the owner.

4. Mount, PID, IPC, and network namespaces, private procfs and runtime directories, private sockets and PTYs, and descriptor allowlisting remain **required supporting controls**. None substitutes for the identity split.

5. A compartmented or multilevel profile **may** use an allocated per-session SELinux type as the primary boundary instead of a distinct UID. A VM-backed profile may use the VM boundary as the execution identity, provided the allocator, reuse policy, and policy analysis are part of that profile's conformance evidence. Neither is a baseline option.

## Consequences

- Invariant 17 becomes a determinate, testable property in the Unix-governed profile.
- The launch record must carry the execution-identity-to-principal-and-session mapping, since kernel audit will report the execution UID.
- The constructor needs an identity allocator whose lifecycle is specified before implementation (Phase 1 plan, WP0): host-local versus fleet-wide uniqueness, allocation source, the managed reclamation domain and its condition, discovery or elimination of owned objects within it, crash-recovery and exhaustion behaviour, and interaction with backups carrying numeric ownership. The allocator is part of the trusted computing base.
- Ownership of durable state is decoupled from execution, which permits a storage broker or a stable owner UID interchangeably and makes a no-stable-host-UID deployment possible.
- Same-principal isolation tests must attempt, from one session against a concurrent sibling: `/proc/<hostpid>`, `kill` and `pidfd_send_signal`, `ptrace` and `process_vm_*`, `/run` and `/tmp` paths, pathname and abstract Unix sockets, shared supplementary-group permissions on durable partitions, broker socket reuse, and every inherited descriptor.
- The position paper's statement that a durable identity "may be projected into a stable, dynamic, or namespace-local UID" is narrowed: a stable UID is an ownership projection only.

## Revision note

VM-backed and MAC-separated profiles follow items 1, 2, and 5. Reuse is bounded by a declared managed domain and its export rule, rather than by every reachable filesystem.

## Alternatives considered

- **Shared durable UID plus namespaces and `ptrace_scope`.** Rejected: not an authorization boundary; defeated by any shared object or identifier leak.
- **Per-session SELinux type only, shared UID.** Viable for MAC profiles with a scalable allocator; rejected as the baseline because the Unix-governed profile must not depend on an installed MAC policy.
- **User namespaces mapping a shared UID to distinct host UIDs.** Provides the host-side split but introduces namespace-scoped capabilities and mount authority for the workload; acceptable only with tightly controlled mappings and no writable mounts, and not preferred over a plain distinct UID.
- **MicroVM per session with a shared UID inside.** Achieves isolation by substrate; compatible with this decision (the VM boundary becomes the execution identity) and is the required control arm in evaluation.
