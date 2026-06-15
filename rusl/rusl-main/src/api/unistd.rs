//! unistd 模块对外接口

use core::ffi::{c_int, c_char, c_uint, c_void};

// ---------- 类型 ----------
pub type size_t  = usize;
pub type ssize_t = isize;
pub type uid_t   = u32;
pub type gid_t   = u32;
pub type off_t   = i64;
pub type pid_t   = i32;

#[repr(C)]
pub struct iovec {
    pub iov_base: *mut c_void,
    pub iov_len: usize,
}

// ---------- 宏常量 ----------
pub const STDIN_FILENO: c_int  = 0;
pub const STDOUT_FILENO: c_int = 1;
pub const STDERR_FILENO: c_int = 2;
pub const SEEK_SET: c_int = 0;
pub const SEEK_CUR: c_int = 1;
pub const SEEK_END: c_int = 2;
pub const F_OK: c_int = 0;
pub const R_OK: c_int = 4;
pub const W_OK: c_int = 2;
pub const X_OK: c_int = 1;

// ---------- 内部 FFI ----------
extern "C" {
    // 1. 基本 I/O
    #[link_name = "read"]
    fn musl_read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    #[link_name = "write"]
    fn musl_write(fd: c_int, buf: *const c_void, count: usize) -> isize;
    #[link_name = "close"]
    fn musl_close(fd: c_int) -> c_int;
    #[link_name = "dup"]
    fn musl_dup(fd: c_int) -> c_int;
    #[link_name = "dup2"]
    fn musl_dup2(old: c_int, new: c_int) -> c_int;
    #[link_name = "dup3"]
    fn musl_dup3(old: c_int, new: c_int, flags: c_int) -> c_int;
    #[link_name = "lseek"]
    fn musl_lseek(fd: c_int, offset: off_t, whence: c_int) -> off_t;
    #[link_name = "pipe"]
    fn musl_pipe(fd: *mut c_int) -> c_int;
    #[link_name = "pipe2"]
    fn musl_pipe2(fd: *mut c_int, flags: c_int) -> c_int;
    #[link_name = "pread"]
    fn musl_pread(fd: c_int, buf: *mut c_void, size: usize, ofs: off_t) -> isize;
    #[link_name = "pwrite"]
    fn musl_pwrite(fd: c_int, buf: *const c_void, size: usize, ofs: off_t) -> isize;
    #[link_name = "readv"]
    fn musl_readv(fd: c_int, iov: *const iovec, count: c_int) -> isize;
    #[link_name = "writev"]
    fn musl_writev(fd: c_int, iov: *const iovec, count: c_int) -> isize;
    #[link_name = "preadv"]
    fn musl_preadv(fd: c_int, iov: *const iovec, count: c_int, ofs: off_t) -> isize;
    #[link_name = "pwritev"]
    fn musl_pwritev(fd: c_int, iov: *const iovec, count: c_int, ofs: off_t) -> isize;
    #[link_name = "fsync"]
    fn musl_fsync(fd: c_int) -> c_int;
    #[link_name = "fdatasync"]
    fn musl_fdatasync(fd: c_int) -> c_int;
    #[link_name = "ftruncate"]
    fn musl_ftruncate(fd: c_int, length: off_t) -> c_int;
    #[link_name = "posix_close"]
    fn musl_posix_close(fd: c_int, flags: c_int) -> c_int;

    // 2. 文件系统操作
    #[link_name = "access"]
    fn musl_access(filename: *const c_char, amode: c_int) -> c_int;
    #[link_name = "faccessat"]
    fn musl_faccessat(fd: c_int, filename: *const c_char, amode: c_int, flag: c_int) -> c_int;
    #[link_name = "chdir"]
    fn musl_chdir(path: *const c_char) -> c_int;
    #[link_name = "fchdir"]
    fn musl_fchdir(fd: c_int) -> c_int;
    #[link_name = "getcwd"]
    fn musl_getcwd(buf: *mut c_char, size: usize) -> *mut c_char;
    #[link_name = "chown"]
    fn musl_chown(path: *const c_char, uid: uid_t, gid: gid_t) -> c_int;
    #[link_name = "fchown"]
    fn musl_fchown(fd: c_int, uid: uid_t, gid: gid_t) -> c_int;
    #[link_name = "lchown"]
    fn musl_lchown(path: *const c_char, uid: uid_t, gid: gid_t) -> c_int;
    #[link_name = "fchownat"]
    fn musl_fchownat(fd: c_int, path: *const c_char, uid: uid_t, gid: gid_t, flag: c_int) -> c_int;
    #[link_name = "link"]
    fn musl_link(existing: *const c_char, new: *const c_char) -> c_int;
    #[link_name = "linkat"]
    fn musl_linkat(fd1: c_int, existing: *const c_char, fd2: c_int, new: *const c_char, flag: c_int) -> c_int;
    #[link_name = "symlink"]
    fn musl_symlink(existing: *const c_char, new: *const c_char) -> c_int;
    #[link_name = "symlinkat"]
    fn musl_symlinkat(existing: *const c_char, fd: c_int, new: *const c_char) -> c_int;
    #[link_name = "readlink"]
    fn musl_readlink(path: *const c_char, buf: *mut c_char, bufsize: usize) -> isize;
    #[link_name = "readlinkat"]
    fn musl_readlinkat(fd: c_int, path: *const c_char, buf: *mut c_char, bufsize: usize) -> isize;
    #[link_name = "unlink"]
    fn musl_unlink(path: *const c_char) -> c_int;
    #[link_name = "unlinkat"]
    fn musl_unlinkat(fd: c_int, path: *const c_char, flag: c_int) -> c_int;
    #[link_name = "rmdir"]
    fn musl_rmdir(path: *const c_char) -> c_int;
    #[link_name = "renameat"]
    fn musl_renameat(oldfd: c_int, old: *const c_char, newfd: c_int, new: *const c_char) -> c_int;
    #[link_name = "truncate"]
    fn musl_truncate(path: *const c_char, length: off_t) -> c_int;
    #[link_name = "sync"]
    fn musl_sync();
    #[link_name = "acct"]
    fn musl_acct(filename: *const c_char) -> c_int;

    // 3. 进程控制
    #[link_name = "_exit"]
    fn musl__exit(status: c_int) -> !;
    #[link_name = "alarm"]
    fn musl_alarm(seconds: c_uint) -> c_uint;
    #[link_name = "sleep"]
    fn musl_sleep(seconds: c_uint) -> c_uint;
    #[link_name = "pause"]
    fn musl_pause() -> c_int;
    #[link_name = "getpid"]
    fn musl_getpid() -> pid_t;
    #[link_name = "getppid"]
    fn musl_getppid() -> pid_t;
    #[link_name = "getpgrp"]
    fn musl_getpgrp() -> pid_t;
    #[link_name = "getpgid"]
    fn musl_getpgid(pid: pid_t) -> pid_t;
    #[link_name = "setpgid"]
    fn musl_setpgid(pid: pid_t, pgid: pid_t) -> c_int;
    #[link_name = "setsid"]
    fn musl_setsid() -> pid_t;
    #[link_name = "getsid"]
    fn musl_getsid(pid: pid_t) -> pid_t;
    #[link_name = "setpgrp"]
    fn musl_setpgrp() -> pid_t;
    #[link_name = "nice"]
    fn musl_nice(inc: c_int) -> c_int;

    // 4. 用户标识和组
    #[link_name = "getuid"]
    fn musl_getuid() -> uid_t;
    #[link_name = "geteuid"]
    fn musl_geteuid() -> uid_t;
    #[link_name = "getgid"]
    fn musl_getgid() -> gid_t;
    #[link_name = "getegid"]
    fn musl_getegid() -> gid_t;
    #[link_name = "getgroups"]
    fn musl_getgroups(count: c_int, list: *mut gid_t) -> c_int;
    #[link_name = "setuid"]
    fn musl_setuid(uid: uid_t) -> c_int;
    #[link_name = "seteuid"]
    fn musl_seteuid(euid: uid_t) -> c_int;
    #[link_name = "setgid"]
    fn musl_setgid(gid: gid_t) -> c_int;
    #[link_name = "setegid"]
    fn musl_setegid(egid: gid_t) -> c_int;
    #[link_name = "setreuid"]
    fn musl_setreuid(ruid: uid_t, euid: uid_t) -> c_int;
    #[link_name = "setregid"]
    fn musl_setregid(rgid: gid_t, egid: gid_t) -> c_int;

    // 5. GNU 扩展
    #[link_name = "setresuid"]
    fn musl_setresuid(ruid: uid_t, euid: uid_t, suid: uid_t) -> c_int;
    #[link_name = "setresgid"]
    fn musl_setresgid(rgid: gid_t, egid: gid_t, sgid: gid_t) -> c_int;

    // 6. 终端操作
    #[link_name = "ttyname"]
    fn musl_ttyname(fd: c_int) -> *mut c_char;
    #[link_name = "ttyname_r"]
    fn musl_ttyname_r(fd: c_int, name: *mut c_char, size: usize) -> c_int;
    #[link_name = "isatty"]
    fn musl_isatty(fd: c_int) -> c_int;
    #[link_name = "tcgetpgrp"]
    fn musl_tcgetpgrp(fd: c_int) -> pid_t;
    #[link_name = "tcsetpgrp"]
    fn musl_tcsetpgrp(fd: c_int, pgrp: pid_t) -> c_int;
    #[link_name = "ctermid"]
    fn musl_ctermid(s: *mut c_char) -> *mut c_char;

    // 7. 其他系统信息
    #[link_name = "gethostname"]
    fn musl_gethostname(name: *mut c_char, len: usize) -> c_int;
    #[link_name = "getlogin"]
    fn musl_getlogin() -> *mut c_char;
    #[link_name = "getlogin_r"]
    fn musl_getlogin_r(name: *mut c_char, size: usize) -> c_int;

    // 8. BSD/GNU 扩展 — 时间
    #[link_name = "usleep"]
    fn musl_usleep(useconds: c_uint) -> c_int;
    #[link_name = "ualarm"]
    fn musl_ualarm(value: c_uint, interval: c_uint) -> c_uint;
}

