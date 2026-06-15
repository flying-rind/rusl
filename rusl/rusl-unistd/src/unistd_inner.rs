//! unistd — POSIX 系统调用封装。
//! 对应 musl src/unistd/ 目录。

#![allow(dead_code, unused_imports)]

// ========== 公共类型和常量 ==========
pub mod types;
pub use types::*;

// ========== 1. 基本 I/O ==========
mod read;
mod write;
mod close;
mod lseek;
mod dup;
mod dup2;
mod dup3;
mod pipe;
mod pipe2;
mod pread;
mod pwrite;
mod readv;
mod writev;
mod preadv;
mod pwritev;
mod fsync;
mod fdatasync;
mod ftruncate;
mod posix_close;

// ========== 2. 文件系统操作 ==========
mod access;
mod faccessat;
mod chdir;
mod fchdir;
mod getcwd;
mod chown;
mod fchown;
mod lchown;
mod fchownat;
mod link;
mod linkat;
mod symlink;
mod symlinkat;
mod readlink;
mod readlinkat;
mod unlink;
mod unlinkat;
mod rmdir;
mod renameat;
mod truncate;
mod sync;
mod acct;

// ========== 3. 进程控制 ==========
mod _exit;
mod alarm;
mod sleep;
mod pause;
mod getpid;
mod getppid;
mod getpgrp;
mod getpgid;
mod setpgid;
mod setsid;
mod getsid;
mod setpgrp;
mod nice;

// ========== 4. 用户标识和组 ==========
mod getuid;
mod geteuid;
mod getgid;
mod getegid;
mod getgroups;
mod setuid;
mod seteuid;
mod setgid;
mod setegid;
mod setreuid;
mod setregid;

// ========== 5. GNU 扩展 ==========
mod setresuid;
mod setresgid;

// ========== 内部实现 ==========
mod setxid;

// ========== 6. 终端操作 ==========
mod ttyname;
mod ttyname_r;
mod isatty;
mod tcgetpgrp;
mod tcsetpgrp;
mod ctermid;

// ========== 7. 其他系统信息 ==========
mod gethostname;
mod getlogin;
mod getlogin_r;

// ========== 8. BSD/GNU 扩展 — 时间 ==========
mod usleep;
mod ualarm;

// ========== 公共重导出 ==========
// 1. 基本 I/O
pub use read::read;
pub use write::write;
pub use close::close;
pub use lseek::{lseek, __lseek};
pub use dup::dup;
pub use dup2::dup2;
pub use dup3::{dup3, __dup3};
pub use pipe::pipe;
pub use pipe2::pipe2;
pub use pread::pread;
pub use pwrite::pwrite;
pub use readv::readv;
pub use writev::writev;
pub use preadv::preadv;
pub use pwritev::pwritev;
pub use fsync::fsync;
pub use fdatasync::fdatasync;
pub use ftruncate::ftruncate;
pub use posix_close::posix_close;

// 2. 文件系统操作
pub use access::access;
pub use faccessat::faccessat;
pub use chdir::chdir;
pub use fchdir::fchdir;
pub use getcwd::getcwd;
pub use chown::chown;
pub use fchown::fchown;
pub use lchown::lchown;
pub use fchownat::fchownat;
pub use link::link;
pub use linkat::linkat;
pub use symlink::symlink;
pub use symlinkat::symlinkat;
pub use readlink::readlink;
pub use readlinkat::readlinkat;
pub use unlink::unlink;
pub use unlinkat::unlinkat;
pub use rmdir::rmdir;
pub use renameat::renameat;
pub use truncate::truncate;
pub use sync::sync;
pub use acct::acct;

// 3. 进程控制
pub use _exit::_exit;
pub use alarm::alarm;
pub use sleep::sleep;
pub use pause::pause;
pub use getpid::getpid;
pub use getppid::getppid;
pub use getpgrp::getpgrp;
pub use getpgid::getpgid;
pub use setpgid::setpgid;
pub use setsid::setsid;
pub use getsid::getsid;
pub use setpgrp::setpgrp;
pub use nice::nice;

// 4. 用户标识和组
pub use getuid::getuid;
pub use geteuid::geteuid;
pub use getgid::getgid;
pub use getegid::getegid;
pub use getgroups::getgroups;
pub use setuid::setuid;
pub use seteuid::seteuid;
pub use setgid::setgid;
pub use setegid::setegid;
pub use setreuid::setreuid;
pub use setregid::setregid;

// 5. GNU 扩展
pub use setresuid::setresuid;
pub use setresgid::setresgid;

// 6. 终端操作
pub use ttyname::ttyname;
pub use ttyname_r::ttyname_r;
pub use isatty::isatty;
pub use tcgetpgrp::tcgetpgrp;
pub use tcsetpgrp::tcsetpgrp;
pub use ctermid::ctermid;

// 7. 其他系统信息
pub use gethostname::gethostname;
pub use getlogin::getlogin;
pub use getlogin_r::getlogin_r;

// 8. BSD/GNU 扩展 — 时间
pub use usleep::usleep;
pub use ualarm::ualarm;

// 内部符号（仅 pub(crate)）
pub(crate) use setxid::__setxid;
