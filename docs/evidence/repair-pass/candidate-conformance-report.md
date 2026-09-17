# Agentbound conformance run — 1A + 1B rows (machine output)

- Host: agentbound-dev
- Kernel: 6.12.107+deb13-cloud-amd64
- systemd: systemd 257 (257.13-1~deb13u1)
- git: git version 2.47.3
- Date: 2026-09-17T10:38:20Z
- Run id: run-1026275340586965
- Source commit (embedded at build from the sending checkout): 3b038cb (DIRTY — not attributable to a reviewed commit)
- Installed binaries: all installed binaries embed the same commit and dirty state as this runner
- Expected population: 121 catalogue ids (test-catalogue 1A+1B)
- Assertions: 107 PASS, 1 WEAK, 2 RECORDED, 42 FAIL (4 fixtures excluded)
- Catalogue coverage: 70 PASS, 0 WEAK, 2 RECORDED, 37 FAIL, **12 NOT-EXECUTED**
- Duplicate row ids: none
- Row ids outside the catalogue: GW-COMPLETE
- **Suite verdict: FAIL** (no assertion false)
- **Coverage verdict: FAIL (coverage, duplicate, extra, or PROVENANCE failure)** (every frozen id executed once, none outside the catalogue)
- **Requirements verdict: FAIL** (every mandatory id satisfies its required verdict; unmet: D-02=FAIL(required RECORDED), D-03=FAIL(required PASS), D-05=FAIL(required PASS), D-08=FAIL(required PASS), D-09=NOT-EXECUTED(required PASS), D-10=FAIL(required PASS), D-12=FAIL(required PASS), D-13=FAIL(required PASS), D-16=FAIL(required PASS), D4=FAIL(required PASS), D7-8=FAIL(required PASS), D7-9=FAIL(required PASS), F-T-01=FAIL(required PASS), F-T-05=FAIL(required PASS), F-T-06=FAIL(required PASS), F-T-07=FAIL(required PASS), F-T-08=FAIL(required PASS), F-T-10=FAIL(required PASS), F-T-11=FAIL(required PASS), T-6.1-003=FAIL(required RECORDED), T-6.1-013=FAIL(required PASS), T-6.3-001=NOT-EXECUTED(required PASS), T-6.3-002=NOT-EXECUTED(required PASS), T-6.3-003=NOT-EXECUTED(required PASS), T-6.3-004=NOT-EXECUTED(required PASS), T-6.3-006=NOT-EXECUTED(required PASS), T-6.3-007=FAIL(required PASS), T-6.4-001=NOT-EXECUTED(required PASS), T-6.4-002=FAIL(required PASS), T-6.4-003=FAIL(required PASS), T-6.4-004=FAIL(required PASS), T-6.4-005=FAIL(required PASS), T-6.4-006=NOT-EXECUTED(required PASS), T-6.4-007=NOT-EXECUTED(required PASS), T-6.4-008=FAIL(required PASS), T-6.4-010=NOT-EXECUTED(required PASS), T-6.4-011=NOT-EXECUTED(required PASS), T-6.4-014=FAIL(required PASS), T-6.5-003=FAIL(required PASS), T-6.5-005=FAIL(required PASS), T-6.5-008=FAIL(required PASS), T-6.6-007=FAIL(required PASS), T-6.7-001=FAIL(required PASS), T-6.8-008=FAIL(required PASS), T-6.8-009=FAIL(required PASS), T-6.8-012=FAIL(required PASS), T-6.9-005=FAIL(required PASS), T-6.9-006=NOT-EXECUTED(required PASS), T-6.9-008=FAIL(required PASS))
- **Run verdict: FAIL (coverage or assertion)** — PASS only when all three hold

## Provenance

| Artifact | SHA-256 | Embedded commit | Dirty |
|---|---|---|---|
| `agentbound` | `b2a3ec9399a86b67510297fc96e01be9254584468c3225eaffed0387e331929d` | `3b038cb` | true |
| `agentbound-launch` | `55dd37fb9f001bae0b636095c13ae4f3e43be93badab7ebd1097a9c242cd22ed` | `3b038cb` | true |
| `agentbound-lifecycle` | `1f4006ad99113187fb787b95a7ef7f337d0878205229edca1826a8c675faf322` | `3b038cb` | true |
| `agentbound-policy` | `760f770a408758236b976a9a999663490c12f3f4c7dbeab12ca0c34ea6afe727` | `3b038cb` | true |
| `agentbound-audit` | `4d54af94ce57a149c86b7fc807a4f8c018a7e6e3352d851322d5fd50d7b94305` | `3b038cb` | true |
| `agentbound-gateway` | `badf00d9049cf2045bbe9150e6c5cff060d935f11e5bbd38cf319bdda8e83408` | `3b038cb` | true |
| `ab-conformance` | `a5024eafaa7ff709e8616ecdc2399358f4bdc2458cba52f6675b2e9d6adbb068` | `3b038cb` | true |
| `ab-gwclient` (in image) | `1c72f94aa4d6ec94bd535d9256d47ac3432aa952c004ca486d80cb4cb4c9a637` | — | — |
| `/etc/agentbound/catalogue.json` | `90a43ff1f4d8bbf3202a49ad826a8deb2046f7d06e4ffca01e2976b60d6c4bf0` | — | — |

## Catalogue coverage

