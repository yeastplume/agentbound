//! In-memory session table: held pidfds and cgroup directory descriptors,
//! current state, observation sequence. Durable truth is in `store`; this is
//! the live-evidence layer (component interfaces §8.2 precedence level 3).

use ab_common::audit::Correlation;
use std::collections::BTreeMap;
use std::os::fd::OwnedFd;

pub struct Session {
    pub lrd: String, pub allocation_id: String, pub authorization_id: String, pub scope_id: String, pub pidns_id: String, pub session_id: String, pub trace_id: String,
    pub uid: u32, pub gid: u32, pub domain_id: String, pub state: String, pub reason: Option<String>, pub observation_seq: i64,
    pub init_pid: i32, pub init_pidfd: Option<OwnedFd>, pub cgroup_dir: Option<OwnedFd>, pub deadline_mono_ns: Option<i64>, pub session_dir: Option<String>,
}

#[derive(Default)]
pub struct Sessions { pub by_lrd: BTreeMap<String, Session>, pub pending_reclaim: Vec<(String, Option<String>)> }

impl Sessions {
    pub fn bind(&mut self, aid: &str, lrd: &str, az: &str, scope_id: &str, session_id: &str, trace_id: &str, uid: u32, gid: u32, domain_id: &str) {
        self.by_lrd.insert(lrd.into(), Session { lrd: lrd.into(), allocation_id: aid.into(), authorization_id: az.into(), scope_id: scope_id.into(), pidns_id: String::new(), session_id: session_id.into(), trace_id: trace_id.into(),
            uid, gid, domain_id: domain_id.into(), state: "constructing".into(), reason: None, observation_seq: 1, init_pid: 0, init_pidfd: None, cgroup_dir: None, deadline_mono_ns: None, session_dir: None });
    }
    pub fn register(&mut self, lrd: &str, pidfd: OwnedFd, cgroup: OwnedFd, init_pid: i32, scope_id: &str, pidns_id: &str) -> Result<(), &'static str> {
        let s = self.by_lrd.get_mut(lrd).ok_or("unknown_record")?;
        if s.init_pidfd.is_some() { return Err("already_registered"); }
        if s.scope_id != scope_id { return Err("scope_mismatch"); }
        s.init_pidfd = Some(pidfd); s.cgroup_dir = Some(cgroup); s.init_pid = init_pid; s.pidns_id = pidns_id.into(); s.observation_seq += 1; Ok(())
    }
    pub fn get(&self, lrd: &str) -> Option<&Session> { self.by_lrd.get(lrd) }
    pub fn get_mut(&mut self, lrd: &str) -> Option<&mut Session> { self.by_lrd.get_mut(lrd) }
    pub fn by_authorization(&self, az: &str) -> Option<String> { self.by_lrd.values().find(|s| s.authorization_id == az).map(|s| s.lrd.clone()) }
    pub fn all(&self) -> Vec<&Session> { self.by_lrd.values().collect() }
    pub fn set_state(&mut self, lrd: &str, state: &str, reason: Option<&str>) {
        if let Some(s) = self.by_lrd.get_mut(lrd) { s.state = state.into(); if reason.is_some() { s.reason = reason.map(str::to_string); } s.observation_seq += 1; }
    }
    pub fn reclaim_later(&mut self, aid: &str, lrd: Option<&str>) { self.pending_reclaim.push((aid.into(), lrd.map(str::to_string))); }
    pub fn correlation(&self, lrd: &str) -> Correlation {
        match self.by_lrd.get(lrd) { Some(s) => Correlation { authorization_id: Some(s.authorization_id.clone()), launch_record_digest: Some(s.lrd.clone()), allocation_id: Some(s.allocation_id.clone()), session_id: Some(s.session_id.clone()), trace_id: Some(s.trace_id.clone()), execution_uid: Some(s.uid) }, None => Correlation::default() }
    }
}
