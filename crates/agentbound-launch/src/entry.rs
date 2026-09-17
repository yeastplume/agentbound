//! Security boundary for the installed, non-setuid launcher. Keep all argument
//! and environment validation ahead of configuration reads and output creation.
use std::collections::BTreeMap;
use std::ffi::OsString;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Invocation {
    Admin,
    Sudo,
}

/// SUDO_UID is NOT an authentication token. Ordinary users can invent it.
/// Only kernel real/effective/saved root credentials reach either mode; an
/// accidental setuid installation is rejected. Any sudo marker restricts the
/// invocation, even an empty, non-Unicode, malformed or zero SUDO_UID.
///
/// This distinction relies on deploy/provision.sh: mode 0755 (no file caps),
/// root-owned installation, NOSETENV/env_reset, and a sudoers argument regex
/// permitting ONLY --authorization <single-component-id> or --provenance.
/// Thus sudo cannot request admin options even if provenance is lost. Do not
/// replace that policy with an unrestricted sudo rule or a setuid installation.
pub fn invocation(uids: [u32; 3], env: &[(OsString, OsString)]) -> Result<Invocation, String> {
    if uids != [0, 0, 0] {
        return Err("launch requires real, effective and saved root UIDs; setuid execution is unsupported".into());
    }
    Ok(if env.iter().any(|(k, _)| k.as_encoded_bytes().starts_with(b"SUDO_")) {
        Invocation::Sudo
    } else {
        Invocation::Admin
    })
}

pub fn kernel_uids() -> Result<[u32; 3], String> {
    let (mut real, mut effective, mut saved) = (0, 0, 0);
    if unsafe { libc::getresuid(&mut real, &mut effective, &mut saved) } != 0 {
        return Err("cannot establish launcher credentials".into());
    }
    Ok([real, effective, saved])
}

pub const OPTIONS: &[(&str, &str)] = &[
    ("--keyring", "/etc/agentbound/keyring.json"),
    ("--key", "/etc/agentbound/launch.key"),
    ("--key-id", "key:launch-ed25519-01"),
    ("--catalogue", "/etc/agentbound/catalogue.json"),
    ("--spool", "/var/lib/agentbound/spool"),
    ("--lease-dir", "/run/agentbound/leases"),
    ("--session-root", "/var/lib/agentbound/sessions"),
    ("--image-base", "/var/lib/agentbound/images"),
    ("--lifecycle-socket", "/run/agentbound/lifecycle.sock"),
    ("--gateway-socket", "/run/agentbound/gateway.sock"),
    ("--audit-spool", "/var/lib/agentbound/audit-launch.jsonl"),
    ("--policy-uid", ""),
    ("--fault", ""),
];
const FAULTS: &[&str] = &[
    "mount-private", "mount-symlink", "pivot-root", "proc-mount", "fd-leak",
    "pre-commit-crash", "post-commit-crash", "pre-activate-crash", "barrier-hold",
];

#[derive(Debug)]
pub struct LaunchArgs {
    pub authorization: String,
    overrides: BTreeMap<String, String>,
}

impl LaunchArgs {
    /// No positional arguments, aliases, equals syntax, duplicates, unknown
    /// options or implicit values. Even admin inputs must be well-formed.
    pub fn parse(args: Vec<OsString>, mode: Invocation) -> Result<Self, String> {
        let args: Vec<String> = args.into_iter().map(|s| s.into_string()
            .map_err(|_| "arguments must be UTF-8".to_string())).collect::<Result<_, _>>()?;
        let mut authorization = None;
        let mut overrides = BTreeMap::new();
        let mut it = args.into_iter();
        while let Some(key) = it.next() {
            if key != "--authorization" && !OPTIONS.iter().any(|(k, _)| *k == key) {
                return Err(format!("unknown argument: {key}"));
            }
            if key != "--authorization" && mode != Invocation::Admin {
                return Err(format!("{key} is restricted to direct root administration"));
            }
            let value = it.next().ok_or_else(|| format!("missing value for {key}"))?;
            if value.is_empty() || value.starts_with('-') || value.chars().any(char::is_control) {
                return Err(format!("invalid value for {key}"));
            }
            if key == "--authorization" {
                if authorization.is_some() { return Err("duplicate --authorization".into()); }
                if !valid_authorization(&value) { return Err("invalid authorization identifier (expected a bounded single filename component)".into()); }
                authorization = Some(value);
            } else {
                if key == "--policy-uid" { parse_uid(&value)?; }
                if key == "--fault" && !FAULTS.contains(&value.as_str()) { return Err("unknown fault name".into()); }
                if key == "--key-id" && !ab_common::schema::is_catalogue_id(&value) { return Err("invalid key identifier".into()); }
                if overrides.insert(key.clone(), value).is_some() { return Err(format!("duplicate {key}")); }
            }
        }
        Ok(Self { authorization: authorization.ok_or("--authorization required")?, overrides })
    }

