/// 模块: getenv_test
/// `getenv` 集成测试
///
/// 基于 `src/env/spec/getenv.md` 规约生成。
///
/// ## 测试覆盖
///
/// - getenv 基本功能: 存在/不存在变量
/// - setenv/getenv/unsetenv 联调
/// - 空字符串名 -> NULL
/// - 含 '=' 名 -> NULL
/// - overwrite 行为验证
/// - errno 不变性
/// - 大小写敏感性
/// - 前缀部分匹配不误匹配
/// - 多次调用指针一致性
/// - 长值 / 特殊字符值

use core::ffi::{c_char, c_int, CStr};

use test_framework::test;

// ===========================================================================
// C ABI 声明
// ===========================================================================
// getenv / setenv / unsetenv / clearenv 均为 POSIX <stdlib.h> 对外 API。
// 符号在 c-test 模式下由 musl libc 提供，非 c-test 模式下由 rusl::env 模块提供
// (若 rusl 尚未实现 env 模块，则仅 c-test 模式可编译通过)。

extern "C" {
    /// POSIX: 获取环境变量值
    fn getenv(name: *const c_char) -> *mut c_char;
    /// POSIX: 设置环境变量 (拷贝)
    fn setenv(name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;
    /// POSIX: 移除环境变量
    fn unsetenv(name: *const c_char) -> c_int;
    /// POSIX: 清空所有环境变量
    fn clearenv() -> c_int;
    /// musl 内部: 获取 errno 指针
    fn __errno_location() -> *mut c_int;
}

// ===========================================================================
// 辅助函数
// ===========================================================================

/// 将 getenv 返回的指针转为 `&CStr`,若为 NULL 返回 None。
unsafe fn getenv_cstr(name: &CStr) -> Option<&'static CStr> {
    let ptr = getenv(name.as_ptr());
    if ptr.is_null() {
        None
    } else {
        Some(CStr::from_ptr(ptr))
    }
}

/// 比较 getenv 返回值与预期字节串。
unsafe fn getenv_equals(name: &CStr, expected: &[u8]) -> bool {
    match getenv_cstr(name) {
        Some(cstr) => cstr.to_bytes() == expected,
        None => false,
    }
}

// ===========================================================================
// 基本功能
// ===========================================================================

test!("test_getenv_existing_path" {
    // PATH 环境变量通常在所有系统上都存在。
    unsafe {
        // 确保 PATH 存在（可能被之前运行的其他测试清除了环境）
        setenv(c"PATH".as_ptr(), c"/usr/bin".as_ptr(), 1);
        let result = getenv(c"PATH".as_ptr());
        assert!(!result.is_null(), "getenv(\"PATH\") 应返回非 NULL");
        // 返回的指针应可解引用，第一个字节非 '\0'
        let first = *result;
        assert_ne!(first, 0, "PATH 的值不应为空字符串");
    }
});

test!("test_getenv_nonexistent" {
    // 查询不存在的环境变量应返回 NULL。
    unsafe {
        let result = getenv(c"__RUSL_NONEXISTENT_VAR_XYZ123__".as_ptr());
        assert!(result.is_null(), "getenv 不存在变量应返回 NULL");
    }
});

// ===========================================================================
// setenv / getenv 联调
// ===========================================================================

test!("test_setenv_getenv_basic" {
    // 设置自定义变量，用 getenv 读取验证。
    unsafe {
        let name = c"RUSL_TEST_VAR";
        let value = c"hello_world";

        let set_ret = setenv(name.as_ptr(), value.as_ptr(), 1);
        assert_eq!(set_ret, 0, "setenv 应返回 0");

        let result = getenv(name.as_ptr());
        assert!(!result.is_null(), "getenv 刚设置的变量应返回非 NULL");

        let result_cstr = CStr::from_ptr(result);
        assert_eq!(
            result_cstr.to_bytes(),
            b"hello_world",
            "getenv 返回值应为 \"hello_world\""
        );

        // 清理
        unsetenv(name.as_ptr());
    }
});

test!("test_setenv_getenv_empty_value" {
    // 设置值为空字符串的环境变量。
    unsafe {
        clearenv();
        let name = c"RUSL_TEST_EMPTY";

        setenv(name.as_ptr(), c"".as_ptr(), 1);

        let result = getenv(name.as_ptr());
        assert!(!result.is_null(), "getenv 值为空的变量应返回非 NULL");
        // 值应为空字符串，第一个字节为 '\0'
        assert_eq!(*result, 0, "空值第一个字节应为 '\\0'");

        // 清理
        unsetenv(name.as_ptr());
    }
});

// ===========================================================================
// 边界情况 (来自 spec)
// ===========================================================================

test!("test_empty_name" {
    // spec: name 长度 == 0 时返回 NULL。
    unsafe {
        let result = getenv(c"".as_ptr());
        assert!(result.is_null(), "getenv(\"\") 应返回 NULL");
    }
});

