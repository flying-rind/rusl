//! dup 函数集成测试

use core::ffi::c_void;
use test_framework::test;

test!("test_dup_invalid_fd" {
        // dup 错误处理: 无效 fd
    {
        let ret = super::dup(-1);
        assert_eq!(ret, -1, "dup(-1) should return -1 (EBADF)");
    }
});

test!("test_dup_pipe_fd" {
        // dup 基本功能: 复制管道 fd
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 复制读端
        let new_fd = super::dup(fds[0]);
        assert!(new_fd >= 0, "dup should return a valid new fd");
        assert_ne!(new_fd, fds[0], "new fd should differ from original");

        // 通过新 fd 读取应能工作: 向写端写入, 通过新 fd 读取
        let data: [u8; 4] = [0xAA, 0xBB, 0xCC, 0xDD];
        let w = super::write(fds[1], data.as_ptr() as *const c_void, 4);
        assert_eq!(w, 4, "write should succeed");

        let mut buf: [u8; 8] = [0; 8];
        let r = super::read(new_fd, buf.as_mut_ptr() as *mut c_void, 8);
        assert_eq!(r, 4, "read via dup'd fd should succeed");
        assert_eq!(&buf[..4], &data[..], "data should match");

        super::close(new_fd);
        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_dup_stdout" {
        // dup 边界条件: 复制 stdout, 验证新 fd 不为 0/1/2
    {
        let new_fd = super::dup(super::STDOUT_FILENO);
        assert!(new_fd >= 0, "dup(stdout) should return a valid new fd");
        assert!(new_fd > super::STDERR_FILENO,
            "dup'd fd should be greater than 2");

        super::close(new_fd);
    }
});

test!("test_dup_multiple" {
        // dup 边界条件: 多次 dup 获得不同的 fd
    {
        let a = super::dup(super::STDOUT_FILENO);
        let b = super::dup(super::STDOUT_FILENO);
        let c = super::dup(super::STDOUT_FILENO);

        assert!(a >= 0, "first dup should succeed");
        assert!(b >= 0, "second dup should succeed");
        assert!(c >= 0, "third dup should succeed");
        assert_ne!(a, b, "dup'd fds should differ");
        assert_ne!(a, c, "dup'd fds should differ");
        assert_ne!(b, c, "dup'd fds should differ");

        super::close(a);
        super::close(b);
        super::close(c);
    }
});
