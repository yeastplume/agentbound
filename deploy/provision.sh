#!/bin/sh
# Provision a Debian 13 host for the 1A reference deployment. Idempotent. Run as root from the repo root.
set -eu
getent group agentbound >/dev/null || groupadd agentbound
for u in agentbound-policy agentbound-audit agentbound-gateway; do id "$u" >/dev/null 2>&1 || useradd --system --no-create-home --shell /usr/sbin/nologin -g agentbound "$u"; done
usermod -aG agentbound agentbound-policy 2>/dev/null || true
# operator/initiator accounts named in the reference catalogue
# durable storage principals: the host users that own what a session leaves in a workspace (manifest durable_ownership_projection)
for n in storage-engineering storage-finance; do id $n >/dev/null 2>&1 || useradd --system --no-create-home --shell /usr/sbin/nologin -g agentbound $n; done
i=1001; for n in alice bob carol cron; do id $n >/dev/null 2>&1 || useradd -u $i -m -G agentbound $n; i=$((i+1)); done
install -d -m 0755 /etc/agentbound
install -d -m 1770 -o root -g agentbound /run/agentbound
install -d -m 0750 -o root -g agentbound /run/agentbound/leases
printf 'd /run/agentbound 1770 root agentbound -\nd /run/agentbound/leases 0750 root agentbound -\nd /run/agentbound/gw 0770 agentbound-gateway agentbound -\n' > /etc/tmpfiles.d/agentbound.conf
install -d -m 0750 /var/lib/agentbound /var/lib/agentbound/sessions /var/lib/agentbound/images
install -d -m 2750 -o agentbound-policy -g agentbound /var/lib/agentbound/spool
install -d -m 0750 -o agentbound-audit -g agentbound /var/lib/agentbound/audit
install -d -m 0750 -o agentbound-gateway -g agentbound /var/lib/agentbound/gateway /var/lib/agentbound/gateway/quarantine
install -d -m 0770 -o agentbound-gateway -g agentbound /run/agentbound/gw
# demo upstream: a bare repository standing in for the Git host, protected branch enforced by its own hook (WP1 GS-6 composition)
install -d -m 0750 -o agentbound-gateway -g agentbound /var/lib/agentbound/git
if [ ! -d /var/lib/agentbound/git/demo.git ]; then su -s /bin/sh agentbound-gateway -c 'git init -q --bare /var/lib/agentbound/git/demo.git && cd /tmp && rm -rf seed && git init -q seed && cd seed && git -c user.name=seed -c user.email=seed@example.invalid commit -q --allow-empty -m initial && git push -q /var/lib/agentbound/git/demo.git HEAD:refs/heads/main'; fi
su -s /bin/sh agentbound-gateway -c 'git -C /var/lib/agentbound/git/demo.git config receive.advertisePushOptions true && git -C /var/lib/agentbound/git/demo.git config receive.denyNonFastForwards true'
install -m 0755 deploy/hooks/pre-receive /var/lib/agentbound/git/demo.git/hooks/pre-receive; chown agentbound-gateway:agentbound /var/lib/agentbound/git/demo.git/hooks/pre-receive
# gateway credential: readable by the gateway user only (R-GW-6 / GS-7)
[ -f /var/lib/agentbound/gateway/credential ] || head -c 32 /dev/urandom | base64 > /var/lib/agentbound/gateway/credential; chmod 0600 /var/lib/agentbound/gateway/credential; chown agentbound-gateway:agentbound /var/lib/agentbound/gateway/credential
install -d -m 0755 /var/lib/agentbound/workspaces; install -d -m 0770 -o root -g root /var/lib/agentbound/workspaces/finance /var/lib/agentbound/workspaces/eng
touch /var/lib/agentbound/audit-lifecycle.jsonl /var/lib/agentbound/audit-launch.jsonl /var/lib/agentbound/audit-policy.jsonl /var/lib/agentbound/policy.jsonl
chown agentbound-policy:agentbound /var/lib/agentbound/audit-policy.jsonl /var/lib/agentbound/policy.jsonl
chmod 0750 /var/lib/agentbound; chgrp agentbound /var/lib/agentbound
install -m 0755 target/release/agentbound-lifecycle target/release/agentbound-policy target/release/agentbound-audit target/release/agentbound-launch target/release/agentbound target/release/agentbound-gateway /usr/local/bin/
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
  printf "#!/bin/sh\nwhile :; do sleep 1; done\n" > $img/loop.sh; chmod 0755 $img/loop.sh
fi
install -m 0755 target/release/ab-conformance /usr/local/bin/ab-conformance
install -m 0755 crates/ab-conformance/probe/probe.sh $img/probe.sh
# 1B: session-side gateway client (static) and git with its shared libraries copied into the image (no network tooling)
install -m 0755 target/release/ab-gwclient $img/bin/ab-gwclient
for a in awk sed wc tr basename dirname date sort uniq find xargs test printf sha256sum; do ln -sf busybox $img/bin/$a; done
rm -f $img/.libs-done; if [ ! -f $img/.libs-done ]; then
  install -d $img/usr/lib/git-core $img/usr/share/git-core/templates $img/etc
  cp /usr/bin/git $img/usr/bin/git
  for x in git git-remote-http git-upload-pack git-receive-pack; do [ -f /usr/lib/git-core/$x ] && cp /usr/lib/git-core/$x $img/usr/lib/git-core/$x; done
  for f in $(for b in /usr/bin/git $img/usr/lib/git-core/* target/release/ab-gwclient; do ldd $b 2>/dev/null | awk '/=> \//{print $3} /^\s*\/lib64/{print $1}'; done | sort -u); do d=$img$(dirname $f); install -d $d; cp -L $f $d/; done
  cp -L /lib64/ld-linux-x86-64.so.2 $img/lib64/ 2>/dev/null || true
  printf 'root:x:0:0::/:/bin/sh\n' > $img/etc/passwd; printf 'root:x:0:\n' > $img/etc/group
  touch $img/.libs-done
fi
install -m 0755 crates/ab-conformance/probe/git-worker.sh $img/git-worker.sh
# CLI users may invoke the constructor as root, nothing else
printf '%%agentbound ALL=(root) NOPASSWD: /usr/local/bin/agentbound-launch\n' > /etc/sudoers.d/agentbound; chmod 0440 /etc/sudoers.d/agentbound
puid=$(id -u agentbound-policy)
guid=$(id -u agentbound-gateway)
for u in lifecycle policy audit gateway; do sed "s/POLICY_UID/$puid/; s/GATEWAY_UID/$guid/" deploy/units/agentbound-$u.service > /etc/systemd/system/agentbound-$u.service; done
systemctl daemon-reload
systemctl enable --now agentbound-audit agentbound-lifecycle agentbound-policy agentbound-gateway >/dev/null 2>&1 || true
systemctl reset-failed agentbound-audit agentbound-lifecycle agentbound-policy agentbound-gateway 2>/dev/null || true
systemctl restart agentbound-audit agentbound-lifecycle agentbound-policy agentbound-gateway
sleep 0.5; systemctl --no-pager --plain is-active agentbound-audit agentbound-lifecycle agentbound-policy agentbound-gateway; ls -la /run/agentbound/