// ---------- 安全公共封装 ----------

// 1. 基本 I/O
pub extern "C" fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize { unsafe { musl_read(fd, buf, count) } }
pub extern "C" fn write(fd: c_int, buf: *const c_void, count: usize) -> isize { unsafe { musl_write(fd, buf, count) } }
pub extern "C" fn close(fd: c_int) -> c_int { unsafe { musl_close(fd) } }
pub extern "C" fn dup(fd: c_int) -> c_int { unsafe { musl_dup(fd) } }
pub extern "C" fn dup2(old: c_int, new: c_int) -> c_int { unsafe { musl_dup2(old, new) } }
pub extern "C" fn dup3(old: c_int, new: c_int, flags: c_int) -> c_int { unsafe { musl_dup3(old, new, flags) } }
pub extern "C" fn lseek(fd: c_int, offset: off_t, whence: c_int) -> off_t { unsafe { musl_lseek(fd, offset, whence) } }
pub extern "C" fn pipe(fd: *mut c_int) -> c_int { unsafe { musl_pipe(fd) } }
pub extern "C" fn pipe2(fd: *mut c_int, flags: c_int) -> c_int { unsafe { musl_pipe2(fd, flags) } }
pub extern "C" fn pread(fd: c_int, buf: *mut c_void, size: usize, ofs: off_t) -> isize { unsafe { musl_pread(fd, buf, size, ofs) } }
pub extern "C" fn pwrite(fd: c_int, buf: *const c_void, size: usize, ofs: off_t) -> isize { unsafe { musl_pwrite(fd, buf, size, ofs) } }
pub extern "C" fn readv(fd: c_int, iov: *const iovec, count: c_int) -> isize { unsafe { musl_readv(fd, iov, count) } }
pub extern "C" fn writev(fd: c_int, iov: *const iovec, count: c_int) -> isize { unsafe { musl_writev(fd, iov, count) } }
pub extern "C" fn preadv(fd: c_int, iov: *const iovec, count: c_int, ofs: off_t) -> isize { unsafe { musl_preadv(fd, iov, count, ofs) } }
pub extern "C" fn pwritev(fd: c_int, iov: *const iovec, count: c_int, ofs: off_t) -> isize { unsafe { musl_pwritev(fd, iov, count, ofs) } }
pub extern "C" fn fsync(fd: c_int) -> c_int { unsafe { musl_fsync(fd) } }
pub extern "C" fn fdatasync(fd: c_int) -> c_int { unsafe { musl_fdatasync(fd) } }
pub extern "C" fn ftruncate(fd: c_int, length: off_t) -> c_int { unsafe { musl_ftruncate(fd, length) } }
pub extern "C" fn posix_close(fd: c_int, flags: c_int) -> c_int { unsafe { musl_posix_close(fd, flags) } }

