//! fdatasync 函数集成测试

use test_framework::test;

test!("test_fdatasync_invalid_fd" {
        // fdatasync 错误处理: 无效 fd
    {
        let ret = super::fdatasync(-1);
        assert_eq!(ret, -1, "fdatasync on fd=-1 should return -1 (EBADF)");
    }
});

test!("test_fdatasync_on_pipe" {
        // fdatasync 基本功能: 对管道 fd 调用
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 对管道 fdatasync 在 Linux 上应返回 -1 (EINVAL)
        let r = super::fdatasync(fds[0]);
        assert_eq!(r, -1, "fdatasync on pipe should return -1 (EINVAL)");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_fdatasync_stdout" {
        // fdatasync 基本功能: 对 stdout 调用
    {
        let ret = super::fdatasync(super::STDOUT_FILENO);
        // stdout 上 fdatasync 取决于类型
        // 仅验证不会 crash
        let _ = ret;
    }
});

test!("test_fdatasync_closed_fd" {
        // fdatasync 边界条件: 对已关闭的 fd 调用
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        super::close(fds[1]);

        let r = super::fdatasync(fds[1]);
        assert_eq!(r, -1, "fdatasync on closed fd should return -1 (EBADF)");

        super::close(fds[0]);
    }
});
