//! lchown 函数集成测试

use core::ffi::c_char;
use test_framework::test;

const NOCHANGE: u32 = !0u32;

test!("test_lchown_nochange" {
    {
        let path = b"/dev/null\0";
        let ret = super::lchown(
            path.as_ptr() as *const c_char,
            NOCHANGE,
            NOCHANGE,
        );
        assert_eq!(ret, 0);
    }
});

test!("test_lchown_noent" {
    {
        let path = b"/nonexistent_file_rusl_test_xyz\0";
        let ret = super::lchown(
            path.as_ptr() as *const c_char,
            NOCHANGE,
            NOCHANGE,
        );
        assert_eq!(ret, -1);
    }
});