// 2. 文件系统操作
pub extern "C" fn access(filename: *const c_char, amode: c_int) -> c_int { unsafe { musl_access(filename, amode) } }
pub extern "C" fn faccessat(fd: c_int, filename: *const c_char, amode: c_int, flag: c_int) -> c_int { unsafe { musl_faccessat(fd, filename, amode, flag) } }
pub extern "C" fn chdir(path: *const c_char) -> c_int { unsafe { musl_chdir(path) } }
pub extern "C" fn fchdir(fd: c_int) -> c_int { unsafe { musl_fchdir(fd) } }
pub extern "C" fn getcwd(buf: *mut c_char, size: usize) -> *mut c_char { unsafe { musl_getcwd(buf, size) } }
pub extern "C" fn chown(path: *const c_char, uid: uid_t, gid: gid_t) -> c_int { unsafe { musl_chown(path, uid, gid) } }
pub extern "C" fn fchown(fd: c_int, uid: uid_t, gid: gid_t) -> c_int { unsafe { musl_fchown(fd, uid, gid) } }
pub extern "C" fn lchown(path: *const c_char, uid: uid_t, gid: gid_t) -> c_int { unsafe { musl_lchown(path, uid, gid) } }
pub extern "C" fn fchownat(fd: c_int, path: *const c_char, uid: uid_t, gid: gid_t, flag: c_int) -> c_int { unsafe { musl_fchownat(fd, path, uid, gid, flag) } }
pub extern "C" fn link(existing: *const c_char, new: *const c_char) -> c_int { unsafe { musl_link(existing, new) } }
pub extern "C" fn linkat(fd1: c_int, existing: *const c_char, fd2: c_int, new: *const c_char, flag: c_int) -> c_int { unsafe { musl_linkat(fd1, existing, fd2, new, flag) } }
pub extern "C" fn symlink(existing: *const c_char, new: *const c_char) -> c_int { unsafe { musl_symlink(existing, new) } }
pub extern "C" fn symlinkat(existing: *const c_char, fd: c_int, new: *const c_char) -> c_int { unsafe { musl_symlinkat(existing, fd, new) } }
pub extern "C" fn readlink(path: *const c_char, buf: *mut c_char, bufsize: usize) -> isize { unsafe { musl_readlink(path, buf, bufsize) } }
pub extern "C" fn readlinkat(fd: c_int, path: *const c_char, buf: *mut c_char, bufsize: usize) -> isize { unsafe { musl_readlinkat(fd, path, buf, bufsize) } }
pub extern "C" fn unlink(path: *const c_char) -> c_int { unsafe { musl_unlink(path) } }
pub extern "C" fn unlinkat(fd: c_int, path: *const c_char, flag: c_int) -> c_int { unsafe { musl_unlinkat(fd, path, flag) } }
pub extern "C" fn rmdir(path: *const c_char) -> c_int { unsafe { musl_rmdir(path) } }
pub extern "C" fn renameat(oldfd: c_int, old: *const c_char, newfd: c_int, new: *const c_char) -> c_int { unsafe { musl_renameat(oldfd, old, newfd, new) } }
pub extern "C" fn truncate(path: *const c_char, length: off_t) -> c_int { unsafe { musl_truncate(path, length) } }
pub extern "C" fn sync() { unsafe { musl_sync() } }
pub extern "C" fn acct(filename: *const c_char) -> c_int { unsafe { musl_acct(filename) } }

