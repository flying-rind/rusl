//! close 函数集成测试

use test_framework::test;

test!("test_close_invalid_fd" {
        // close 错误处理: 无效 fd
    {
        let ret = super::close(-1);
        assert_eq!(ret, -1, "close(-1) should return -1 (EBADF)");
    }
});

test!("test_close_valid_fd" {
        // close 基本功能: 关闭有效管道 fd
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 关闭写端应成功
        let r = super::close(fds[1]);
        assert_eq!(r, 0, "close on valid fd should return 0");

        // 关闭读端也应成功
        let r2 = super::close(fds[0]);
        assert_eq!(r2, 0, "close on valid fd should return 0");
    }
});

test!("test_close_double_close" {
        // close 边界条件: 重复关闭同一 fd (double close)
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 第一次关闭应成功
        let r = super::close(fds[0]);
        assert_eq!(r, 0, "first close should succeed");

        // 第二次关闭应失败 (EBADF)
        let r2 = super::close(fds[0]);
        assert_eq!(r2, -1, "double close should return -1 (EBADF)");

        super::close(fds[1]);
    }
});

test!("test_close_stdin" {
        // close 边界条件: 关闭 fd 0 (stdin)
    {
        // /dev/null 以只读打开来获得一个 fd
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");
        assert!(fds[0] >= 0, "pipe read fd should be valid");

        let r = super::close(fds[0]);
        assert_eq!(r, 0, "closing pipe read end should succeed");

        super::close(fds[1]);
    }
});