| Catalogue id | Milestone | Best verdict | Required | Satisfied |
|---|---|---|---|---|
| D-01 | 1A | PASS | PASS | yes |
| D-02 | 1A | FAIL | RECORDED | **NO** |
| D-03 | 1A | FAIL | PASS | **NO** |
| D-04 | 1A | PASS | PASS | yes |
| D-05 | 1A | FAIL | PASS | **NO** |
| D-06 | 1A | PASS | PASS | yes |
| D-07 | 1A | PASS | PASS | yes |
| D-08 | 1A | FAIL | PASS | **NO** |
| D-09 | 1B | NOT-EXECUTED | PASS | **NO** |
| D-10 | 1B | FAIL | PASS | **NO** |
| D-11 | 1A | PASS | PASS | yes |
| D-12 | 1B | FAIL | PASS | **NO** |
| D-13 | 1B | FAIL | PASS | **NO** |
| D-15 | 1A | RECORDED | RECORDED | yes |
| D-16 | 1A/1B/1C | FAIL | PASS | **NO** |
| D4 | 1B | FAIL | PASS | **NO** |
| D7-8 | 1B | FAIL | PASS | **NO** |
| D7-9 | 1B | FAIL | PASS | **NO** |
| F-C-01 | 1A | PASS | PASS | yes |
| F-C-02 | 1A | PASS | PASS | yes |
| F-C-03 | 1A | PASS | PASS | yes |
| F-C-04 | 1A | PASS | PASS | yes |
| F-C-05 | 1A | PASS | PASS | yes |
| F-C-06 | 1A | PASS | PASS | yes |
| F-C-07 | 1A | PASS | PASS | yes |
| F-C-08 | 1B | PASS | PASS | yes |
| F-C-09 | 1A | PASS | PASS | yes |
| F-T-01 | 1A/1B | FAIL | PASS | **NO** |
| F-T-02 | 1A | PASS | PASS | yes |
| F-T-03 | 1A | PASS | PASS | yes |
| F-T-04 | 1A | PASS | PASS | yes |
| F-T-05 | 1A | FAIL | PASS | **NO** |
| F-T-06 | 1B | FAIL | PASS | **NO** |
| F-T-07 | 1B | FAIL | PASS | **NO** |
| F-T-08 | 1A | FAIL | PASS | **NO** |
| F-T-09 | 1B | PASS | PASS | yes |
| F-T-10 | 1A | FAIL | PASS | **NO** |
| F-T-11 | 1A | FAIL | PASS | **NO** |
| T-6.1-001 | 1A | PASS | PASS | yes |
| T-6.1-002 | 1A | PASS | PASS | yes |
| T-6.1-003 | 1A | FAIL | RECORDED | **NO** |
| T-6.1-004 | 1A | PASS | PASS | yes |
| T-6.1-005 | 1A | PASS | PASS | yes |
| T-6.1-006 | 1A | PASS | PASS | yes |
| T-6.1-007 | 1A | PASS | PASS | yes |
| T-6.1-008 | 1A | PASS | PASS | yes |
| T-6.1-009 | 1A | PASS | PASS | yes |
| T-6.1-010 | 1A | PASS | PASS | yes |
| T-6.1-011 | 1A | PASS | PASS | yes |
| T-6.1-012 | 1A | PASS | PASS | yes |
| T-6.1-013 | 1A | FAIL | PASS | **NO** |
| T-6.2-001 | 1A | PASS | PASS | yes |
| T-6.2-002 | 1A | PASS | PASS | yes |
| T-6.2-003 | 1A | PASS | PASS | yes |
| T-6.2-004 | 1A | PASS | PASS | yes |
| T-6.2-005 | 1A | PASS | PASS | yes |
| T-6.2-006 | 1A | PASS | PASS | yes |
| T-6.2-007 | 1A | PASS | PASS | yes |
| T-6.2-008 | 1A | RECORDED | RECORDED | yes |
| T-6.2-009 | 1A | PASS | PASS | yes |
| T-6.3-001 | 1B | NOT-EXECUTED | PASS | **NO** |
| T-6.3-002 | 1B | NOT-EXECUTED | PASS | **NO** |
| T-6.3-003 | 1B | NOT-EXECUTED | PASS | **NO** |
| T-6.3-004 | 1B | NOT-EXECUTED | PASS | **NO** |
| T-6.3-005 | 1B | PASS | PASS | yes |
| T-6.3-006 | 1B | NOT-EXECUTED | PASS | **NO** |
| T-6.3-007 | 1B | FAIL | PASS | **NO** |
| T-6.3-008 | 1B | PASS | PASS | yes |
| T-6.4-001 | 1B | NOT-EXECUTED | PASS | **NO** |
| T-6.4-002 | 1B | FAIL | PASS | **NO** |
| T-6.4-003 | 1B | FAIL | PASS | **NO** |
| T-6.4-004 | 1B | FAIL | PASS | **NO** |
| T-6.4-005 | 1B | FAIL | PASS | **NO** |
| T-6.4-006 | 1B | NOT-EXECUTED | PASS | **NO** |
| T-6.4-007 | 1B | NOT-EXECUTED | PASS | **NO** |
| T-6.4-008 | 1B | FAIL | PASS | **NO** |
| T-6.4-009 | 1B | PASS | PASS | yes |
| T-6.4-010 | 1B | NOT-EXECUTED | PASS | **NO** |
| T-6.4-011 | 1B | NOT-EXECUTED | PASS | **NO** |
| T-6.4-012 | 1B | PASS | PASS | yes |
| T-6.4-013 | 1B | PASS | PASS | yes |
| T-6.4-014 | 1B | FAIL | PASS | **NO** |
| T-6.5-001 | 1A | PASS | PASS | yes |
| T-6.5-002 | 1A | PASS | PASS | yes |
| T-6.5-003 | 1A | FAIL | PASS | **NO** |
| T-6.5-004 | 1A | PASS | PASS | yes |
| T-6.5-005 | 1A | FAIL | PASS | **NO** |
| T-6.5-006 | 1A | PASS | PASS | yes |
| T-6.5-007 | 1A | PASS | PASS | yes |
| T-6.5-008 | 1A | FAIL | PASS | **NO** |
| T-6.5-009 | 1A | PASS | PASS | yes |
| T-6.5-010 | 1A | PASS | PASS | yes |
| T-6.6-001 | 1A | PASS | PASS | yes |
| T-6.6-002 | 1A | PASS | PASS | yes |
| T-6.6-003 | 1A | PASS | PASS | yes |
| T-6.6-004 | 1A | PASS | PASS | yes |
| T-6.6-005 | 1A | PASS | PASS | yes |
| T-6.6-006 | 1A | PASS | PASS | yes |
| T-6.6-007 | 1A | FAIL | PASS | **NO** |
| T-6.6-008 | 1A | PASS | PASS | yes |
| T-6.7-001 | 1A | FAIL | PASS | **NO** |
| T-6.8-001 | 1A | PASS | PASS | yes |
| T-6.8-002 | 1A | PASS | PASS | yes |
| T-6.8-003 | 1A | PASS | PASS | yes |
| T-6.8-004 | 1A | PASS | PASS | yes |
| T-6.8-005 | 1A | PASS | PASS | yes |
| T-6.8-006 | 1A | PASS | PASS | yes |
| T-6.8-007 | 1A | PASS | PASS | yes |
| T-6.8-008 | 1B | FAIL | PASS | **NO** |
| T-6.8-009 | 1B | FAIL | PASS | **NO** |
| T-6.8-011 | 1A | PASS | PASS | yes |
| T-6.8-012 | 1A | FAIL | PASS | **NO** |
| T-6.8-013 | 1A | PASS | PASS | yes |
| T-6.9-001 | 1A | PASS | PASS | yes |
| T-6.9-002 | 1A | PASS | PASS | yes |
| T-6.9-003 | 1A | PASS | PASS | yes |
| T-6.9-004 | 1A | PASS | PASS | yes |
| T-6.9-005 | 1B | FAIL | PASS | **NO** |
| T-6.9-006 | 1B | NOT-EXECUTED | PASS | **NO** |
| T-6.9-007 | 1A | PASS | PASS | yes |
| T-6.9-008 | 1B/1C | FAIL | PASS | **NO** |