// 3. 进程控制
pub extern "C" fn _exit(status: c_int) -> ! { unsafe { musl__exit(status) } }
pub extern "C" fn alarm(seconds: c_uint) -> c_uint { unsafe { musl_alarm(seconds) } }
pub extern "C" fn sleep(seconds: c_uint) -> c_uint { unsafe { musl_sleep(seconds) } }
pub extern "C" fn pause() -> c_int { unsafe { musl_pause() } }
pub extern "C" fn getpid() -> pid_t { unsafe { musl_getpid() } }
pub extern "C" fn getppid() -> pid_t { unsafe { musl_getppid() } }
pub extern "C" fn getpgrp() -> pid_t { unsafe { musl_getpgrp() } }
pub extern "C" fn getpgid(pid: pid_t) -> pid_t { unsafe { musl_getpgid(pid) } }
pub extern "C" fn setpgid(pid: pid_t, pgid: pid_t) -> c_int { unsafe { musl_setpgid(pid, pgid) } }
pub extern "C" fn setsid() -> pid_t { unsafe { musl_setsid() } }
pub extern "C" fn getsid(pid: pid_t) -> pid_t { unsafe { musl_getsid(pid) } }
pub extern "C" fn setpgrp() -> pid_t { unsafe { musl_setpgrp() } }
pub extern "C" fn nice(inc: c_int) -> c_int { unsafe { musl_nice(inc) } }

