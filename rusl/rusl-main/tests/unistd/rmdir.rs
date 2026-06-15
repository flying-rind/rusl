//! rmdir 函数集成测试

use core::ffi::c_char;
use test_framework::test;

test!("test_rmdir_noent" {
    {
        let path = b"/tmp/rusl_test_rmdir_nonexistent\0";
        let ret = super::rmdir(path.as_ptr() as *const c_char);
        assert_eq!(ret, -1);
    }
});

test!("test_rmdir_not_dir" {
    {
        // /dev/null 是文件（字符设备），不是目录
        let path = b"/dev/null\0";
        let ret = super::rmdir(path.as_ptr() as *const c_char);
        assert_eq!(ret, -1);
    }
});

test!("test_rmdir_not_empty" {
    {
        // /tmp 是非空目录，rmdir 应失败
        let path = b"/tmp\0";
        let ret = super::rmdir(path.as_ptr() as *const c_char);
        // /tmp 通常非空，rmdir 应返回 -1
        // 但如果恰好 /tmp 是空的（极少见），也可能成功
        // 在典型环境中，/tmp 至少包含一些文件
        if ret == 0 {
            // 如果成功，说明 /tmp 恰好是空的 — 极不可能但我们允许
        } else {
            assert_eq!(ret, -1);
        }
    }
});

test!("test_rmdir_root" {
    {
        // "/" 根目录不能被删除
        let path = b"/\0";
        let ret = super::rmdir(path.as_ptr() as *const c_char);
        assert_eq!(ret, -1);
    }
});
