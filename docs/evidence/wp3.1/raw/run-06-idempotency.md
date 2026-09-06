# Agentbound conformance run — 1A + 1B rows (machine output)

- Host: agentbound-dev
- Kernel: 6.12.107+deb13-cloud-amd64
- systemd: systemd 257 (257.13-1~deb13u1)
- git: git version 2.47.3
- Date: 2026-09-06T19:23:32Z
- Run id: run-107386829544278
- Repository commit: 207930e
- Binary digests (sha256/16): agentbound=0bb30cc19d3be3da agentbound-launch=1f23b7aa21395193 agentbound-lifecycle=220a58fb660be865 agentbound-policy=9b5e4b855ead2e50 agentbound-audit=b3dd4569f15d3e39 agentbound-gateway=2331342c2f451fbe ab-conformance=26c8b7f7f412c693 ab-gwclient=ffb60468a223bfbb probe.sh=4c6151f65c9add3c git-worker.sh=39f5bc9ca667dc97
- Expected population: 121 catalogue ids (test-catalogue 1A+1B)
- Assertions: 175 PASS, 3 WEAK, 4 RECORDED, 0 FAIL (7 fixtures excluded)
- Catalogue coverage: 115 PASS, 2 WEAK, 4 RECORDED, 0 FAIL, **0 NOT-EXECUTED**
- Duplicate row ids: none
- Row ids outside the catalogue: none
- **Run verdict: PASS**

## Catalogue coverage

| Catalogue id | Milestone | Best verdict |
|---|---|---|
| D-01 | 1A | PASS |
| D-02 | 1A | RECORDED |
| D-03 | 1A | PASS |
| D-04 | 1A | PASS |
| D-05 | 1A | PASS |
| D-06 | 1A | PASS |
| D-07 | 1A | PASS |
| D-08 | 1A | PASS |
| D-09 | 1B | PASS |
| D-10 | 1B | PASS |
| D-11 | 1A | PASS |
| D-12 | 1B | WEAK |
| D-13 | 1B | PASS |
| D-15 | 1A | RECORDED |
| D-16 | 1A/1B/1C | PASS |
| D4 | 1B | PASS |
| D7-8 | 1B | PASS |
| D7-9 | 1B | PASS |
| F-C-01 | 1A | PASS |
| F-C-02 | 1A | PASS |
| F-C-03 | 1A | PASS |
| F-C-04 | 1A | PASS |
| F-C-05 | 1A | PASS |
| F-C-06 | 1A | PASS |
| F-C-07 | 1A | PASS |
| F-C-08 | 1B | PASS |
| F-C-09 | 1A | PASS |
| F-T-01 | 1A/1B | PASS |
| F-T-02 | 1A | PASS |
| F-T-03 | 1A | PASS |
| F-T-04 | 1A | PASS |
| F-T-05 | 1A | PASS |
| F-T-06 | 1B | PASS |
| F-T-07 | 1B | PASS |
| F-T-08 | 1A | PASS |
| F-T-09 | 1B | PASS |
| F-T-10 | 1A | PASS |
| F-T-11 | 1A | PASS |
| T-6.1-001 | 1A | PASS |
| T-6.1-002 | 1A | PASS |
| T-6.1-003 | 1A | RECORDED |
| T-6.1-004 | 1A | PASS |
| T-6.1-005 | 1A | PASS |
| T-6.1-006 | 1A | PASS |
| T-6.1-007 | 1A | PASS |
| T-6.1-008 | 1A | PASS |
| T-6.1-009 | 1A | PASS |
| T-6.1-010 | 1A | PASS |
| T-6.1-011 | 1A | PASS |
| T-6.1-012 | 1A | PASS |
| T-6.1-013 | 1A | PASS |
| T-6.2-001 | 1A | PASS |
| T-6.2-002 | 1A | PASS |
| T-6.2-003 | 1A | PASS |
| T-6.2-004 | 1A | PASS |
| T-6.2-005 | 1A | PASS |
| T-6.2-006 | 1A | PASS |
| T-6.2-007 | 1A | PASS |
| T-6.2-008 | 1A | RECORDED |
| T-6.2-009 | 1A | PASS |
| T-6.3-001 | 1B | PASS |
| T-6.3-002 | 1B | PASS |
| T-6.3-003 | 1B | PASS |
| T-6.3-004 | 1B | PASS |
| T-6.3-005 | 1B | PASS |
| T-6.3-006 | 1B | PASS |
| T-6.3-007 | 1B | PASS |
| T-6.3-008 | 1B | PASS |
| T-6.4-001 | 1B | PASS |
| T-6.4-002 | 1B | PASS |
| T-6.4-003 | 1B | PASS |
| T-6.4-004 | 1B | PASS |
| T-6.4-005 | 1B | PASS |
| T-6.4-006 | 1B | PASS |
| T-6.4-007 | 1B | PASS |
| T-6.4-008 | 1B | PASS |
| T-6.4-009 | 1B | PASS |
| T-6.4-010 | 1B | PASS |
| T-6.4-011 | 1B | PASS |
| T-6.4-012 | 1B | PASS |
| T-6.4-013 | 1B | PASS |
| T-6.4-014 | 1B | PASS |
| T-6.5-001 | 1A | PASS |
| T-6.5-002 | 1A | PASS |
| T-6.5-003 | 1A | PASS |
| T-6.5-004 | 1A | PASS |
| T-6.5-005 | 1A | PASS |
| T-6.5-006 | 1A | PASS |
| T-6.5-007 | 1A | PASS |
| T-6.5-008 | 1A | PASS |
| T-6.5-009 | 1A | PASS |
| T-6.5-010 | 1A | PASS |
| T-6.6-001 | 1A | PASS |
| T-6.6-002 | 1A | PASS |
| T-6.6-003 | 1A | PASS |
| T-6.6-004 | 1A | PASS |
| T-6.6-005 | 1A | PASS |
| T-6.6-006 | 1A | PASS |
| T-6.6-007 | 1A | PASS |
| T-6.6-008 | 1A | PASS |
| T-6.7-001 | 1A | PASS |
| T-6.8-001 | 1A | PASS |
| T-6.8-002 | 1A | PASS |
| T-6.8-003 | 1A | PASS |
| T-6.8-004 | 1A | PASS |
| T-6.8-005 | 1A | PASS |
| T-6.8-006 | 1A | PASS |
| T-6.8-007 | 1A | PASS |
| T-6.8-008 | 1B | PASS |
| T-6.8-009 | 1B | PASS |
| T-6.8-011 | 1A | PASS |
| T-6.8-012 | 1A | PASS |
| T-6.8-013 | 1A | PASS |
| T-6.9-001 | 1A | PASS |
| T-6.9-002 | 1A | PASS |
| T-6.9-003 | 1A | PASS |
| T-6.9-004 | 1A | PASS |
| T-6.9-005 | 1B | PASS |
| T-6.9-006 | 1B | WEAK |
| T-6.9-007 | 1A | PASS |
| T-6.9-008 | 1B/1C | PASS |

