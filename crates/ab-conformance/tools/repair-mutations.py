#!/usr/bin/env python3
"""Targeted unit-level negative controls; never deploy mutated binaries.
Run from a candidate checkout with cargo available. Output directory must not
exist. This supplements, not replaces, the full live negative-control harness.
"""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

CONTROLS = [
    ('no-gateway-waiver-removed', 'crates/agentbound-lifecycle/src/session.rs',
     'topology == "none" || (topology == "local-socket" && reply',
     '(topology == "local-socket" && reply',
     'agentbound-lifecycle', 'session::regression_tests::no_gateway_does_not_require_a_gateway_reply'),
    ('cleanup-schema-member-removed', 'crates/agentbound-audit/src/events.rs',
     '"acl_entries_removed", "acl_removal_failures", "grants"',
     '"acl_entries_removed", "grants"',
     'agentbound-lifecycle', 'session::regression_tests::actual_cleanup_payload_matches_receiver_schema'),
    ('sudo-config-guard-removed', 'crates/agentbound-launch/src/entry.rs',
     'if key != "--authorization" && mode != Invocation::Admin {',
     'if false && key != "--authorization" && mode != Invocation::Admin {',
     'agentbound-launch', 'entry::tests::sudo_allows_only_authorization_and_uses_fixed_defaults'),
]


def main():
    root = Path.cwd()
    evidence = Path(sys.argv[1]).resolve()
    evidence.mkdir(parents=True, exist_ok=False)
    result = []
    with tempfile.TemporaryDirectory(prefix='agentbound-unit-mutations-') as directory:
        candidate = Path(directory)
        for name in ['Cargo.toml', 'Cargo.lock']:
            shutil.copy2(root / name, candidate / name)
        shutil.copytree(root / 'crates', candidate / 'crates', ignore=shutil.ignore_patterns('__pycache__'))
        for name, path, old, new, package, test in CONTROLS:
            source = candidate / path
            original = source.read_bytes()
            text = original.decode()
            assert text.count(old) == 1, f'{name}: mutation target missing or ambiguous'
            command = ['cargo', 'test', '--locked', '-p', package, test, '--', '--exact']
            baseline = subprocess.run(command, cwd=candidate, capture_output=True, text=True)
            (evidence / f'{name}-clean.log').write_text(baseline.stdout + baseline.stderr)
            assert baseline.returncode == 0 and f'test {test} ... ok' in baseline.stdout, f'{name}: baseline not passing'
            try:
                source.write_text(text.replace(old, new, 1))
                mutated = subprocess.run(command, cwd=candidate, capture_output=True, text=True)
                output = mutated.stdout + mutated.stderr
                (evidence / f'{name}-mutant.log').write_text(output)
                discriminates = mutated.returncode != 0 and f'test {test} ... FAILED' in output
                result.append({'control': name, 'test': test, 'rc': mutated.returncode,
                               'discriminates': discriminates,
                               'original_sha256': hashlib.sha256(original).hexdigest(),
                               'mutation': {'old': old, 'new': new}})
                print(json.dumps(result[-1]), flush=True)
            finally:
                source.write_bytes(original)
                assert source.read_bytes() == original
    (evidence / 'results.json').write_text(json.dumps(result, indent=2) + '\n')
    assert len(result) == len(CONTROLS) and all(r['discriminates'] for r in result)


if __name__ == '__main__':
    main()
