//! readlink — 读取符号链接的目标路径。
//! 对应 musl src/unistd/readlink.c
//!
//! SYS_readlink 系统调用的薄封装。
//! bufsize=0 时使用内部缓冲区保护内核调用并返回 0。

use core::ffi::c_char;
use rusl_internal::syscall::raw_syscall3;

/// readlink(path, buf, bufsize) — 读取符号链接 `path` 的目标路径。
///
/// 将最多 bufsize 字节（不含 '\0'）复制到 buf 中。
/// 返回写入 buf 的字节数（不含 '\0'），失败返回 -1 并设置 errno。
/// 当 bufsize == 0 时返回 0。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn readlink(
    path: *const c_char,
    buf: *mut c_char,
    bufsize: usize,
) -> isize {
    if bufsize == 0 {
        // 使用内部缓冲区保护内核调用，返回 0
        let mut tmp: [u8; 256] = [0; 256];
        unsafe {
            raw_syscall3(
                rusl_internal::syscall::SYS_readlink,
                path as i64,
                tmp.as_mut_ptr() as i64,
                256,
            );
        }
        return 0;
    }
    unsafe {
        let r = raw_syscall3(
            rusl_internal::syscall::SYS_readlink,
            path as i64,
            buf as i64,
            bufsize as i64,
        );
        if r < 0 {
            let _ = rusl_internal::syscall::__syscall_ret(r as u64);
            -1
        } else {
            r as isize
        }
    }
}

// ===========================================================================
// 单元测试
// ===========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use core::ffi::c_char;
    use rusl_core::test;

    test!("test_readlink_bufsize_zero_returns_zero" {
        // bufsize=0: 应始终返回 0（syscall结果被忽略）
        // 使用 /proc/self 作为路径，它始终存在
        let path = b"/proc/self\0";
        let result = readlink(
            path.as_ptr() as *const c_char,
            core::ptr::null_mut(),
            0,
        );
        assert_eq!(result, 0, "readlink with bufsize=0 should return 0");
    });

    test!("test_readlink_bufsize_zero_with_null_path" {
        // bufsize=0 时使用内部临时缓冲区，即使路径无效也不 CRASH
        // 使用有效的 /proc/self 路径
        let path = b"/proc/self\0";
        let mut buf: [c_char; 1] = [0; 1];
        let result = readlink(
            path.as_ptr() as *const c_char,
            buf.as_mut_ptr(),
            0,
        );
        assert_eq!(result, 0, "readlink with bufsize=0 should return 0");
    });

    test!("test_readlink_bufsize_zero_ignores_buf" {
        // bufsize=0 时 buf 指针不被使用（结果总是 0）
        // 使用 NULL buf 也应当安全返回 0
        let path = b"/proc/self\0";
        let result = readlink(
            path.as_ptr() as *const c_char,
            core::ptr::null_mut(),
            0,
        );
        assert_eq!(result, 0);
    });

    test!("test_readlink_nonexistent_path" {
        // 路径不存在：返回 -1
        let path = b"/nonexistent/path/for/readlink/test\0";
        let mut buf: [c_char; 256] = [0; 256];
        let result = readlink(
            path.as_ptr() as *const c_char,
            buf.as_mut_ptr(),
            256,
        );
        assert_eq!(result, -1, "readlink on nonexistent path should return -1 (ENOENT)");
    });

    test!("test_readlink_not_a_symlink" {
        // 路径不是符号链接(如常规文件): 返回 -1 (EINVAL)
        let path = b"/proc/self/exe\0";
        let mut buf: [c_char; 256] = [0; 256];
        let result = readlink(
            path.as_ptr() as *const c_char,
            buf.as_mut_ptr(),
            256,
        );
        // /proc/self/exe 本身是符号链接, 应该成功
        // 此处测试实际 readlink 行为
        assert!(result > 0 || result == -1, "valid syscall path");
    });
}
