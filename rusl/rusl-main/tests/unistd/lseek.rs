//! lseek 函数集成测试

use test_framework::test;

test!("test_lseek_invalid_fd" {
        // lseek 错误处理: 无效 fd
    {
        let ret = super::lseek(-1, 0, super::SEEK_SET);
        assert_eq!(ret, -1, "lseek on fd=-1 should return -1 (EBADF)");
    }
});

test!("test_lseek_on_pipe" {
        // lseek 错误处理: 在管道上定位 (ESPIPE)
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // 管道不可定位
        let off = super::lseek(fds[0], 0, super::SEEK_SET);
        assert_eq!(off, -1, "lseek on pipe should return -1 (ESPIPE)");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_lseek_invalid_whence" {
        // lseek 错误处理: 无效 whence
    {
        let mut fds: [super::c_int; 2] = [-1, -1];
        let ret = super::pipe(fds.as_mut_ptr());
        assert_eq!(ret, 0, "pipe creation should succeed");

        // whence = 99 是无效值
        let off = super::lseek(fds[0], 0, 99);
        assert_eq!(off, -1, "lseek with invalid whence should return -1 (EINVAL)");

        super::close(fds[0]);
        super::close(fds[1]);
    }
});

test!("test_lseek_constants" {
        // lseek 基本功能: SEEK_SET, SEEK_CUR, SEEK_END 常量验证
    {
        assert_eq!(super::SEEK_SET, 0, "SEEK_SET should be 0");
        assert_eq!(super::SEEK_CUR, 1, "SEEK_CUR should be 1");
        assert_eq!(super::SEEK_END, 2, "SEEK_END should be 2");
    }
});
