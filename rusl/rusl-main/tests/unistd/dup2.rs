//! dup2 函数集成测试

use core::ffi::c_void;
use test_framework::test;

test!("test_dup2_invalid_old" {
        // dup2 错误处理: 无效 old fd
    {
        let ret = super::dup2(-1, 10);
        assert_eq!(ret, -1, "dup2 with invalid old should return -1 (EBADF)");
    }
});

test!("test_dup2_basic" {
        // dup2 基本功能: 复制 fd 到指定编号
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 复制写端到新 fd (比如 100 不太可能与现有 fd 冲突)
        // 使用较大编号以避免冲突
        let new_fd = super::dup(fds[0]);
        assert!(new_fd >= 0, "dup should succeed");

        let new_fd2 = super::dup2(fds[0], new_fd);
        assert_eq!(new_fd2, new_fd, "dup2 should return target fd");

        // 通过新 fd 验证功能: 写入管道, 通过 dup2 目标 fd 读取
        let data: [u8; 4] = [0x11, 0x22, 0x33, 0x44];
        let w = super::write(fds[1], data.as_ptr() as *const c_void, 4);
        assert_eq!(w, 4, "write should succeed");

        let mut buf: [u8; 8] = [0; 8];
        let r = super::read(new_fd2, buf.as_mut_ptr() as *mut c_void, 8);
        assert_eq!(r, 4, "read via dup2'd fd should succeed");
        assert_eq!(&buf[..4], &data[..], "data should match");

        super::close(new_fd2);
        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_dup2_same_fd" {
        // dup2 边界条件: old == new (有效 fd)
    {
        // dup2(fd, fd) 当 fd 有效时应返回 fd
        let ret = super::dup2(super::STDOUT_FILENO, super::STDOUT_FILENO);
        assert_eq!(ret, super::STDOUT_FILENO, "dup2(old, old) with valid old should return old");
    }
});

test!("test_dup2_redirect" {
        // dup2 基本功能: 重定向, 关闭原有 new fd
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 先 dup 读端到 safe_fd
        let safe_fd = super::dup(fds[0]);
        assert!(safe_fd >= 0, "first dup should succeed");

        // 再将同一个读端 dup2 到 safe_fd, 覆盖原 fd
        let ret2 = super::dup2(fds[0], safe_fd);
        assert_eq!(ret2, safe_fd, "dup2 should return target fd");

        // 验证通过新 fd 可以读取管道数据
        let data: [u8; 3] = [0xAA, 0xBB, 0xCC];
        super::write(fds[1], data.as_ptr() as *const c_void, 3);

        let mut buf: [u8; 4] = [0; 4];
        let r = super::read(safe_fd, buf.as_mut_ptr() as *mut c_void, 4);
        assert_eq!(r, 3, "should read 3 bytes");

        super::close(safe_fd);
        super::close(fds[0]);
        super::close(fds[1]);
    }
});
