//! symlink 函数集成测试

use core::ffi::c_char;
use test_framework::test;

static SYMLINK_PATH: &[u8] = b"/tmp/rusl_test_symlink_target\0";
static SYMLINK_NAME: &[u8] = b"/tmp/rusl_test_symlink\0";

test!("test_symlink_basic" {
    {
        // 创建符号链接
        let ret = super::symlink(
            SYMLINK_PATH.as_ptr() as *const c_char,
            SYMLINK_NAME.as_ptr() as *const c_char,
        );
        assert_eq!(ret, 0);

        // 清理
        let _ = super::unlink(SYMLINK_NAME.as_ptr() as *const c_char);
    }
});

test!("test_symlink_eexist" {
    {
        // 先创建一个符号链接
        let _ = super::symlink(
            SYMLINK_PATH.as_ptr() as *const c_char,
            SYMLINK_NAME.as_ptr() as *const c_char,
        );
        // 相同路径再次创建应失败 (EEXIST)
        let ret = super::symlink(
            SYMLINK_PATH.as_ptr() as *const c_char,
            SYMLINK_NAME.as_ptr() as *const c_char,
        );
        assert_eq!(ret, -1);

        // 清理
        let _ = super::unlink(SYMLINK_NAME.as_ptr() as *const c_char);
    }
});

test!("test_symlink_to_self" {
    {
        // 符号链接目标可以是任意字符串，即使指向不存在的文件
        let target = b"/a/b/c/nonexistent_target\0";
        let name = b"/tmp/rusl_test_symlink_dangling\0";
        let ret = super::symlink(
            target.as_ptr() as *const c_char,
            name.as_ptr() as *const c_char,
        );
        assert_eq!(ret, 0);

        // 清理
        let _ = super::unlink(name.as_ptr() as *const c_char);
    }
});