test!("test_name_with_equals" {
    // spec: name 中包含 '=' 字符时返回 NULL。
    unsafe {
        let result = getenv(c"KEY=VALUE".as_ptr());
        assert!(result.is_null(), "getenv(\"KEY=VALUE\") 含 '=' 应返回 NULL");
    }
});

test!("test_name_ends_with_equals" {
    // boundary: name 以 '=' 结尾也应返回 NULL (因为后面没有值)。
    unsafe {
        let result = getenv(c"ONLY_KEY=".as_ptr());
        assert!(result.is_null(), "getenv(\"ONLY_KEY=\") 含 '=' 应返回 NULL");
    }
});

// ===========================================================================
// overwrite 行为
// ===========================================================================

test!("test_setenv_overwrite" {
    // overwrite=1 应覆盖已有值。
    unsafe {
        clearenv();
        let name = c"RUSL_OVERWRITE_VAR";

        setenv(name.as_ptr(), c"first".as_ptr(), 1);
        setenv(name.as_ptr(), c"second".as_ptr(), 1);

        assert!(getenv_equals(name, b"second"), "overwrite=1 应覆盖为 second");

        unsetenv(name.as_ptr());
    }
});

test!("test_setenv_no_overwrite" {
    // overwrite=0 不应覆盖已有值。
    unsafe {
        clearenv();
        let name = c"RUSL_NO_OVERWRITE_VAR";

        setenv(name.as_ptr(), c"original".as_ptr(), 1);
        let ret = setenv(name.as_ptr(), c"ignored".as_ptr(), 0);

        // setenv 返回 0 表示"成功"
        assert_eq!(ret, 0);
        assert!(getenv_equals(name, b"original"), "overwrite=0 应保留 original");

        unsetenv(name.as_ptr());
    }
});

// ===========================================================================
// unsetenv
// ===========================================================================

test!("test_unsetenv_then_get" {
    // unsetenv 后 getenv 应返回 NULL。
    unsafe {
        clearenv();
        let name = c"RUSL_UNSET_TEST";

        setenv(name.as_ptr(), c"temp".as_ptr(), 1);

        // 确认设置前 getenv 非 NULL
        let before = getenv(name.as_ptr());
        assert!(!before.is_null(), "unset 前应能获取到变量");

        unsetenv(name.as_ptr());

        let after = getenv(name.as_ptr());
        assert!(after.is_null(), "unsetenv 后 getenv 应返回 NULL");
    }
});

// ===========================================================================
// errno 不变性
// ===========================================================================

test!("test_getenv_no_errno_change" {
    // spec: getenv 不设置 errno。
    unsafe {
        *__errno_location() = 42;

        let _result = getenv(c"PATH".as_ptr());

        assert_eq!(*__errno_location(), 42, "getenv 不应修改 errno");
    }
});

test!("test_getenv_nonexistent_no_errno" {
    // 即使未找到，getenv 也不应设置 errno。
    unsafe {
        *__errno_location() = 99;

        let result = getenv(c"__RUSL_NONEXISTENT_XYZ__".as_ptr());
        assert!(result.is_null(), "不存在的变量应返回 NULL");
        assert_eq!(*__errno_location(), 99, "getenv 返回 NULL 时也不应修改 errno");
    }
});

// ===========================================================================
// 大小写敏感性
// ===========================================================================

test!("test_case_sensitive" {
    // musl 实现区分大小写，"path" != "PATH"。
    unsafe {
        let upper_result = getenv(c"PATH".as_ptr());
        let lower_result = getenv(c"path".as_ptr());

        if upper_result.is_null() {
            // PATH 不存在是异常情况，跳过详细比较
            return;
        }

        // PATH 通常全大写存在，小写 path 通常不存在
        // 但如果 path 也存在 (部分系统)，两者应指向不同值
        if !lower_result.is_null() {
            assert_ne!(
                upper_result, lower_result,
                "\"PATH\" 和 \"path\" 应返回不同结果 (大小写敏感)"
            );
        }
    }
});

// ===========================================================================
// 前缀部分匹配不误匹配 (来自 spec 算法要点4)
// ===========================================================================

test!("test_partial_name_no_match" {
    // "PAT" 不应匹配 "PATH" (防止前缀误匹配)。
    unsafe {
        let full_result = getenv(c"PATH".as_ptr());
        let partial_result = getenv(c"PAT".as_ptr());

        // PAT 独立作为环境变量不应返回与 PATH 相同值
        // (除非系统恰好同时设置了 PAT 环境变量)
        if !full_result.is_null() && !partial_result.is_null() {
            assert_ne!(
                full_result, partial_result,
                "\"PAT\" 不应匹配到 \"PATH\""
            );
        }
    }
});

test!("test_prefix_of_env_var_no_match" {
    // 设置 "RUSL_PREFIX_VAR" 后，"RUSL_PREFIX" 不应匹配到它。
    unsafe {
        clearenv();
        let set_name = c"RUSL_PREFIX_VAR";

        setenv(set_name.as_ptr(), c"some_value".as_ptr(), 1);

        let result = getenv(c"RUSL_PREFIX".as_ptr());
        assert!(
            result.is_null(),
            "\"RUSL_PREFIX\" 不应匹配到 \"RUSL_PREFIX_VAR\""
        );

        // 验证原始变量确实存在
        assert!(
            getenv_equals(set_name, b"some_value"),
            "完整名称应能匹配"
        );

        unsetenv(set_name.as_ptr());
    }
});

