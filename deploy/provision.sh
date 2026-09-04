#!/bin/sh
# Provision a Debian 13 host for the 1A reference deployment. Idempotent. Run as root from the repo root.
set -eu
getent group agentbound >/dev/null || groupadd agentbound
for u in agentbound-policy agentbound-audit; do id "$u" >/dev/null 2>&1 || useradd --system --no-create-home --shell /usr/sbin/nologin -g agentbound "$u"; done
usermod -aG agentbound agentbound-policy 2>/dev/null || true
# operator/initiator accounts named in the reference catalogue
i=1001; for n in alice bob carol cron; do id $n >/dev/null 2>&1 || useradd -u $i -m -G agentbound $n; i=$((i+1)); done
install -d -m 0755 /etc/agentbound
install -d -m 1770 -o root -g agentbound /run/agentbound
install -d -m 0750 -o root -g agentbound /run/agentbound/leases
printf 'd /run/agentbound 1770 root agentbound -\nd /run/agentbound/leases 0750 root agentbound -\n' > /etc/tmpfiles.d/agentbound.conf
install -d -m 0750 /var/lib/agentbound /var/lib/agentbound/sessions /var/lib/agentbound/images
install -d -m 2750 -o agentbound-policy -g agentbound /var/lib/agentbound/spool
install -d -m 0750 -o agentbound-audit -g agentbound /var/lib/agentbound/audit
install -d -m 0755 /var/lib/agentbound/workspaces; install -d -m 0770 -o root -g root /var/lib/agentbound/workspaces/finance /var/lib/agentbound/workspaces/eng
touch /var/lib/agentbound/audit-lifecycle.jsonl /var/lib/agentbound/audit-launch.jsonl /var/lib/agentbound/audit-policy.jsonl /var/lib/agentbound/policy.jsonl
chown agentbound-policy:agentbound /var/lib/agentbound/audit-policy.jsonl /var/lib/agentbound/policy.jsonl
chmod 0750 /var/lib/agentbound; chgrp agentbound /var/lib/agentbound
install -m 0755 target/release/agentbound-lifecycle target/release/agentbound-policy target/release/agentbound-audit target/release/agentbound-launch target/release/agentbound /usr/local/bin/
install -m 0644 deploy/catalogue.json /etc/agentbound/catalogue.json
# keys and keyring (once)
if [ ! -f /etc/agentbound/keyring.json ]; then
  p=$(agentbound-policy keygen /etc/agentbound/policy.key key:policy-ed25519-01 policy); chown agentbound-policy /etc/agentbound/policy.key
  l=$(agentbound-policy keygen /etc/agentbound/launch.key key:launch-ed25519-01 launch)
  printf '[%s,%s]\n' "$p" "$l" > /etc/agentbound/keyring.json; chmod 0644 /etc/agentbound/keyring.json
fi
# runtime image: busybox-static rootfs
img=/var/lib/agentbound/images/rootfs
if [ ! -x $img/bin/sh ]; then
  install -d $img/bin $img/usr/bin $img/lib $img/lib64 $img/sbin $img/image
  cp /bin/busybox $img/bin/busybox
  for a in sh ls cat echo sleep id ps mount touch cp rm mkdir ln env true false kill sync dd head tail grep readlink stat uname hostname; do ln -sf busybox $img/bin/$a; done
  printf '#!/bin/sh\nwhile :; do sleep 1; done\n' > $img/loop.sh; chmod 0755 $img/loop.sh
fi
# CLI users may invoke the constructor as root, nothing else
printf '%%agentbound ALL=(root) NOPASSWD: /usr/local/bin/agentbound-launch\n' > /etc/sudoers.d/agentbound; chmod 0440 /etc/sudoers.d/agentbound
puid=$(id -u agentbound-policy)
for u in lifecycle policy audit; do sed "s/POLICY_UID/$puid/" deploy/units/agentbound-$u.service > /etc/systemd/system/agentbound-$u.service; done
systemctl daemon-reload
systemctl enable --now agentbound-audit agentbound-lifecycle agentbound-policy >/dev/null 2>&1 || true
systemctl reset-failed agentbound-audit agentbound-lifecycle agentbound-policy 2>/dev/null || true
systemctl restart agentbound-audit agentbound-lifecycle agentbound-policy
sleep 0.5; systemctl --no-pager --plain is-active agentbound-audit agentbound-lifecycle agentbound-policy; ls -la /run/agentbound/
