//! pipe 函数集成测试

use core::ffi::c_void;
use test_framework::test;

test!("test_pipe_basic" {
        // pipe 基本功能: 创建管道并验证 fd
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe should return 0 on success");
        assert!(fds[0] >= 0, "read fd should be valid (non-negative)");
        assert!(fds[1] >= 0, "write fd should be valid (non-negative)");
        assert_ne!(fds[0], fds[1], "read and write fds should differ");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_pipe_read_write" {
        // pipe 基本功能: 通过管道读写数据
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe should be created");

        // 写入数据
        let write_data: [u8; 7] = [10, 20, 30, 40, 50, 60, 70];
        let n = super::write(fds[1], write_data.as_ptr() as *const c_void, 7);
        assert_eq!(n, 7, "should write 7 bytes");

        // 读取数据
        let mut read_buf: [u8; 16] = [0; 16];
        let r = super::read(fds[0], read_buf.as_mut_ptr() as *mut c_void, 16);
        assert_eq!(r, 7, "should read 7 bytes");
        assert_eq!(&read_buf[..7], &write_data[..], "data should match");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_pipe_two_pipes" {
        // pipe 基本功能: 两次独立创建管道
    {
        let mut fds1: [super::c_int; 2] = [-1, -1];
        let mut fds2: [super::c_int; 2] = [-1, -1];

        let r1 = super::pipe(fds1.as_mut_ptr());
        let r2 = super::pipe(fds2.as_mut_ptr());
        assert_eq!(r1, 0, "first pipe should succeed");
        assert_eq!(r2, 0, "second pipe should succeed");

        // 四个 fd 应互不相同
        assert_ne!(fds1[0], fds2[0], "different pipe read ends should differ");
        assert_ne!(fds1[1], fds2[1], "different pipe write ends should differ");

        super::close(fds1[0]);
        super::close(fds1[1]);
        super::close(fds2[0]);
        super::close(fds2[1]);
    }
});

test!("test_pipe_buffer" {
        // pipe 边界条件: 管道缓冲区大小测试
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe should be created");

        // 写入 1024 字节, 读取 1024 字节
        let data: [u8; 1024] = [0x42; 1024];
        let n = super::write(fds[1], data.as_ptr() as *const c_void, 1024);
        assert_eq!(n, 1024, "should write 1024 bytes to pipe");

        let mut buf: [u8; 1024] = [0; 1024];
        let r = super::read(fds[0], buf.as_mut_ptr() as *mut c_void, 1024);
        assert_eq!(r, 1024, "should read 1024 bytes back");
        assert_eq!(&buf[..], &data[..], "buffer contents should match");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});
