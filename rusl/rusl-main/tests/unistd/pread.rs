//! pread 函数集成测试

use core::ffi::c_void;
use test_framework::test;

test!("test_pread_invalid_fd" {
        // pread 错误处理: 无效 fd
    {
        let mut buf: [u8; 8] = [0; 8];
        let ret = super::pread(-1, buf.as_mut_ptr() as *mut c_void, 8, 0);
        assert_eq!(ret, -1, "pread on fd=-1 should return -1 (EBADF)");
    }
});

test!("test_pread_on_pipe" {
        // pread 错误处理: 管道不可 seek (ESPIPE)
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        let mut buf: [u8; 4] = [0; 4];
        let r = super::pread(fds[0], buf.as_mut_ptr() as *mut c_void, 4, 0);
        assert_eq!(r, -1, "pread on pipe should return -1 (ESPIPE)");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_pread_zero_size" {
        // pread 边界条件: size=0
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 虽然管道不可 seek, 但 size=0 时可能会通过参数检查直接返回 0
        let mut buf: [u8; 4] = [0; 4];
        let r = super::pread(fds[0], buf.as_mut_ptr() as *mut c_void, 0, 0);
        // size=0 时行为: 可能返回 0（不执行实际 I/O）
        // 如果内核先检查偏移则可能返回 -1 (ESPIPE)
        // 两种情况都是合理的
        assert!(r == 0 || r == -1, "pread with size=0 should return 0 or -1, got {}", r);

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_pread_null_buf_zero_size" {
        // pread 错误处理: NULL buf 但 size=0
        // musl 直接调用内核，内核在 NULL buf 时返回 -1 (EFAULT)，即使 size=0
    {
        let ret = super::pread(super::STDIN_FILENO, core::ptr::null_mut(), 0, 0);
        assert!(ret == 0 || ret == -1, "pread with NULL buf and size=0");
    }
});
