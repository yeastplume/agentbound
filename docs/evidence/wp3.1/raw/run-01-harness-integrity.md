# Agentbound conformance run — 1A + 1B rows (machine output)

- Host: agentbound-dev
- Kernel: 6.12.107+deb13-cloud-amd64
- systemd: systemd 257 (257.13-1~deb13u1)
- git: git version 2.47.3
- Date: 2026-09-05T19:35:37Z
- Run id: run-21712751418417
- Repository commit: 207930e
- Binary digests (sha256/16): agentbound=3412be1850c23ee4 agentbound-launch=b43ea487ccdd9e28 agentbound-lifecycle=cd1f34bffc0012a8 agentbound-policy=406033d7336b0e85 agentbound-audit=8b2583b34fe29451 agentbound-gateway=a19d06602ddfa91e ab-conformance=9de92a0bbc72e9b9 ab-gwclient=0b339712d695397c probe.sh=0da2cdb1bef3a16c git-worker.sh=4ebb0429093be4bc
- Expected population: 121 catalogue ids (test-catalogue 1A+1B)
- Assertions: 126 PASS, 3 WEAK, 4 RECORDED, 0 FAIL (6 fixtures excluded)
- Catalogue coverage: 85 PASS, 2 WEAK, 4 RECORDED, 0 FAIL, **30 NOT-EXECUTED**
- Duplicate row ids: none
- Row ids outside the catalogue: none
- **Run verdict: FAIL (coverage or assertion)**

## Catalogue coverage

