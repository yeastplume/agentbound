//! Connection establishment (ADR-0002 Decision 2): SO_PEERCRED → pidfd → process instance,
//! bound to exactly one projected allocation. Start time corroborates; pidfs inode is the key.
use crate::{session::Conn, Projection};
use ab_common::wire;

pub fn establish(c: &wire::Conn, p: &Projection, existing: usize, max: usize) -> Result<Conn, (&'static str, String)> {
    if !p.admission { return Err(("admission_closed", "session not active".into())); }
    if c.peer.uid != p.uid { return Err(("uid_mismatch", format!("peer uid {} allocation uid {}", c.peer.uid, p.uid))); }
    if existing >= max { return Err(("connection_limit", format!("{existing} connections"))); }
    let (pidfd, inst) = wire::proc_instance(c.peer.pid).map_err(|e| ("pidfd_unavailable", e.to_string()))?;
    // scope check: the establishing process must live in this allocation's scope cgroup
    let suffix = p.allocation_id.rsplit(':').next().unwrap_or("");
    if !inst.cgroup.contains(&format!("agentbound-{suffix}.scope")) { return Err(("scope_mismatch", format!("cgroup {}", inst.cgroup))); }
    // one connection per process instance
    wire::set_passcred(c.fd.as_raw_fd()).map_err(|e| ("passcred", e.to_string()))?;
    let fd = unsafe { std::os::fd::OwnedFd::from_raw_fd(libc::dup(c.fd.as_raw_fd())) };
    Ok(Conn { fd, pidfd, inst, allocation_id: p.allocation_id.clone(), uid: c.peer.uid, gid: c.peer.gid, ops: 0, last_cred_pid: 0, pending: None })
}
use std::os::fd::{AsRawFd, FromRawFd};
