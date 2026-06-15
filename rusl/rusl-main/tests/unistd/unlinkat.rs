//! unlinkat 函数集成测试

use core::ffi::c_char;
use test_framework::test;

const AT_FDCWD: i32 = -100;

test!("test_unlinkat_symlink" {
    {
        // 创建符号链接，然后通过 unlinkat 删除
        let target = b"unlinkat_target\0";
        let name = b"/tmp/rusl_test_unlinkat\0";
        let _ = super::symlinkat(
            target.as_ptr() as *const c_char,
            AT_FDCWD,
            name.as_ptr() as *const c_char,
        );

        let ret = super::unlinkat(
            AT_FDCWD,
            name.as_ptr() as *const c_char,
            0,
        );
        assert_eq!(ret, 0);
    }
});

test!("test_unlinkat_noent" {
    {
        let path = b"/tmp/rusl_test_unlinkat_nonexistent\0";
        let ret = super::unlinkat(
            AT_FDCWD,
            path.as_ptr() as *const c_char,
            0,
        );
        assert_eq!(ret, -1);
    }
});

test!("test_unlinkat_invalid_fd" {
    {
        let path = b"somefile\0";
        let ret = super::unlinkat(
            -1,
            path.as_ptr() as *const c_char,
            0,
        );
        assert_eq!(ret, -1);
    }
});
