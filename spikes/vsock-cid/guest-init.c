/* WP1 spike VM-1: guest init for a Firecracker microVM. Reports its local CID,
 * connects to the host on the single permitted port, tries a second port, and
 * exits by reboot. Static; not TCB; not SLOC-counted. */
#define _GNU_SOURCE
#include <sys/socket.h>
#include <linux/vm_sockets.h>
#include <sys/ioctl.h>
#include <fcntl.h>
#include <unistd.h>
#include <stdio.h>
#include <string.h>
#include <errno.h>
#include <sys/reboot.h>
#include <sys/mount.h>
#include <sys/utsname.h>
static int say(int s, const char *m) { char b[512]; int n; if (s < 0) return -1; write(s, m, strlen(m)); n = read(s, b, 512); return n; }
int main(void) {
  mount("devtmpfs", "/dev", "devtmpfs", 0, 0); mount("proc", "/proc", "proc", 0, 0);
  struct utsname u; uname(&u);
  int f = open("/dev/vsock", O_RDONLY); unsigned cid = 0; int ic = -1; if (f >= 0) ic = ioctl(f, IOCTL_VM_SOCKETS_GET_LOCAL_CID, &cid);
  struct sockaddr_vm a; memset(&a, 0, sizeof a); a.svm_family = AF_VSOCK; a.svm_cid = VMADDR_CID_HOST; a.svm_port = 5000;
  int s = socket(AF_VSOCK, SOCK_STREAM, 0); int r = connect(s, (void *)&a, sizeof a); int e = errno;
  char buf[512];
  snprintf(buf, 512, "guest kernel=%s local_cid=%u ioctl_rc=%d connect_5000_rc=%d errno=%d\n", u.release, cid, ic, r, e); say(r == 0 ? s : -1, buf);
  int s2 = socket(AF_VSOCK, SOCK_STREAM, 0); a.svm_port = 5001; int r2 = connect(s2, (void *)&a, sizeof a); e = errno;
  snprintf(buf, 512, "second_port_5001_rc=%d errno=%d\n", r2, e); say(r == 0 ? s : -1, buf);
  /* a guest cannot choose its CID: try to bind a socket with a forged CID */
  int s3 = socket(AF_VSOCK, SOCK_STREAM, 0); struct sockaddr_vm b; memset(&b, 0, sizeof b); b.svm_family = AF_VSOCK; b.svm_cid = 999; b.svm_port = 7000; int rb = bind(s3, (void *)&b, sizeof b); e = errno;
  snprintf(buf, 512, "bind_forged_cid_999_rc=%d errno=%d\n", rb, e); say(r == 0 ? s : -1, buf);
  /* the same guest opens a second connection on the permitted port: host must see the same CID */
  int s4 = socket(AF_VSOCK, SOCK_STREAM, 0); a.svm_port = 5000; int r4 = connect(s4, (void *)&a, sizeof a);
  snprintf(buf, 512, "second_connection rc=%d\n", r4); if (r4 == 0) { write(s4, buf, strlen(buf)); read(s4, buf, 512); }
  say(r == 0 ? s : -1, "done\n");
  sync(); reboot(RB_AUTOBOOT); return 0;
}
