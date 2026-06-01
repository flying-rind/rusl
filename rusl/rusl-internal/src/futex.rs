//! futex 模块 — Linux `futex(2)` 系统调用操作码常量。
//!
//! 本模块定义了 rusl 内部线程同步所需的所有 futex 操作码。
//! futex (fast userspace mutex) 是 Linux 内核提供的轻量级同步原语，
//! rusl 使用它实现锁、信号量、条件变量、屏障等所有 pthread 同步机制。
//!
//! 所有常量与 Linux 内核 `include/uapi/linux/futex.h` 保持严格一致。
//!
//! # 模块可见性
//!
//! `pub(crate)` — 仅在 rusl crate 内部使用，不对外导出。

use core::ffi::c_int;

// ---------------------------------------------------------------------------
// 基本 futex 操作码 (与 Linux futex.h 一致)
// ---------------------------------------------------------------------------

/// futex 等待：线程挂起直到 futex 字的值不等于期望值 `val`。
pub const FUTEX_WAIT: c_int = 0;

/// futex 唤醒：最多唤醒 `val` 个等待在 futex 字上的线程。
pub const FUTEX_WAKE: c_int = 1;

/// futex 关联到文件描述符（已废弃，保留以兼容）。
pub const FUTEX_FD: c_int = 2;

/// futex 迁移：将等待者从主 futex 迁移到另一个 futex。
pub const FUTEX_REQUEUE: c_int = 3;

/// futex 条件迁移：原子化地检查 futex 字值并迁移等待者。
pub const FUTEX_CMP_REQUEUE: c_int = 4;

/// futex 操作并唤醒：原子化地修改 futex 字并唤醒等待者。
pub const FUTEX_WAKE_OP: c_int = 5;

// ---------------------------------------------------------------------------
// 优先级继承 (PI) futex 操作码
// ---------------------------------------------------------------------------

/// 带优先级继承的 futex 加锁操作。
pub const FUTEX_LOCK_PI: c_int = 6;

/// 带优先级继承的 futex 解锁操作。
pub const FUTEX_UNLOCK_PI: c_int = 7;

/// 带优先级继承的 futex 尝试加锁操作（非阻塞）。
pub const FUTEX_TRYLOCK_PI: c_int = 8;

// ---------------------------------------------------------------------------
// bitset / 高级 futex 操作码
// ---------------------------------------------------------------------------

/// 带 bitset 的 futex 等待，允许按位掩码选择性等待。
pub const FUTEX_WAIT_BITSET: c_int = 9;

// ---------------------------------------------------------------------------
// 修饰标志 (通过位或 `|` 与操作码组合使用)
// ---------------------------------------------------------------------------

/// 进程内私有标志：futex 仅在进程内共享，内核可跳过全局哈希表查找。
pub const FUTEX_PRIVATE: c_int = 128;

/// 超时使用 `CLOCK_REALTIME` 而非默认的 `CLOCK_MONOTONIC`。
pub const FUTEX_CLOCK_REALTIME: c_int = 256;