/// 模块: unsetenv_test
/// `unsetenv` 集成测试
///
/// 基于 `src/env/spec/unsetenv.md` 规约及 POSIX 标准生成。
///
/// ## 函数签名
///
/// ```c
/// int unsetenv(const char *name);
/// ```
///
/// ## 测试覆盖
///
/// 1. 移除已存在的变量返回 0
/// 2. 移除后 getenv 返回 NULL
/// 3. 移除不存在的变量返回 0（幂等）
/// 4. name 为空字符串时返回 -1 且 errno=EINVAL
/// 5. name 含 '=' 时返回 -1 且 errno=EINVAL
/// 6. 移除后其他变量不受影响
/// 7. 与 setenv 的交互（setenv 后 unsetenv）
/// 8. 连续 unsetenv 多个变量
/// 9. 空环境上 unsetenv（clearenv 后 unsetenv）返回 0
/// 10. 重复移除同一变量（幂等性验证）
///
/// ## spec 关键约束 (src/env/unsetenv.c)
///
/// - 算法为单趟双指针原地压缩, O(n) 时间, O(1) 空间
/// - `l = __strchrnul(name, '=') - name` 计算 name 长度
/// - `!l || name[l]` 触发 EINVAL（空名或含 '='）
/// - `__environ == NULL` 时直接返回 0（无操作）
/// - 匹配条件: `strncmp(name, *e, l) == 0 && (*e)[l] == '='`
/// - 线程不安全（符合 POSIX 语义）


use core::ffi::{c_char, c_int};
use core::ffi::CStr;

use test_framework::test;

// ============================================================================
// 常量
// ============================================================================

/// EINVAL 错误码 (musl: 22)
const EINVAL: c_int = 22;

// ============================================================================
// C ABI 符号声明 (c-test 模式下由 musl libc 提供)
// ============================================================================

