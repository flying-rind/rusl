//! vdso 模块 — vDSO 符号查找。
//!
//! 对应 musl src/internal/vdso.c
//!
//! 在 ELF 辅助向量 (auxv) 中查找 vDSO (virtual Dynamic Shared Object)，
//! 并搜索指定符号。当返回 NULL 时，调用方回退到直接系统调用。

/// C ABI: `void *__vdsosym(const char *vername, const char *name)`
///
/// 在当前 ELF 进程的 vDSO 映像中按名称查找符号。
/// 返回符号地址或 NULL（NULL 表示 vDSO 不可用或符号未找到）。
///
/// 当前为占位实现（返回 NULL），调用方回退到直接 syscall。
#[no_mangle]
pub unsafe extern "C" fn __vdsosym(
    _vername: *const u8,
    _name: *const u8,
) -> *mut core::ffi::c_void {
    core::ptr::null_mut()
}

#[cfg(test)]
mod tests {
    use rusl_core::test;

    test!("vdsosym_returns_null" {
        assert!(unsafe { super::__vdsosym(b"LINUX_2.6\0".as_ptr(), b"clock_gettime\0".as_ptr()) }.is_null());
    });
}