// 4. 用户标识和组
pub extern "C" fn getuid() -> uid_t { unsafe { musl_getuid() } }
pub extern "C" fn geteuid() -> uid_t { unsafe { musl_geteuid() } }
pub extern "C" fn getgid() -> gid_t { unsafe { musl_getgid() } }
pub extern "C" fn getegid() -> gid_t { unsafe { musl_getegid() } }
pub extern "C" fn getgroups(count: c_int, list: *mut gid_t) -> c_int { unsafe { musl_getgroups(count, list) } }
pub extern "C" fn setuid(uid: uid_t) -> c_int { unsafe { musl_setuid(uid) } }
pub extern "C" fn seteuid(euid: uid_t) -> c_int { unsafe { musl_seteuid(euid) } }
pub extern "C" fn setgid(gid: gid_t) -> c_int { unsafe { musl_setgid(gid) } }
pub extern "C" fn setegid(egid: gid_t) -> c_int { unsafe { musl_setegid(egid) } }
pub extern "C" fn setreuid(ruid: uid_t, euid: uid_t) -> c_int { unsafe { musl_setreuid(ruid, euid) } }
pub extern "C" fn setregid(rgid: gid_t, egid: gid_t) -> c_int { unsafe { musl_setregid(rgid, egid) } }

// 5. GNU 扩展
pub extern "C" fn setresuid(ruid: uid_t, euid: uid_t, suid: uid_t) -> c_int { unsafe { musl_setresuid(ruid, euid, suid) } }
pub extern "C" fn setresgid(rgid: gid_t, egid: gid_t, sgid: gid_t) -> c_int { unsafe { musl_setresgid(rgid, egid, sgid) } }

// 6. 终端操作
pub extern "C" fn ttyname(fd: c_int) -> *mut c_char { unsafe { musl_ttyname(fd) } }
pub extern "C" fn ttyname_r(fd: c_int, name: *mut c_char, size: usize) -> c_int { unsafe { musl_ttyname_r(fd, name, size) } }
pub extern "C" fn isatty(fd: c_int) -> c_int { unsafe { musl_isatty(fd) } }
pub extern "C" fn tcgetpgrp(fd: c_int) -> pid_t { unsafe { musl_tcgetpgrp(fd) } }
pub extern "C" fn tcsetpgrp(fd: c_int, pgrp: pid_t) -> c_int { unsafe { musl_tcsetpgrp(fd, pgrp) } }
pub extern "C" fn ctermid(s: *mut c_char) -> *mut c_char { unsafe { musl_ctermid(s) } }

// 7. 其他系统信息
pub extern "C" fn gethostname(name: *mut c_char, len: usize) -> c_int { unsafe { musl_gethostname(name, len) } }
pub extern "C" fn getlogin() -> *mut c_char { unsafe { musl_getlogin() } }
pub extern "C" fn getlogin_r(name: *mut c_char, size: usize) -> c_int { unsafe { musl_getlogin_r(name, size) } }

// 8. BSD/GNU 扩展 — 时间
pub extern "C" fn usleep(useconds: c_uint) -> c_int { unsafe { musl_usleep(useconds) } }
pub extern "C" fn ualarm(value: c_uint, interval: c_uint) -> c_uint { unsafe { musl_ualarm(value, interval) } }
