//! pipe2 函数集成测试

use core::ffi::c_void;
use test_framework::test;

// Linux flag 常量
const O_NONBLOCK: super::c_int = 0o4000;

test!("test_pipe2_zero_flags" {
        // pipe2 基本功能: flags=0（等价于 pipe）
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe2(fds.as_mut_ptr(), 0);
        assert_eq!(ret, 0, "pipe2 with flags=0 should return 0");
        assert!(fds[0] >= 0, "read fd should be valid");
        assert!(fds[1] >= 0, "write fd should be valid");
        assert_ne!(fds[0], fds[1], "read and write fds should differ");

        // 验证可读写
        let data: [u8; 4] = [1, 2, 3, 4];
        let n = super::write(fds[1], data.as_ptr() as *const c_void, 4);
        assert_eq!(n, 4, "should write 4 bytes");

        let mut buf: [u8; 4] = [0; 4];
        let r = super::read(fds[0], buf.as_mut_ptr() as *mut c_void, 4);
        assert_eq!(r, 4, "should read 4 bytes");
        assert_eq!(&buf[..], &data[..], "data should match");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_pipe2_nonblock" {
        // pipe2 基本功能: O_NONBLOCK 标志
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe2(fds.as_mut_ptr(), O_NONBLOCK);
        assert_eq!(ret, 0, "pipe2 with O_NONBLOCK should return 0");
        assert!(fds[0] >= 0, "read fd should be valid");
        assert!(fds[1] >= 0, "write fd should be valid");

        // 非阻塞读取空管道应返回 -1 (EAGAIN/EWOULDBLOCK)
        let mut buf: [u8; 8] = [0; 8];
        let r = super::read(fds[0], buf.as_mut_ptr() as *mut c_void, 8);
        assert_eq!(r, -1, "non-blocking read on empty pipe should return -1 (EAGAIN)");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_pipe2_nonblock_read_after_write" {
        // pipe2 基本功能: 写入后非阻塞读取
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe2(fds.as_mut_ptr(), O_NONBLOCK);
        assert_eq!(ret, 0, "pipe2 with O_NONBLOCK should succeed");

        // 写入一些数据
        let data: [u8; 5] = [10, 20, 30, 40, 50];
        let w = super::write(fds[1], data.as_ptr() as *const c_void, 5);
        assert_eq!(w, 5, "should write 5 bytes");

        // 非阻塞读取应有数据
        let mut buf: [u8; 16] = [0; 16];
        let r = super::read(fds[0], buf.as_mut_ptr() as *mut c_void, 16);
        assert_eq!(r, 5, "should read 5 bytes");
        assert_eq!(&buf[..5], &data[..], "data should match");

        // 再次读取应返回 -1 (EAGAIN)
        let mut buf2: [u8; 4] = [0; 4];
        let r2 = super::read(fds[0], buf2.as_mut_ptr() as *mut c_void, 4);
        assert_eq!(r2, -1, "second non-blocking read should return -1 (EAGAIN)");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});
