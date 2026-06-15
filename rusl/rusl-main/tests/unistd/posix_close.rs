//! posix_close 函数集成测试

use test_framework::test;

test!("test_posix_close_invalid_fd" {
        // posix_close 错误处理: 无效 fd
    {
        let ret = super::posix_close(-1, 0);
        assert_eq!(ret, -1, "posix_close on fd=-1 should return -1 (EBADF)");
    }
});

test!("test_posix_close_valid_fd" {
        // posix_close 基本功能: 关闭有效管道 fd
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        let r = super::posix_close(fds[1], 0);
        assert_eq!(r, 0, "posix_close on valid fd should return 0");

        let r2 = super::posix_close(fds[0], 0);
        assert_eq!(r2, 0, "posix_close on valid fd should return 0");
    }
});

test!("test_posix_close_double_close" {
        // posix_close 边界条件: 重复关闭同一 fd (double close)
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 第一次关闭成功
        let r = super::posix_close(fds[0], 0);
        assert_eq!(r, 0, "first posix_close should succeed");

        // 第二次关闭应失败
        let r2 = super::posix_close(fds[0], 0);
        assert_eq!(r2, -1, "double posix_close should return -1 (EBADF)");

        super::close(fds[1]);
    }
});

test!("test_posix_close_vs_close" {
        // posix_close 基本功能: 与 close 行为一致
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // posix_close 关闭写端, close 关闭读端
        let r1 = super::posix_close(fds[1], 0);
        assert_eq!(r1, 0, "posix_close should succeed");

        let r2 = super::close(fds[0]);
        assert_eq!(r2, 0, "close should succeed");
    }
});
