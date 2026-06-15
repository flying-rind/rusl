//! pwritev 函数集成测试

use core::ffi::c_void;
use test_framework::test;

test!("test_pwritev_invalid_fd" {
        // pwritev 错误处理: 无效 fd
    {
        let data: [u8; 4] = [1, 2, 3, 4];
        let iov: [super::iovec; 1] = [
            super::iovec { iov_base: data.as_ptr() as *mut c_void, iov_len: 4 },
        ];
        let ret = super::pwritev(-1, iov.as_ptr(), 1, 0);
        assert_eq!(ret, -1, "pwritev on fd=-1 should return -1 (EBADF)");
    }
});

test!("test_pwritev_on_pipe" {
        // pwritev 错误处理: 管道不可 seek
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        let data: [u8; 4] = [1, 2, 3, 4];
        let iov: [super::iovec; 1] = [
            super::iovec { iov_base: data.as_ptr() as *mut c_void, iov_len: 4 },
        ];
        let r = super::pwritev(fds[1], iov.as_ptr(), 1, 0);
        assert_eq!(r, -1, "pwritev on pipe should return -1 (ESPIPE)");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_pwritev_zero_count" {
        // pwritev 边界条件: count=0, 对不可 seek 的 fd (stdout)
        // 内核在验证 fd seekability 后才检查参数, 所以可能返回 -1 (ESPIPE)
    {
        let ret = super::pwritev(super::STDOUT_FILENO, core::ptr::null(), 0, 0);
        assert!(ret == -1 || ret == 0, "pwritev with count=0 on non-seekable fd, got {}", ret);
    }
});

test!("test_pwritev_zero_len" {
        // pwritev 边界条件: iov_len=0, 对不可 seek 的 fd (stdout)
    {
        let dummy: [u8; 1] = [0];
        let iov: [super::iovec; 1] = [
            super::iovec { iov_base: dummy.as_ptr() as *mut c_void, iov_len: 0 },
        ];
        let ret = super::pwritev(super::STDOUT_FILENO, iov.as_ptr(), 1, 0);
        assert!(ret == -1 || ret == 0, "pwritev with iov_len=0 on non-seekable fd, got {}", ret);
    }
});

test!("test_pwritev_null_iov_zero_count" {
        // pwritev 边界条件: NULL iov 但 count=0
        // 注意: 内核可能先校验 iov 指针再检查 count, 所以可能返回 -1 (EFAULT)
    {
        let ret = super::pwritev(super::STDOUT_FILENO, core::ptr::null(), 0, 0);
        // Linux 内核可能验证指针再检查 count, 返回 -1; 也可能返回 0
        assert!(ret == -1 || ret == 0, "pwritev with NULL iov and count=0 should return -1 or 0, got {}", ret);
    }
});
