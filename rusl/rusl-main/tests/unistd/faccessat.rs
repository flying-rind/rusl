//! faccessat 函数集成测试

use core::ffi::c_char;
use test_framework::test;

// AT_FDCWD = -100 (相对当前工作目录)
const AT_FDCWD: i32 = -100;

test!("test_faccessat_f_ok_exists" {
    {
        let path = b"/dev/null\0";
        let ret = super::faccessat(
            AT_FDCWD,
            path.as_ptr() as *const c_char,
            super::F_OK,
            0,
        );
        assert_eq!(ret, 0);
    }
});

test!("test_faccessat_r_ok" {
    {
        let path = b"/dev/null\0";
        let ret = super::faccessat(
            AT_FDCWD,
            path.as_ptr() as *const c_char,
            super::R_OK,
            0,
        );
        assert_eq!(ret, 0);
    }
});

test!("test_faccessat_noent" {
    {
        let path = b"/nonexistent_path_rusl_test_xyz\0";
        let ret = super::faccessat(
            AT_FDCWD,
            path.as_ptr() as *const c_char,
            super::F_OK,
            0,
        );
        assert_eq!(ret, -1);
    }
});

test!("test_faccessat_invalid_fd" {
    {
        // Use an invalid fd (non-directory) with a relative path
        // This should fail
        let path = b"somefile\0";
        let ret = super::faccessat(
            -1,
            path.as_ptr() as *const c_char,
            super::F_OK,
            0,
        );
        assert_eq!(ret, -1);
    }
});