    pub fn get(&self, key: &str) -> &str {
        self.overrides.get(key).map(String::as_str)
            .unwrap_or_else(|| OPTIONS.iter().find(|(k, _)| *k == key).expect("known option").1)
    }

    pub fn policy_uid(&self) -> Result<u32, String> {
        if let Some(uid) = self.overrides.get("--policy-uid") { return parse_uid(uid); }
        let passwd = std::fs::read_to_string("/etc/passwd").map_err(|_| "cannot read policy account from /etc/passwd")?;
        policy_uid_from_passwd(&passwd)
    }
}

/// Schema catalogue IDs also allow '/', which is NOT safe when interpolated
/// into spool/lease paths. Preserve both schema ID forms, minus path syntax.
fn valid_authorization(id: &str) -> bool {
    !id.contains('/') && !id.contains('\\')
        && (ab_common::schema::is_opaque_local(id) || ab_common::schema::is_catalogue_id(id))
}

fn parse_uid(s: &str) -> Result<u32, String> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) { return Err("invalid policy UID".into()); }
    s.parse::<u32>().ok().filter(|u| *u != u32::MAX).ok_or_else(|| "invalid policy UID".into())
}

fn policy_uid_from_passwd(passwd: &str) -> Result<u32, String> {
    let mut entries = passwd.lines().filter(|l| l.split(':').next() == Some("agentbound-policy"));
    let fields: Vec<_> = entries.next().ok_or("agentbound-policy account missing")?.split(':').collect();
    if fields.len() != 7 || entries.next().is_some() { return Err("malformed or duplicate agentbound-policy account".into()); }
    let uid = parse_uid(fields[2])?;
    if uid == 0 { return Err("agentbound-policy must not be root".into()); }
    Ok(uid)
}

/// Reject explicit environment configuration before doing any file I/O. The
/// restricted constructor then inherits only this fixed environment: protects
/// audit forwarding, PATH-resolved busctl, D-Bus and catalogue env imports.
pub fn launch_environment(mode: Invocation, env: &[(OsString, OsString)]) -> Result<Vec<(OsString, OsString)>, String> {
    if mode == Invocation::Admin { return Ok(env.to_vec()); }
    if env.iter().any(|(k, _)| k.as_encoded_bytes().starts_with(b"AGENTBOUND_")) {
        return Err("AGENTBOUND_* environment overrides are restricted to direct root administration".into());
    }
    Ok(vec![("PATH".into(), "/usr/sbin:/usr/bin:/sbin:/bin".into()), ("LANG".into(), "C".into())])
}

