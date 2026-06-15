//! pwrite 函数集成测试

use core::ffi::c_void;
use test_framework::test;

test!("test_pwrite_invalid_fd" {
        // pwrite 错误处理: 无效 fd
    {
        let data: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        let ret = super::pwrite(-1, data.as_ptr() as *const c_void, 8, 0);
        assert_eq!(ret, -1, "pwrite on fd=-1 should return -1 (EBADF)");
    }
});

test!("test_pwrite_on_pipe" {
        // pwrite 错误处理: 管道不可 seek
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        let data: [u8; 4] = [1, 2, 3, 4];
        let r = super::pwrite(fds[1], data.as_ptr() as *const c_void, 4, 0);
        assert_eq!(r, -1, "pwrite on pipe should return -1 (ESPIPE)");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_pwrite_zero_size" {
        // pwrite 边界条件: size=0, 对不可 seek 的 fd (stdout)
        // 内核在验证 fd seekability 后才检查 size, 所以可能返回 -1 (ESPIPE)
    {
        let data: [u8; 4] = [1, 2, 3, 4];
        let ret = super::pwrite(super::STDOUT_FILENO, data.as_ptr() as *const c_void, 0, 0);
        // 不可 seek 的 fd 会返回 -1 (ESPIPE)
        assert!(ret == -1 || ret == 0, "pwrite with size=0 on non-seekable fd, got {}", ret);
    }
});

test!("test_pwrite_null_buf_zero_size" {
        // pwrite 边界条件: NULL buf 但 size=0, 对不可 seek 的 fd
    {
        let ret = super::pwrite(super::STDOUT_FILENO, core::ptr::null(), 0, 0);
        assert!(ret == -1 || ret == 0, "pwrite with NULL buf and size=0 on non-seekable fd, got {}", ret);
    }
});

test!("test_pwrite_null_buf_positive_size" {
        // pwrite 错误处理: NULL buf 且 size>0
    {
        let ret = super::pwrite(super::STDOUT_FILENO, core::ptr::null(), 4, 0);
        assert_eq!(ret, -1, "pwrite with NULL buf and size>0 should return -1 (EFAULT)");
    }
});
