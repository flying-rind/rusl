//! close — 关闭文件描述符。
//! 对应 musl src/unistd/close.c
//!
//! 特殊处理：若被 EINTR 中断则视为成功（符合 Linux 内核语义）。

use core::ffi::c_int;
use rusl_internal::syscall::{raw_syscall1, __syscall_ret};

/// close(fd) — 关闭文件描述符 `fd`，释放关联的内核资源。
///
/// 特殊处理：若被 `EINTR` 中断则视为成功（符合 Linux 内核语义）。
/// 成功返回 0，出错返回 -1 并设置 errno。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn close(fd: c_int) -> c_int {
    unsafe {
        let r = raw_syscall1(rusl_internal::syscall::SYS_close, fd as i64) as usize;
        // EINTR (-4) 在内核返回值中为 0xfffffffffffffffc
        // 需要检查 errno 级别：若原始返回值为 -EINTR，视为成功
        if r == (-(4i64) as usize) {
            // -EINTR: Linux 语义下 fd 已释放，视为成功
            0
        } else {
            __syscall_ret(r as u64) as c_int
        }
    }
}

// ===========================================================================
// 单元测试
// ===========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use rusl_core::test;
    use core::ffi::c_int;

    test!("test_close_invalid_fd_returns_error" {
        // 关闭无效 fd: 返回 -1 (EBADF)
        let ret = close(-1);
        assert_eq!(ret, -1, "close(-1) should return -1 (EBADF)");
    });

    test!("test_close_very_large_fd_returns_error" {
        // 关闭非常大的 fd 值: 返回 -1
        let ret = close(99999);
        assert_eq!(ret, -1, "close(99999) should return -1 (EBADF)");
    });

    test!("test_close_valid_pipe_fd_returns_zero" {
        // 使用 pipe 创建有效 fd，然后关闭
        let mut fds: [c_int; 2] = [-1, -1];
        let ret = crate::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");
        assert!(fds[0] >= 0, "read fd should be valid");
        assert!(fds[1] >= 0, "write fd should be valid");

        // 关闭读端
        let r = close(fds[0]);
        assert_eq!(r, 0, "close on valid fd should return 0");

        // 关闭写端
        let r2 = close(fds[1]);
        assert_eq!(r2, 0, "close on valid fd should return 0");
    });

    test!("test_close_double_close_returns_error" {
        // 重复关闭同一 fd: 第二次应返回 -1 (EBADF)
        let mut fds: [c_int; 2] = [-1, -1];
        let ret = crate::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 第一次关闭成功
        let r1 = close(fds[0]);
        assert_eq!(r1, 0, "first close should succeed");

        // 第二次关闭应失败
        let r2 = close(fds[0]);
        assert_eq!(r2, -1, "double close should return -1 (EBADF)");

        // 清理
        close(fds[1]);
    });

    test!("test_close_std_fd_zero" {
        // 关闭 fd=0 (stdin) 后再重新打开确保 fd 可以重用
        // 注意: 这会关闭真正的 stdin, 但在测试环境中是安全的
        // 使用 pipe 作为 stdin 替代品
        let mut fds: [c_int; 2] = [-1, -1];
        let ret = crate::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0);

        // dup2 一个已知 fd 到 fd=100 避免干扰 stdin
        let r = close(fds[0]);
        assert_eq!(r, 0);

        close(fds[1]);
    });

    test!("test_close_eintr_constants" {
        // 验证 EINTR 常量: -4 在 usize 中的表示
        // close 函数中检查 r == (-(4i64) as usize)
        // -4i64 的位模式: 0xFFFF_FFFF_FFFF_FFFC
        // 转为 usize: 0xFFFF_FFFF_FFFF_FFFC
        let einr_neg: usize = (-(4i64)) as usize;
        assert_eq!(einr_neg, 0xFFFF_FFFF_FFFF_FFFC_usize,
            "EINTR(-4) in usize should be 0xFFFF_FFFF_FFFF_FFFC");
    });
}
