//! ksigaction 模块 — Linux 内核信号动作结构体与 sigreturn 蹦床。
//!
//! 本模块定义了与 Linux 内核 `rt_sigaction` 系统调用直接交互的
//! `KSigAction` 结构体，以及信号处理函数返回后恢复上下文所需的
//! `__restore` / `__restore_rt` 蹦床函数。
//!
//! `KSigAction` 是用户态 `struct sigaction`（POSIX）与内核态
//! `struct sigaction`（Linux）之间的桥梁。
//!
//! # 模块可见性
//!
//! `pub(crate)` — 仅在 rusl crate 内部使用。

#![allow(unused)]

use core::ffi::{c_int, c_uint, c_ulong, c_void};

/// 架构是否定义了 SA_RESTORER（需要 sigreturn 蹦床）。
///
/// x86_64 使用 SA_RESTORER 机制；aarch64 通过 VDSO 提供内核恢复。
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
const HAS_SA_RESTORER: bool = true;

#[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
const HAS_SA_RESTORER: bool = false;

/// 与 Linux 内核 `rt_sigaction` 系统调用交互的信号动作结构体。
///
/// Linux 内核期望的信号动作结构体布局与 POSIX `struct sigaction`
/// 不同。`KSigAction` 充当适配层：
/// - `mask[2]`: 内核使用固定 2 个 `c_uint`（共 64 位）存储信号掩码
/// - `restorer`/`unused`: 架构相关字段
///
/// # 布局不变量
///
/// * `size_of::<KSigAction>()` 必须与 Linux 内核中的结构体大小完全一致
/// * `#[repr(C)]` 确保与 C 端内核 ABI 兼容
/// * `restorer` 和 `unused` 在同一偏移量上互斥（条件编译控制）
#[repr(C)]
pub struct KSigAction {
    /// 信号处理函数指针。
    /// `None` = `SIG_DFL`（空指针），`Some(1 as fn)` = `SIG_IGN`。
    pub handler: Option<unsafe extern "C" fn(c_int)>,

    /// `SA_*` 标志位集合。
    pub flags: c_ulong,

    /// 信号恢复蹦床指针（仅当架构定义了 `SA_RESTORER` 时存在，如 x86_64）。
    /// 指向 `__restore_rt`，内核在信号处理返回后跳转到此地址恢复上下文。
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    pub restorer: Option<unsafe extern "C" fn()>,

    /// 信号掩码（内核格式，64 位）。
    /// 每个位对应一个信号编号（1-64）。
    pub mask: [c_uint; 2],

    /// 对齐填充（仅当架构未定义 `SA_RESTORER` 时存在，如 aarch64）。
    /// 大小 = `sizeof(usize)`，与 `restorer` 字段共享内存位置。
    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
    pub unused: *mut c_void,
}

// ---------------------------------------------------------------------------
// sigreturn 蹦床函数声明
// ---------------------------------------------------------------------------

// 旧式信号恢复蹦床（`sigreturn` 系统调用）。
//
// 此函数极其特殊：它永远不会被正常 Rust 代码调用，
// 而是由内核在信号处理返回时直接跳转到其地址。
//
// 在支持 SA_RESTORER 的架构上，其地址被写入 KSigAction.restorer 字段。
extern "C" {
    pub fn __restore();
}

// 实时信号恢复蹦床（`rt_sigreturn` 系统调用）。
//
// 与 __restore 类似，但用于 SA_SIGINFO 标志设置的信号处理。
// 内部执行 rt_sigreturn 系统调用来恢复原始上下文。
extern "C" {
    pub fn __restore_rt();
}