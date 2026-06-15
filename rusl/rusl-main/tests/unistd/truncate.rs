//! truncate 函数集成测试

use core::ffi::c_char;
use test_framework::test;

fn cstr(s: &[u8]) -> *const c_char {
    s.as_ptr() as *const c_char
}

test!("test_truncate_null_path" {
        // truncate 错误处理: NULL path
    {
        let ret = super::truncate(core::ptr::null(), 0);
        assert_eq!(ret, -1, "truncate with NULL path should return -1 (EFAULT)");
    }
});

test!("test_truncate_nonexistent" {
        // truncate 错误处理: 不存在的路径
    {
        let path = b"/nonexistent_path_rusl_test_xyz123\0";
        let ret = super::truncate(cstr(path), 0);
        assert_eq!(ret, -1, "truncate on nonexistent path should return -1 (ENOENT)");
    }
});

test!("test_truncate_empty_path" {
        // truncate 错误处理: 空路径
    {
        let path = b"\0";
        let ret = super::truncate(cstr(path), 0);
        assert_eq!(ret, -1, "truncate with empty path should return -1 (ENOENT)");
    }
});

test!("test_truncate_zero_length_nonexistent" {
        // truncate 边界条件: length=0 对不存在路径
    {
        let path = b"/nonexistent_path_for_truncate_test\0";
        let ret = super::truncate(cstr(path), 0);
        assert_eq!(ret, -1, "truncate on nonexistent path should return -1 (ENOENT)");
    }
});

test!("test_truncate_negative_length" {
        // truncate 边界条件: 负数 length 对不存在路径
    {
        let path = b"/nonexistent_path_neg_len\0";
        let ret = super::truncate(cstr(path), -1);
        assert_eq!(ret, -1, "truncate with negative length should return -1");
    }
});
