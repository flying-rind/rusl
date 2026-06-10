//! defsysinfo 模块 — 存储 ELF 辅助向量中的 vDSO 地址。
//!
//! 对应 musl src/internal/defsysinfo.c

/// vDSO 辅助代码页地址。
///
/// 由 `__init_libc` (src/env/__libc_start_main.c) 在启动时从
/// ELF 辅助向量 AT_SYSINFO 中初始化。`__init_tls` 读取此值
/// 设置线程本地存储的 sysinfo 字段。
///
/// 类型: `size_t`，C ABI 兼容。
#[no_mangle]
pub static mut __sysinfo: usize = 0;

#[cfg(test)]
mod tests {
    use rusl_core::test;

    test!("sysinfo_initial_zero" {
        assert_eq!(unsafe { super::__sysinfo }, 0);
    });

    test!("sysinfo_set_get" {
        unsafe { super::__sysinfo = 42; }
        assert_eq!(unsafe { super::__sysinfo }, 42);
        unsafe { super::__sysinfo = 0; }
    });
}
