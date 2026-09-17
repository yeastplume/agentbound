#!/usr/bin/env python3
"""Root-only smoke tests for the authorized development VM repair candidate.
No store resets. Each test launch is terminated in a finally block.
"""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time


def command(args, timeout=45):
    try:
        result = subprocess.run(args, text=True, capture_output=True, timeout=timeout)
    except subprocess.TimeoutExpired as error:
        print(json.dumps({'command': args, 'timeout_s': timeout, 'error': str(error)}), flush=True)
        raise
    print(json.dumps({'command': args, 'rc': result.returncode,
                      'stdout': result.stdout, 'stderr': result.stderr}), flush=True)
    return result


def replies(text):
    values = []
    for line in text.splitlines():
        try:
            values.append(json.loads(line))
        except ValueError:
            pass
    return values


def operator(args):
    return command(['runuser', '-u', 'alice', '--', *args])


def main():
    assert os.getuid() == 0
    marker = '/root/agentbound-repair-backup-20260917/candidate-must-not-create.jsonl'
    assert not Path(marker).exists(), 'refuse to reuse a sentinel'
    result = operator(['sudo', '-n', '/usr/local/bin/agentbound-launch',
                       '--authorization', 'launchrec:repair-nonexistent', '--audit-spool', marker])
    assert result.returncode != 0 and not Path(marker).exists()
    result = operator(['sudo', '-n', 'AGENTBOUND_AUDIT_SOCKET=/tmp/untrusted.sock',
                       '/usr/local/bin/agentbound-launch', '--authorization', 'launchrec:repair-nonexistent'])
    assert result.returncode != 0
    result = operator(['sudo', '-n', '/usr/local/bin/agentbound-launch',
                       '--authorization', 'launchrec:repair-nonexistent', '--fault', 'fd-leak'])
    assert result.returncode != 0
    print('PASS restricted sudo arguments and environment', flush=True)
    # Exercise launcher guard independently of sudoers; do not rely on the outer rule alone.
    result = command(['env', 'SUDO_UID=0', '/usr/local/bin/agentbound-launch',
                      '--authorization', 'launchrec:repair-nonexistent', '--audit-spool', marker])
    assert result.returncode == 2 and not Path(marker).exists()
    print('PASS launcher pre-I/O guard', flush=True)

    request = {
        'schema_version': 'agentbound.session-request.v0.1',
        'agent_principal_id': 'agent:finance-agent',
        'task_purpose_id': 'task:redwood-analysis',
        'requested_runtime': 'runtime:scripted-loop',
        'requested_resources': ['resource:workspace-finance'],
        'initiator_credential_ref': 'authn:alice-session-0001',
        'approval_references': [],
    }
    fd, path = tempfile.mkstemp(prefix='repair-request-', suffix='.json')
    lrd = None
    try:
        with os.fdopen(fd, 'w') as stream:
            json.dump(request, stream)
        os.chmod(path, 0o644)
        result = operator(['/usr/local/bin/agentbound', 'request', path])
        values = replies(result.stdout)
        assert result.returncode == 0, 'normal operator launch failed'
        for value in values:
            body = value.get('body', value)
            if body.get('launch_record_digest'):
                lrd = body['launch_record_digest']
        assert lrd, 'launch reply has no record digest'
        result = command(['/usr/local/bin/agentbound', 'terminate', lrd, 'repair-smoke'])
        assert result.returncode == 0
        deadline = time.monotonic() + 30
        while True:
            result = command(['/usr/local/bin/agentbound', 'status', lrd])
            body = json.loads(result.stdout)['body']
            state = body.get('state')
            if state == 'cleaned/sealed':
                break
            assert time.monotonic() < deadline, f'cleanup did not seal: {state}'
            time.sleep(0.25)
        result = command(['/usr/local/bin/agentbound', 'audit', lrd])
        rows = json.loads(result.stdout)['body']['rows']
        events = [row['event'] for row in rows]
        cleanup = [event for event in events if event['event'] == 'session.cleanup_completed']
        assert cleanup, 'central receiver did not accept cleanup event'
        assert any(event['event'] == 'session.sealed' for event in events)
        assert all('acl_removal_failures' in event['detail'] for event in cleanup)
        print('PASS no-gateway launch/termination/seal and central cleanup audit', flush=True)
    finally:
        if lrd:
            # A sealed session has no live workload and cannot be terminated again.
            status = command(['/usr/local/bin/agentbound', 'status', lrd])
            if json.loads(status.stdout).get('body', {}).get('state') != 'cleaned/sealed':
                command(['/usr/local/bin/agentbound', 'terminate', lrd, 'repair-smoke-finally'])
        os.unlink(path)


if __name__ == '__main__':
    main()