| Catalogue id | Milestone | Best verdict |
|---|---|---|
| D-01 | 1A | PASS |
| D-02 | 1A | RECORDED |
| D-03 | 1A | NOT-EXECUTED |
| D-04 | 1A | PASS |
| D-05 | 1A | NOT-EXECUTED |
| D-06 | 1A | PASS |
| D-07 | 1A | PASS |
| D-08 | 1A | PASS |
| D-09 | 1B | PASS |
| D-10 | 1B | PASS |
| D-11 | 1A | PASS |
| D-12 | 1B | PASS |
| D-13 | 1B | PASS |
| D-15 | 1A | RECORDED |
| D-16 | 1A/1B/1C | NOT-EXECUTED |
| D4 | 1B | PASS |
| D7-8 | 1B | PASS |
| D7-9 | 1B | PASS |
| F-C-01 | 1A | NOT-EXECUTED |
| F-C-02 | 1A | NOT-EXECUTED |
| F-C-03 | 1A | PASS |
| F-C-04 | 1A | NOT-EXECUTED |
| F-C-05 | 1A | NOT-EXECUTED |
| F-C-06 | 1A | NOT-EXECUTED |
| F-C-07 | 1A | PASS |
| F-C-08 | 1B | NOT-EXECUTED |
| F-C-09 | 1A | PASS |
| F-T-01 | 1A/1B | NOT-EXECUTED |
| F-T-02 | 1A | PASS |
| F-T-03 | 1A | PASS |
| F-T-04 | 1A | PASS |
| F-T-05 | 1A | NOT-EXECUTED |
| F-T-06 | 1B | NOT-EXECUTED |
| F-T-07 | 1B | NOT-EXECUTED |
| F-T-08 | 1A | PASS |
| F-T-09 | 1B | NOT-EXECUTED |
| F-T-10 | 1A | PASS |
| F-T-11 | 1A | PASS |
| T-6.1-001 | 1A | PASS |
| T-6.1-002 | 1A | PASS |
| T-6.1-003 | 1A | RECORDED |
| T-6.1-004 | 1A | PASS |
| T-6.1-005 | 1A | PASS |
| T-6.1-006 | 1A | NOT-EXECUTED |
| T-6.1-007 | 1A | PASS |
| T-6.1-008 | 1A | NOT-EXECUTED |
| T-6.1-009 | 1A | PASS |
| T-6.1-010 | 1A | NOT-EXECUTED |
| T-6.1-011 | 1A | NOT-EXECUTED |
| T-6.1-012 | 1A | NOT-EXECUTED |
| T-6.1-013 | 1A | PASS |
| T-6.2-001 | 1A | PASS |
| T-6.2-002 | 1A | PASS |
| T-6.2-003 | 1A | PASS |
| T-6.2-004 | 1A | PASS |
| T-6.2-005 | 1A | NOT-EXECUTED |
| T-6.2-006 | 1A | PASS |
| T-6.2-007 | 1A | PASS |
| T-6.2-008 | 1A | RECORDED |
| T-6.2-009 | 1A | PASS |
| T-6.3-001 | 1B | PASS |
| T-6.3-002 | 1B | PASS |
| T-6.3-003 | 1B | PASS |
| T-6.3-004 | 1B | PASS |
| T-6.3-005 | 1B | NOT-EXECUTED |
| T-6.3-006 | 1B | PASS |
| T-6.3-007 | 1B | PASS |
| T-6.3-008 | 1B | NOT-EXECUTED |
| T-6.4-001 | 1B | PASS |
| T-6.4-002 | 1B | PASS |
| T-6.4-003 | 1B | PASS |
| T-6.4-004 | 1B | PASS |
| T-6.4-005 | 1B | PASS |
| T-6.4-006 | 1B | PASS |
| T-6.4-007 | 1B | PASS |
| T-6.4-008 | 1B | PASS |
| T-6.4-009 | 1B | WEAK |
| T-6.4-010 | 1B | PASS |
| T-6.4-011 | 1B | PASS |
| T-6.4-012 | 1B | PASS |
| T-6.4-013 | 1B | PASS |
| T-6.4-014 | 1B | PASS |
| T-6.5-001 | 1A | PASS |
| T-6.5-002 | 1A | PASS |
| T-6.5-003 | 1A | PASS |
| T-6.5-004 | 1A | PASS |
| T-6.5-005 | 1A | NOT-EXECUTED |
| T-6.5-006 | 1A | PASS |
| T-6.5-007 | 1A | PASS |
| T-6.5-008 | 1A | NOT-EXECUTED |
| T-6.5-009 | 1A | PASS |
| T-6.5-010 | 1A | PASS |
| T-6.6-001 | 1A | PASS |
| T-6.6-002 | 1A | PASS |
| T-6.6-003 | 1A | PASS |
| T-6.6-004 | 1A | PASS |
| T-6.6-005 | 1A | PASS |
| T-6.6-006 | 1A | PASS |
| T-6.6-007 | 1A | NOT-EXECUTED |
| T-6.6-008 | 1A | PASS |
| T-6.7-001 | 1A | NOT-EXECUTED |
| T-6.8-001 | 1A | PASS |
| T-6.8-002 | 1A | PASS |
| T-6.8-003 | 1A | PASS |
| T-6.8-004 | 1A | PASS |
| T-6.8-005 | 1A | PASS |
| T-6.8-006 | 1A | PASS |
| T-6.8-007 | 1A | PASS |
| T-6.8-008 | 1B | NOT-EXECUTED |
| T-6.8-009 | 1B | NOT-EXECUTED |
| T-6.8-011 | 1A | PASS |
| T-6.8-012 | 1A | PASS |
| T-6.8-013 | 1A | PASS |
| T-6.9-001 | 1A | PASS |
| T-6.9-002 | 1A | PASS |
| T-6.9-003 | 1A | NOT-EXECUTED |
| T-6.9-004 | 1A | PASS |
| T-6.9-005 | 1B | PASS |
| T-6.9-006 | 1B | WEAK |
| T-6.9-007 | 1A | PASS |
| T-6.9-008 | 1B/1C | NOT-EXECUTED |

## Rows

