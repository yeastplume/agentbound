#!/usr/bin/env bash
# Sync the spikes/ tree to the WP1 baseline VM, build one spike, run it, and
# capture raw output under docs/evidence/wp1/raw/<spike>.txt.
# Usage: spikes/run.sh <spike-dir-name> [args…]
set -euo pipefail
SPIKE="$1"; shift
HOST=root@10.20.44.12
SSH=(ssh -i /root/.ssh/id_ed25519_agentbound_dev -o UserKnownHostsFile=/tmp/kh -o StrictHostKeyChecking=accept-new "$HOST")
REPO="$(cd "$(dirname "$0")/.." && pwd)"
RAW="$REPO/docs/evidence/wp1/raw"; mkdir -p "$RAW"
tar -C "$REPO" -cf - spikes | "${SSH[@]}" 'rm -rf /root/wp1 && mkdir -p /root/wp1 && tar -C /root/wp1 -xf -'
{
  echo "# spike: $SPIKE"
  echo "# host: $("${SSH[@]}" 'hostname; uname -r; systemctl --version | head -1' | tr "\n" ";")"
  echo "# commit: $(git -C "$REPO" rev-parse --short HEAD)$(git -C "$REPO" diff --quiet || echo '+dirty')"
  echo "# date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# args: $*"
  echo
  if [ -x "$REPO/spikes/$SPIKE/run.sh" ]; then
    "${SSH[@]}" "cd /root/wp1/spikes/$SPIKE && ./run.sh $*" 2>&1
  else
    "${SSH[@]}" "export PATH=\$HOME/.cargo/bin:\$PATH; cd /root/wp1/spikes/$SPIKE && cargo build --release -q 2>&1 && ./target/release/$SPIKE $*" 2>&1
  fi
  echo; echo "# exit: ${PIPESTATUS[0]:-$?}"
} | tee "$RAW/$SPIKE.txt"
