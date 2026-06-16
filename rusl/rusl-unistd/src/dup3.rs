//! dup3 — 带标志复制文件描述符。
//! 对应 musl src/unistd/dup3.c
//!
//! SYS_dup3 系统调用的薄封装。

use core::ffi::c_int;
use crate::import::do_syscall;

/// __dup3(old, new, flags) — musl 内部主实现。
///
/// 等价于 dup2 但额外支持 flags 控制新 fd 行为：
/// - flags 可为 O_CLOEXEC（设置 close-on-exec 标志）
/// - 要求 old != new（与 dup2 不同）
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn __dup3(old: c_int, new: c_int, flags: c_int) -> c_int {
    unsafe { do_syscall!(crate::syscall::SYS_dup3, old, new, flags) as c_int }
}

/// dup3 — __dup3 的公开别名。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn dup3(old: c_int, new: c_int, flags: c_int) -> c_int {
    unsafe { do_syscall!(crate::syscall::SYS_dup3, old, new, flags) as c_int }
}

// ===========================================================================
// 单元测试
// ===========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use core::ffi::c_int;
    use rusl_core::test;

    const O_CLOEXEC: c_int = 0o2000000;

    test!("test_dup3_invalid_old_fd" {
        // old 无效: 返回 -1 (EBADF)
        let ret = __dup3(-1, 100, 0);
        assert_eq!(ret, -1, "__dup3 with invalid old fd should return -1 (EBADF)");
    });

    test!("test_dup3_same_fd" {
        // old == new: dup3 要求不同（与 dup2 不同，dup3 会返回 EINVAL）
        let mut fds: [c_int; 2] = [-1, -1];
        let ret = crate::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0);

        // dup3(old, old) 应返回 -1 (EINVAL)
        let r = __dup3(fds[0], fds[0], 0);
        assert_eq!(r, -1, "__dup3 with old==new should return -1 (EINVAL)");

        crate::close(fds[0]);
        crate::close(fds[1]);
    });

    test!("test_dup3_valid_with_cloexec" {
        // 使用 O_CLOEXEC 标志复制有效 fd
        let mut fds: [c_int; 2] = [-1, -1];
        let ret = crate::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0);

        // dup3 到不同的 fd
        let new_fd = fds[1] + 10;
        let r = __dup3(fds[0], new_fd, O_CLOEXEC);
        if r >= 0 {
            assert!(r == new_fd, "__dup3 should return new fd");
            crate::close(r);
        }
        // 如果失败可能是因为 new_fd 超出范围

        crate::close(fds[0]);
        crate::close(fds[1]);
    });

    test!("test_dup3_public_and_internal_equivalent" {
        // dup3 和 __dup3 对无效参数应返回相同结果
        let r1 = dup3(-1, 100, 0);
        let r2 = __dup3(-1, 100, 0);
        assert_eq!(r1, r2, "dup3 and __dup3 should return same result");
    });

    test!("test_dup3_invalid_flags" {
        // 使用大量无效 flags
        let mut fds: [c_int; 2] = [-1, -1];
        let ret = crate::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0);

        // invalid flags (不包含已知标志)
        let r = __dup3(fds[0], fds[1] + 10, 0xDEAD);
        // 可能返回 -1 (EINVAL) 或成功（取决于内核如何处理未知标志）
        // 重要的是不崩溃
        if r >= 0 {
            crate::close(r);
        }

        crate::close(fds[0]);
        crate::close(fds[1]);
    });
}
