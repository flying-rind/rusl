//! readlinkat — 相对于目录文件描述符读取符号链接的目标路径。
//! 对应 musl src/unistd/readlinkat.c
//!
//! SYS_readlinkat 系统调用的薄封装。
//! bufsize=0 时使用内部缓冲区保护内核调用并返回 0。

use core::ffi::{c_char, c_int};
use crate::syscall::raw_syscall4;

/// readlinkat(fd, path, buf, bufsize) — 相对于目录 `fd` 读取符号链接的目标路径。
///
/// - fd: 目录文件描述符或 AT_FDCWD
/// - 将最多 bufsize 字节（不含 '\0'）复制到 buf 中
///
/// 返回写入 buf 的字节数（不含 '\0'），失败返回 -1 并设置 errno。
/// 当 bufsize == 0 时返回 0。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn readlinkat(
    fd: c_int,
    path: *const c_char,
    buf: *mut c_char,
    bufsize: usize,
) -> isize {
    if bufsize == 0 {
        // 使用内部缓冲区保护内核调用，返回 0
        let mut tmp: [u8; 256] = [0; 256];
        unsafe {
            raw_syscall4(
                crate::syscall::SYS_readlinkat,
                fd as i64,
                path as i64,
                tmp.as_mut_ptr() as i64,
                256,
            );
        }
        return 0;
    }
    unsafe {
        let r = raw_syscall4(
            crate::syscall::SYS_readlinkat,
            fd as i64,
            path as i64,
            buf as i64,
            bufsize as i64,
        );
        if r < 0 {
            let _ = crate::syscall::__syscall_ret(r as u64);
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

    /// AT_FDCWD — 指示路径相对于当前工作目录
    const AT_FDCWD: i32 = -100;

    test!("test_readlinkat_bufsize_zero_returns_zero" {
        // bufsize=0: 应始终返回 0
        let path = b"/proc/self\0";
        let result = readlinkat(
            AT_FDCWD,
            path.as_ptr() as *const c_char,
            core::ptr::null_mut(),
            0,
        );
        assert_eq!(result, 0, "readlinkat with bufsize=0 should return 0");
    });

    test!("test_readlinkat_bufsize_zero_ignores_buf_null" {
        // bufsize=0 时 buf=NULL 应安全返回 0
        let path = b"/proc/self\0";
        let result = readlinkat(
            AT_FDCWD,
            path.as_ptr() as *const c_char,
            core::ptr::null_mut(),
            0,
        );
        assert_eq!(result, 0);
    });

    test!("test_readlinkat_bufsize_zero_with_nonexistent_path" {
        // bufsize=0 即使路径不存在也应返回 0
        let path = b"/nonexistent/readlinkat/test\0";
        let mut buf: [c_char; 1] = [0; 1];
        let result = readlinkat(
            AT_FDCWD,
            path.as_ptr() as *const c_char,
            buf.as_mut_ptr(),
            0,
        );
        assert_eq!(result, 0, "bufsize=0 should always return 0");
    });

    test!("test_readlinkat_nonexistent_path_returns_error" {
        // 正常 bufsize 下路径不存在返回 -1
        let path = b"/nonexistent/readlinkat/test\0";
        let mut buf: [c_char; 256] = [0; 256];
        let result = readlinkat(
            AT_FDCWD,
            path.as_ptr() as *const c_char,
            buf.as_mut_ptr(),
            256,
        );
        assert_eq!(result, -1, "readlinkat on nonexistent path should return -1");
    });

    test!("test_readlinkat_invalid_fd" {
        // 使用无效 fd 和相对路径: 内核无法解析相对路径从无效 fd
        // 注意: 绝对路径会忽略 fd 参数
        let path = b"nonexistent_relative_path\0";
        let mut buf: [c_char; 256] = [0; 256];
        let result = readlinkat(
            99999, // 无效 fd
            path.as_ptr() as *const c_char,
            buf.as_mut_ptr(),
            256,
        );
        assert_eq!(result, -1, "readlinkat with invalid fd and relative path should return -1");
    });
}
