//! renameat 函数集成测试

use core::ffi::c_char;
use test_framework::test;

const AT_FDCWD: i32 = -100;

test!("test_renameat_basic" {
    {
        let target = b"renameat_target\0";
        let old_name = b"/tmp/rusl_test_renameat_old\0";
        let new_name = b"/tmp/rusl_test_renameat_new\0";

        let _ = super::unlink(old_name.as_ptr() as *const c_char);
        let _ = super::unlink(new_name.as_ptr() as *const c_char);

        let ret = super::symlink(
            target.as_ptr() as *const c_char,
            old_name.as_ptr() as *const c_char,
        );
        assert_eq!(ret, 0);

        let ret = super::renameat(
            AT_FDCWD,
            old_name.as_ptr() as *const c_char,
            AT_FDCWD,
            new_name.as_ptr() as *const c_char,
        );
        assert_eq!(ret, 0);

        // 旧路径应不存在
        let ret = super::access(
            old_name.as_ptr() as *const c_char,
            super::F_OK,
        );
        assert_eq!(ret, -1);

        // 清理
        let _ = super::unlink(new_name.as_ptr() as *const c_char);
    }
});

test!("test_renameat_noent" {
    {
        let old = b"/tmp/rusl_test_renameat_nonexistent\0";
        let new = b"/tmp/rusl_test_renameat_new_noent\0";
        let _ = super::unlink(new.as_ptr() as *const c_char);

        let ret = super::renameat(
            AT_FDCWD,
            old.as_ptr() as *const c_char,
            AT_FDCWD,
            new.as_ptr() as *const c_char,
        );
        assert_eq!(ret, -1);
    }
});

test!("test_renameat_same_path" {
    {
        let target = b"renameat_same\0";
        let name = b"/tmp/rusl_test_renameat_same\0";
        let _ = super::unlink(name.as_ptr() as *const c_char);

        let ret = super::symlink(
            target.as_ptr() as *const c_char,
            name.as_ptr() as *const c_char,
        );
        assert_eq!(ret, 0);

        let ret = super::renameat(
            AT_FDCWD,
            name.as_ptr() as *const c_char,
            AT_FDCWD,
            name.as_ptr() as *const c_char,
        );
        assert_eq!(ret, 0);

        let _ = super::unlink(name.as_ptr() as *const c_char);
    }
});

test!("test_renameat_invalid_fd" {
    {
        let old = b"nonexistent_old\0";
        let new = b"nonexistent_new\0";
        let ret = super::renameat(
            -1,
            old.as_ptr() as *const c_char,
            AT_FDCWD,
            new.as_ptr() as *const c_char,
        );
        assert_eq!(ret, -1);
    }
});
