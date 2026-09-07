//! Minimal POSIX.1e access-ACL support, written directly as the `system.posix_acl_access` extended attribute.
//!
//! Why this exists (WP3.1, exposed by the D-12 eight-session profile): a read-write workspace grant used to be implemented by
//! chowning the shared workspace directory to the session's primary GID and setting mode 2770. That makes concurrent sessions on the
//! SAME workspace mutually exclusive — the last launch wins the group and the other seven cannot write at all. It presented as a
//! capacity limit ("only 2–3 of 8 sessions get admitted") but it is a correctness defect, and it contradicts
//! execution-identity-lifecycle §7, which prescribes exactly the mechanism implemented here: a per-session ACL entry naming the
//! allocated group, which `agentbound-lifecycle` MUST remove during reclamation before the identity may enter quarantine.
//!
//! The on-disk format is a 4-byte little-endian version header (2) followed by 8-byte entries `{u16 tag, u16 perm, u32 id}`. The
//! three base entries (`USER_OBJ`, `GROUP_OBJ`, `OTHER`) are always present; a `MASK` entry is mandatory once any named entry exists.
//! Doing this with `setxattr` avoids adding a C library dependency to a crate that counts toward the R-CON-8 direct-SLOC bound.

const XATTR: &[u8] = b"system.posix_acl_access\0";
const VERSION: u32 = 2;
const TAG_USER_OBJ: u16 = 0x01;
const TAG_USER: u16 = 0x02;
const TAG_GROUP_OBJ: u16 = 0x04;
const TAG_GROUP: u16 = 0x08;
const TAG_MASK: u16 = 0x10;
const TAG_OTHER: u16 = 0x20;
const UNDEFINED_ID: u32 = u32::MAX;

/// `rwx`, the permission a read-write workspace grant carries.
pub const PERM_RWX: u16 = 7;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Entry {
    pub tag: u16,
    pub perm: u16,
    pub id: u32,
}

fn parse(buf: &[u8]) -> Option<Vec<Entry>> {
    if buf.len() < 4 || u32::from_le_bytes(buf[0..4].try_into().ok()?) != VERSION || (buf.len() - 4) % 8 != 0 {
        return None;
    }
    Some(buf[4..].chunks_exact(8).map(|c| Entry {
        tag: u16::from_le_bytes([c[0], c[1]]),
        perm: u16::from_le_bytes([c[2], c[3]]),
        id: u32::from_le_bytes([c[4], c[5], c[6], c[7]]),
    }).collect())
}

fn serialize(es: &[Entry]) -> Vec<u8> {
    let mut v = VERSION.to_le_bytes().to_vec();
    for e in es {
        v.extend_from_slice(&e.tag.to_le_bytes());
        v.extend_from_slice(&e.perm.to_le_bytes());
        v.extend_from_slice(&e.id.to_le_bytes());
    }
    v
}

