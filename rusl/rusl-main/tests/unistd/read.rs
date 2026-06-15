//! read 函数集成测试

use core::ffi::c_void;
use test_framework::test;

test!("test_read_invalid_fd" {
        // read 错误处理: 无效 fd
    {
        let mut buf: [u8; 4] = [0; 4];
        let ret = super::read(-1, buf.as_mut_ptr() as *mut c_void, 4);
        assert_eq!(ret, -1, "read from fd=-1 should return -1 (EBADF)");
    }
});

test!("test_read_zero_count" {
        // read 边界条件: count 为 0
    {
        let mut buf: [u8; 4] = [0; 4];
        let ret = super::read(super::STDIN_FILENO, buf.as_mut_ptr() as *mut c_void, 0);
        assert_eq!(ret, 0, "read with count=0 should return 0");
    }
});

test!("test_read_from_pipe" {
        // read 基本功能: 从管道读取数据
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 写入数据
        let data: [u8; 8] = [10, 20, 30, 40, 50, 60, 70, 80];
        let w = super::write(fds[1], data.as_ptr() as *const c_void, 8);
        assert_eq!(w, 8, "write should succeed");

        // 读取数据
        let mut buf: [u8; 16] = [0; 16];
        let r = super::read(fds[0], buf.as_mut_ptr() as *mut c_void, 16);
        assert_eq!(r, 8, "read should return 8 bytes");
        assert_eq!(&buf[..8], &data[..], "data should match");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_read_closed_pipe" {
        // read 基本功能: 从已关闭的管道读取 (应返回 0, 即 EOF)
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 先写入一些数据再关闭写端, 确保读端能读到数据后再遇到 EOF
        let data: [u8; 3] = [1, 2, 3];
        super::write(fds[1], data.as_ptr() as *const c_void, 3);
        super::close(fds[1]);

        // 读取数据
        let mut buf: [u8; 8] = [0; 8];
        let r = super::read(fds[0], buf.as_mut_ptr() as *mut c_void, 8);
        assert_eq!(r, 3, "should read 3 bytes");
        assert_eq!(&buf[..3], &data[..], "data should match");

        // 再次读取应返回 0 (EOF)
        let mut buf2: [u8; 4] = [0; 4];
        let r2 = super::read(fds[0], buf2.as_mut_ptr() as *mut c_void, 4);
        assert_eq!(r2, 0, "read on closed pipe should return 0 (EOF)");

        super::close(fds[0]);
    }
});

test!("test_read_null_buf_zero_count" {
        // read 边界条件: NULL buf 但 count 为 0
    {
        let ret = super::read(super::STDIN_FILENO, core::ptr::null_mut(), 0);
        assert_eq!(ret, 0, "read with NULL buf and count=0 should return 0");
    }
});