## Rows

| Row | Verdict | Evidence |
|---|---|---|
| D-01 | PASS | rc=0 lrd=sha256:01ec38da42b0812476df180d36d9c2665a376a0f58431c10d45d3aabe7470255 {"allocation_id":"allocation:45ed3a21-00002973","console":"/var/lib/agentbound/sessions/45ed3a21-00002973/console.log","init_pid":901489,"launch_record_digest":"sha256:01ec38da42b0812476df180d36d9c266 |
| T-6.1-001 | PASS | pids_visible=5 |
| T-6.1-001.foreign | PASS | all visible pids ours |
| T-6.2-006.pidns | PASS | pid=2 |
| T-6.1-001.init-environ | PASS | denied rc=1 (init environ) |
| T-6.1-002 | PASS | denied rc=1 kill -0 host-range pid |
| T-6.1-002.pid300 | PASS | denied rc=1 kill -0 300 (exists on host) |
| T-6.1-004 | PASS | denied rc=1 ls /run/agentbound |
| T-6.1-005 | PASS | no /dev/shm; ipc ns private |
| T-6.1-007 | PASS | denied rc=1 ls /var/lib/agentbound |
| T-6.1-007.etc | PASS | denied rc=1 ls /etc/agentbound |
| T-6.1-009 | PASS | fds=0 1 2 3  |
| T-6.2-001 | PASS | denied rc=1 write cgroup.procs |
| T-6.2-001.sysfs | PASS | no cgroupfs |
| T-6.2-002.mount | PASS | denied rc=255 mount tmpfs |
| T-6.2-006.proc | PASS | denied rc=1 mount proc |
| T-6.2-003 | PASS | denied rc=1 setuid copy (nosuid tmpfs) |
| T-6.2-004 | PASS | CapEff=0 |
| T-6.2-004.nnp | PASS | nnp=1 |
| T-6.2-007.image | PASS | denied rc=1 write image (ro) |
| T-6.2-007.root | PASS | denied rc=1 write root tmpfs |
| T-6.2-007.workspace | PASS | workspace writable as 200002 |
| T-6.1-010 | PASS | pidfd_open=901347 rc=-1 errno=3 |
| T-6.1-010.own-ns | FIXTURE | pidfd against our own init: pidfd_open=1 rc=3 send_signal_rc=0 send_signal_errno=0 |
| T-6.1-011 | PASS | process_vm_readv pid=901347 rc=-1 errno=3 |
| T-6.1-012 | PASS | host abstract name unreachable (abstract connect name=agentbound-conf-abs rc=-1 errno=111) and the same name binds freely here (abstract bind name=agentbound-conf-abs rc=0 errno=0) — separate abstract namespaces |
| T-6.1-006 | PASS | symlinks to host catalogue, lifecycle store and / are all unusable from the session temp dir (writes refused; the targets do not exist in this mount namespace) |
| T-6.1-008 | PASS | no sibling startup file could be written (cannot write a shared /workspace/.profile); this session's environment is private to it |
| T-6.2-009 | PASS | denied rc=1 ls /sys/class/net |
| T-6.2-002.netdev | PASS | denied rc=1 interfaces other than lo |
| T-6.2-005 | PASS | double-forked orphan pid=62 reparented to in-namespace ppid=1 and still visible in this pid namespace (contained); reaping asserted by D-07 at termination |
| T-6.9-002 | PASS | opened=1021 stopped at RLIMIT_NOFILE=1024 with EMFILE(24) |
| T-6.9-004.bytes | PASS | /tmp tmpfs capacity=262144KiB; write stopped at 264105984 bytes: SIGKILL by memory cgroup (tmpfs pages charged to memory.max; disk bound not reached first) |
| T-6.9-004.inodes | PASS | nr_inodes=65536; creation refused after 65523 files, IFree=0 |
| T-6.9-003.memory | PASS | 512 MiB request against memory.max=not-visible-in-session (read back by the driver): refused (rc=137 ) |
| T-6.9-001 | PASS | procs=63 (TasksMax bound) |
| PROBE-COMPLETE | FIXTURE | probe lines=42 |
| T-6.1-012.host-name | FIXTURE | host abstract name agentbound-conf-abs bound in the host netns while the probe ran (ss listeners for the name = 1) |
| D-01.status | PASS | {"identity_state":"in-use","observation_seq":3,"reason":null,"record_ref":"sha256:01ec38da42b0812476df180d36d9c2665a376a0f58431c10d45d3aabe7470255","state":"active"} |
| D-06 | PASS | scope procs=64 (init + workload + orphan/fan-out survivors) |
| T-6.9-004.readback | PASS | binding installed_value vs kernel (binding/kernel): pids=64/64 memory_bytes=268435456/268435456 cpu=1000/1000 io_bandwidth=52428800/52428800 disk_bytes=268435456/268435456 disk_inodes=65536/65536 file_descriptors=1024/1024  |
| T-6.9-003.owners | PASS | audit_capacity installed_value=Some(10000) (reserved with the receiver, which is reachable), delegation_fanout installed_value=Some(0) |
| T-6.9-003.cpu | PASS | cpu.max quota=Some(1000) milli-cpu (read back); cpu.stat nr_throttled=97 (quota installed and accounted; throttling occurs only under contention, which this probe does not guarantee) |
| D-04.host-view | PASS | uid 200002 processes outside scope=0 |
| D-08 | FAIL | {"cgroup_kill_written":true,"cgroup_procs_remaining":[],"credential_scan_inside_scope":[901489],"credential_scan_outside_scope":[],"d_state":[],"elapsed_ms":2038,"freeze_written":true,"frozen_observed":false,"gateway_admission_denied":null,"init_pid":901489,"init_pidfd_exited":true,"sigterm_sent":true} |
| F-T-03 | PASS | {"cgroup_kill_written":true,"cgroup_procs_remaining":[],"credential_scan_inside_scope":[901489],"credential_scan_outside_scope":[],"d_state":[],"elapsed_ms":2038,"freeze_written":true,"frozen_observed":false,"gateway_admission_denied":null,"init_pid":901489,"init_pidfd_exited":true,"sigterm_sent":true} |
| F-T-04 | PASS | kill written without waiting for frozen 1; procs empty; pidfd exited |
| D-07 | PASS | orphan/double-fork survivors killed with the scope; host credential scan clean |
| F-T-10 | FAIL | {"identity_state":"in-use","observation_seq":7,"reason":"retry","record_ref":"sha256:01ec38da42b0812476df180d36d9c2665a376a0f58431c10d45d3aabe7470255","state":"termination-incomplete"} |
| F-T-11 | FAIL | audit kinds=["session.launch_record_committed", "session.activated", "session.termination_started", "session.termination_incomplete", "session.termination_started", "session.termination_incomplete"] |
| F-T-08 | FAIL | session dir removed=false; workspace root retained by durable owner |
| T-6.2-007.host | PASS | workspace root after cleanup: root:root 2770 |
| T-6.5-001.unknown | PASS | class=reject rule=request_schema detail=unknown-member at request: uid |
| T-6.5-001.dup | PASS | class=reject rule=duplicate-member detail=DuplicateMember("approval_references") |
| T-6.5-007 | PASS | class=reject rule=request_schema detail=unknown-member at request: mount |
| T-6.5-006 | PASS | class=reject rule=request_schema detail=version at request.schema_version: unsupported |
| T-6.5-002.deep | PASS | class=reject rule=depth-limit detail=TooDeep(5) |
| T-6.5-002.big | PASS | class=reject rule=size-limit detail=TooLarge(20021) |
| T-6.6-001.principal | PASS | class=reject rule=unknown_principal detail=agent:nobody |
| T-6.6-001.authority | PASS | class=reject rule=authority_exceeded detail=resource resource:workspace-eng |
| T-6.6-003 | PASS | class=reject rule=approval_missing detail=1 required |
| T-6.6-002.expired | PASS | class=reject rule=approval_expired detail=approval:eng-1234-expired |
| T-6.6-002.stale | PASS | class=reject rule=approval_replayed detail=key:dave seq 1 |
| T-6.6-005 | PASS | class=reject rule=budget_exceeds_policy detail=pids |
| T-6.6-006 | PASS | class=reject rule=unknown_runtime detail=runtime:evil |
| T-6.6-008 | PASS | class=reject rule=request_schema detail=grammar at request.agent_principal_id: catalogue identifier |
| T-6.8-013 | PASS | class=reject rule=continue_degraded_not_permitted detail=continue-degraded-not-permitted at manifest.revocation.task_cancelled: only policy_service_unavailable and audit_pipelin |
| T-6.5-010.wrong-caller | PASS | class=reject rule=initiator_unauthenticated detail=credential reference not bound to caller |
| T-6.8-001.disabled | PASS | class=reject rule=initiator_disabled detail= |
| T-6.6-002.replayed | PASS | first rc=1 rule=approval_replayed second rule=approval_replayed (durable consumption across runs) |
| T-6.6-004 | PASS | scheduled_without_owner |
| T-6.6-004.owned | PASS | {"approvers":[],"initiators":[{"credential_reference":"authn:cron-owned","id":"svc:cron","relationship":"scheduled"}],"owner":"human:alice","scheduler":"svc:cron"} |
| T-6.5-010.lifecycle | PASS | {"body":{"detail":"uid 1001 may not call reserve_identity","rule":"peer_not_permitted"},"class":"unauthenticated","ok":false,"v":"agentbound.wire.v0.1"} |
| T-6.6-001.audit | PASS | session.rejected events with failed_input=1619 |
| F-C-01 | PASS | barrier never released: child pid=967475 reaped by rollback=true, still alive=false; identity=quarantined; scopes_left=0; rollback=["child killed and reaped","cgroup.kill","scope stopped","identity → reclaiming"] |
| F-C-02 | PASS | step=2 rule=child_step_failed identity=quarantined scopes_left=0 rollback=["child killed and reaped","cgroup.kill","scope stopped","identity → reclaiming"] |
| F-C-03 | PASS | step=3 rule=mount_source_escape identity=quarantined scopes_left=0 rollback=["cgroup.kill","scope stopped","identity → reclaiming"] |
| F-C-04 | PASS | step=4 rule=child_step_failed identity=quarantined scopes_left=0 rollback=["child killed and reaped","cgroup.kill","scope stopped","identity → reclaiming"] |
| F-C-05 | PASS | step=5 rule=child_step_failed identity=quarantined scopes_left=0 rollback=["child killed and reaped","cgroup.kill","scope stopped","identity → reclaiming"] |
| F-C-05.no-host-proc | PASS | host mount table shows 0 session-tree mounts after the aborted construction (the child's proc/sysfs died with its namespace) |
| F-C-06 | PASS | step=6 rule=child_step_failed identity=quarantined scopes_left=0 rollback=["child killed and reaped","cgroup.kill","scope stopped","identity → reclaiming"] |
| F-C-06.own-check | PASS | a descriptor surviving the closure pass was caught by step 6's own verification through the fresh /proc: leaked 3:/image; the session never activated (audit kinds=[]) |
| F-C-07 | PASS | step=7 rule=fault_injected identity=quarantined scopes_left=0 rollback=["child killed and reaped","cgroup.kill","scope stopped","identity → reclaiming"] |
| F-C-09 | PASS | step=8 rule=fault_injected identity=quarantined scopes_left=0 rollback=["child killed and reaped","cgroup.kill","scope stopped","identity → reclaiming"] |
| F-C-09.record | PASS | lrd=sha256:d280843bd38acc0d44730fc7852a860654c8066b6bc1170b0d48427d45a535bb kinds=["session.launch_record_committed", "session.construction_failed", "session.construction_failed", "session.ownership_projected", "session.cleanup_completed", "session.identity_released", "session.sealed"] |
| D-11 | PASS | all 11 constructor fault rows (F-C-01..09, one per failing step): no runnable session, identity held, scope gone |
| T-6.5-004 | PASS | activations=1 refusals=1 |
| T-6.5-009 | PASS | allocator latest states: [('free', 2950), ('quarantined', 32)]; free-before-floor violations=0 |
| T-6.8-setup | FIXTURE | sha256:97f4bb8cf9d591032717f1589d070036fdd46b8a80b9f25d4a8bfd62605b804a |
| T-6.8-006 | PASS | {"behaviour":"continue-degraded","state":"active"} |
| T-6.8-011 | PASS | {"behaviour":"continue-degraded","state":"active"} |
| T-6.8-007 | PASS | {"behaviour":"quiesce","state":"quiescing"} |
| F-T-02 | PASS | populated 1 frozen 1 |
| T-6.8-003 | PASS | {"behaviour":"terminate","state":"cleaned/sealed"} |
| T-6.8-006.audit | PASS | ["session.launch_record_committed", "session.activated", "session.revocation_received", "session.degraded", "session.revocation_received", "session.degraded", "session.revocation_received", "session.quiesce_started", "session.revocation_received", "session.termination_started", "session.terminated", "session.ownership_projected", "session.cleanup_completed", "session.identity_released", "session.sealed"] |
| T-6.8-001 | PASS | trigger=initiator_disabled behaviour=terminate state=cleaned/sealed |
| T-6.8-002 | PASS | trigger=approval_expired behaviour=quiesce state=quiescing |
| T-6.8-004 | PASS | trigger=catalogue_withdrawn behaviour=quiesce state=quiescing |
| T-6.8-004.policy | PASS | trigger=policy_withdrawn behaviour=terminate state=cleaned/sealed |
| T-6.8-005 | PASS | trigger=task_cancelled behaviour=terminate state=cleaned/sealed |
| T-6.8-012 | FAIL | procs_while_down=3 (containment held, no authority available: daemon_reachable=false) cli_reply={"class":"unavailable","detail":"Connection refused (os erro after_restart= kinds=["session.launch_record_committed", "session.activated"] |
| T-6.8-012.contained | FAIL | state= identity= procs=3 |
| T-6.9-007 | PASS | audit chain head=sha256:6f47ffbc2c51c35f1a24769cd7e4457eb37d5d900f3bb5a3980e349397e5ebd9 seq=228 lost=0 |
| T-6.7-001 | FAIL | axes measured against the committed manifest: 0 declared mount intents vs 0 mounts observed in the session, 0 declared grants (1A: none), 0 descriptors held by the workload; a child of the workload gained nothing (mounts/fds = nsenter: failed to parse pid: '-m' ). Every recovery path was refused: remount a mount read-write → nsenter: failed to parse pid: ; mount a fresh tmpfs (new mount authority) → nsenter: failed to parse pid: ; raise its own pid limit → nsenter: failed to parse pid: ; raise its own memory limit → nsenter: failed to parse pid: ; raise RLIMIT_NOFILE above the installed hard bound → nsenter: failed to parse pid: ; re-exec with elevated privilege → nsenter: failed to parse pid: ; acquire a new grant by writing a catalogue → nsenter: failed to parse pid: . No axis increases, and authority cannot be restored from inside. |
| T-6.5-005 | FAIL | the catalogue's pids limit was rewritten every 150 ms while eight requests raced it: 0 admitted, 8 rejected. Every admitted session's installed pids.max equals the value in its OWN committed manifest, never a mixture and never a value that only existed between decisions: ["rc=1  installed=", "rc=1  installed=", "rc=1  installed=", "rc=1  installed=", "rc=1  installed=", "rc=1  installed=", "rc=1  installed=", "rc=1  installed="]. A request either serializes on one coherent catalogue state or is rejected. |
| D-05 | FAIL | three runtimes (runtime:sh, runtime:scripted-loop, runtime:probe) on the same task and resource: each got its own uid from the allocator range ([0, 0, 0], all distinct=false) and its own scope cgroup (distinct=false); the committed namespace set is identical across all three (true: ) with every namespace the record commits as private in fact distinct from PID 1's (false); and each session recorded the same boundary-establishing audit events (true: [0, 0, 0] of 3 present in every case). Substituting the runtime changes the workload, not the boundary. |
| D-03 | FAIL | setup failed: first principal's session rc=1, second principal's session rc=1 rule= (uid_a= uid_b=, approval sequence 33) — the row cannot be evaluated without two live sessions of different principals |
| T-6.5-008 | FAIL | four confusions of a genuine committed record, each replayed to commit_binding: (1) constructor envelope presented as the policy envelope, (2) manifest mutated after signing, (3) valid policy signature over the wrong object, (4) signature bytes corrupted. All refused, each naming the check that caught it: ["/", "/", "/", "/"] |
| T-6.6-007 | FAIL | replaying a correctly signed binding for an allocation that has already advanced is refused: class= rule= detail=; and with the catalogue's policy_version rolled back from  to policy:v0 a fresh request returns rc=0 rule= — no session is derived from a rolled-back policy |
| T-6.5-003 | FAIL | rule=lifecycle_unavailable detail=Connection refused (os error 111) |
| D-10.launch | FAIL | rc=1 lrd= topology=local-socket |
| GW-COMPLETE | FAIL | worker lines=0 |
| D-13 | FAIL | staging ref for session : false; main d5552a130bbe2bcd1eb2874644bb26717bd265b8→d5552a130bbe2bcd1eb2874644bb26717bd265b8 |
| D-13.trace | PASS | host hook log carries trace  |
| T-6.4-012.host-hook | PASS | direct push to main as gateway user refused by host hook: remote: protected: refs/heads/main |
| T-6.4-002 | FAIL | session netns interfaces: nsenter: failed to parse pid: '-m' |
| T-6.4-003 | FAIL | host socket dir from session: nsenter: failed to parse pid: '-m' |
| T-6.4-003.only | FAIL | exactly one socket node in /run: nsenter: failed to parse pid: '-m'  |
| T-6.4-004 | FAIL | positive control: host abstract socket bound=true reachable from host=connected; from session netns=nsenter: failed to parse pid: '-n' (ECONNREFUSED=111: abstract namespace is per-netns) |
| T-6.4-005 | FAIL | outside-scope peer with session uid: nsenter: failed to parse pid: '-m' 190  |
| T-6.4-008 | FAIL | malformed SCM_CREDENTIALS from a peer the gateway does authenticate (session uid , session scope and namespaces), so the per-packet rule is the only thing left that can refuse: none → sh: 1: cannot create /sys/fs/cgroup/system.slice/agentbound-45ed3a21-00002989.scope/cgroup \| two → sh: 1: cannot create /sys/fs/cgroup/system.slice/agentbound-45ed3a21-00002989.scope/cgroup \| short → sh: 1: cannot create /sys/fs/cgroup/system.slice/agentbound-45ed3a21-00002989.scope/cgroup \| forged → sh: 1: cannot create /sys/fs/cgroup/system.slice/agentbound-45ed3a21-00002989.scope/cgroup; rules recorded: Usage: grep [OPTION]... PATTERNS [FILE]... Try 'grep --help' for more information. |
| T-6.4-014 | FAIL | quiesce state= gateway admission= new-conn-while-quiesced= (control: the identical peer got  while the session was admitting, so the refusal is by admission state, not peer identity) behaviour=; held connection's post-denial packet: cat: /var/lib/agentbound/workspaces/eng/held-.out: No such file or directory ; revocation reply: {"body":{"detail":"","rule":"unknown_record"},"class":"invalid","ok":false,"v":"agentbound.wire.v0.1"}; status after seal: unknown_record |
| D-12.chain-shape | FAIL | one launch record carries 8/15 expected event kinds; missing=["gateway.grants_loaded", "gateway.connection_established", "gateway.operation_admitted", "gateway.operation_completed", "gateway.operation_denied", "gateway.admission_denied", "gateway.released"] (fixture — NOT the §5 attribution metric, which is D-12 below) |
| D-16 | FAIL | 9/11 triggers in the frozen vocabulary exercised with a declared action and a session.revocation_received record in the hash-chained log: ["approval_expired", "audit_pipeline_degraded_below_stop_threshold", "authority_revoked", "catalogue_withdrawn", "initiator_disabled", "policy_service_unavailable", "policy_withdrawn", "reclassification", "task_cancelled"]; NOT exercised: ["gateway_grant_withdrawn", "gateway_unavailable"]. Invariant 21 stays incomplete until 1C (inference grant/binding revoked), per R-LC-3. |
| D-12 | FAIL | §5 NOMINAL metric computed from /var/lib/agentbound/evidence/d12: 10 result files, 10 unreadable ["rep-1: NonIntegerNumber", "rep-2: NonIntegerNumber", "rep-3: NonIntegerNumber", "rep-4: NonIntegerNumber", "rep-5: NonIntegerNumber", "rep-6: NonIntegerNumber", "rep-7: NonIntegerNumber", "rep-8: NonIntegerNumber", "rep-9: NonIntegerNumber", "rep-10: NonIntegerNumber"], 0 valid repetitions (10 required); 1B bar = 100% over the finite gateway-operation corpus: 0/0 = 0.0% (NOT 100% in every valid run); whole-ontology aggregate \|C\|/\|G\| = 0/0 = 0.0% — REPORTED, owed by D-12.full at 1C (>= 99% there), NOT whole-ontology attribution at 1B; invalid/aborted repetitions retained: []; per-rep: []; per-class (rep 1):  |
| T-6.3-007 | FAIL | post-termination: projection released, record sealed, socket node removed with the mount namespace |
| T-6.3-007.socket | FAIL | host-side socket nodes left for this allocation: Usage: grep [OPTION]... PATTERNS [FILE]... Try 'grep --help' for more information. |
| F-C-08 | PASS | step=8: record committed (lrd=sha256:9e1a5b0b7de4e8ed33458d26349487204e169af0bfbf00a4bb7d76b464045af6, audit kinds=["session.launch_record_committed", "session.construction_failed", "session.construction_failed"]) and socket bound, activation never reached; rollback=["child killed and reaped","cgroup.kill","scope stopped","gateway projection released","identity → reclaiming"]; gateway holds no projection for the record (status rule=unknown_record)=true; socket node for 45ed3a21-00002990 gone=true; identity=quarantined |
| F-T-01 | FAIL | 1B: step 1 admission closure failed (evidence gateway_admission_denied=false). The other guarantee still holds: releasing the projection at step 6 removed the socket node (present before=true, after=true) and a connect to it is connected, no reply: TimeoutError, so no new operation can be admitted; identity=quarantined (not free) |
| F-T-01.resumable | PASS | with the fault removed the protocol completed: repeated terminate returned state= (rule=terminal_state: already terminal) and the session's final state is cleaned/sealed (evidence from the faulted attempt retained: {"cgroup_kill_written":false,"cgroup_procs_remaining":[],"cr) |
| F-T-05 | FAIL | could not launch a session for the no-live-process confirmation fault: rc=1 |
| F-T-06 | FAIL | gateway grant/connection closure failed: session.cleanup_completed recorded outcome=ok with grants={"broker_closed":true,"connections_closed":1,"released":true,"remaining":0}; the record was not sealed (sealed=true) and no identity release preceded the failure; state=termination-incomplete, status=cleaned/sealed, identity=quarantined |
| F-T-06.resumable | PASS | with the fault removed the protocol completed: repeated terminate returned state= (rule=terminal_state: already terminal) and the session's final state is cleaned/sealed (evidence from the faulted attempt retained: {"cgroup_kill_written":false,"cgroup_procs_remaining":[],"cr) |
| F-T-07 | FAIL | could not launch a session for the broker/credential closure fault: rc=1 |
| F-T-09 | PASS | step 9 failed: the node was present before (true) and after (true) termination, but the projection is released, so a connect+send is connected, no reply: TimeoutError — the gateway is inaccessible through it; the launch record is retained=true; identity=quarantined |
| F-T-09.resumable | PASS | with the fault removed the protocol completed: repeated terminate returned state= (rule=terminal_state: already terminal) and the session's final state is cleaned/sealed (evidence from the faulted attempt retained: {"cgroup_kill_written":false,"cgroup_procs_remaining":[],"cr) |
| T-6.8-008 | FAIL | declared behaviour= (manifest, for trigger gateway_grant_withdrawn); a gateway ping succeeded before the signal (false) and after it the same in-scope peer gets: nsenter: failed to parse pid: '-m'; session state=; revocation recorded in the chain=false |
| T-6.8-009 | FAIL | the gateway was stopped for this row (control socket reachable while down=false); the declared behaviour for gateway_unavailable is quiesce and it was reached without the gateway: state=quiescing, and session.quiesce_started records admission=closure-unconfirmed — the availability of the gateway at that moment, not an assumption |
| T-6.3-008 | PASS | session B (allocation:45ed3a21-00003000, uid 200029) connected to session A's socket node /run/agentbound/gw/45ed3a21-00002999.sock (A's projection admitted and its node present at that moment=true). Two results: from inside B's own mount namespace the node does not exist at all (connect errno=No such file or directory (os error ), and a peer that can reach it — B's uid, B's cgroup, and group traversal into the gateway's socket directory granted explicitly — is refused by A's projection on credentials alone (reply: closed by gateway; refusals 5 -> 6, rule recorded against A after this attempt: "rule":"uid_mismatch"). The peer credentials, not the path, decide |
| T-6.3-005 | PASS | the socket node is a broker capability, not a transferable object: copying it out of the session produced no usable object (cp: cannot open '/run/agentbound/gw/45ed3a21-00002999.sock' , exists=false); handing the connected descriptor to another process is refused (7dc2afbc2512177c0b1b555cf552f5140","requirement_id":"R-GW-3"); and root on the host connecting to the same node from outside the session's mount namespace is refused (closed by gateway) — every use is authenticated per peer instance |
| T-6.4-013 | PASS | caller-supplied session/trace refused (closed argument set); no ref under the other session's namespace: {"body":{"authorization_id":"launchrec:fix-issue-1235-003156","detail":"Unexpected(0, \"non-canonical\")","launch_record_digest":"sha256:0308e05cc114d87ee1835a7b8cf1566ac49a2561b068ce6c33516771c95c47fc","requirement_id":"R-GW-1","rule":"parse","trace_id":"trace:78f770f5200abaf4256eb84012b435ca"},"cl |
| D4.7-reconstruct | PASS | socket before restart=1; chained reconstruction events 4→5 (exactly one for this restart); event: "projections":1 "stale_descriptors_dropped":0; ping from an in-scope session peer after restart: {"body":{"operation_seq":30,"result":{"pong":true},"trace_id":"trace:78f770f5200abaf4256eb84012b435ca"},"class":"ok","ok":true,"v":"agentbound.wire.v0.1"}  |
| T-6.9-005.no-deadlock | FAIL | a session issued gateway operations in a loop (each persisting its budget through lifecycle) while the driver terminated it from the other side — the exact crossing that deadlocked both daemons before cross-daemon calls were bounded: terminate returned in 4075 ms, both daemons answered afterwards in 2092 ms (lifecycle=true, gateway=false), final state=cleaned/sealed |
| T-6.9-008 | FAIL | gateway budget classes present in the catalogue: ["bytes", "bytes_per_operation", "connection_count", "objects", "operations"]; each is enforced, with denials recorded in the hash-chained log by THIS run: operations → budget_operations=0, bytes_per_operation / bytes → budget_bytes=3, objects → budget_objects=3, connection_count → connection_limit=0; exhaustion session rc=1 ((not run)); classes absent at 1B and deferred to 1C under R-GW-9: ["rate", "spend", "tokens"] (this row does not claim them) |
| T-6.9-005.budget-persist | PASS | 5 pings admitted (session op_count 30->35); lifecycle budget record then held op:gateway-ping operations=25 [{"op:gateway-ping":{"bytes":0,"operations":25},"op:git-push-staging":{"bytes":5552,"operations":10}}]; gateway restarted: session op_count restored to 35 (not reset); then 39 more pings admitted and 31 refused budget_operations (gateway.operation_denied events for this allocation); stored op:gateway-ping operations=64 == budget 64, and 25+39=64 |
| D4.7-idempotency-persist | FAIL | first: seq=28 ops=28; same key+input: seq=28 ops=28 (no consumption); different input: conflict/idempotency_conflict; lifecycle store holds outcome for key: true; gateway restarted: ops restored=28; same key after restart from new connection: seq=28 ops=28; operation_completed for key=0 (exactly once) replayed=14 |
| T-6.4-009 | PASS | PID reuse construction succeeded: establishing pid 545 was recycled inside the session pidns [stat: cannot statx '/proc/545/ns/pid': No such file or directory recycled_pid 54] with the same pid, uid and scope cgroup. Outcome: the connection was closed on establisher exit (112 connection_closed events for this allocation) and NO operation was admitted for the establishing pid afterwards (holder ops=0); process_mismatch denials 4->4 ; last gateway events for this allocation after the holder was triggered: "rule":"budget_operations" "reason":"peer_closed" "reason":"peer_exited" . The gateway's peer-pidfd poll closes the connection before a recycled instance can present itself, so the packet-level inode comparison is not reached live; that branch is covered deterministically by the unit test session::tests::recycled_pid_rejected |
| T-6.4-012 | WEAK | caller-supplied url ignored; bundle path enforced: rule":"args_schema"}  |
| D7-9.diagnostics | FAIL | requirement=R-GW-4 authorization=launchrec:fix-issue-1235-003156 lrd-matches=true trace=trace:78f770f5200abaf4256eb84012b435ca foreign-ids-absent=false |
| D7-8.audit-loss | FAIL | gateway with no audit path (receiver down, spool unwritable): first op's event lost → admission closed + revocation_signal; lifecycle "trigger":"audit_pipeline_degraded_below_stop_threshold" → state=termination-incomplete; second attempt: nsenter: cannot open /proc/970608/ns/net: No such file or directory |
| D-06.storage-principal | PASS | work dir owner after seal: storage-engineering agentbound; files still owned by ephemeral uid: 0; "detail":{"bytes":9782,"failed":0,"files":106,"storage_principal":"storage:engineering-agent"} |
| D-02.1B | FAIL | descriptor allowlist entries=0 0 (stdin, stdout, stderr, gateway_socket mount); no attach/PTY path exists to deny — partial stays recorded |
| T-6.1-003.1B | FAIL | no PTY projected under local-socket either; N/A stays recorded |
| T-6.1-013 | FAIL | sealed session's socket: nodes left=err 2 connect=Try 'grep --help' for more information. |
| T-6.2-008.1B | RECORDED | loaders/interpreters beyond sh+git in image: 0 |
| D-15.1B | RECORDED | delegation operations in catalogue: [] — residual stays recorded (no delegation path to narrow) |
