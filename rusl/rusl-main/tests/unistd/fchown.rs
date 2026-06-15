//! fchown 函数集成测试

use test_framework::test;

const NOCHANGE: u32 = !0u32;

test!("test_fchown_invalid_fd" {
    {
        // -1 is an invalid fd, should return -1 (EBADF)
        let ret = super::fchown(-1, NOCHANGE, NOCHANGE);
        assert_eq!(ret, -1);
    }
});

test!("test_fchown_large_invalid_fd" {
    {
        let ret = super::fchown(999999, NOCHANGE, NOCHANGE);
        assert_eq!(ret, -1);
    }
});
