//! ttyname — 获取文件描述符关联的终端设备路径名（非线程安全）。
//! 对应 musl src/unistd/ttyname.c
//!
//! 将 ttyname_r 的结果存入内部静态缓冲区。

use core::ffi::{c_char, c_int};
use core::ptr;

/// TTY_NAME_MAX — 终端名称最大长度
const TTY_NAME_MAX: usize = 32;

/// POSIX `ttyname` — 返回 `fd` 所关联终端设备的路径名。
///
/// 将 [`ttyname_r`] 的结果存入内部静态缓冲区后返回指针。
/// 后续调用会覆盖缓冲区内容。此函数非线程安全。
/// 成功返回指向路径名的指针，失败返回 NULL 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn ttyname(fd: c_int) -> *mut c_char {
    // 可以使用外部声明来访问 errno 设置
    extern "C" {
        fn __errno_location() -> *mut c_int;
    }

    static mut TTY_BUF: [c_char; TTY_NAME_MAX] = [0; TTY_NAME_MAX];

    let result = super::ttyname_r(fd, unsafe { TTY_BUF.as_mut_ptr() }, TTY_NAME_MAX);
    if result != 0 {
        // 设置 errno
        unsafe {
            *__errno_location() = result;
        }
        return ptr::null_mut();
    }
    unsafe { TTY_BUF.as_mut_ptr() }
}

// ===========================================================================
// 单元测试
// ===========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use rusl_core::test;

    test!("test_ttyname_invalid_fd" {
        // 无效 fd: 返回 NULL
        let result = ttyname(-1);
        assert!(result.is_null(), "ttyname(-1) should return NULL");
    });

    test!("test_ttyname_very_large_fd" {
        // 非常大的 fd: 返回 NULL
        let result = ttyname(99999);
        assert!(result.is_null(), "ttyname(99999) should return NULL");
    });

    test!("test_ttyname_pipe_fd" {
        // 管道 fd: 返回 NULL (不是终端)
        let mut fds: [c_int; 2] = [-1, -1];
        let ret = crate::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0);

        let result = ttyname(fds[0]);
        assert!(result.is_null(), "ttyname on pipe fd should return NULL");

        crate::close(fds[0]);
        crate::close(fds[1]);
    });

    test!("test_ttyname_tty_name_max" {
        // 验证 TTY_NAME_MAX 常量
        assert_eq!(TTY_NAME_MAX, 32, "TTY_NAME_MAX should be 32");
    });

    test!("test_ttyname_is_atty_basic" {
        // 基本验证: ttyname 对于非终端 fd 返回 NULL
        // 对 stdin/stdout/stderr 的测试取决于运行环境
        // 在非交互式环境下（如 CI），这些可能不是终端
        let r0 = ttyname(0);
        let r1 = ttyname(1);
        let r2 = ttyname(2);
        // 只验证不崩溃
        let _ = (r0, r1, r2);
    });
}
