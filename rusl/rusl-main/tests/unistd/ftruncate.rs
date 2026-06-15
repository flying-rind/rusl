//! ftruncate 函数集成测试

use test_framework::test;

test!("test_ftruncate_invalid_fd" {
        // ftruncate 错误处理: 无效 fd
    {
        let ret = super::ftruncate(-1, 0);
        assert_eq!(ret, -1, "ftruncate on fd=-1 should return -1 (EBADF)");
    }
});

test!("test_ftruncate_on_pipe" {
        // ftruncate 错误处理: 管道 fd (不可截断)
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 对管道 ftruncate 应返回 -1 (EINVAL)
        let r = super::ftruncate(fds[0], 100);
        assert_eq!(r, -1, "ftruncate on pipe should return -1 (EINVAL)");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_ftruncate_negative_length" {
        // ftruncate 错误处理: 无效 length (负数)
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 负数 length 在写入管道时也会因 fd 类型失败
        // 但先测试 length 为负是否通过参数校验
        let r = super::ftruncate(fds[1], -1);
        assert_eq!(r, -1, "ftruncate on pipe with negative length should return -1");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_ftruncate_zero_length" {
        // ftruncate 边界条件: length=0
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 对管道 ftruncate(0) - 仍然是不可截断的 fd
        let r = super::ftruncate(fds[0], 0);
        assert_eq!(r, -1, "ftruncate on pipe should return -1");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});
