//! unlink 函数集成测试

use core::ffi::c_char;
use test_framework::test;

test!("test_unlink_symlink" {
    {
        // 先创建符号链接，再删除
        let target = b"unlink_target\0";
        let name = b"/tmp/rusl_test_unlink\0";
        let ret = super::symlink(
            target.as_ptr() as *const c_char,
            name.as_ptr() as *const c_char,
        );
        assert_eq!(ret, 0);

        let ret = super::unlink(name.as_ptr() as *const c_char);
        assert_eq!(ret, 0);
    }
});

test!("test_unlink_noent" {
    {
        let path = b"/tmp/rusl_test_unlink_nonexistent\0";
        let ret = super::unlink(path.as_ptr() as *const c_char);
        assert_eq!(ret, -1);
    }
});

test!("test_unlink_double" {
    {
        // 创建符号链接，删除，再次删除应失败
        let target = b"unlink_double_target\0";
        let name = b"/tmp/rusl_test_unlink_double\0";
        let ret = super::symlink(
            target.as_ptr() as *const c_char,
            name.as_ptr() as *const c_char,
        );
        assert_eq!(ret, 0);

        // 第一次删除成功
        let ret = super::unlink(name.as_ptr() as *const c_char);
        assert_eq!(ret, 0);

        // 第二次删除应失败 (ENOENT)
        let ret = super::unlink(name.as_ptr() as *const c_char);
        assert_eq!(ret, -1);
    }
});

test!("test_unlink_directory" {
    {
        // unlink 不能删除目录
        let path = b"/tmp\0";
        let ret = super::unlink(path.as_ptr() as *const c_char);
        // 应返回 -1 (EPERM 或 EISDIR)
        assert_eq!(ret, -1);
    }
});