/// Called once in the single-threaded entrypoint, before constructing any sink
/// or spawning children. Tests exercise the pure environment planner above.
pub fn install_environment(env: Vec<(OsString, OsString)>) {
    for (key, _) in std::env::vars_os() { std::env::remove_var(key); }
    for (key, value) in env { std::env::set_var(key, value); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::ffi::OsStringExt;

    fn args(a: &[&str]) -> Vec<OsString> { a.iter().map(|s| OsString::from(*s)).collect() }
    fn env(a: &[(&str, &str)]) -> Vec<(OsString, OsString)> { a.iter().map(|(k, v)| ((*k).into(), (*v).into())).collect() }

    #[test]
    fn sudo_markers_never_grant_admin() {
        for value in ["1001", "0", "", "garbage", "-1", "4294967296"] {
            assert_eq!(invocation([0, 0, 0], &env(&[("SUDO_UID", value)])).unwrap(), Invocation::Sudo);
            assert!(invocation([1001, 1001, 1001], &env(&[("SUDO_UID", value)])).is_err());
        }
        for key in ["SUDO_USER", "SUDO_COMMAND", "SUDO_GID", "SUDO_FAKE"] {
            assert_eq!(invocation([0, 0, 0], &env(&[(key, "")])).unwrap(), Invocation::Sudo);
        }
        let non_unicode = vec![("SUDO_UID".into(), OsString::from_vec(vec![0xff]))];
        assert_eq!(invocation([0, 0, 0], &non_unicode).unwrap(), Invocation::Sudo);
        assert_eq!(invocation([0, 0, 0], &[]).unwrap(), Invocation::Admin);
    }

    #[test]
    fn kernel_credentials_reject_setuid_and_saved_uid_edges() {
        for uids in [[1001, 0, 0], [0, 1001, 0], [0, 0, 1001], [1001, 1001, 0]] {
            assert!(invocation(uids, &[]).is_err());
            assert!(invocation(uids, &env(&[("SUDO_UID", "0")])).is_err());
        }
    }

    #[test]
    fn sudo_allows_only_authorization_and_uses_fixed_defaults() {
        let parsed = LaunchArgs::parse(args(&["--authorization", "launchrec:eng-000001"]), Invocation::Sudo).unwrap();
        assert_eq!(parsed.authorization, "launchrec:eng-000001");
        for (key, default) in OPTIONS {
            assert_eq!(parsed.get(key), *default);
            // Even an override that repeats the default is forbidden.
            assert!(LaunchArgs::parse(args(&["--authorization", "a", key, default]), Invocation::Sudo).is_err(), "{key}");
            assert!(LaunchArgs::parse(args(&[key, default, "--authorization", "a"]), Invocation::Sudo).is_err(), "{key}");
        }
    }

    #[test]
    fn parser_rejects_unknown_duplicate_missing_and_malformed_arguments() {
        for bad in [vec![], vec!["--authorization"], vec!["--authorization", ""],
            vec!["--authorization=a"], vec!["a"], vec!["--authorization", "a", "extra"],
            vec!["--authorization", "a", "--unknown", "x"],
            vec!["--authorization", "a", "--authorization", "b"],
            vec!["--authorization", "a", "--key", "x", "--key", "y"],
            vec!["--authorization", "a", "--key"], vec!["--authorization", "a", "--key", "--fault"],
            vec!["--authorization", "a", "--fault", "not-a-fault"],
            vec!["--authorization", "a", "--key", "x\n"],
            vec!["--authorization", "a", "--provenance"]] {
            assert!(LaunchArgs::parse(args(&bad), Invocation::Admin).is_err(), "{bad:?}");
        }
        for (key, _) in OPTIONS {
            let value = match *key { "--fault" => "fd-leak", "--policy-uid" => "123", "--key-id" => "key:test", _ => "/tmp/test" };
            assert!(LaunchArgs::parse(args(&["--authorization", "a", key, value, key, value]), Invocation::Admin).is_err());
        }
        assert!(LaunchArgs::parse(vec![OsString::from_vec(vec![0xff])], Invocation::Admin).is_err());
    }

    #[test]
    fn authorization_cannot_escape_spool_or_lease_directory() {
        for bad in ["", ".", "..", "../a", "/tmp/a", "a/../../etc/shadow", "launchrec:a/../../x", "a\\b", "a\0b", "a b", "a\nb", "--key", "a%2fb", "é"] {
            assert!(!valid_authorization(bad), "{bad:?}");
            for mode in [Invocation::Admin, Invocation::Sudo] {
                assert!(LaunchArgs::parse(args(&["--authorization", bad]), mode).is_err());
            }
        }
        assert!(!valid_authorization(&"a".repeat(256)));
        for good in ["launchrec:eng-000001", "a", "A_1.2:3-4", "launchrec:a..b"] { assert!(valid_authorization(good)); }
    }

    #[test]
    fn root_can_explicitly_use_test_configuration_and_known_faults() {
        for (key, _) in OPTIONS {
            let value = match *key { "--fault" => "mount-symlink", "--policy-uid" => "0", "--key-id" => "key:test", _ => "/tmp/test" };
            let parsed = LaunchArgs::parse(args(&[key, value, "--authorization", "test:a"]), Invocation::Admin).unwrap();
            assert_eq!(parsed.get(key), value);
        }
        for fault in FAULTS {
            assert!(LaunchArgs::parse(args(&["--authorization", "a", "--fault", fault]), Invocation::Admin).is_ok());
        }
        for bad in ["-1", "+1", " 1", "1 ", "x", "4294967295", "4294967296"] {
            assert!(LaunchArgs::parse(args(&["--authorization", "a", "--policy-uid", bad]), Invocation::Admin).is_err());
        }
    }

    #[test]
    fn missing_or_invalid_policy_account_never_falls_back_to_root() {
        for bad in ["", "root:x:0:0::/:/bin/sh\n", "agentbound-policy:x:0:1::/:/bin/false",
            "agentbound-policy:x:x:1::/:/bin/false", "agentbound-policy:x:123",
            "agentbound-policy:x:123:1::/:/bin/false\nagentbound-policy:x:456:1::/:/bin/false"] {
            assert!(policy_uid_from_passwd(bad).is_err());
        }
        assert_eq!(policy_uid_from_passwd("root:x:0:0::/:/bin/sh\nagentbound-policy:x:123:1::/:/bin/false\n").unwrap(), 123);
    }

    #[test]
    fn audit_override_is_rejected_and_other_inherited_environment_is_scrubbed() {
        for key in ["AGENTBOUND_AUDIT_SOCKET", "AGENTBOUND_POLICY_SOCKET", "AGENTBOUND_FAULT"] {
            let supplied = env(&[(key, "/tmp/attacker.sock")]);
            assert!(launch_environment(Invocation::Sudo, &supplied).is_err());
            assert_eq!(launch_environment(Invocation::Admin, &supplied).unwrap(), supplied);
        }
        let supplied = env(&[("PATH", "/tmp/attacker"), ("DBUS_SYSTEM_BUS_ADDRESS", "unix:path=/tmp/fake"),
            ("LD_PRELOAD", "/tmp/evil.so"), ("SUDO_UID", "0"), ("HOME", "/tmp"), ("OTHER", "secret")]);
        assert_eq!(launch_environment(Invocation::Sudo, &supplied).unwrap(),
            env(&[("PATH", "/usr/sbin:/usr/bin:/sbin:/bin"), ("LANG", "C")]));
    }
}