## Rows

| Row | Verdict | Evidence |
|---|---|---|
| D-01 | PASS | rc=0 lrd=sha256:4e222b9c1d398a9063004d0e7d11e4e6be9d6a8ab3907ad5f3a5dafb78057814 {"allocation_id":"allocation:45ed3a21-00001256","console":"/var/lib/agentbound/sessions/45ed3a21-00001256/console.log","init_pid":1871664,"launch_record_digest":"sha256:4e222b9c1d398a9063004d0e7d11e4e |
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
| T-6.2-007.workspace | PASS | workspace writable as 200845 |
| T-6.1-010 | PASS | pidfd_open=1852034 rc=-1 errno=3 |
| T-6.1-010.own-ns | FIXTURE | pidfd against our own init: pidfd_open=1 rc=3 send_signal_rc=0 send_signal_errno=0 |
| T-6.1-011 | PASS | process_vm_readv pid=1852034 rc=-1 errno=3 |
| T-6.1-012 | PASS | host abstract name unreachable (abstract connect name=agentbound-conf-abs rc=-1 errno=111) and the same name binds freely here (abstract bind name=agentbound-conf-abs rc=0 errno=0) — separate abstract namespaces |
| T-6.1-006 | PASS | symlinks to host catalogue, lifecycle store and / are all unusable from the session temp dir (writes refused; the targets do not exist in this mount namespace) |
| T-6.1-008 | PASS | no sibling startup file could be written (cannot write a shared /workspace/.profile); this session's environment is private to it |
| T-6.2-009 | PASS | denied rc=1 ls /sys/class/net |
| T-6.2-002.netdev | PASS | denied rc=1 interfaces other than lo |
| T-6.2-005 | PASS | double-forked orphan pid=62 reparented to in-namespace ppid=1 and still visible in this pid namespace (contained); reaping asserted by D-07 at termination |
| T-6.9-002 | PASS | opened=1021 stopped at RLIMIT_NOFILE=1024 with EMFILE(24) |
| T-6.9-004.bytes | PASS | /tmp tmpfs capacity=262144KiB; write stopped at 264118272 bytes: SIGKILL by memory cgroup (tmpfs pages charged to memory.max; disk bound not reached first) |
| T-6.9-004.inodes | PASS | nr_inodes=65536; creation refused after 65523 files, IFree=0 |
| T-6.9-003.memory | PASS | 512 MiB request against memory.max=not-visible-in-session (read back by the driver): refused (rc=137 ) |
| T-6.9-001 | PASS | procs=63 (TasksMax bound) |
| PROBE-COMPLETE | FIXTURE | probe lines=42 |
| T-6.1-012.host-name | FIXTURE | host abstract name agentbound-conf-abs bound in the host netns while the probe ran (ss listeners for the name = 1) |
| D-01.status | PASS | {"identity_state":"in-use","observation_seq":3,"reason":null,"record_ref":"sha256:4e222b9c1d398a9063004d0e7d11e4e6be9d6a8ab3907ad5f3a5dafb78057814","state":"active"} |
| D-06 | PASS | scope procs=64 (init + workload + orphan/fan-out survivors) |
| T-6.9-004.readback | PASS | binding installed_value vs kernel (binding/kernel): pids=64/64 memory_bytes=268435456/268435456 cpu=1000/1000 io_bandwidth=52428800/52428800 disk_bytes=268435456/268435456 disk_inodes=65536/65536 file_descriptors=1024/1024  |
| T-6.9-003.owners | PASS | audit_capacity installed_value=Some(10000) (reserved with the receiver, which is reachable), delegation_fanout installed_value=Some(0) |
| T-6.9-003.cpu | PASS | cpu.max quota=Some(1000) milli-cpu (read back); cpu.stat nr_throttled=98 (quota installed and accounted; throttling occurs only under contention, which this probe does not guarantee) |
| D-04.host-view | PASS | uid 200845 processes outside scope=0 |
| D-08 | PASS | {"cgroup_kill_written":true,"cgroup_procs_remaining":[],"credential_scan_inside_scope":[],"credential_scan_outside_scope":[],"d_state":[],"elapsed_ms":2043,"freeze_written":true,"frozen_observed":false,"gateway_admission_denied":false,"init_pid":1871664,"init_pidfd_exited":true,"sigterm_sent":true} |
| F-T-03 | PASS | {"cgroup_kill_written":true,"cgroup_procs_remaining":[],"credential_scan_inside_scope":[],"credential_scan_outside_scope":[],"d_state":[],"elapsed_ms":2043,"freeze_written":true,"frozen_observed":false,"gateway_admission_denied":false,"init_pid":1871664,"init_pidfd_exited":true,"sigterm_sent":true} |
| F-T-04 | PASS | kill written without waiting for frozen 1; procs empty; pidfd exited |
| D-07 | PASS | orphan/double-fork survivors killed with the scope; host credential scan clean |
| F-T-10 | PASS | {"identity_state":"quarantined","observation_seq":6,"reason":"conformance","record_ref":"sha256:4e222b9c1d398a9063004d0e7d11e4e6be9d6a8ab3907ad5f3a5dafb78057814","state":"cleaned/sealed"} |
| F-T-11 | PASS | audit kinds=["session.launch_record_committed", "session.activated", "session.termination_started", "session.terminated", "session.ownership_projected", "session.cleanup_completed", "session.identity_released", "session.sealed"] |
| F-T-08 | PASS | session dir removed=true; workspace root retained by durable owner |
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
| T-6.6-001.audit | PASS | session.rejected events with failed_input=1072 |
| F-C-01 | PASS | barrier never released: child pid=1937630 reaped by rollback=true, still alive=false; identity=quarantined; scopes_left=0; rollback=["child killed and reaped","cgroup.kill","scope stopped","identity → reclaiming"] |
| F-C-02 | PASS | step=2 rule=child_step_failed identity=quarantined scopes_left=0 rollback=["child killed and reaped","cgroup.kill","scope stopped","identity → reclaiming"] |
| F-C-03 | PASS | step=3 rule=mount_source_escape identity=quarantined scopes_left=0 rollback=["cgroup.kill","scope stopped","identity → reclaiming"] |
| F-C-04 | PASS | step=4 rule=child_step_failed identity=quarantined scopes_left=0 rollback=["child killed and reaped","cgroup.kill","scope stopped","identity → reclaiming"] |
| F-C-05 | PASS | step=5 rule=child_step_failed identity=quarantined scopes_left=0 rollback=["child killed and reaped","cgroup.kill","scope stopped","identity → reclaiming"] |
| F-C-05.no-host-proc | PASS | host mount table shows 0 session-tree mounts after the aborted construction (the child's proc/sysfs died with its namespace) |
| F-C-06 | PASS | step=6 rule=child_step_failed identity=quarantined scopes_left=0 rollback=["child killed and reaped","cgroup.kill","scope stopped","identity → reclaiming"] |
| F-C-06.own-check | PASS | a descriptor surviving the closure pass was caught by step 6's own verification through the fresh /proc: leaked 3:/image; the session never activated (audit kinds=[]) |
| F-C-07 | PASS | step=7 rule=fault_injected identity=quarantined scopes_left=0 rollback=["child killed and reaped","cgroup.kill","scope stopped","identity → reclaiming"] |
| F-C-09 | PASS | step=8 rule=fault_injected identity=quarantined scopes_left=0 rollback=["child killed and reaped","cgroup.kill","scope stopped","identity → reclaiming"] |
| F-C-09.record | PASS | lrd=sha256:242bdbde456bf190d5ccaba3c6e6e52939083fae0830aab8598f6791f69de70c kinds=["session.launch_record_committed", "session.construction_failed", "session.ownership_projected", "session.cleanup_completed", "session.identity_released", "session.sealed"] |
| D-11 | PASS | all 11 constructor fault rows (F-C-01..09, one per failing step): no runnable session, identity held, scope gone |
| T-6.5-004 | PASS | activations=1 refusals=1 |
| T-6.5-009 | PASS | allocator latest states: [('free', 410), ('quarantined', 852), ('reclaiming', 3)]; free-before-floor violations=0 |
| T-6.8-setup | FIXTURE | sha256:bbcd20c03781612a843244600e3d4cfcfd94dc779c2fbce111ea1450661c098d |
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
| T-6.8-012 | PASS | procs_while_down=3 (containment held, no authority available: daemon_reachable=false) cli_reply={"class":"unavailable","detail":"Connection refused (os erro after_restart=cleaned/sealed kinds=["session.launch_record_committed", "session.activated", "session.recovery_reconciled"] |
| T-6.8-012.contained | PASS | state=cleaned/sealed identity=quarantined procs=0 |
| T-6.9-007 | PASS | audit chain head=sha256:b3ba26b4dbaee11fa3c435972f3f810a0670ed89609d0ae5dfd4a56588f98f0d seq=48574 lost=0 |
| T-6.7-001 | PASS | axes measured against the committed manifest: 1 declared mount intents vs 0 mounts observed in the session, 0 declared grants (1A: none), 3 descriptors held by the workload; a child of the workload gained nothing (mounts/fds = 0 4 ). Every recovery path was refused: remount a mount read-write → mount: permission denied (are ; mount a fresh tmpfs (new mount authority) → mount: mounting none on /mnt f; raise its own pid limit → sh: can't create /sys/fs/cgrou; raise its own memory limit → sh: can't create /sys/fs/cgrou; raise RLIMIT_NOFILE above the installed hard bound → sh: error setting limit: Opera; re-exec with elevated privilege → su: must be suid to work prope; acquire a new grant by writing a catalogue → sh: can't create /etc/agentbou. No axis increases, and authority cannot be restored from inside. |
| T-6.5-005 | PASS | the catalogue's pids limit was rewritten every 150 ms while eight requests raced it: 8 admitted, 0 rejected. Every admitted session's installed pids.max equals the value in its OWN committed manifest, never a mixture and never a value that only existed between decisions: ["rc=0 declared=64 installed=64", "rc=0 declared=64 installed=64", "rc=0 declared=64 installed=64", "rc=0 declared=64 installed=64", "rc=0 declared=64 installed=64", "rc=0 declared=64 installed=64", "rc=0 declared=64 installed=64", "rc=0 declared=64 installed=64"]. A request either serializes on one coherent catalogue state or is rejected. |
| D-05 | PASS | three runtimes (runtime:sh, runtime:scripted-loop, runtime:probe) on the same task and resource: each got its own uid from the allocator range ([200871, 200872, 200873], all distinct=true) and its own scope cgroup (distinct=true); the committed namespace set is identical across all three (true: {"ipc":"private","mount":"private","pid":"private","user":"inherited","uts":"private"}) with every namespace the record commits as private in fact distinct from PID 1's (true); and each session recorded the same boundary-establishing audit events (true: [2, 2, 2] of 3 present in every case). Substituting the runtime changes the workload, not the boundary. |
| D-03 | PASS | two concurrent sessions of different principals (uid 200874/finance-agent and uid 200875/engineering-agent). Every private-state interface of one, attempted with the other's identity, had no effect on it: write A's memory.max → sh: 1: cannot create /sys/fs/cgroup/system.s; signal A's workload → sh: 1: kill: Operation not permitted (A's li; read A's process environment → cat: /proc/1953780/environ: Permission denie; read A's session root → ls: cannot access '/proc/1953780/root/': Per; create a file in A's workspace → touch: cannot touch '/var/lib/agentbound/wor; read a file in A's workspace → cat: /var/lib/agentbound/workspaces/finance/. The reverse direction is also ineffective — B's workload survived A's attempt to signal it and its limits were unchanged (sh: 1: kill: Operation not permitted  sh). No read of private state, no signal, no cgroup write and no workspace write crossed the principal boundary. |
| T-6.5-008 | PASS | four confusions of a genuine committed record, each replayed to commit_binding: (1) constructor envelope presented as the policy envelope, (2) manifest mutated after signing, (3) valid policy signature over the wrong object, (4) signature bytes corrupted. All refused, each naming the check that caught it: ["manifest_envelope/EnvelopeShape(\"expected exac", "manifest_envelope/EnvelopeShape(\"authorization", "binding_schema/unknown-member at binding: a", "manifest_envelope/BadSignatureForm"] |
| T-6.6-007 | PASS | replaying a correctly signed binding for an allocation that has already advanced is refused: class=conflict rule=binding_allocation_mismatch detail=binding does not match the reservation; and with the catalogue's policy_version rolled back from policy:v2026-08-28 to policy:v0 a fresh request returns rc=0 rule= — no session is derived from a rolled-back policy |
| T-6.5-003 | PASS | rule=mount_source_escape detail=../../../etc errno=18 |
| D-10.launch | PASS | rc=0 lrd=sha256:2869a81ad6ed0baa128380bda0c4505ea821b2a6f41018684b5e3d7a4bcde243 topology=local-socket |
| T-6.4-003.projected | PASS | gateway socket node present |
| D-09 | PASS | authenticated connection, typed ping admitted: "pong":true |
| T-6.3-001 | PASS | no credential in environment |
| T-6.3-002 | PASS | credential path absent/unreadable |
| T-6.1-007.sibling | PASS | sibling /workspace/work-200005 not writable |
| T-6.3-002.no-remote | PASS | no git remote or credential in the worker repository (WP1 GS-1) |
| D-10.bundle | FIXTURE | tip=68f67e0f60c214914a6b87027005b9e06a31ce9b bytes=302 |
| D-10 | PASS | push_staging accepted rc=0 |
| T-6.4-011.gs4[../main] | PASS | "rule":"ref_tail_grammar" |
| T-6.4-011.gs4[main:refs/heads/main] | PASS | "rule":"ref_tail_grammar" |
| T-6.4-011.gs4[+fix] | PASS | "rule":"ref_tail_marker" |
| T-6.4-011.gs4[fix.lock] | PASS | "rule":"ref_tail_grammar" |
| T-6.4-011.gs4[a_b] | PASS | "rule":"ref_tail_charset" |
| T-6.4-011.gs4[] | PASS | "rule":"ref_tail_empty_or_long" |
| T-6.4-011.gs4[refs/heads/main] | PASS | "rule":"ref_tail_names_ref" |
| T-6.4-011 | PASS | "rule":"scope_repository" |
| T-6.4-011.force | PASS | "rule":"operation_not_granted" |
| T-6.9-008.objects | PASS |  |
| T-6.4-006 | PASS | "rule":"descriptor_transfer" |
| T-6.4-007 | PASS | "rule":"process_mismatch" |
| T-6.4-001 | PASS | inet errno=1 inet6 errno=1 packet errno=1 netlink errno=1 vsock errno=1 |
| T-6.4-008.none | PASS | no SCM_CREDENTIALS sent: kernel synthesised the true credential and the gateway answered as this process (SO_PASSCRED guarantees exactly one; the count!=1 branch is unreachable from a session peer) |
| T-6.4-008.two | PASS | two SCM_CREDENTIALS cmsgs collapsed to one true credential by the kernel; gateway answered as this process, no identity substitution |
| T-6.4-008.forged | PASS | forged pid=1/uid=0 refused by the kernel at sendmsg (EPERM=1): an unprivileged session peer cannot even emit a false credential |
| T-6.4-008.short | PASS | truncated ucred refused by the kernel at sendmsg (EINVAL=22) |
| T-6.3-003 | PASS | fds: 0 /dev/null 1 pipe:[675647] 2 pipe:[675647]  |
| T-6.3-004 | PASS | child env credential hits=0 fds=4; inherited connection refused ("rule":"process_mismatch"); child's own connection authenticated |
| T-6.3-006 | PASS | no credential-like text in gateway replies/adapter output |
| T-6.4-010.stream | PASS | connect errno=Protocol wrong type for socket (os error 91)  |
| T-6.4-010.dgram | PASS | connect errno=Protocol wrong type for socket (os error 91)  |
| T-6.9-005 | PASS | "rule":"budget_bytes" |
| T-6.9-006 | WEAK | held=16 refused=4 (limit 16) |
| GW-HELD | FIXTURE | connection held for revocation test |
| GW-COMPLETE | FIXTURE | worker lines=36 |
| D-13 | PASS | staging ref for session cd33df45f40b63f7: true; main d5552a130bbe2bcd1eb2874644bb26717bd265b8→d5552a130bbe2bcd1eb2874644bb26717bd265b8 |
| D-13.trace | PASS | host hook log carries trace trace:4d9cce14b7b32837ab19e51d5ca8b824 |
| T-6.4-012.host-hook | PASS | direct push to main as gateway user refused by host hook: remote: protected: refs/heads/main |
| T-6.4-002 | PASS | session netns interfaces: ls: /sys/class/net: No such file or directory     lo |
| T-6.4-003 | PASS | host socket dir from session: ls: /run/agentbound: No such file or directory ls: /var/run/agentbound: No such file or directory |
| T-6.4-003.only | PASS | exactly one socket node in /run: srw-rw-rw-    1 994      1000             0 Sep  6 19:20 /run/gateway.sock 1  |
| T-6.4-004 | PASS | positive control: host abstract socket bound=true reachable from host=connected; from session netns=err 111 (ECONNREFUSED=111: abstract namespace is per-netns) |
| T-6.4-005 | PASS | outside-scope peer with session uid: closed by gateway 76  |
| T-6.4-008 | PASS | DENY host-root-peer closed 104 \| DENY forged-pid closed 104 \| DENY two-creds closed 104 \|  |
| T-6.4-014 | PASS | quiesce state=quiescing gateway admission=false new-conn-while-quiesced="rule":"admission_closed" (control: the identical peer got "rule":"uid_mismatch" while the session was admitting, so the refusal is by admission state, not peer identity) behaviour=terminate; held connection's post-denial packet: {"body":{"authorization_id":"launchrec:fix-issue-1235-001363","detail":"session not admitting operations","launch_record ; revocation reply: {"body":{"behaviour":"terminate","state":"cleaned/sealed"},"class":"ok","ok":true,"v":"agentbound.wire.v0.1"}; status after seal: unknown_record |
| D-16 | PASS | 11/11 triggers in the frozen vocabulary exercised with a declared action and a session.revocation_received record in the hash-chained log: ["approval_expired", "audit_pipeline_degraded_below_stop_threshold", "authority_revoked", "catalogue_withdrawn", "gateway_grant_withdrawn", "gateway_unavailable", "initiator_disabled", "policy_service_unavailable", "policy_withdrawn", "reclassification", "task_cancelled"]. Invariant 21 stays incomplete until 1C (inference grant/binding revoked), per R-LC-3. |
| D-12 | WEAK | presence check only, NOT the pre-registered metric: 15/15 required kinds on one launch record; missing=[] |
| T-6.3-007 | PASS | post-termination: projection released, record sealed, socket node removed with the mount namespace |
| T-6.3-007.socket | PASS | host-side socket nodes left for this allocation: 0 |
| F-C-08 | PASS | step=8: record committed (lrd=sha256:939e6af0d16ebed9c849c50b2b5e2430f1ba0dd3b9b90451f3db0b8e84b580c4, audit kinds=["session.launch_record_committed", "session.construction_failed"]) and socket bound, activation never reached; rollback=["child killed and reaped","cgroup.kill","scope stopped","gateway projection released","identity → reclaiming"]; gateway holds no projection for the record (status rule=unknown_record)=true; socket node for 45ed3a21-00001291 gone=true; identity=quarantined |
| F-T-01 | PASS | 1B: step 1 admission closure failed (evidence gateway_admission_denied=false). The other guarantee still holds: releasing the projection at step 6 removed the socket node (present before=true, after=false) and a connect to it is refused 2, so no new operation can be admitted; identity=quarantined (not free) |
| F-T-01.resumable | PASS | with the fault removed the protocol completed: repeated terminate returned state= (rule=terminal_state: already terminal) and the session's final state is cleaned/sealed (evidence from the faulted attempt retained: {"cgroup_kill_written":true,"cgroup_procs_remaining":[],"cre) |
| F-T-05 | PASS | the attempt reported state=termination-incomplete and recorded session.termination_incomplete; no identity release preceded it (released_before_failure=false); status when read afterwards=cleaned/sealed (the poller's unfaulted retry may already have completed it), identity=quarantined |
| F-T-05.resumable | PASS | with the fault removed the protocol completed: repeated terminate returned state= (rule=terminal_state: already terminal) and the session's final state is cleaned/sealed (evidence from the faulted attempt retained: {"cgroup_kill_written":false,"cgroup_procs_remaining":[],"cr) |
| F-T-06 | PASS | gateway grant/connection closure failed: session.cleanup_completed recorded outcome=hold with grants={"broker_closed":true,"connections_closed":0,"released":false,"remaining":"gateway unreachable"}; the record was not sealed (sealed=false) and no identity release preceded the failure; state=terminated, status=terminated, identity=reclaiming |
| F-T-06.resumable | PASS | with the fault removed the protocol completed: repeated terminate returned state= (rule=terminal_state: already terminal) and the session's final state is terminated (evidence from the faulted attempt retained: {"cgroup_kill_written":false,"cgroup_procs_remaining":[],"cr) |
| F-T-07 | PASS | broker/credential closure failed: session.cleanup_completed recorded outcome=hold with grants={"broker_closed":false,"connections_closed":0,"released":true,"remaining":0}; the record was not sealed (sealed=false) and no identity release preceded the failure; state=terminated, status=terminated, identity=reclaiming |
| F-T-07.resumable | PASS | with the fault removed the protocol completed: repeated terminate returned state= (rule=terminal_state: already terminal) and the session's final state is terminated (evidence from the faulted attempt retained: {"cgroup_kill_written":false,"cgroup_procs_remaining":[],"cr) |
| F-T-09 | PASS | step 9 failed: the node was present before (true) and after (true) termination, but the projection is released, so a connect+send is connected, no reply: ConnectionResetError — the gateway is inaccessible through it; the launch record is retained=true; identity=quarantined |
| F-T-09.resumable | PASS | with the fault removed the protocol completed: repeated terminate returned state= (rule=terminal_state: already terminal) and the session's final state is cleaned/sealed (evidence from the faulted attempt retained: {"cgroup_kill_written":false,"cgroup_procs_remaining":[],"cr) |
| T-6.8-008 | PASS | declared behaviour=terminate (manifest, for trigger gateway_grant_withdrawn); a gateway ping succeeded before the signal (true) and after it the same in-scope peer gets: nsenter: cannot open /proc/1954956/ns/net: No such file or directory; session state=cleaned/sealed; revocation recorded in the chain=true |
| T-6.8-009 | PASS | the gateway was stopped for this row (control socket reachable while down=false); the declared behaviour for gateway_unavailable is quiesce and it was reached without the gateway: state=quiescing, and session.quiesce_started records admission=denied-no-gateway — the availability of the gateway at that moment, not an assumption |
| T-6.3-008 | PASS | session B (allocation:45ed3a21-00001301, uid 200890) connected to session A's socket node /run/agentbound/gw/45ed3a21-00001300.sock (A's projection admitted and its node present at that moment=true). Two results: from inside B's own mount namespace the node does not exist at all (connect errno=No such file or directory (os error ), and a peer that can reach it — B's uid, B's cgroup, and group traversal into the gateway's socket directory granted explicitly — is refused by A's projection on credentials alone (reply: closed by gateway; refusals 4568 -> 4579, rule recorded against A after this attempt: "rule":"uid_mismatch"). The peer credentials, not the path, decide |
| T-6.3-005 | PASS | the socket node is a broker capability, not a transferable object: copying it out of the session produced no usable object (cp: cannot open '/run/agentbound/gw/45ed3a21-00001300.sock' , exists=false); handing the connected descriptor to another process is refused (closed by gateway); and root on the host connecting to the same node from outside the session's mount namespace is refused (closed by gateway) — every use is authenticated per peer instance |
| T-6.4-013 | PASS | caller-supplied session/trace refused (closed argument set); no ref under the other session's namespace: {"body":{"authorization_id":"launchrec:fix-issue-1235-001372","detail":"Unexpected(0, \"non-canonical\")","launch_record_digest":"sha256:3ceb511e1bbc6897467b69a8ecf0817a058d548eb6315e2fef5254ad0d384eed","requirement_id":"R-GW-1","rule":"parse","trace_id":"trace:5c68f7b2470f8f2f5851543f300d0a3e"},"cl |
| D4.7-reconstruct | PASS | socket before restart=1; chained reconstruction events 145→146 (exactly one for this restart); event: "projections":1 "stale_descriptors_dropped":0; ping from an in-scope session peer after restart: {"body":{"operation_seq":31,"result":{"pong":true},"trace_id":"trace:5c68f7b2470f8f2f5851543f300d0a3e"},"class":"ok","ok":true,"v":"agentbound.wire.v0.1"}  |
| T-6.9-005.no-deadlock | PASS | a session issued gateway operations in a loop (each persisting its budget through lifecycle) while the driver terminated it from the other side — the exact crossing that deadlocked both daemons before cross-daemon calls were bounded: terminate returned in 2124 ms, both daemons answered afterwards in 30 ms (lifecycle=true, gateway=true), final state=cleaned/sealed |
| T-6.9-008 | PASS | gateway budget classes present in the catalogue: ["bytes", "bytes_per_operation", "connection_count", "objects", "operations"]; each is enforced, with denials recorded in the hash-chained log: operations → budget_operations=906, bytes_per_operation / bytes → budget_bytes=134, objects → budget_objects=25, connection_count → connection_limit=538; classes absent at 1B and deferred to 1C under R-GW-9: ["rate", "spend", "tokens"] (this row does not claim them) |
| T-6.9-005.budget-persist | PASS | 5 pings admitted (session op_count 31->36); lifecycle budget record then held op:gateway-ping operations=26 [{"op:gateway-ping":{"bytes":0,"operations":26},"op:git-push-staging":{"bytes":5558,"operations":10}}]; gateway restarted: session op_count restored to 36 (not reset); then 38 more pings admitted and 32 refused budget_operations (gateway.operation_denied events for this allocation); stored op:gateway-ping operations=64 == budget 64, and 26+38=64 |
| T-6.4-009 | PASS | PID reuse construction succeeded: establishing pid 486 was recycled inside the session pidns [stat: cannot statx '/proc/486/ns/pid': No such file or directory recycled_pid 48] with the same pid, uid and scope cgroup. Outcome: the connection was closed on establisher exit (114 connection_closed events for this allocation) and NO operation was admitted for the establishing pid afterwards (holder ops=0); process_mismatch denials 4->4 ; last gateway events for this allocation after the holder was triggered: "rule":"budget_operations" "reason":"peer_closed" "reason":"peer_exited" . The gateway's peer-pidfd poll closes the connection before a recycled instance can present itself, so the packet-level inode comparison is not reached live; that branch is covered deterministically by the unit test session::tests::recycled_pid_rejected |
| T-6.4-012 | WEAK | caller-supplied url ignored; bundle path enforced: rule":"args_schema"}  |
| D7-9.diagnostics | PASS | requirement=R-GW-4 authorization=launchrec:fix-issue-1235-001372 lrd-matches=true trace=trace:5c68f7b2470f8f2f5851543f300d0a3e foreign-ids-absent=true |
| D7-8.audit-loss | PASS | gateway with no audit path (receiver down, spool unwritable): first op's event lost → admission closed + revocation_signal; lifecycle "trigger":"audit_pipeline_degraded_below_stop_threshold" → state=terminated; second attempt: nsenter: cannot open /proc/1955401/ns/net: No such file or directory |
| D-06.storage-principal | PASS | work dir owner after seal: storage-engineering agentbound; files still owned by ephemeral uid: 0; "detail":{"bytes":9777,"failed":0,"files":106,"storage_principal":"storage:engineering-agent"} |
| D-02.1B | RECORDED | descriptor allowlist entries=0 4 (stdin, stdout, stderr, gateway_socket mount); no attach/PTY path exists to deny — partial stays recorded |
| T-6.1-003.1B | RECORDED | no PTY projected under local-socket either; N/A stays recorded |
| T-6.1-013 | PASS | sealed session's socket: nodes left=0 connect=err 2 |
| T-6.2-008.1B | RECORDED | loaders/interpreters beyond sh+git in image: 0 |
| D-15.1B | RECORDED | delegation operations in catalogue: [] — residual stays recorded (no delegation path to narrow) |
