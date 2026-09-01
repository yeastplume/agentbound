# ADR-0001: Per-session execution identity is distinct from the durable principal identity

**Status:** Accepted  
**Date:** 28 August 2026  
**Applies to:** all deployment profiles; normative for the Unix-governed profile  
**Related:** technical report §1.2, §4.1, Invariant 17; Phase 1 plan Gate 2

## Context

The architecture identifies an organizational agent with a durable security principal and states that concurrent sessions of the same agent must not interfere with each other (Invariant 17). Early drafts allowed the durable principal's host UID to be the identity under which session processes run, and listed PID/IPC namespaces, private procfs, `hidepid`, and Yama `ptrace_scope` as possible ways to isolate same-UID sessions.

Independent mechanism review established that this does not work. Two processes with the same UID pass ordinary discretionary access checks and signal permission checks against each other. PID namespaces hide identifiers but do not change authorization for a process that obtains a usable PID or pidfd. `hidepid` and `ptrace_scope` distinguish UIDs, not sessions, and are configurable mitigations rather than boundaries. A single leaked descriptor, shared `/run` path, host PID, broker socket, or supplementary-group permission defeats same-UID "isolation."

Separately, practitioner review observed that host UIDs carry little meaning in fleet deployments: remote services authorize workload identity and gateway policy, not UIDs, and stable per-agent host accounts add provisioning, reconciliation, collision, and reuse burden without providing the isolation the design needs.

## Decision

1. The **durable agent principal** is a policy, ownership, and audit identity with a stable global identifier and an accountable owner. It may be projected into a stable host UID (or represented by a storage service) for the purpose of owning durable state. That identity **never executes session code**.

2. Every session runs under a **per-session execution identity**: a non-reusable local UID with its own supplementary groups and, in MAC profiles, its own type or category set. Execution identities are allocated by the constructor, never shared between concurrent sessions, and not reused while any object or record bound to the previous session remains ambiguous.

3. Durable state activated for a session is reached through **per-session grants**—bind mounts, ACL entries, inherited descriptors, or a storage broker operating on the session's behalf—not by running as the owner.

4. Mount, PID, IPC, and network namespaces, private procfs and runtime directories, private sockets and PTYs, and descriptor allowlisting remain **required supporting controls**, but none substitutes for the identity split.

5. A compartmented or multilevel profile **may** use a rigorously allocated per-session SELinux type as the primary boundary instead of a distinct UID, provided the allocator, reuse policy, and policy analysis are part of that profile's conformance evidence. This is not a baseline option.

## Consequences

- Invariant 17 becomes a determinate, testable property in the Unix-governed profile.
- The launch record must carry the execution-identity-to-principal-and-session mapping, since kernel audit will report the execution UID.
- The constructor needs an identity allocator with a reuse policy and quarantine period; this is part of the trusted computing base and must be reviewed.
- Ownership of durable state is decoupled from execution, which permits a storage broker or a stable owner UID interchangeably and makes a no-stable-host-UID deployment possible.
- Same-principal isolation tests must attempt, from one session against a concurrent sibling: `/proc/<hostpid>`, `kill` and `pidfd_send_signal`, `ptrace` and `process_vm_*`, `/run` and `/tmp` paths, pathname and abstract Unix sockets, shared supplementary-group permissions on durable partitions, broker socket reuse, and every inherited descriptor.
- The position paper's statement that a durable identity "may be projected into a stable, dynamic, or namespace-local UID" is narrowed: a stable UID is an ownership projection only.

## Alternatives considered

- **Shared durable UID plus namespaces and `ptrace_scope`.** Rejected: not an authorization boundary; defeated by any shared object or identifier leak.
- **Per-session SELinux type only, shared UID.** Viable for MAC profiles with a scalable allocator; rejected as the baseline because the Unix-governed profile must not depend on an installed MAC policy.
- **User namespaces mapping a shared UID to distinct host UIDs.** Provides the host-side split but introduces namespace-scoped capabilities and mount authority for the workload; acceptable only with tightly controlled mappings and no writable mounts, and not preferred over a plain distinct UID.
- **MicroVM per session with a shared UID inside.** Achieves isolation by substrate; compatible with this decision (the VM boundary becomes the execution identity) and is the required control arm in evaluation.
