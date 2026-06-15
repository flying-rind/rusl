//! unistd 集成测试子模块

#[allow(unused)]
pub use rusl::api::unistd::*;
pub use core::ffi::c_int;

// ===========================================================================
// 基本 I/O
// ===========================================================================
mod write;
mod read;
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
mod truncate;
mod posix_close;

// 用户/组 ID 获取
mod getuid;
mod geteuid;
mod getgid;
mod getegid;

// 用户/组 ID 设置
mod setuid;
mod seteuid;
mod setgid;
mod setegid;
mod getgroups;
mod setreuid;
mod setregid;
mod setresuid;
mod setresgid;

// 文件系统操作
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
mod symlinkat;
mod symlink;
mod readlink;
mod readlinkat;
mod unlink;
mod unlinkat;
mod rmdir;
mod renameat;
mod sync;

// 进程控制
mod _exit;
mod getpid;
mod getppid;
mod getpgid;
mod setpgid;
mod getpgrp;
mod setpgrp;
mod getsid;
mod setsid;
mod alarm;
mod sleep;
mod pause;
mod nice;
mod usleep;
mod ualarm;
mod acct;

// 终端相关
mod isatty;
mod ttyname;
mod ttyname_r;
mod tcgetpgrp;
mod tcsetpgrp;
mod ctermid;
mod getlogin;
mod getlogin_r;
mod gethostname;
