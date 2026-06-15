//! fchdir 函数集成测试

use test_framework::test;

test!("test_fchdir_invalid_fd" {
    {
        // -1 is an invalid fd, should return -1 (EBADF)
        let ret = super::fchdir(-1);
        assert_eq!(ret, -1);
    }
});

test!("test_fchdir_very_invalid_fd" {
    {
        // Very large invalid fd number
        let ret = super::fchdir(999999);
        assert_eq!(ret, -1);
    }
});
