#!/bin/sh
# Development VM only. Run as root from a built candidate checkout after
# preserving stopped-service state. Does not reset catalogues, keys or evidence.
set -eu
[ "$(id -u)" = 0 ] || { echo 'root required' >&2; exit 1; }
for service in policy lifecycle gateway audit; do
    if systemctl is-active --quiet "agentbound-$service"; then
        echo "stop agentbound-$service and back up state first" >&2; exit 1
    fi
done
[ -f /root/agentbound-repair-backup-20260917/state.tgz ] || { echo 'repair backup missing' >&2; exit 1; }
tmp=$(mktemp /etc/sudoers.d/.agentbound.XXXXXX)
trap 'rm -f "$tmp"' EXIT HUP INT TERM
printf '%s\n' \
 'Defaults!/usr/local/bin/agentbound-launch env_reset, !setenv, secure_path="/usr/sbin:/usr/bin:/sbin:/bin"' \
 '%agentbound ALL=(root) NOPASSWD: NOSETENV: /usr/local/bin/agentbound-launch ^--authorization [A-Za-z][A-Za-z0-9._:-]{0,160}$' \
 '%agentbound ALL=(root) NOPASSWD: NOSETENV: /usr/local/bin/agentbound-launch ^--provenance$' > "$tmp"
chmod 0440 "$tmp"; chown root:root "$tmp"
visudo -cf "$tmp"
# Replace restrictive sudo policy before introducing the new binaries.
mv -f "$tmp" /etc/sudoers.d/agentbound
trap - EXIT HUP INT TERM
for binary in agentbound agentbound-launch agentbound-lifecycle agentbound-policy agentbound-audit agentbound-gateway ab-conformance ab-gwclient; do
    install -o root -g root -m 0755 "target/release/$binary" "/usr/local/bin/$binary"
    sha256sum "/usr/local/bin/$binary"
    # ab-gwclient does not implement the component provenance option.
    if [ "$binary" != ab-gwclient ]; then "/usr/local/bin/$binary" --provenance; fi
done
install -m 0755 target/release/ab-gwclient /var/lib/agentbound/images/rootfs/bin/ab-gwclient
systemctl start agentbound-audit agentbound-lifecycle agentbound-policy agentbound-gateway
systemctl --no-pager --plain is-active agentbound-audit agentbound-lifecycle agentbound-policy agentbound-gateway
