#!/bin/sh
# Sync the workspace to VM 110 and run a cargo command there.
#   crates/build.sh test            -> cargo test --workspace
#   crates/build.sh build --release -> cargo build --workspace --release
# The Cargo.lock is copied back so it can be committed.
set -eu
VM=root@10.20.44.12
SSH="ssh -i /root/.ssh/id_ed25519_agentbound_dev -o UserKnownHostsFile=/tmp/kh -o StrictHostKeyChecking=accept-new"
ROOT=$(cd "$(dirname "$0")/.." && pwd)
$SSH $VM 'mkdir -p /root/wp2' 2>/dev/null
# Source provenance travels with the sources (independent WP3.1 validation, finding 4). Written from THIS checkout on every sync, so
# it cannot go stale; `dirty` covers tracked modifications and untracked files under crates/ and deploy/, because a binary built from
# an uncommitted tree cannot be attributed to any reviewed commit.
COMMIT=$(git -C "$ROOT" rev-parse HEAD 2>/dev/null || echo unknown)
if [ -n "$(git -C "$ROOT" status --porcelain -- crates deploy Cargo.toml Cargo.lock 2>/dev/null)" ]; then DIRTY=true; else DIRTY=false; fi
TREE=$(git -C "$ROOT" rev-parse "HEAD:crates" 2>/dev/null || echo unknown)
printf 'commit=%s\ndirty=%s\ntree=%s\n' "$COMMIT" "$DIRTY" "$TREE" > "$ROOT/crates/SOURCE-PROVENANCE"
tar -C "$ROOT" --exclude target -cf - Cargo.toml $( [ -f "$ROOT/Cargo.lock" ] && echo Cargo.lock ) crates | $SSH $VM 'tar -C /root/wp2 -xf -' 2>/dev/null
# the stale hand-written marker is removed so nothing can ever read it again
$SSH $VM 'rm -f /root/wp2/COMMIT' 2>/dev/null
$SSH $VM "cd /root/wp2 && export PATH=\$HOME/.cargo/bin:\$PATH && cargo $* --workspace 2>&1" 2>/dev/null || status=$?
$SSH $VM 'cat /root/wp2/Cargo.lock' 2>/dev/null > "$ROOT/Cargo.lock"
exit ${status:-0}
