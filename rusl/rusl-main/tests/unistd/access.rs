//! access 函数集成测试

use core::ffi::c_char;
use test_framework::test;

test!("test_access_f_ok_exists" {
    {
        // /dev/null is always present and accessible
        let path = b"/dev/null\0";
        let ret = super::access(path.as_ptr() as *const c_char, super::F_OK);
        assert_eq!(ret, 0);
    }
});

test!("test_access_r_ok" {
    {
        let path = b"/dev/null\0";
        let ret = super::access(path.as_ptr() as *const c_char, super::R_OK);
        assert_eq!(ret, 0);
    }
});

test!("test_access_w_ok_dev_null" {
    {
        // /dev/null is writable
        let path = b"/dev/null\0";
        let ret = super::access(path.as_ptr() as *const c_char, super::W_OK);
        assert_eq!(ret, 0);
    }
});

test!("test_access_noent" {
    {
        let path = b"/nonexistent_path_rusl_test_xyz\0";
        let ret = super::access(path.as_ptr() as *const c_char, super::F_OK);
        assert_eq!(ret, -1);
    }
});

test!("test_access_multiple_flags" {
    {
        // R_OK | W_OK on /dev/null
        let path = b"/dev/null\0";
        let ret = super::access(path.as_ptr() as *const c_char, super::R_OK | super::W_OK);
        assert_eq!(ret, 0);
    }
});
