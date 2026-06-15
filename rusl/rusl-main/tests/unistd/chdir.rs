//! chdir 函数集成测试

use core::ffi::c_char;
use test_framework::test;

test!("test_chdir_tmp" {
    {
        // /tmp always exists and is a directory
        let path = b"/tmp\0";
        let ret = super::chdir(path.as_ptr() as *const c_char);
        assert_eq!(ret, 0);
    }
});

test!("test_chdir_root" {
    {
        let path = b"/\0";
        let ret = super::chdir(path.as_ptr() as *const c_char);
        assert_eq!(ret, 0);
    }
});

test!("test_chdir_noent" {
    {
        let path = b"/nonexistent_dir_rusl_test_xyz\0";
        let ret = super::chdir(path.as_ptr() as *const c_char);
        assert_eq!(ret, -1);
    }
});

test!("test_chdir_not_dir" {
    {
        // /dev/null is a file, not a directory
        let path = b"/dev/null\0";
        let ret = super::chdir(path.as_ptr() as *const c_char);
        assert_eq!(ret, -1);
    }
});
