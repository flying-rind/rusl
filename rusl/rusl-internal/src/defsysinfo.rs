//! defsysinfo 模块 — 存储 ELF 辅助向量中的 vDSO 地址。
//!
//! 本模块定义全局静态变量 `__SYSINFO`，存储 Linux vDSO
//! (virtual Dynamic Shared Object) 中的内核辅助代码页地址，
//! 用于加速系统调用（如 `clock_gettime`），避免陷入内核态。
//!
//! # 模块可见性
//!
//! `pub(crate)` — 仅在 rusl crate 内部使用，由 `__init_libc` 初始化，
//! 由 `__syscall` 族函数读取。

use core::sync::atomic::AtomicUsize;

/// vDSO 辅助代码页地址。
///
/// 初始值为 `0`（未初始化/系统不支持 vDSO）。
/// 初始化后若值非零，则为有效的用户空间可执行内存地址。
///
/// # 读写约定
///
/// | 操作     | 方法                     | 说明           |
/// |----------|--------------------------|----------------|
/// | 写入     | `store(addr, Release)`   | 初始化阶段写入 |
/// | 读取     | `load(Acquire)`          | 关键路径读取   |
/// | 快速检查 | `load(Relaxed)`          | 仅检查是否为零 |
///
/// # 不变量
///
/// - 在进程启动后仅写入一次（由 `__init_libc`），后续为只读。
/// - `__SYSINFO == 0` 表示未初始化或系统不支持 vDSO。
pub static __SYSINFO: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
mod tests {
    use rusl_core::test;
    use super::__SYSINFO;
    use core::sync::atomic::Ordering;

    test!("sysinfo_initial_zero" {
        assert_eq!(__SYSINFO.load(Ordering::Relaxed), 0);
    });

    test!("sysinfo_store_load" {
        let test_addr: usize = 0x7fff_1000;
        __SYSINFO.store(test_addr, Ordering::Release);
        assert_eq!(__SYSINFO.load(Ordering::Acquire), test_addr);
        __SYSINFO.store(0, Ordering::Release);
    });

    test!("sysinfo_atomic_swap" {
        __SYSINFO.store(42, Ordering::SeqCst);
        let old = __SYSINFO.swap(100, Ordering::SeqCst);
        assert_eq!(old, 42);
        assert_eq!(__SYSINFO.load(Ordering::SeqCst), 100);
        __SYSINFO.store(0, Ordering::Release);
    });
}