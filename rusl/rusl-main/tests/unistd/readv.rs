//! readv 函数集成测试

use core::ffi::c_void;
use test_framework::test;

test!("test_readv_invalid_fd" {
        // readv 错误处理: 无效 fd
    {
        let mut buf1: [u8; 4] = [0; 4];
        let mut buf2: [u8; 4] = [0; 4];
        let iov: [super::iovec; 2] = [
            super::iovec { iov_base: buf1.as_mut_ptr() as *mut c_void, iov_len: 4 },
            super::iovec { iov_base: buf2.as_mut_ptr() as *mut c_void, iov_len: 4 },
        ];
        let ret = super::readv(-1, iov.as_ptr(), 2);
        assert_eq!(ret, -1, "readv on fd=-1 should return -1 (EBADF)");
    }
});

test!("test_readv_zero_count" {
        // readv 边界条件: count=0
    {
        let ret = super::readv(super::STDIN_FILENO, core::ptr::null(), 0);
        assert_eq!(ret, 0, "readv with count=0 should return 0");
    }
});

test!("test_readv_null_iov_zero_count_valid_fd" {
        // readv 边界条件: NULL iov 但 count=0, 使用有效 fd
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        let r = super::readv(fds[0], core::ptr::null(), 0);
        assert_eq!(r, 0, "readv with NULL iov and count=0 should return 0");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_readv_from_pipe" {
        // readv 基本功能: 从管道分散读取
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 写入 8 字节
        let data: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        let w = super::write(fds[1], data.as_ptr() as *const c_void, 8);
        assert_eq!(w, 8, "should write 8 bytes");

        // 分散读取到两个 4 字节缓冲区
        let mut buf1: [u8; 4] = [0; 4];
        let mut buf2: [u8; 4] = [0; 4];
        let iov: [super::iovec; 2] = [
            super::iovec { iov_base: buf1.as_mut_ptr() as *mut c_void, iov_len: 4 },
            super::iovec { iov_base: buf2.as_mut_ptr() as *mut c_void, iov_len: 4 },
        ];
        let r = super::readv(fds[0], iov.as_ptr(), 2);
        assert_eq!(r, 8, "readv should return 8 total bytes");
        assert_eq!(&buf1[..], &data[..4], "first buffer should match");
        assert_eq!(&buf2[..], &data[4..], "second buffer should match");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_readv_single_iov" {
        // readv 基本功能: 单 iovec 读取
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        let data: [u8; 5] = [10, 20, 30, 40, 50];
        super::write(fds[1], data.as_ptr() as *const c_void, 5);

        let mut buf: [u8; 8] = [0; 8];
        let iov: [super::iovec; 1] = [
            super::iovec { iov_base: buf.as_mut_ptr() as *mut c_void, iov_len: 8 },
        ];
        let r = super::readv(fds[0], iov.as_ptr(), 1);
        assert_eq!(r, 5, "readv should return 5 bytes");
        assert_eq!(&buf[..5], &data[..], "data should match");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_readv_zero_len_iov" {
        // readv 基本功能: iov_len=0 的 iovec 项
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        let data: [u8; 3] = [7, 8, 9];
        super::write(fds[1], data.as_ptr() as *const c_void, 3);

        let mut dummy: [u8; 1] = [0];
        let mut buf: [u8; 4] = [0; 4];
        let iov: [super::iovec; 2] = [
            super::iovec { iov_base: dummy.as_mut_ptr() as *mut c_void, iov_len: 0 },
            super::iovec { iov_base: buf.as_mut_ptr() as *mut c_void, iov_len: 4 },
        ];
        let r = super::readv(fds[0], iov.as_ptr(), 2);
        assert_eq!(r, 3, "readv should return 3 bytes (skipping len=0)");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});