| Row | Verdict | Evidence |
|---|---|---|
| D-01 | PASS | rc=0 lrd=sha256:7f563c6f3fdcca040de5e059c60dbfe283c2d04e0a780d002503661691c3600c {"allocation_id":"allocation:45ed3a21-00000454","console":"/var/lib/agentbound/sessions/45ed3a21-00000454/console.log","init_pid":17636,"launch_record_digest":"sha256:7f563c6f3fdcca040de5e059c60dbfe28 |
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
| T-6.2-007.workspace | PASS | workspace writable as 200357 |
| T-6.2-009 | PASS | denied rc=1 ls /sys/class/net |
| T-6.2-002.netdev | PASS | denied rc=1 interfaces other than lo |
| T-6.2-005 | FIXTURE | orphan spawned; asserted by D-07 at termination |
| T-6.9-001 | PASS | procs=63 (TasksMax bound) |
| T-6.9-002 | PASS | fds_opened= |
| T-6.9-004 | PASS | denied rc=1 dd 100M into tmpfs |
| PROBE-COMPLETE | FIXTURE | probe lines=29 |
| D-01.status | PASS | {"identity_state":"in-use","observation_seq":3,"reason":null,"record_ref":"sha256:7f563c6f3fdcca040de5e059c60dbfe283c2d04e0a780d002503661691c3600c","state":"active"} |
| D-06 | PASS | scope procs=64 (init + workload + orphan/fan-out survivors) |
| D-04.host-view | PASS | uid 200357 processes outside scope=0 |
| D-08 | PASS | {"cgroup_kill_written":true,"cgroup_procs_remaining":[],"credential_scan_inside_scope":[],"credential_scan_outside_scope":[],"d_state":[],"elapsed_ms":2032,"freeze_written":true,"frozen_observed":false,"gateway_admission_denied":false,"init_pid":17636,"init_pidfd_exited":true,"sigterm_sent":true} |
| F-T-03 | PASS | {"cgroup_kill_written":true,"cgroup_procs_remaining":[],"credential_scan_inside_scope":[],"credential_scan_outside_scope":[],"d_state":[],"elapsed_ms":2032,"freeze_written":true,"frozen_observed":false,"gateway_admission_denied":false,"init_pid":17636,"init_pidfd_exited":true,"sigterm_sent":true} |
| F-T-04 | PASS | kill written without waiting for frozen 1; procs empty; pidfd exited |
| D-07 | PASS | orphan/double-fork survivors killed with the scope; host credential scan clean |
| F-T-10 | PASS | {"identity_state":"quarantined","observation_seq":6,"reason":"conformance","record_ref":"sha256:7f563c6f3fdcca040de5e059c60dbfe283c2d04e0a780d002503661691c3600c","state":"cleaned/sealed"} |
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
| T-6.6-001.audit | PASS | session.rejected events with failed_input=509 |
| F-C-03 | PASS | step=3 rule=mount_source_escape identity=quarantined scopes_left=0 rollback=["cgroup.kill","scope stopped","identity → reclaiming"] |
| F-C-07 | PASS | step=7 rule=fault_injected identity=quarantined scopes_left=0 rollback=["child killed and reaped","cgroup.kill","scope stopped","identity → reclaiming"] |
| F-C-09 | PASS | step=8 rule=fault_injected identity=quarantined scopes_left=0 rollback=["child killed and reaped","cgroup.kill","scope stopped","identity → reclaiming"] |
| F-C-09.record | PASS | lrd=sha256:292e4727655b5f648d87eaf358167f24824166a56bcadc4ce78b4d4e4dbcea31 kinds=["session.launch_record_committed", "session.construction_failed", "session.ownership_projected", "session.cleanup_completed", "session.identity_released", "session.sealed"] |
| D-11 | PASS | constructor fault rows F-C-03/07/09: no runnable session, identity held, scope gone |
| T-6.5-004 | PASS | activations=1 refusals=1 |
| T-6.5-009 | PASS | allocator latest states: [('free', 96), ('quarantined', 361), ('reclaiming', 1)]; free-before-floor violations=0 |
| T-6.8-setup | FIXTURE | sha256:b865b5959d045e01cef096335407b4160e4367c9971c6570f029a8be62084f42 |
| T-6.8-006 | PASS | {"behaviour":"continue-degraded","state":"active"} |
| T-6.8-011 | PASS | {"behaviour":"continue-degraded","state":"active"} |
| T-6.8-007 | PASS | {"behaviour":"quiesce","state":"quiescing"} |
| F-T-02 | PASS | populated 1 frozen 1 |
| T-6.8-003 | PASS | {"behaviour":"terminate","state":"cleaned/sealed"} |
| T-6.8-006.audit | PASS | ["session.launch_record_committed", "session.activated", "session.revocation_received", "session.degraded", "session.revocation_received", "session.degraded", "session.revocation_received", "session.quiesce_started", "session.revocation_received", "session.termination_started", "session.terminated", "session.ownership_projected", "session.cleanup_completed", "session.identity_released", "session.sealed"] |
| T-6.8-001 | PASS | trigger=initiator_disabled behaviour=terminate state=cleaned/sealed |
| T-6.8-002 | PASS | trigger=approval_expired behaviour=quiesce state=quiescing |
| T-6.8-004 | PASS | trigger=catalogue_withdrawn behaviour=quiesce state=quiescing |
| T-6.8-005 | PASS | trigger=task_cancelled behaviour=terminate state=cleaned/sealed |
| T-6.8-012 | PASS | procs_while_down=3 (containment held, no authority available: daemon_reachable=false) cli_reply={"class":"unavailable","detail":"Connection refused (os erro after_restart=cleaned/sealed kinds=["session.launch_record_committed", "session.activated", "session.recovery_reconciled", "session.recovery_reconciled", "session.ownership_projected", "session.cleanup_completed", "session.identity_released", "session.sealed"] |
| T-6.8-012.contained | PASS | state=cleaned/sealed identity=quarantined procs=0 |
| T-6.9-007 | PASS | audit chain head=sha256:81c9abcbe61598bace9cd24e7082017bf394a4c7fd74f499d2b0c356fee6af30 seq=13536 lost=0 |
| T-6.5-003 | PASS | rule=mount_source_escape detail=../../../etc errno=18 |
| D-10.launch | PASS | rc=0 lrd=sha256:106babf5cda1cd0a9c34091a1d9ab6b62d29c1f89a6bf339052000e6c8ad413e topology=local-socket |
| T-6.4-003.projected | PASS | gateway socket node present |
| D-09 | PASS | authenticated connection, typed ping admitted: "pong":true |
| T-6.3-001 | PASS | no credential in environment |
| T-6.3-002 | PASS | credential path absent/unreadable |
| T-6.1-007.sibling | PASS | sibling /workspace/work-200005 not writable |
| T-6.3-002.no-remote | PASS | no git remote or credential in the worker repository (WP1 GS-1) |
| D-10.bundle | FIXTURE | tip=7312bff9ef2883d858bf8b9442a5d44ccea0c0c0 bytes=303 |
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
| T-6.4-006 | PASS | "rule":"descriptor_transfer" |
| T-6.4-007 | PASS | "rule":"process_mismatch" |
| T-6.4-001 | PASS | inet errno=1 inet6 errno=1 packet errno=1 netlink errno=1 vsock errno=1 |
| T-6.3-003 | PASS | fds: 0 /dev/null 1 pipe:[97982] 2 pipe:[97982]  |
| T-6.3-004 | PASS | child env credential hits=0 fds=4 |
| T-6.3-006 | PASS | no credential-like text in gateway replies/adapter output |
| T-6.4-010.stream | PASS | connect errno=Protocol wrong type for socket (os error 91)  |
| T-6.4-010.dgram | PASS | connect errno=Protocol wrong type for socket (os error 91)  |
| T-6.9-005 | PASS | "rule":"budget_bytes" |
| T-6.9-006 | WEAK | held=16 refused=4 (limit 16) |
| GW-HELD | FIXTURE | connection held for revocation test |
| GW-COMPLETE | FIXTURE | worker lines=31 |
| D-13 | PASS | staging ref for session 0ca6ce9922eff69e: true; main d5552a130bbe2bcd1eb2874644bb26717bd265b8→d5552a130bbe2bcd1eb2874644bb26717bd265b8 |
| D-13.trace | PASS | host hook log carries trace trace:e5d06f0a162331c2b9064f13a877d221 |
| T-6.4-012.host-hook | PASS | direct push to main as gateway user refused by host hook: remote: protected: refs/heads/main |
| T-6.4-002 | PASS | session netns interfaces: ls: /sys/class/net: No such file or directory     lo |
| T-6.4-003 | PASS | host socket dir from session: ls: /run/agentbound: No such file or directory ls: /var/run/agentbound: No such file or directory |
| T-6.4-003.only | PASS | exactly one socket node in /run: srw-rw-rw-    1 994      1000             0 Sep  5 19:34 /run/gateway.sock 1  |
| T-6.4-004 | PASS | abstract socket from session netns: err 111 |
| T-6.4-005 | PASS | outside-scope peer with session uid: closed by gateway 45  |
| T-6.4-008 | PASS | DENY host-root-peer closed 104 \| DENY forged-pid closed 104 \| DENY two-creds closed 104 \|  |
| T-6.4-014 | PASS | quiesce state=quiescing gateway admission=false new-conn-while-quiesced=DENY host-root-peer closed 104 behaviour=terminate; held connection's post-denial packet: {"body":{"authorization_id":"launchrec:fix-issue-1235-000500","detail":"session not admitting operations","launch_record ; status after seal: unknown_record |
| D-12 | PASS | completeness: 15/15 required kinds on record; missing=[] |
| T-6.3-007 | PASS | post-termination: projection released, record sealed, socket node removed with the mount namespace |
| T-6.3-007.socket | PASS | host-side socket nodes left for this allocation: 0 |
| T-6.4-013 | PASS | caller-supplied session/trace refused (closed argument set); no ref under the other session's namespace: {"body":{"authorization_id":"launchrec:fix-issue-1235-000501","detail":"Unexpected(0, \"non-canonical\")","launch_record_digest":"sha256:c80dae36eaa1d43bc8e0c7bc4cef17fb479483ccb5609f591cfe2820c93f92a1","requirement_id":"R-GW-1","rule":"parse","trace_id":"trace:c4fcf0fa17ecf43dc24273a2771a55f9"},"cl |
| D4.7-reconstruct | PASS | socket before restart=1 "projections":1 ping after restart: {"body":{"operation_seq":2,"result":{"pong":true},"trace_id":"trace:c4fcf0fa17ecf43dc24273a2771a55f9"},"class":"ok","ok":true,"v":"agentbound.wire.v0.1"}  |
| T-6.4-009 | WEAK | process-instance denials=116; classes: 1 "detail":"credential pid 9870 uid 200300 vs establishing 9869 200300";      1 "detail":"credential pid 9491 uid 200299 vs establishing 9490 200299";      1 "detail":"credential pid 8394 uid 200286 vs establishing 8393 200286"; (pidfs inode is the instance key; start time corroborating; a same-tick PID reuse is not reproducible on demand — the check is inode-based so the tick is irrelevant) |
| T-6.4-012 | WEAK | caller-supplied url ignored; bundle path enforced: rule":"args_schema"}  |
| D7-9.diagnostics | PASS | requirement=R-GW-4 authorization=launchrec:fix-issue-1235-000501 lrd-matches=true trace=trace:c4fcf0fa17ecf43dc24273a2771a55f9 foreign-ids-absent=true |
| D7-8.audit-loss | PASS | gateway with no audit path (receiver down, spool unwritable): first op's event lost → admission closed + revocation_signal; lifecycle "trigger":"audit_pipeline_degraded_below_stop_threshold" → state=terminated; second attempt: nsenter: cannot open /proc/18597/ns/net: No such file or directory |
| D-06.storage-principal | PASS | work dir owner after seal: storage-engineering agentbound; files still owned by ephemeral uid: 0; "detail":{"bytes":1041,"failed":0,"files":26,"storage_principal":"storage:engineering-agent"} |
| D-02.1B | RECORDED | descriptor allowlist entries=0 4 (stdin, stdout, stderr, gateway_socket mount); no attach/PTY path exists to deny — partial stays recorded |
| T-6.1-003.1B | RECORDED | no PTY projected under local-socket either; N/A stays recorded |
| T-6.1-013 | PASS | sealed session's socket: nodes left=0 connect=err 2 |
| T-6.2-008.1B | RECORDED | loaders/interpreters beyond sh+git in image: 0 |
| D-15.1B | RECORDED | delegation operations in catalogue: [] — residual stays recorded (no delegation path to narrow) |
