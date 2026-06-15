use core::ffi::c_char;
use super::acct;
use test_framework::test;


test!("test_acct_disable" {
        // 测试 1 — 禁用记账（filename = NULL）
    {
        let ret = acct(core::ptr::null());
        // 没有特权时通常返回 -1 (EPERM)
        // 有特权且之前已启用时返回 0
        assert!(ret == 0 || ret == -1,
                "acct(NULL) 应有权限时返回 0，无权限时返回 -1");
    }
});

test!("test_acct_invalid_path" {
    // 测试 2 — 使用无效文件路径（无特权用户）
    {
        let path = b"/nonexistent/path/for/acct\0";
        let ret = acct(path.as_ptr() as *const c_char);
        // 无特权时应返回 -1 (EPERM)
        // 有权限时可能返回 -1 (ENOENT) 或其他错误
        assert!(ret == -1, "acct(无效路径) 应返回 -1（无权限或路径不存在）");
    }
});

test!("test_acct_empty_path" {
        // 测试 3 — 空字符串路径
    {
        let path = b"\0";
        let ret = acct(path.as_ptr() as *const c_char);
        // 应返回 -1（路径无效或无权限）
        assert!(ret == -1, "acct(\"\") 应返回 -1");
    }
});
