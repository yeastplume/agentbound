# Architecture specifications

This directory contains implementation-level specifications and architecture decision records (ADRs) that refine the normative [technical report](../papers/technical-report.md).

## Decision records

- [ADR-0001: Per-session execution identity is distinct from the durable principal identity](ADR-0001-execution-identity.md)

## Planned contents

- normative requirements and threat-model scope for each phase;
- session request and effective-manifest schemas;
- lifecycle and failure-state definitions;
- privileged/unprivileged interface specifications;
- gateway operation and trace-identity schemas;
- audit event and correlation schemas;
- further ADRs as decisions are made.

ADRs record decisions that constrain implementation. The technical report remains the source for invariants and threat model; an ADR that changes either must be reflected there.
