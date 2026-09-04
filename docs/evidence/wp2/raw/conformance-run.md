# WP2 conformance run (machine output)

- Host: agentbound-dev
- Kernel: 6.12.107+deb13-cloud-amd64
- systemd: systemd 257 (257.13-1~deb13u1)
- Rows: 84 PASS / 0 FAIL

| Row | Verdict | Evidence |
|---|---|---|
| D-01 | PASS | rc=0 lrd=sha256:547cfdfd7ba46f37c3c236d79f6163546004b68b21c61174d3fd4f24934729c6 {"allocation_id":"allocation:45ed3a21-00000086","console":"/var/lib/agentbound/sessions/45ed3a21-00000086/console.log","init_pid":43262,"launch_record_digest":"sha256:547cfdfd7ba46f37c3c236d79f6163546 |
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
| T-6.2-007.workspace | PASS | workspace writable as 200085 |
| T-6.2-009 | PASS | denied rc=1 ls /sys/class/net |
| T-6.2-002.netdev | PASS | denied rc=1 interfaces other than lo |
| T-6.2-005 | PASS | orphan spawned |
| T-6.9-001 | PASS | procs=63 (TasksMax bound) |
| T-6.9-002 | PASS | fds_opened= |
| T-6.9-004 | PASS | denied rc=1 dd 100M into tmpfs |
| PROBE-COMPLETE | PASS | probe lines=29 |
| D-01.status | PASS | {"identity_state":"in-use","observation_seq":3,"reason":null,"record_ref":"sha256:547cfdfd7ba46f37c3c236d79f6163546004b68b21c61174d3fd4f24934729c6","state":"active"} |
| D-06 | PASS | scope procs=64 (init + workload + orphan/fan-out survivors) |
| D-04.host-view | PASS | uid 200085 processes outside scope=0 |
| D-08 | PASS | {"cgroup_kill_written":true,"cgroup_procs_remaining":[],"credential_scan_inside_scope":[],"credential_scan_outside_scope":[],"d_state":[],"elapsed_ms":2033,"freeze_written":true,"frozen_observed":false,"init_pid":43262,"init_pidfd_exited":true,"sigterm_sent":true} |
| F-T-03 | PASS | {"cgroup_kill_written":true,"cgroup_procs_remaining":[],"credential_scan_inside_scope":[],"credential_scan_outside_scope":[],"d_state":[],"elapsed_ms":2033,"freeze_written":true,"frozen_observed":false,"init_pid":43262,"init_pidfd_exited":true,"sigterm_sent":true} |
| F-T-04 | PASS | kill written without waiting for frozen 1; procs empty; pidfd exited |
| D-07 | PASS | orphan/double-fork survivors killed with the scope; host credential scan clean |
| F-T-10 | PASS | {"identity_state":"quarantined","observation_seq":6,"reason":"conformance","record_ref":"sha256:547cfdfd7ba46f37c3c236d79f6163546004b68b21c61174d3fd4f24934729c6","state":"cleaned/sealed"} |
| F-T-11 | PASS | audit kinds=["session.launch_record_committed", "session.activated", "session.termination_started", "session.terminated", "session.cleanup_completed", "session.identity_released", "session.sealed"] |
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
| T-6.6-001.audit | PASS | session.rejected events with failed_input=101 |
| F-C-03 | PASS | step=3 rule=mount_source_escape identity=quarantined scopes_left=0 rollback=["cgroup.kill","scope stopped","identity → reclaiming"] |
| F-C-07 | PASS | step=7 rule=fault_injected identity=quarantined scopes_left=0 rollback=["child killed and reaped","cgroup.kill","scope stopped","identity → reclaiming"] |
| F-C-09 | PASS | step=8 rule=fault_injected identity=quarantined scopes_left=0 rollback=["child killed and reaped","cgroup.kill","scope stopped","identity → reclaiming"] |
| F-C-09.record | PASS | lrd=sha256:d358fb10d5d9d5b66b388dc2dd66725d734b85372dde854adbbd23469fea0ddb kinds=["session.launch_record_committed", "session.construction_failed", "session.cleanup_completed", "session.identity_released", "session.sealed"] |
| D-11 | PASS | constructor fault rows F-C-03/07/09: no runnable session, identity held, scope gone |
| T-6.5-004 | PASS | activations=1 refusals=1 |
| T-6.5-009 | PASS | allocator latest states: [('quarantined', 90)] |
| T-6.8.setup | PASS | sha256:c176d23b2a6b55c2883c51091fc772dad042697dd5f09c697b0ac1a81809b258 |
| T-6.8-006 | PASS | {"behaviour":"continue-degraded","state":"active"} |
| T-6.8-011 | PASS | {"behaviour":"continue-degraded","state":"active"} |
| T-6.8-007 | PASS | {"behaviour":"quiesce","state":"quiescing"} |
| F-T-02 | PASS | populated 1 frozen 1 |
| T-6.8-003 | PASS | {"behaviour":"terminate","state":"cleaned/sealed"} |
| T-6.8.audit | PASS | ["session.launch_record_committed", "session.activated", "session.revocation_received", "session.degraded", "session.revocation_received", "session.degraded", "session.revocation_received", "session.quiesce_started", "session.revocation_received", "session.termination_started", "session.terminated", "session.cleanup_completed", "session.identity_released", "session.sealed"] |
| T-6.8-001 | PASS | trigger=initiator_disabled behaviour=terminate state=cleaned/sealed |
| T-6.8-002 | PASS | trigger=approval_expired behaviour=quiesce state=quiescing |
| T-6.8-004 | PASS | trigger=catalogue_withdrawn behaviour=quiesce state=quiescing |
| T-6.8-005 | PASS | trigger=task_cancelled behaviour=terminate state=cleaned/sealed |
| T-6.8-012 | PASS | procs_while_down=3 (containment held, no authority available: daemon_reachable=false) cli_reply={"class":"unavailable","detail":"Connection refused (os erro after_restart=termination-incomplete kinds=["session.launch_record_committed", "session.activated", "session.recovery_reconciled"] |
| T-6.8-012.contained | PASS | state=termination-incomplete identity=in-use procs=0 |
| T-6.9-007 | PASS | audit chain head=sha256:8d95cbdc83524936ed6bc15162fbb4fddb7abd55a32651859dd5eb3614299de3 seq=1009 lost=0 |
| T-6.5-003 | PASS | rule=mount_source_escape detail=../../../etc errno=18 |
