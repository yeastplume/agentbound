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
tar -C "$ROOT" --exclude target -cf - Cargo.toml $( [ -f "$ROOT/Cargo.lock" ] && echo Cargo.lock ) crates | $SSH $VM 'tar -C /root/wp2 -xf -' 2>/dev/null
$SSH $VM "cd /root/wp2 && export PATH=\$HOME/.cargo/bin:\$PATH && cargo $* --workspace 2>&1" 2>/dev/null || status=$?
$SSH $VM 'cat /root/wp2/Cargo.lock' 2>/dev/null > "$ROOT/Cargo.lock"
exit ${status:-0}
