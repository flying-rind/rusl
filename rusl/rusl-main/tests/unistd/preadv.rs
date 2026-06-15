//! preadv 函数集成测试

use core::ffi::c_void;
use test_framework::test;

test!("test_preadv_invalid_fd" {
        // preadv 错误处理: 无效 fd
    {
        let mut buf: [u8; 4] = [0; 4];
        let iov: [super::iovec; 1] = [
            super::iovec { iov_base: buf.as_mut_ptr() as *mut c_void, iov_len: 4 },
        ];
        let ret = super::preadv(-1, iov.as_ptr(), 1, 0);
        assert_eq!(ret, -1, "preadv on fd=-1 should return -1 (EBADF)");
    }
});

test!("test_preadv_on_pipe" {
        // preadv 错误处理: 管道不可 seek
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        let mut buf: [u8; 4] = [0; 4];
        let iov: [super::iovec; 1] = [
            super::iovec { iov_base: buf.as_mut_ptr() as *mut c_void, iov_len: 4 },
        ];
        let r = super::preadv(fds[0], iov.as_ptr(), 1, 0);
        assert_eq!(r, -1, "preadv on pipe should return -1 (ESPIPE)");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_preadv_zero_count" {
        // preadv 边界条件: count=0, NULL iov
        // musl 直接调用内核，内核在 NULL iov 时返回 -1 (EFAULT)，即使 count=0
    {
        let ret = super::preadv(super::STDIN_FILENO, core::ptr::null(), 0, 0);
        assert!(ret == 0 || ret == -1, "preadv with count=0, NULL iov");
    }
});

test!("test_preadv_zero_len" {
        // preadv 边界条件: iov_len=0, count=0
        // musl 直接调用内核，结果取决于内核行为（0 或 -1）
    {
        let mut dummy: [u8; 1] = [0];
        let iov: [super::iovec; 1] = [
            super::iovec { iov_base: dummy.as_mut_ptr() as *mut c_void, iov_len: 0 },
        ];
        let ret = super::preadv(super::STDIN_FILENO, iov.as_ptr(), 0, 0);
        assert!(ret == 0 || ret == -1, "preadv with iov_len=0, count=0");
    }
});

test!("test_preadv_null_iov_zero_count" {
        // preadv 边界条件: NULL iov 但 count=0
        // 注意: 内核可能先校验 iov 指针再检查 count, 所以可能返回 -1 (EFAULT)
    {
        let ret = super::preadv(super::STDIN_FILENO, core::ptr::null(), 0, 0);
        assert!(ret == -1 || ret == 0, "preadv with NULL iov and count=0 should return -1 or 0, got {}", ret);
    }
});