// ===========================================================================
// 多次调用一致性 (来自 spec 不变量)
// ===========================================================================

test!("test_multiple_calls_same_pointer" {
    // 同一变量多次 getenv 在未修改环境时返回相同指针。
    unsafe {
        let r1 = getenv(c"PATH".as_ptr());
        let r2 = getenv(c"PATH".as_ptr());
        let r3 = getenv(c"PATH".as_ptr());

        if !r1.is_null() {
            assert_eq!(r1, r2, "连续调用应返回相同指针");
            assert_eq!(r2, r3, "连续调用应返回相同指针");
        }
    }
});

// ===========================================================================
// 多个变量
// ===========================================================================

test!("test_multiple_variables" {
    // 同时设置多个变量，分别验证。
    unsafe {
        clearenv();

        setenv(c"RUSL_MULTI_A".as_ptr(), c"alpha".as_ptr(), 1);
        setenv(c"RUSL_MULTI_B".as_ptr(), c"beta".as_ptr(), 1);
        setenv(c"RUSL_MULTI_C".as_ptr(), c"gamma".as_ptr(), 1);

        assert!(getenv_equals(c"RUSL_MULTI_A", b"alpha"), "变量 A");
        assert!(getenv_equals(c"RUSL_MULTI_B", b"beta"), "变量 B");
        assert!(getenv_equals(c"RUSL_MULTI_C", b"gamma"), "变量 C");

        // 清理
        unsetenv(c"RUSL_MULTI_A".as_ptr());
        unsetenv(c"RUSL_MULTI_B".as_ptr());
        unsetenv(c"RUSL_MULTI_C".as_ptr());
    }
});

// ===========================================================================
// 长值
// ===========================================================================

test!("test_long_value" {
    // 测试较长值字符串。
    unsafe {
        clearenv();
        let name = c"RUSL_LONG_VAL";

        setenv(name.as_ptr(), c"abcdefghijklmnopqrstuvwxyz_0123456789".as_ptr(), 1);

        assert!(
            getenv_equals(name, b"abcdefghijklmnopqrstuvwxyz_0123456789"),
            "长值应完整返回"
        );

        unsetenv(name.as_ptr());
    }
});

// ===========================================================================
// 值中包含 '=' 字符 (来自 spec: 值可含 '=', 不影响查找)
// ===========================================================================

test!("test_value_with_equals_sign" {
    // 值中可以包含 '=' 字符，getenv 应正确返回以 '\0' 结尾的完整值。
    unsafe {
        clearenv();
        let name = c"RUSL_EQUAL_VAL";

        setenv(name.as_ptr(), c"key=value=pair".as_ptr(), 1);

        assert!(
            getenv_equals(name, b"key=value=pair"),
            "值中的 '=' 应完整保留"
        );

        unsetenv(name.as_ptr());
    }
});

// ===========================================================================
// 特殊字符
// ===========================================================================

test!("test_value_with_special_chars" {
    // 测试值中包含空格、标点等特殊字符。
    unsafe {
        clearenv();
        let name = c"RUSL_SPECIAL_VAL";

        setenv(name.as_ptr(), c"hello world!@#$%^&*()".as_ptr(), 1);

        let result = getenv(name.as_ptr());
        assert!(!result.is_null(), "getenv 带特殊字符的变量应返回非 NULL");

        let result_cstr = CStr::from_ptr(result);
        assert_eq!(
            result_cstr.to_bytes(),
            b"hello world!@#$%^&*()",
            "特殊字符值应完整保留"
        );

        unsetenv(name.as_ptr());
    }
});

// ===========================================================================
// 返回值是环境内存内部指针 (来自 spec 不变量)
// ===========================================================================

test!("test_return_pointer_to_value_not_name" {
    // getenv 返回值应指向 value 部分,而非 "NAME=" 前缀。
    unsafe {
        clearenv();
        let name = c"RUSL_RETPTR";

        setenv(name.as_ptr(), c"myvalue".as_ptr(), 1);

        let result = getenv(name.as_ptr());
        assert!(!result.is_null(), "getenv 应返回非 NULL");

        let result_cstr = CStr::from_ptr(result);
        // 返回的字节不应包含 "RUSL_RETPTR=" 前缀
        assert_eq!(result_cstr.to_bytes(), b"myvalue", "返回值应为纯 value");

        unsetenv(name.as_ptr());
    }
});

// ===========================================================================
// NULL 参数 (可选 — spec 前置条件要求 name != NULL，行为未定义)
// ===========================================================================

// test_getenv_null_name 已移除: getenv(NULL) 违反前置条件,
// musl 的 __strchrnul(NULL, '=') 会触发 SIGSEGV 导致测试进程崩溃。
// SIGSEGV 无法被 setjmp/longjmp 捕获, 因此该测试不可运行。