fn get(path: &str) -> Option<Vec<Entry>> {
    let c = std::ffi::CString::new(path).ok()?;
    let mut buf = [0u8; 1024];
    let n = unsafe { libc::getxattr(c.as_ptr(), XATTR.as_ptr() as *const libc::c_char, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
    if n <= 0 { return None; }
    parse(&buf[..n as usize])
}

/// The ACL implied by the file mode, used when a path carries no ACL yet.
fn from_mode(path: &str) -> Option<Vec<Entry>> {
    let md = std::fs::metadata(path).ok()?;
    let m = std::os::unix::fs::MetadataExt::mode(&md) as u16;
    Some(vec![
        Entry { tag: TAG_USER_OBJ, perm: (m >> 6) & 7, id: UNDEFINED_ID },
        Entry { tag: TAG_GROUP_OBJ, perm: (m >> 3) & 7, id: UNDEFINED_ID },
        Entry { tag: TAG_OTHER, perm: m & 7, id: UNDEFINED_ID },
    ])
}

fn ord(t: u16) -> u8 {
    match t { TAG_USER_OBJ => 0, TAG_USER => 1, TAG_GROUP_OBJ => 2, TAG_GROUP => 3, TAG_MASK => 4, _ => 5 }
}

fn put(path: &str, es: &[Entry]) -> Result<(), i32> {
    let mut es = es.to_vec();
    es.sort_by_key(|e| (ord(e.tag), e.id));
    // A MASK entry is mandatory once a named user/group entry exists, and it must not mask off the permissions just granted.
    if es.iter().any(|e| e.tag == TAG_USER || e.tag == TAG_GROUP) {
        let need = es.iter().filter(|e| e.tag == TAG_USER || e.tag == TAG_GROUP || e.tag == TAG_GROUP_OBJ).fold(0, |a, e| a | e.perm);
        match es.iter_mut().find(|e| e.tag == TAG_MASK) {
            Some(m) => m.perm |= need,
            None => es.push(Entry { tag: TAG_MASK, perm: need, id: UNDEFINED_ID }),
        }
        es.sort_by_key(|e| (ord(e.tag), e.id));
    }
    let buf = serialize(&es);
    let c = std::ffi::CString::new(path).map_err(|_| libc::EINVAL)?;
    let r = unsafe { libc::setxattr(c.as_ptr(), XATTR.as_ptr() as *const libc::c_char, buf.as_ptr() as *const libc::c_void, buf.len(), 0) };
    if r == 0 { Ok(()) } else { Err(unsafe { *libc::__errno_location() }) }
}

/// Grant `gid` `perm` on `path` through a named-group ACL entry, leaving ownership and every other entry untouched.
/// Idempotent: re-granting the same gid updates that one entry rather than adding a second.
pub fn grant_group(path: &str, gid: u32, perm: u16) -> Result<(), i32> {
    let mut es = get(path).or_else(|| from_mode(path)).ok_or(libc::ENOENT)?;
    match es.iter_mut().find(|e| e.tag == TAG_GROUP && e.id == gid) {
        Some(e) => e.perm |= perm,
        None => es.push(Entry { tag: TAG_GROUP, perm, id: gid }),
    }
    put(path, &es)
}

/// Remove every named entry for `gid` (execution-identity-lifecycle §7: reclamation MUST remove it before quarantine).
/// Returns the number of entries removed, so lifecycle can report `acl_entries_removed` truthfully instead of a hard-coded zero.
pub fn revoke_group(path: &str, gid: u32) -> Result<usize, i32> {
    let Some(es) = get(path) else { return Ok(0) };
    let before = es.len();
    let kept: Vec<Entry> = es.into_iter().filter(|e| !(e.tag == TAG_GROUP && e.id == gid)).collect();
    let removed = before - kept.len();
    if removed == 0 { return Ok(0); }
    // dropping the last named entry also drops the now-meaningless MASK
    let kept: Vec<Entry> = if kept.iter().any(|e| e.tag == TAG_USER || e.tag == TAG_GROUP) {
        kept
    } else {
        kept.into_iter().filter(|e| e.tag != TAG_MASK).collect()
    };
    put(path, &kept)?;
    Ok(removed)
}

/// Named group entries present on `path`, for the reclamation scan's verification that removal actually happened.
pub fn named_groups(path: &str) -> Vec<u32> {
    get(path).unwrap_or_default().into_iter().filter(|e| e.tag == TAG_GROUP).map(|e| e.id).collect()
}

/// Revoke `gid` from `root` and every directory beneath it, to `max_depth` levels.
///
/// A grant is placed on the mount *source* directory (`…/workspaces/eng`), while lifecycle knows only its manifest-registered roots
/// (`/var/lib/agentbound`), so verifying removal "within all manifest-registered paths" as execution-identity-lifecycle §4.1 requires
/// means descending. Only directories are visited: a named-group entry is only ever placed on a projected directory, and bounding the
/// depth keeps a session's own deep tree from turning reclamation into an unbounded walk. Returns `(entries_removed, failures)`;
/// any failure MUST hold the identity in `reclaiming`.
pub fn revoke_group_tree(root: &str, gid: u32, max_depth: u32) -> (usize, Vec<String>) {
    let mut removed = 0usize;
    let mut failures = Vec::new();
    let mut queue = vec![(root.to_string(), 0u32)];
    while let Some((p, depth)) = queue.pop() {
        match revoke_group(&p, gid) {
            Ok(n) => removed += n,
            Err(e) => failures.push(format!("{p}: errno={e}")),
        }
        if named_groups(&p).contains(&gid) { failures.push(format!("{p}: entry for gid {gid} still present")); }
        if depth >= max_depth { continue; }
        if let Ok(rd) = std::fs::read_dir(&p) {
            for e in rd.flatten() {
                if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    if let Some(s) = e.path().to_str() { queue.push((s.to_string(), depth + 1)); }
                }
            }
        }
    }
    (removed, failures)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two concurrent sessions must both hold a write grant on one shared workspace — the property the chown approach could not have.
    #[test]
    fn two_named_groups_coexist_and_revoke_independently() {
        let d = std::env::temp_dir().join(format!("ab-acl-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let p = d.to_str().unwrap();
        grant_group(p, 200001, PERM_RWX).unwrap();
        grant_group(p, 200002, PERM_RWX).unwrap();
        let mut g = named_groups(p); g.sort();
        assert_eq!(g, vec![200001, 200002]);
        assert_eq!(revoke_group(p, 200001).unwrap(), 1);
        assert_eq!(named_groups(p), vec![200002]);
        assert_eq!(revoke_group(p, 200002).unwrap(), 1);
        assert!(named_groups(p).is_empty());
        // revoking a gid that holds no entry is not an error and removes nothing
        assert_eq!(revoke_group(p, 200003).unwrap(), 0);
        std::fs::remove_dir_all(&d).ok();
    }
}
