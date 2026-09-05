# Agentbound conformance run — 1A + 1B rows (machine output)

- Host: agentbound-dev
- Kernel: 6.12.107+deb13-cloud-amd64
- systemd: systemd 257 (257.13-1~deb13u1)
- git: git version 2.47.3
- Date: 2026-09-05T17:37:47Z
- Rows: 139 PASS / 0 FAIL

| Row | Verdict | Evidence |
|---|---|---|
| D-01 | PASS | rc=0 lrd=sha256:eda685849edef3b7a40964a2d9dfdb65ee5a19eb5cfd29cdcb6a4f19df721c08 {"allocation_id":"allocation:45ed3a21-00000398","console":"/var/lib/agentbound/sessions/45ed3a21-00000398/console.log","init_pid":10767,"launch_record_digest":"sha256:eda685849edef3b7a40964a2d9dfdb65e |
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
| T-6.2-007.workspace | PASS | workspace writable as 200301 |
| T-6.2-009 | PASS | denied rc=1 ls /sys/class/net |
| T-6.2-002.netdev | PASS | denied rc=1 interfaces other than lo |
| T-6.2-005 | PASS | orphan spawned |
| T-6.9-001 | PASS | procs=63 (TasksMax bound) |
| T-6.9-002 | PASS | fds_opened= |
| T-6.9-004 | PASS | denied rc=1 dd 100M into tmpfs |
| PROBE-COMPLETE | PASS | probe lines=29 |
| D-01.status | PASS | {"identity_state":"in-use","observation_seq":3,"reason":null,"record_ref":"sha256:eda685849edef3b7a40964a2d9dfdb65ee5a19eb5cfd29cdcb6a4f19df721c08","state":"active"} |
| D-06 | PASS | scope procs=64 (init + workload + orphan/fan-out survivors) |
| D-04.host-view | PASS | uid 200301 processes outside scope=0 |
| D-08 | PASS | {"cgroup_kill_written":true,"cgroup_procs_remaining":[],"credential_scan_inside_scope":[],"credential_scan_outside_scope":[],"d_state":[],"elapsed_ms":2031,"freeze_written":true,"frozen_observed":false,"gateway_admission_denied":false,"init_pid":10767,"init_pidfd_exited":true,"sigterm_sent":true} |
| F-T-03 | PASS | {"cgroup_kill_written":true,"cgroup_procs_remaining":[],"credential_scan_inside_scope":[],"credential_scan_outside_scope":[],"d_state":[],"elapsed_ms":2031,"freeze_written":true,"frozen_observed":false,"gateway_admission_denied":false,"init_pid":10767,"init_pidfd_exited":true,"sigterm_sent":true} |
| F-T-04 | PASS | kill written without waiting for frozen 1; procs empty; pidfd exited |
| D-07 | PASS | orphan/double-fork survivors killed with the scope; host credential scan clean |
| F-T-10 | PASS | {"identity_state":"quarantined","observation_seq":6,"reason":"conformance","record_ref":"sha256:eda685849edef3b7a40964a2d9dfdb65ee5a19eb5cfd29cdcb6a4f19df721c08","state":"cleaned/sealed"} |
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
| T-6.6-001.audit | PASS | session.rejected events with failed_input=441 |
| F-C-03 | PASS | step=3 rule=mount_source_escape identity=quarantined scopes_left=0 rollback=["cgroup.kill","scope stopped","identity → reclaiming"] |
| F-C-07 | PASS | step=7 rule=fault_injected identity=quarantined scopes_left=0 rollback=["child killed and reaped","cgroup.kill","scope stopped","identity → reclaiming"] |
| F-C-09 | PASS | step=8 rule=fault_injected identity=quarantined scopes_left=0 rollback=["child killed and reaped","cgroup.kill","scope stopped","identity → reclaiming"] |
| F-C-09.record | PASS | lrd=sha256:f9429a453c4275488f2941104f22c71a53dadd2c18b3d6c1b68652a5697eb35e kinds=["session.launch_record_committed", "session.construction_failed", "session.ownership_projected", "session.cleanup_completed", "session.identity_released", "session.sealed"] |
| D-11 | PASS | constructor fault rows F-C-03/07/09: no runnable session, identity held, scope gone |
| T-6.5-004 | PASS | activations=1 refusals=1 |
| T-6.5-009 | PASS | allocator latest states: [('free', 96), ('quarantined', 305), ('reclaiming', 1)]; free-before-floor violations=0 |
| T-6.8.setup | PASS | sha256:050cf6ebb109f4fc1d27d8fbb18d2bf1da4d1e004f5f95f7f7092fce512bdd2c |
| T-6.8-006 | PASS | {"behaviour":"continue-degraded","state":"active"} |
| T-6.8-011 | PASS | {"behaviour":"continue-degraded","state":"active"} |
| T-6.8-007 | PASS | {"behaviour":"quiesce","state":"quiescing"} |
| F-T-02 | PASS | populated 1 frozen 1 |
| T-6.8-003 | PASS | {"behaviour":"terminate","state":"cleaned/sealed"} |
| T-6.8.audit | PASS | ["session.launch_record_committed", "session.activated", "session.revocation_received", "session.degraded", "session.revocation_received", "session.degraded", "session.revocation_received", "session.quiesce_started", "session.revocation_received", "session.termination_started", "session.terminated", "session.ownership_projected", "session.cleanup_completed", "session.identity_released", "session.sealed"] |
| T-6.8-001 | PASS | trigger=initiator_disabled behaviour=terminate state=cleaned/sealed |
| T-6.8-002 | PASS | trigger=approval_expired behaviour=quiesce state=quiescing |
| T-6.8-004 | PASS | trigger=catalogue_withdrawn behaviour=quiesce state=quiescing |
| T-6.8-005 | PASS | trigger=task_cancelled behaviour=terminate state=cleaned/sealed |
| T-6.8-012 | PASS | procs_while_down=3 (containment held, no authority available: daemon_reachable=false) cli_reply={"class":"unavailable","detail":"Connection refused (os erro after_restart=cleaned/sealed kinds=["session.launch_record_committed", "session.activated", "session.recovery_reconciled", "session.recovery_reconciled", "session.ownership_projected", "session.cleanup_completed", "session.identity_released", "session.sealed"] |
| T-6.8-012.contained | PASS | state=cleaned/sealed identity=quarantined procs=0 |
| T-6.9-007 | PASS | audit chain head=sha256:066cf4ac502bcbab22c71db7368ec8fc4542fdc64d33314191560368c658e4a7 seq=11380 lost=0 |
| T-6.5-003 | PASS | rule=mount_source_escape detail=../../../etc errno=18 |
| D-10.launch | PASS | rc=0 lrd=sha256:53ddd29c376f88f55d53485af235db88a5deb89729c6738a34683495deaa82ae topology=local-socket |
| T-6.4-003.projected | PASS | gateway socket node present |
| D-09 | PASS | authenticated connection, typed ping admitted: "pong":true |
| T-6.3-001 | PASS | no credential in environment |
| T-6.3-002 | PASS | credential path absent/unreadable |
| T-6.1-007.sibling | PASS | sibling /workspace/work-200005 not writable |
| GS-1 | PASS | no remote |
| bundle | PASS | tip=b506d9120b77eb905545674bd652412d29d06e6e bytes=303 |
| D-10 | PASS | push_staging accepted rc=0 |
| GS-4[../main] | PASS | "rule":"ref_tail_grammar" |
| GS-4[main:refs/heads/main] | PASS | "rule":"ref_tail_grammar" |
| GS-4[+fix] | PASS | "rule":"ref_tail_marker" |
| GS-4[fix.lock] | PASS | "rule":"ref_tail_grammar" |
| GS-4[a_b] | PASS | "rule":"ref_tail_charset" |
| GS-4[] | PASS | "rule":"ref_tail_empty_or_long" |
| GS-4[refs/heads/main] | PASS | "rule":"ref_tail_names_ref" |
| T-6.4-011 | PASS | "rule":"scope_repository" |
| GS-8.force | PASS | "rule":"operation_not_granted" |
| T-6.4-006 | PASS | "rule":"descriptor_transfer" |
| T-6.4-007 | PASS | "rule":"process_mismatch" |
| T-6.4-001 | PASS | inet errno=1 inet6 errno=1 packet errno=1 netlink errno=1 vsock errno=1 |
| T-6.3-003 | PASS | fds: 0 /dev/null 1 pipe:[64080] 2 pipe:[64080]  |
| T-6.3-004 | PASS | child env credential hits=0 fds=4 |
| T-6.3-006 | PASS | no credential-like text in gateway replies/adapter output |
| T-6.4-010.stream | PASS | connect errno=Protocol wrong type for socket (os error 91)  |
| T-6.4-010.dgram | PASS | connect errno=Protocol wrong type for socket (os error 91)  |
| T-6.9-005 | PASS | "rule":"budget_bytes" |
| T-6.9-006 | PASS | held=16 refused=4 (limit 16) |
| GW-HELD | PASS | connection held for revocation test |
| GW-COMPLETE | PASS | worker lines=31 |
| D-13 | PASS | staging ref for session ae70c891c5d5ed07: true; main d5552a130bbe2bcd1eb2874644bb26717bd265b8→d5552a130bbe2bcd1eb2874644bb26717bd265b8 |
| D-13.trace | PASS | host hook log carries trace trace:b9eff18bb2971465f6c46e2026495684 |
| GS-6 | PASS | direct push to main as gateway user refused by host hook: remote: protected: refs/heads/main |
| T-6.4-002 | PASS | session netns interfaces: ls: /sys/class/net: No such file or directory     lo |
| T-6.4-003 | PASS | host socket dir from session: ls: /run/agentbound: No such file or directory ls: /var/run/agentbound: No such file or directory |
| T-6.4-003.only | PASS | exactly one socket node in /run: srw-rw-rw-    1 994      1000             0 Sep  5 17:36 /run/gateway.sock 1  |
| T-6.4-004 | PASS | abstract socket from session netns: err 111 |
| T-6.4-005 | PASS | outside-scope peer with session uid: closed by gateway 41  |
| T-6.4-008 | PASS | DENY host-root-peer closed 104 \| DENY forged-pid closed 104 \| DENY two-creds closed 104 \|  |
| T-6.4-014 | PASS | quiesce state=quiescing gateway admission=false new-conn-while-quiesced=DENY host-root-peer closed 104 behaviour=terminate; held connection's post-denial packet: {"body":{"authorization_id":"launchrec:fix-issue-1235-000440","detail":"session not admitting operations","launch_record ; status after seal: unknown_record |
| D-12 | PASS | completeness: 15/15 required kinds on record; missing=[] |
| T-6.3-007 | PASS | post-termination: projection released, record sealed, socket node removed with the mount namespace |
| T-6.3-007.socket | PASS | host-side socket nodes left for this allocation: 0 |
| T-6.4-013 | PASS | caller-supplied session/trace refused (closed argument set); no ref under the other session's namespace: {"body":{"authorization_id":"launchrec:fix-issue-1235-000441","detail":"Unexpected(0, \"non-canonical\")","launch_record_digest":"sha256:66e63b6b2b23bcf74c782b2fa7f53e8dba72ab894d2a2680d581e676238add5e","requirement_id":"R-GW-1","rule":"parse","trace_id":"trace:e10ac952ac3f2b34597354322d745452"},"cl |
| D4.7-reconstruct | PASS | socket before restart=1 "projections":1 ping after restart: {"body":{"operation_seq":2,"result":{"pong":true},"trace_id":"trace:e10ac952ac3f2b34597354322d745452"},"class":"ok","ok":true,"v":"agentbound.wire.v0.1"}  |
| T-6.4-009 | PASS | process-instance denials=100; classes: 1 "detail":"credential pid 9870 uid 200300 vs establishing 9869 200300";      1 "detail":"credential pid 9491 uid 200299 vs establishing 9490 200299";      1 "detail":"credential pid 8394 uid 200286 vs establishing 8393 200286"; (pidfs inode is the instance key; start time corroborating; a same-tick PID reuse is not reproducible on demand — the check is inode-based so the tick is irrelevant) |
| T-6.4-012 | PASS | caller-supplied url ignored; bundle path enforced: rule":"args_schema"}  |
| D7-9.diagnostics | PASS | requirement=R-GW-4 authorization=launchrec:fix-issue-1235-000441 lrd-matches=true trace=trace:e10ac952ac3f2b34597354322d745452 foreign-ids-absent=true |
| D7-8.audit-loss | PASS | gateway with no audit path (receiver down, spool unwritable): first op's event lost → admission closed + revocation_signal; lifecycle "trigger":"audit_pipeline_degraded_below_stop_threshold" → state=terminated; second attempt: nsenter: cannot open /proc/11734/ns/net: No such file or directory |
| D-06.storage-principal | PASS | work dir owner after seal: storage-engineering agentbound; files still owned by ephemeral uid: 0; "detail":{"bytes":1040,"failed":0,"files":26,"storage_principal":"storage:engineering-agent"} |
| D-02.1B | PASS | descriptor allowlist entries=0 4 (stdin, stdout, stderr, gateway_socket mount); no attach/PTY path exists to deny — partial stays recorded |
| T-6.1-003.1B | PASS | no PTY projected under local-socket either; N/A stays recorded |
| T-6.1-013 | PASS | sealed session's socket: nodes left=0 connect=err 2 |
| T-6.2-008.1B | PASS | loaders/interpreters beyond sh+git in image: 0 |
| D-15.1B | PASS | delegation operations in catalogue: [] — residual stays recorded (no delegation path to narrow) |
