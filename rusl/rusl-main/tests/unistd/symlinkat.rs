//! symlinkat 函数集成测试

use core::ffi::c_char;
use test_framework::test;

const AT_FDCWD: i32 = -100;

test!("test_symlinkat_basic" {
    {
        let target = b"/tmp/rusl_test_symlinkat_target\0";
        let name = b"/tmp/rusl_test_symlinkat_link\0";
        let ret = super::symlinkat(
            target.as_ptr() as *const c_char,
            AT_FDCWD,
            name.as_ptr() as *const c_char,
        );
        assert_eq!(ret, 0);

        // 清理
        let _ = super::unlink(name.as_ptr() as *const c_char);
    }
});

test!("test_symlinkat_eexist" {
    {
        let target = b"/tmp/rusl_test_symlinkat_target\0";
        let name = b"/tmp/rusl_test_symlinkat_link2\0";
        let _ = super::symlinkat(
            target.as_ptr() as *const c_char,
            AT_FDCWD,
            name.as_ptr() as *const c_char,
        );
        let ret = super::symlinkat(
            target.as_ptr() as *const c_char,
            AT_FDCWD,
            name.as_ptr() as *const c_char,
        );
        assert_eq!(ret, -1);

        // 清理
        let _ = super::unlink(name.as_ptr() as *const c_char);
    }
});

test!("test_symlinkat_invalid_fd" {
    {
        let target = b"some_target\0";
        let name = b"some_name\0";
        let ret = super::symlinkat(
            target.as_ptr() as *const c_char,
            -1,
            name.as_ptr() as *const c_char,
        );
        assert_eq!(ret, -1);
    }
});
