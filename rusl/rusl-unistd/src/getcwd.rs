//! getcwd — 获取调用进程的当前工作目录绝对路径名。
//! 对应 musl src/unistd/getcwd.c
//!
//! SYS_getcwd 系统调用封装。

use core::ffi::c_char;
use rusl_internal::syscall::raw_syscall2;

/// getcwd(buf, size) — 获取当前工作目录的绝对路径名。
///
/// - buf != NULL: 写入最多 size 字节到 buf，返回 buf
/// - buf == NULL: 使用内部堆栈缓冲区，返回指向路径名的指针
/// - size == 0 且 buf != NULL: 返回 NULL (EINVAL)
///
/// 成功返回指向路径名的指针，失败返回 NULL。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn getcwd(buf: *mut c_char, size: usize) -> *mut c_char {
    if !buf.is_null() && size == 0 {
        return core::ptr::null_mut();
    }

    // 如果 buf 为 NULL，使用内部缓冲区
    let use_internal = buf.is_null();
    let (work_buf, work_size) = if use_internal {
        // 使用静态可变缓冲区（类似 musl 的 char tmp[PATH_MAX]）
        // 在 no_std 环境下，我们使用固定的 4096 字节缓冲区
        static mut TMP_BUF: [c_char; 4096] = [0; 4096];
        unsafe { (TMP_BUF.as_mut_ptr(), 4096usize) }
    } else {
        (buf, size)
    };

    unsafe {
        let r = raw_syscall2(
            rusl_internal::syscall::SYS_getcwd,
            work_buf as i64,
            work_size as i64,
        );
        // 内核返回用户空间指针（成功）或负错误码（失败）
        if r < 0 {
            core::ptr::null_mut()
        } else if use_internal {
            // buf 为 NULL：返回内部缓冲区的指针（musl 会 strdup，但我们简化）
            work_buf
        } else {
            // buf 非 NULL：返回原始 buf 指针
            buf
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

    test!("test_getcwd_null_buf_with_size_zero_returns_valid" {
        // buf=NULL, size=0: 使用内部缓冲区，应返回有效路径
        let result = getcwd(core::ptr::null_mut(), 0);
        assert!(!result.is_null(), "getcwd(NULL, 0) should return valid path");
        // 路径应以 '/' 开头
        unsafe {
            assert_eq!(*result, '/' as c_char, "path should start with '/'");
        }
    });

    test!("test_getcwd_size_zero_with_nonnull_buf_returns_null" {
        // buf != NULL, size == 0: 返回 NULL（EINVAL）
        let mut buf: [c_char; 1] = [0; 1];
        let result = getcwd(buf.as_mut_ptr(), 0);
        assert!(result.is_null(), "getcwd(nonnull, 0) should return NULL (EINVAL)");
    });

    test!("test_getcwd_small_buffer_returns_null" {
        // buf 太小无法容纳路径: 返回 NULL（ERANGE）
        let mut buf: [c_char; 2] = [0; 2];
        let result = getcwd(buf.as_mut_ptr(), 2);
        assert!(result.is_null(), "getcwd with too-small buffer should return NULL (ERANGE)");
    });

    test!("test_getcwd_valid_buffer_returns_path" {
        // 使用足够大的缓冲区：应返回路径
        let mut buf: [c_char; 4096] = [0; 4096];
        let result = getcwd(buf.as_mut_ptr(), 4096);
        assert!(!result.is_null(), "getcwd should return valid path");
        assert_eq!(result, buf.as_mut_ptr(), "getcwd should return buf pointer");
        unsafe {
            assert_eq!(*result, '/' as c_char, "path should start with '/'");
        }
    });

    test!("test_getcwd_internal_buffer_check" {
        // buf=NULL 使用内部缓冲区，应返回不同指针
        let internal = getcwd(core::ptr::null_mut(), 0);
        assert!(!internal.is_null(), "internal buffer path should be valid");

        // 使用自己的缓冲区
        let mut my_buf: [c_char; 4096] = [0; 4096];
        let my_result = getcwd(my_buf.as_mut_ptr(), 4096);
        assert_eq!(my_result, my_buf.as_mut_ptr(), "should return our buffer");

        // 验证路径以 '/' 开头
        unsafe {
            assert_eq!(*internal, '/' as c_char, "internal path should start with '/'");
            assert_eq!(*my_buf.as_ptr(), '/' as c_char, "user path should start with '/'");
        }
    });
}