extern "C" {
    /// 删除环境变量: `int unsetenv(const char *name);`
    fn unsetenv(name: *const c_char) -> c_int;

    /// 设置环境变量: `int setenv(const char *name, const char *value, int overwrite);`
    fn setenv(name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;

    /// 获取环境变量: `char *getenv(const char *name);`
    fn getenv(name: *const c_char) -> *mut c_char;

    /// 清空所有环境变量: `int clearenv(void);`
    fn clearenv() -> c_int;

    /// 添加环境变量 (原始指针, 不拷贝): `int putenv(char *string);`
    fn putenv(string: *mut c_char) -> c_int;

    /// 获取 errno 地址
    fn __errno_location() -> *mut c_int;
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 获取当前 errno 值
unsafe fn get_errno() -> c_int {
    unsafe { *__errno_location() }
}

/// 设置 errno 为指定值
unsafe fn set_errno(val: c_int) {
    unsafe {
        *__errno_location() = val;
    }
}

/// 检查 getenv(name) 是否返回非 NULL（即变量存在）
unsafe fn env_exists(name: &CStr) -> bool {
    !getenv(name.as_ptr()).is_null()
}

/// 获取环境变量值并转为 `&CStr`，若不存在返回 None
unsafe fn getenv_as_cstr(name: &CStr) -> Option<&'static CStr> {
    let ptr = getenv(name.as_ptr());
    if ptr.is_null() {
        None
    } else {
        Some(CStr::from_ptr(ptr))
    }
}

/// 设置测试环境: FOO=1, BAR=2, BAZ=3
/// 先清空再设置，确保初始状态干净。
/// 返回 true 表示 3 个变量全部设置成功。
unsafe fn env_setup_3vars() -> bool {
    unsafe {
        clearenv();
        let foo = CStr::from_bytes_with_nul(b"FOO\0").unwrap();
        let bar = CStr::from_bytes_with_nul(b"BAR\0").unwrap();
        let baz = CStr::from_bytes_with_nul(b"BAZ\0").unwrap();
        let v1 = CStr::from_bytes_with_nul(b"1\0").unwrap();
        let v2 = CStr::from_bytes_with_nul(b"2\0").unwrap();
        let v3 = CStr::from_bytes_with_nul(b"3\0").unwrap();

        setenv(foo.as_ptr(), v1.as_ptr(), 1) == 0
            && setenv(bar.as_ptr(), v2.as_ptr(), 1) == 0
            && setenv(baz.as_ptr(), v3.as_ptr(), 1) == 0
    }
}

/// 创建 CStr 的便捷宏（编译期常量）
/// 调用方需在 unsafe 上下文中
macro_rules! cstr {
    ($s:literal) => {
        CStr::from_bytes_with_nul(concat!($s, "\0").as_bytes()).unwrap()
    };
}

// ============================================================================
// 场景 1: 移除已存在的变量返回 0
// ============================================================================

// 移除 environ 中第一个匹配项
test!("test_unsetenv_first_entry" {
    unsafe {
        assert!(env_setup_3vars());
        set_errno(0);
        let name = cstr!("FOO");
        assert_eq!(unsetenv(name.as_ptr()), 0, "unsetenv(\"FOO\") should return 0");
        assert!(!env_exists(&name), "FOO should have been removed");
        assert!(env_exists(&cstr!("BAR")), "BAR should still exist");
        assert!(env_exists(&cstr!("BAZ")), "BAZ should still exist");
    }
});

// 移除 environ 中间位置的匹配项（验证压缩算法）
test!("test_unsetenv_middle_entry" {
    unsafe {
        assert!(env_setup_3vars());
        set_errno(0);
        let name = cstr!("BAR");
        assert_eq!(unsetenv(name.as_ptr()), 0, "unsetenv(\"BAR\") should return 0");
        assert!(!env_exists(&name), "BAR should have been removed");
        assert!(env_exists(&cstr!("FOO")), "FOO should still exist");
        assert!(env_exists(&cstr!("BAZ")), "BAZ should still exist");

        // 验证保留条目的 value 未被破坏
        let v = getenv_as_cstr(&cstr!("FOO")).unwrap();
        assert_eq!(v.to_bytes(), b"1");
        let v = getenv_as_cstr(&cstr!("BAZ")).unwrap();
        assert_eq!(v.to_bytes(), b"3");
    }
});

// 移除 environ 末尾的匹配项
test!("test_unsetenv_last_entry" {
    unsafe {
        assert!(env_setup_3vars());
        set_errno(0);
        let name = cstr!("BAZ");
        assert_eq!(unsetenv(name.as_ptr()), 0, "unsetenv(\"BAZ\") should return 0");
        assert!(!env_exists(&name), "BAZ should have been removed");
        assert!(env_exists(&cstr!("FOO")), "FOO should still exist");
        assert!(env_exists(&cstr!("BAR")), "BAR should still exist");
    }
});

// 移除唯一的一个条目
test!("test_unsetenv_single_entry" {
    unsafe {
        clearenv();
        let name = cstr!("ONLY");
        let val = cstr!("42");
        assert_eq!(setenv(name.as_ptr(), val.as_ptr(), 1), 0, "setenv ONLY=42 failed");
        assert!(env_exists(&name), "ONLY should exist before unsetenv");

        set_errno(0);
        assert_eq!(unsetenv(name.as_ptr()), 0, "unsetenv(\"ONLY\") should return 0");
        assert!(!env_exists(&name), "ONLY should have been removed");
    }
});

// ============================================================================
// 场景 2: 移除后 getenv 返回 NULL
// ============================================================================

// 验证 unsetenv 后 getenv 对该变量返回 NULL
test!("test_unsetenv_getenv_returns_null" {
    unsafe {
        assert!(env_setup_3vars());
        let name = cstr!("FOO");
        // 移除前 getenv 应返回非 NULL
        let before = getenv(name.as_ptr());
        assert!(!before.is_null(), "FOO should exist via getenv before unsetenv");

        unsetenv(name.as_ptr());

        // 移除后 getenv 应返回 NULL
        let after = getenv(name.as_ptr());
        assert!(after.is_null(), "getenv(\"FOO\") should return NULL after unsetenv");
    }
});

// 逐个移除所有变量后 getenv 对每个变量返回 NULL
test!("test_unsetenv_getenv_all_null" {
    unsafe {
        assert!(env_setup_3vars());

        let foo = cstr!("FOO");
        let bar = cstr!("BAR");
        let baz = cstr!("BAZ");

        unsetenv(foo.as_ptr());
        unsetenv(bar.as_ptr());
        unsetenv(baz.as_ptr());

        assert!(getenv(foo.as_ptr()).is_null(), "FOO getenv should be NULL");
        assert!(getenv(bar.as_ptr()).is_null(), "BAR getenv should be NULL");
        assert!(getenv(baz.as_ptr()).is_null(), "BAZ getenv should be NULL");
    }
});

// ============================================================================
// 场景 3: 移除不存在的变量返回 0（幂等）
// ============================================================================

// 移除不存在的变量应返回 0
test!("test_unsetenv_nonexistent" {
    unsafe {
        assert!(env_setup_3vars());
        set_errno(0);
        let name = cstr!("NOTEXIST");
        assert_eq!(unsetenv(name.as_ptr()), 0, "unsetenv nonexistent var should return 0");
        assert!(env_exists(&cstr!("FOO")), "existing vars should be unaffected");
        assert!(env_exists(&cstr!("BAR")), "existing vars should be unaffected");
        assert!(env_exists(&cstr!("BAZ")), "existing vars should be unaffected");
    }
});

// 前缀部分匹配不应误删（"FO" 不应匹配 "FOO"）
test!("test_unsetenv_prefix_no_match" {
    unsafe {
        assert!(env_setup_3vars());
        set_errno(0);
        let name = cstr!("FO");
        assert_eq!(unsetenv(name.as_ptr()), 0, "unsetenv(\"FO\") should return 0 (not found)");
        assert!(env_exists(&cstr!("FOO")), "FOO should not have been removed (prefix mismatch)");
    }
});

// 超集名称不应误删（"FOOO" 不应匹配 "FOO"）
test!("test_unsetenv_superset_no_match" {
    unsafe {
        assert!(env_setup_3vars());
        set_errno(0);
        let name = cstr!("FOOO");
        assert_eq!(unsetenv(name.as_ptr()), 0, "unsetenv(\"FOOO\") should return 0 (not found)");
        assert!(env_exists(&cstr!("FOO")), "FOO should not have been removed (superset mismatch)");
    }
});

// 大小写不同的键名不应匹配
test!("test_unsetenv_case_sensitive" {
    unsafe {
        clearenv();
        let key = cstr!("Key");
        let val = cstr!("val");
        setenv(key.as_ptr(), val.as_ptr(), 1);
        assert!(env_exists(&key), "Key should exist");

        let lower = cstr!("key");
        set_errno(0);
        assert_eq!(unsetenv(lower.as_ptr()), 0, "unsetenv(\"key\") should return 0 (not found)");
        assert!(env_exists(&key), "Key should not have been removed (case sensitive)");
    }
});

// ============================================================================
// 场景 4: name 为空字符串时返回 -1 且 errno=EINVAL
// ============================================================================

// 空字符串名应返回 -1, errno=EINVAL
test!("test_unsetenv_empty_name" {
    unsafe {
        assert!(env_setup_3vars());
        set_errno(0);
        let name = cstr!("");
        assert_eq!(unsetenv(name.as_ptr()), -1, "unsetenv(\"\") should return -1");
        assert_eq!(get_errno(), EINVAL, "errno should be EINVAL for empty name");
        // 环境不应被修改
        assert!(env_exists(&cstr!("FOO")), "FOO should still exist after error");
    }
});

// 空环境上空字符串名仍返回 -1
test!("test_unsetenv_empty_name_empty_environ" {
    unsafe {
        clearenv();
        set_errno(0);
        let name = cstr!("");
        assert_eq!(unsetenv(name.as_ptr()), -1, "unsetenv(\"\") on empty environ should return -1");
        assert_eq!(get_errno(), EINVAL, "errno should be EINVAL on empty environ too");
    }
});

// ============================================================================
// 场景 5: name 含 '=' 时返回 -1 且 errno=EINVAL
// ============================================================================

// 名称中包含 '='（"FOO=BAR"）应返回 -1
test!("test_unsetenv_name_with_equal" {
    unsafe {
        assert!(env_setup_3vars());
        set_errno(0);
        let name = cstr!("FOO=BAR");
        assert_eq!(unsetenv(name.as_ptr()), -1, "unsetenv(\"FOO=BAR\") should return -1");
        assert_eq!(get_errno(), EINVAL, "errno should be EINVAL for name with '='");
        // 环境不应被修改
        assert!(env_exists(&cstr!("FOO")), "FOO should still exist after error");
        assert!(env_exists(&cstr!("BAR")), "BAR should still exist after error");
    }
});

// 纯 '=' 字符也应返回 -1
test!("test_unsetenv_name_equal_only" {
    unsafe {
        assert!(env_setup_3vars());
        set_errno(0);
        let name = cstr!("=");
        assert_eq!(unsetenv(name.as_ptr()), -1, "unsetenv(\"=\") should return -1");
        assert_eq!(get_errno(), EINVAL, "errno should be EINVAL for \"=\"");
    }
});

// 名称为 "A=1"（中间含 '='），不应删除 "A"
test!("test_unsetenv_name_equal_middle" {
    unsafe {
        clearenv();
        let a = cstr!("A");
        let v1 = cstr!("1");
        setenv(a.as_ptr(), v1.as_ptr(), 1);
        assert!(env_exists(&a), "A should exist before unsetenv");

        set_errno(0);
        let bad_name = cstr!("A=1");
        assert_eq!(unsetenv(bad_name.as_ptr()), -1, "unsetenv(\"A=1\") should return -1");
        assert_eq!(get_errno(), EINVAL, "errno should be EINVAL");
        assert!(env_exists(&a), "A should not have been removed");
    }
});

// ============================================================================
// 场景 6: 移除后其他变量不受影响
// ============================================================================

// 移除 FOO 后 BAR 和 BAZ 的值保持不变
test!("test_unsetenv_other_vars_unaffected" {
    unsafe {
        assert!(env_setup_3vars());

        let foo = cstr!("FOO");
        unsetenv(foo.as_ptr());

        let bar_val = getenv_as_cstr(&cstr!("BAR")).unwrap();
        assert_eq!(bar_val.to_bytes(), b"2");

        let baz_val = getenv_as_cstr(&cstr!("BAZ")).unwrap();
        assert_eq!(baz_val.to_bytes(), b"3");
    }
});

// 交替删除后保留变量的值仍正确（验证压缩算法不破坏数据）
test!("test_unsetenv_compaction_preserves_values" {
    unsafe {
        clearenv();
        let keys: [&CStr; 5] = [
            &cstr!("A"), &cstr!("B"), &cstr!("C"), &cstr!("D"), &cstr!("E"),
        ];
        let vals: [&CStr; 5] = [
            &cstr!("1"), &cstr!("2"), &cstr!("3"), &cstr!("4"), &cstr!("5"),
        ];
        for i in 0..5 {
            setenv(keys[i].as_ptr(), vals[i].as_ptr(), 1);
        }

        // 交替删除 A, C, E (第 0, 2, 4 位)
        unsetenv(keys[0].as_ptr());
        unsetenv(keys[2].as_ptr());
        unsetenv(keys[4].as_ptr());

        // 验证保留的 B, D 值不变
        let b_val = getenv_as_cstr(keys[1]).unwrap();
        assert_eq!(b_val.to_bytes(), b"2");
        let d_val = getenv_as_cstr(keys[3]).unwrap();
        assert_eq!(d_val.to_bytes(), b"4");

        // 验证删除的 A, C, E 确实不存在
        assert!(getenv(keys[0].as_ptr()).is_null());
        assert!(getenv(keys[2].as_ptr()).is_null());
        assert!(getenv(keys[4].as_ptr()).is_null());
    }
});

// ============================================================================
// 场景 7: 与 setenv 的交互（setenv 后 unsetenv, 再重新 setenv）
// ============================================================================

// setenv 后 unsetenv, 然后再次 setenv 应能恢复正常
test!("test_unsetenv_setenv_re_add" {
    unsafe {
        clearenv();
        let var = cstr!("VAR");
        let old_val = cstr!("old");
        let new_val = cstr!("new");

        // set -> verify -> unset -> verify gone -> re-set -> verify new value
        setenv(var.as_ptr(), old_val.as_ptr(), 1);
        assert!(env_exists(&var), "VAR should exist after setenv");
        assert_eq!(getenv_as_cstr(&var).unwrap().to_bytes(), b"old");

        unsetenv(var.as_ptr());
        assert!(!env_exists(&var), "VAR should be gone after unsetenv");

        setenv(var.as_ptr(), new_val.as_ptr(), 1);
        assert!(env_exists(&var), "VAR should be back after re-setenv");
        assert_eq!(getenv_as_cstr(&var).unwrap().to_bytes(), b"new");
    }
});

// setenv 后 unsetenv 另一变量，第一个变量的值应保持
test!("test_unsetenv_setenv_then_unset_other" {
    unsafe {
        clearenv();
        let a = cstr!("A");
        let b = cstr!("B");
        let v1 = cstr!("1");
        let v2 = cstr!("2");

        setenv(a.as_ptr(), v1.as_ptr(), 1);
        setenv(b.as_ptr(), v2.as_ptr(), 1);

        unsetenv(b.as_ptr());

        assert!(env_exists(&a), "A should still exist");
        assert_eq!(getenv_as_cstr(&a).unwrap().to_bytes(), b"1");
        assert!(!env_exists(&b), "B should be gone");
    }
});

// ============================================================================
// 场景 8: 连续 unsetenv 多个变量
// ============================================================================

// 连续移除所有变量
test!("test_unsetenv_consecutive_remove_all" {
    unsafe {
        assert!(env_setup_3vars());

        unsetenv(cstr!("FOO").as_ptr());
        unsetenv(cstr!("BAR").as_ptr());
        unsetenv(cstr!("BAZ").as_ptr());

        assert!(!env_exists(&cstr!("FOO")), "FOO should be gone");
        assert!(!env_exists(&cstr!("BAR")), "BAR should be gone");
        assert!(!env_exists(&cstr!("BAZ")), "BAZ should be gone");
    }
});

// 连续移除并按相反顺序验证
test!("test_unsetenv_consecutive_reverse" {
    unsafe {
        clearenv();
        let keys: [&CStr; 5] = [
            &cstr!("K1"), &cstr!("K2"), &cstr!("K3"), &cstr!("K4"), &cstr!("K5"),
        ];
        let v = cstr!("x");
        for key in &keys {
            setenv(key.as_ptr(), v.as_ptr(), 1);
        }

        // 按相反顺序移除
        for key in keys.iter().rev() {
            assert_eq!(unsetenv(key.as_ptr()), 0, "unsetenv should return 0");
        }

        // 全部应不存在
        for key in &keys {
            assert!(!env_exists(key), "should be gone after removal");
        }
    }
});

// ============================================================================
// 场景 9: 空环境上 unsetenv（clearenv 后 unsetenv）返回 0
// ============================================================================

// clearenv 后 unsetenv 任意合法名称应返回 0
test!("test_unsetenv_after_clearenv" {
    unsafe {
        clearenv();
        set_errno(0);

        let name = cstr!("ANYTHING");
        assert_eq!(unsetenv(name.as_ptr()), 0, "unsetenv after clearenv should return 0");
    }
});

// clearenv 后 unsetenv 多个不同变量名都返回 0
test!("test_unsetenv_multiple_after_clearenv" {
    unsafe {
        clearenv();
        let names: [&CStr; 5] = [
            &cstr!("X1"), &cstr!("X2"), &cstr!("X3"), &cstr!("X4"), &cstr!("X5"),
        ];
        for name in &names {
            assert_eq!(unsetenv(name.as_ptr()), 0, "unsetenv after clearenv should return 0");
        }
    }
});

// ============================================================================
// 场景 10: 重复移除同一变量（幂等性验证）
// ============================================================================

// 两次 unsetenv 同一变量均返回 0
test!("test_unsetenv_idempotent_twice" {
    unsafe {
        assert!(env_setup_3vars());
        let name = cstr!("FOO");

        // 第一次移除（存在）
        assert_eq!(unsetenv(name.as_ptr()), 0);

        // 第二次移除（已不存在，幂等）
        set_errno(0);
        assert_eq!(unsetenv(name.as_ptr()), 0, "second unsetenv should also return 0");
    }
});

// 多次重复移除同一变量（三次及以上）
test!("test_unsetenv_idempotent_multiple" {
    unsafe {
        assert!(env_setup_3vars());
        let name = cstr!("FOO");

        unsetenv(name.as_ptr());
        unsetenv(name.as_ptr());
        unsetenv(name.as_ptr());

        // 始终返回 0
        set_errno(0);
        assert_eq!(unsetenv(name.as_ptr()), 0, "multiple unsetenv should all return 0");
    }
});

// 未存在过的变量多次 unsetenv（幂等）
test!("test_unsetenv_idempotent_never_existed" {
    unsafe {
        assert!(env_setup_3vars());
        let name = cstr!("NEVER_HERE");

        set_errno(0);
        assert_eq!(unsetenv(name.as_ptr()), 0, "first unsetenv of never-existed var");
        assert_eq!(unsetenv(name.as_ptr()), 0, "second unsetenv of never-existed var");
        assert_eq!(unsetenv(name.as_ptr()), 0, "third unsetenv of never-existed var");
    }
});

// ============================================================================
// 与 putenv 的交互测试
// ============================================================================

// putenv 插入的条目能被 unsetenv 正确移除
test!("test_unsetenv_with_putenv" {
    unsafe {
        clearenv();
        // putenv 插入一个字符串（不由 setenv 管理内存）
        let put_str = b"PUTEST=99\0".as_ptr() as *mut c_char;
        if putenv(put_str) != 0 {
            panic!("putenv failed");
        }

        let name = cstr!("PUTEST");
        assert!(env_exists(&name), "PUTEST should exist after putenv");

        set_errno(0);
        assert_eq!(unsetenv(name.as_ptr()), 0, "unsetenv(\"PUTEST\") should return 0");
        assert!(!env_exists(&name), "PUTEST should have been removed");
    }
});

// ============================================================================
// errno 行为测试
// ============================================================================

// 成功时 errno 不被修改（musl 实现特性）
test!("test_unsetenv_errno_unchanged_on_success" {
    unsafe {
        assert!(env_setup_3vars());
        set_errno(12345); // 非标准值
        unsetenv(cstr!("FOO").as_ptr());
        // musl 的 unsetenv 成功时不修改 errno
        assert_eq!(get_errno(), 12345, "errno should not be modified on success");
    }
});

// EINVAL 错误后 errno 正确设置, 后续成功不修改 errno
test!("test_unsetenv_errno_after_einval" {
    unsafe {
        clearenv();
        set_errno(0);
        unsetenv(cstr!("").as_ptr());
        assert_eq!(get_errno(), EINVAL, "errno should be EINVAL after empty name error");

        // 再调用一次合法操作
        set_errno(0);
        unsetenv(cstr!("SOME_VAR").as_ptr());
        assert_eq!(get_errno(), 0, "errno after success should remain unchanged");
    }
});
