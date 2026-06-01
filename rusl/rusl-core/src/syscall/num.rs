// Linux syscall number constants.
// Architecture-gated via #[cfg(target_arch = "...")].
// Naming follows Linux kernel convention (SYS_read, not SYS_READ).

#![allow(non_upper_case_globals)]
#![allow(dead_code)]  // most constants used in later stages

// --- x86_64 ---
#[cfg(target_arch = "x86_64")]
pub const SYS_read: i64 = 0;
#[cfg(target_arch = "x86_64")]
pub const SYS_write: i64 = 1;
#[cfg(target_arch = "x86_64")]
pub const SYS_open: i64 = 2;
#[cfg(target_arch = "x86_64")]
pub const SYS_close: i64 = 3;
#[cfg(target_arch = "x86_64")]
pub const SYS_mmap: i64 = 9;
#[cfg(target_arch = "x86_64")]
pub const SYS_munmap: i64 = 11;
#[cfg(target_arch = "x86_64")]
pub const SYS_brk: i64 = 12;
#[cfg(target_arch = "x86_64")]
pub const SYS_exit: i64 = 60;
#[cfg(target_arch = "x86_64")]
pub const SYS_exit_group: i64 = 231;
#[cfg(target_arch = "x86_64")]
pub const SYS_kill: i64 = 62;
#[cfg(target_arch = "x86_64")]
pub const SYS_getpid: i64 = 39;
#[cfg(target_arch = "x86_64")]
pub const SYS_set_tid_address: i64 = 218;
#[cfg(target_arch = "x86_64")]
pub const SYS_arch_prctl: i64 = 158;
#[cfg(target_arch = "x86_64")]
pub const SYS_poll: i64 = 7;
#[cfg(target_arch = "x86_64")]
pub const SYS_ppoll: i64 = 271;

// --- aarch64 ---
#[cfg(target_arch = "aarch64")]
pub const SYS_read: i64 = 63;
#[cfg(target_arch = "aarch64")]
pub const SYS_write: i64 = 64;
#[cfg(target_arch = "aarch64")]
pub const SYS_open: i64 = 1024;  // openat
#[cfg(target_arch = "aarch64")]
pub const SYS_close: i64 = 57;
#[cfg(target_arch = "aarch64")]
pub const SYS_mmap: i64 = 222;
#[cfg(target_arch = "aarch64")]
pub const SYS_munmap: i64 = 215;
#[cfg(target_arch = "aarch64")]
pub const SYS_brk: i64 = 214;
#[cfg(target_arch = "aarch64")]
pub const SYS_exit: i64 = 93;
#[cfg(target_arch = "aarch64")]
pub const SYS_exit_group: i64 = 94;
#[cfg(target_arch = "aarch64")]
pub const SYS_kill: i64 = 129;
#[cfg(target_arch = "aarch64")]
pub const SYS_getpid: i64 = 172;
#[cfg(target_arch = "aarch64")]
pub const SYS_set_tid_address: i64 = 96;
#[cfg(target_arch = "aarch64")]
pub const SYS_ppoll: i64 = 73;