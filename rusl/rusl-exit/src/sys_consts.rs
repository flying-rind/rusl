//! Linux x86_64 syscall 编号常量。

pub(crate) const SYS_exit_group: i64 = 231;
pub(crate) const SYS_exit: i64 = 60;
pub(crate) const SYS_kill: i64 = 62;
pub(crate) const SYS_getpid: i64 = 39;
pub(crate) const SYS_gettid: i64 = 186;
pub(crate) const SYS_write: i64 = 1;

/// 标准错误输出文件描述符
pub(crate) const STDERR_FILENO: i64 = 2;

/// SIGABRT 信号编号
pub(crate) const SIGABRT: i64 = 6;
