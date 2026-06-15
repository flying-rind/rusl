//! fsync 函数集成测试

use test_framework::test;

test!("test_fsync_invalid_fd" {
        // fsync 错误处理: 无效 fd
    {
        let ret = super::fsync(-1);
        assert_eq!(ret, -1, "fsync on fd=-1 should return -1 (EBADF)");
    }
});

test!("test_fsync_on_pipe" {
        // fsync 基本功能: 对管道 fd 调用
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 对管道 fsync 在 Linux 上应返回 -1 (EINVAL)
        let r = super::fsync(fds[0]);
        assert_eq!(r, -1, "fsync on pipe should return -1 (EINVAL)");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_fsync_stdout" {
        // fsync 基本功能: 对 stdout 调用
    {
        let ret = super::fsync(super::STDOUT_FILENO);
        // 终端 stdout 上 fsync 取决于 stdout 是否重定向
        // 如果是终端, 可能返回 -1 (EINVAL)
        // 不作严格断言, 只验证不会 crash
        let _ = ret;
    }
});

test!("test_fsync_closed_fd" {
        // fsync 边界条件: 对已关闭的 fd 调用
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        super::close(fds[0]);

        let r = super::fsync(fds[0]);
        assert_eq!(r, -1, "fsync on closed fd should return -1 (EBADF)");

        super::close(fds[1]);
    }
});
