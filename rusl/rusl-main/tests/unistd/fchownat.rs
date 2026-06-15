//! fchownat 函数集成测试

use core::ffi::c_char;
use test_framework::test;

const AT_FDCWD: i32 = -100;
const NOCHANGE: u32 = !0u32;

test!("test_fchownat_nochange" {
    {
        let path = b"/dev/null\0";
        let ret = super::fchownat(
            AT_FDCWD,
            path.as_ptr() as *const c_char,
            NOCHANGE,
            NOCHANGE,
            0,
        );
        assert_eq!(ret, 0);
    }
});

test!("test_fchownat_noent" {
    {
        let path = b"/nonexistent_file_rusl_test_xyz\0";
        let ret = super::fchownat(
            AT_FDCWD,
            path.as_ptr() as *const c_char,
            NOCHANGE,
            NOCHANGE,
            0,
        );
        assert_eq!(ret, -1);
    }
});

test!("test_fchownat_invalid_fd" {
    {
        // Invalid fd with relative path
        let path = b"somefile\0";
        let ret = super::fchownat(
            -1,
            path.as_ptr() as *const c_char,
            NOCHANGE,
            NOCHANGE,
            0,
        );
        assert_eq!(ret, -1);
    }
});
