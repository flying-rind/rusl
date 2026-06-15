//! writev 函数集成测试

use core::ffi::c_void;
use test_framework::test;

test!("test_writev_invalid_fd" {
        // writev 错误处理: 无效 fd
    {
        let data1: [u8; 4] = [1, 2, 3, 4];
        let data2: [u8; 4] = [5, 6, 7, 8];
        let iov: [super::iovec; 2] = [
            super::iovec { iov_base: data1.as_ptr() as *mut c_void, iov_len: 4 },
            super::iovec { iov_base: data2.as_ptr() as *mut c_void, iov_len: 4 },
        ];
        let ret = super::writev(-1, iov.as_ptr(), 2);
        assert_eq!(ret, -1, "writev on fd=-1 should return -1 (EBADF)");
    }
});

test!("test_writev_zero_count" {
        // writev 边界条件: count=0
    {
        let ret = super::writev(super::STDOUT_FILENO, core::ptr::null(), 0);
        assert_eq!(ret, 0, "writev with count=0 should return 0");
    }
});

test!("test_writev_to_pipe" {
        // writev 基本功能: 聚集写入到管道
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        let data1: [u8; 3] = [0xAA, 0xBB, 0xCC];
        let data2: [u8; 3] = [0xDD, 0xEE, 0xFF];
        let iov: [super::iovec; 2] = [
            super::iovec { iov_base: data1.as_ptr() as *mut c_void, iov_len: 3 },
            super::iovec { iov_base: data2.as_ptr() as *mut c_void, iov_len: 3 },
        ];
        let n = super::writev(fds[1], iov.as_ptr(), 2);
        assert_eq!(n, 6, "writev should return 6 total bytes");

        // 读取并验证连接后的数据
        let mut buf: [u8; 8] = [0; 8];
        let r = super::read(fds[0], buf.as_mut_ptr() as *mut c_void, 8);
        assert_eq!(r, 6, "should read 6 bytes");
        assert_eq!(&buf[..3], &data1[..], "first part should match");
        assert_eq!(&buf[3..6], &data2[..], "second part should match");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_writev_single_iov" {
        // writev 基本功能: 单 iovec 写入
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        let data: [u8; 5] = [1, 2, 3, 4, 5];
        let iov: [super::iovec; 1] = [
            super::iovec { iov_base: data.as_ptr() as *mut c_void, iov_len: 5 },
        ];
        let n = super::writev(fds[1], iov.as_ptr(), 1);
        assert_eq!(n, 5, "writev should return 5 bytes");

        let mut buf: [u8; 8] = [0; 8];
        let r = super::read(fds[0], buf.as_mut_ptr() as *mut c_void, 8);
        assert_eq!(r, 5, "should read 5 bytes");
        assert_eq!(&buf[..5], &data[..], "data should match");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_writev_zero_len_iov" {
        // writev 边界条件: iov_len=0 的项
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        let dummy: [u8; 1] = [0];
        let data: [u8; 3] = [0x11, 0x22, 0x33];
        let iov: [super::iovec; 2] = [
            super::iovec { iov_base: dummy.as_ptr() as *mut c_void, iov_len: 0 },
            super::iovec { iov_base: data.as_ptr() as *mut c_void, iov_len: 3 },
        ];
        let n = super::writev(fds[1], iov.as_ptr(), 2);
        assert_eq!(n, 3, "writev should return 3 bytes (skipping len=0)");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});
