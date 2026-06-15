//! write 函数集成测试

use core::ffi::c_void;
use test_framework::test;

test!("test_write_invalid_fd" {
        // write 错误处理: 无效 fd
    {
        let data: [u8; 4] = [1, 2, 3, 4];
        let ret = super::write(-1, data.as_ptr() as *const c_void, 4);
        assert_eq!(ret, -1, "write to fd=-1 should return -1 (EBADF)");
    }
});

test!("test_write_zero_count" {
        // write 边界条件: count 为 0
    {
        let data: [u8; 4] = [1, 2, 3, 4];
        // 向 stdout 写入 0 字节应成功返回 0
        let ret = super::write(super::STDOUT_FILENO, data.as_ptr() as *const c_void, 0);
        assert_eq!(ret, 0, "write with count=0 should return 0");
    }
});

test!("test_write_to_pipe" {
        // write 基本功能: 向管道写入数据
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        let data: [u8; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let n = super::write(fds[1], data.as_ptr() as *const c_void, 10);
        assert_eq!(n, 10, "write should return 10 bytes written");

        // 读取并验证
        let mut buf: [u8; 16] = [0xFF; 16];
        let r = super::read(fds[0], buf.as_mut_ptr() as *mut c_void, 16);
        assert_eq!(r, 10, "should read back 10 bytes");
        assert_eq!(&buf[..10], &data[..], "data should match");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_write_multiple" {
        // write 基本功能: 多次写入管道
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 两次写入
        let data1: [u8; 4] = [1, 2, 3, 4];
        let data2: [u8; 4] = [5, 6, 7, 8];
        let n1 = super::write(fds[1], data1.as_ptr() as *const c_void, 4);
        let n2 = super::write(fds[1], data2.as_ptr() as *const c_void, 4);
        assert_eq!(n1, 4, "first write should return 4");
        assert_eq!(n2, 4, "second write should return 4");

        // 读取全部 8 字节
        let mut buf: [u8; 12] = [0; 12];
        let r = super::read(fds[0], buf.as_mut_ptr() as *mut c_void, 12);
        assert_eq!(r, 8, "should read 8 bytes total");
        assert_eq!(&buf[..4], &data1[..], "first write data should match");
        assert_eq!(&buf[4..8], &data2[..], "second write data should match");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_write_null_buf_zero_count" {
        // write 边界条件: NULL buf 但 count 为 0
    {
        let ret = super::write(super::STDOUT_FILENO, core::ptr::null(), 0);
        assert_eq!(ret, 0, "write with NULL buf and count=0 should return 0");
    }
});
