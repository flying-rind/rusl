//! lseek — 移动文件读写位置。
//! 对应 musl src/unistd/lseek.c
//!
//! SYS_lseek 系统调用的薄封装（64 位平台）。

use core::ffi::c_int;
use crate::import::do_syscall;

/// __lseek(fd, offset, whence) — musl 内部主实现。
///
/// 重新定位文件描述符 `fd` 的读写偏移。whence 取值为 SEEK_SET(0)、SEEK_CUR(1)、
/// SEEK_END(2)、SEEK_DATA(3)、SEEK_HOLE(4)。返回新的文件偏移，出错返回 -1。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn __lseek(fd: c_int, offset: i64, whence: c_int) -> i64 {
    unsafe { do_syscall!(crate::syscall::SYS_lseek, fd, offset, whence) as i64 }
}

/// lseek — __lseek 的公开别名。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn lseek(fd: c_int, offset: i64, whence: c_int) -> i64 {
    unsafe { do_syscall!(crate::syscall::SYS_lseek, fd, offset, whence) as i64 }
}

// ===========================================================================
// 单元测试
// ===========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use rusl_core::test;

    test!("test_lseek_invalid_fd" {
        // 无效 fd: 返回 -1 (EBADF)
        let ret = lseek(-1, 0, 0); // SEEK_SET
        assert_eq!(ret, -1, "lseek(-1, 0, SEEK_SET) should return -1 (EBADF)");
    });

    test!("test_lseek_internal_invalid_fd" {
        // __lseek 内部版本对无效 fd 同样返回 -1
        let ret = __lseek(-1, 0, 0);
        assert_eq!(ret, -1, "__lseek(-1, 0, SEEK_SET) should return -1 (EBADF)");
    });

    test!("test_lseek_internal_on_pipe" {
        // 管道不可定位
        let mut fds: [c_int; 2] = [-1, -1];
        let ret = crate::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0);

        let off = __lseek(fds[0], 0, 0);
        assert_eq!(off, -1, "__lseek on pipe should return -1 (ESPIPE)");

        crate::close(fds[0]);
        crate::close(fds[1]);
    });

    test!("test_lseek_invalid_whence" {
        // 无效 whence 值
        let mut fds: [c_int; 2] = [-1, -1];
        let ret = crate::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0);

        let off = __lseek(fds[0], 0, 99);
        assert_eq!(off, -1, "__lseek with invalid whence should return -1");

        crate::close(fds[0]);
        crate::close(fds[1]);
    });

    test!("test_lseek_public_and_internal_equivalent_invalid" {
        // lseek 和 __lseek 对无效 fd 应返回相同结果
        let r1 = lseek(-1, 0, 0);
        let r2 = __lseek(-1, 0, 0);
        assert_eq!(r1, r2, "lseek and __lseek should return same result for invalid fd");
    });

    test!("test_lseek_constants" {
        assert_eq!(crate::types::SEEK_SET, 0);
        assert_eq!(crate::types::SEEK_CUR, 1);
        assert_eq!(crate::types::SEEK_END, 2);
    });
}
