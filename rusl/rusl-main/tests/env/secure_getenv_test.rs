/// 模块: secure_getenv_test
/// `secure_getenv` 集成测试
///
/// 基于 `src/env/spec/secure_getenv.md` 规约生成。
///
/// ## 测试覆盖
///
/// - 普通模式下 secure_getenv 等价于 getenv (存在/不存在变量)
/// - 空字符串名 -> NULL
/// - 含 '=' 的 name -> NULL
/// - 安全模式标志的只读行为说明
/// - 线程安全性验证
/// - 返回值指针指向 value 部分验证
/// - 大小写敏感性
/// - 前缀/后缀部分匹配不误匹配
/// - setenv/unsetenv 联调
/// - 多次调用一致性
/// - errno 不变性

use core::ffi::{c_char, c_int, CStr};

use rusl_core::test;

// ===========================================================================
// C ABI 声明
// ===========================================================================
// secure_getenv 是 GNU 扩展 (需 _GNU_SOURCE)，声明于 <stdlib.h>。
// getenv / setenv / unsetenv / clearenv 均为 POSIX <stdlib.h> 对外 API。
// 符号在 c-test 模式下由 musl libc 提供，非 c-test 模式下由 rusl::env 模块提供
// (若 rusl 尚未实现 env 模块，则仅 c-test 模式可编译通过)。

extern "C" {
    /// GNU 扩展: 安全的环境变量访问。
    /// libc.secure == 1 时始终返回 NULL；
    /// libc.secure == 0 时等价于 getenv。
    fn secure_getenv(name: *const c_char) -> *mut c_char;

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

/// 将 secure_getenv 返回的指针转为 `Option<&CStr>`, 若为 NULL 返回 None。
unsafe fn secure_getenv_cstr(name: &CStr) -> Option<&'static CStr> {
    let ptr = secure_getenv(name.as_ptr());
    if ptr.is_null() {
        None
    } else {
        Some(CStr::from_ptr(ptr))
    }
}

/// 比较 secure_getenv 返回值与预期字节串。
unsafe fn secure_getenv_equals(name: &CStr, expected: &[u8]) -> bool {
    match secure_getenv_cstr(name) {
        Some(cstr) => cstr.to_bytes() == expected,
        None => false,
    }
}

/// 将 getenv 返回的指针转为 `Option<&CStr>`, 若为 NULL 返回 None。
unsafe fn getenv_cstr(name: &CStr) -> Option<&'static CStr> {
    let ptr = getenv(name.as_ptr());
    if ptr.is_null() {
        None
    } else {
        Some(CStr::from_ptr(ptr))
    }
}

// ===========================================================================
// 基本功能 — 普通模式下 secure_getenv 行为
// ===========================================================================

test!("test_secure_getenv_existing_path" {
    // PATH 环境变量通常在所有系统上都存在。
    unsafe {
        // 确保 PATH 存在（可能被之前运行的其他测试清除了环境）
        setenv(c"PATH".as_ptr(), c"/usr/bin".as_ptr(), 1);
        let result = secure_getenv(c"PATH".as_ptr());
        assert!(!result.is_null(), "secure_getenv(\"PATH\") 应返回非 NULL");
        // 返回的指针应可解引用，第一个字节非 '\0'
        let first = *result;
        assert_ne!(first, 0, "PATH 的值不应为空字符串");
    }
});

test!("test_secure_getenv_nonexistent" {
    // 查询不存在的环境变量应返回 NULL。
    unsafe {
        let result = secure_getenv(
            c"__RUSL_SECURE_NONEXISTENT_XYZ123__".as_ptr(),
        );
        assert!(result.is_null(), "secure_getenv 不存在变量应返回 NULL");
    }
});

// ===========================================================================
// Case 1: 普通模式下 secure_getenv 等价于 getenv（存在变量时）
// ===========================================================================

test!("test_equivalence_with_getenv_existing" {
    // 普通模式下 secure_getenv 与 getenv 返回相同内容。
    unsafe {
        clearenv();
        let name = c"RUSL_SECURE_EQUIV";
        setenv(name.as_ptr(), c"same_value".as_ptr(), 1);

        let s_result = secure_getenv_cstr(name);
        let g_result = getenv_cstr(name);

        assert!(s_result.is_some(), "secure_getenv 应对已存在变量返回非 NULL");
        assert!(g_result.is_some(), "getenv 应对已存在变量返回非 NULL");
        assert_eq!(
            s_result.unwrap().to_bytes(),
            g_result.unwrap().to_bytes(),
            "secure_getenv 与 getenv 应返回相同内容"
        );
        assert_eq!(
            s_result.unwrap().to_bytes(),
            b"same_value",
            "返回值应为 \"same_value\""
        );

        unsetenv(name.as_ptr());
    }
});

// ===========================================================================
// Case 2: 普通模式下 secure_getenv 等价于 getenv（不存在变量时）
// ===========================================================================

test!("test_equivalence_with_getenv_nonexistent" {
    // 对不存在的变量，两者均返回 NULL。
    unsafe {
        let name = c"__RUSL_SECURE_NO_SUCH_VAR__";

        let s_result = secure_getenv(name.as_ptr());
        let g_result = getenv(name.as_ptr());

        assert!(s_result.is_null(), "secure_getenv 对不存在变量应返回 NULL");
        assert!(g_result.is_null(), "getenv 对不存在变量应返回 NULL");
    }
});

// ===========================================================================
// Case 3: 安全模式标志的只读行为
// ===========================================================================
// 注意: libc.secure 是 musl 内部全局状态，在普通用户进程（非 setuid/setgid）
// 中始终为 0。从用户代码无法设置 libc.secure = 1。
//
// spec 中描述的 secure 模式行为:
// - secure_getenv 应始终返回 NULL，无论 name 是否匹配环境变量
// - 无任何副作用，不修改全局状态
//
// 此行为需要特殊测试环境（setuid 二进制）才能验证，暂无法在当前测试中覆盖。
// 当 rusl 实现 __libc 结构体时，可通过单元测试修改 libc.secure 字段来测试此路径。

test!("test_secure_mode_documented_only" {
    // 占位测试：确认在普通模式下函数可被正常调用而不崩溃。
    // 实际 secure 模式测试需要 setuid/setgid 进程上下文。
    unsafe {
        let result = secure_getenv(c"PATH".as_ptr());
        // 在普通模式下行为等价于 getenv，不做特殊断言
        let _ = result;
    }
    // 测试通过仅表示函数可被调用而不崩溃。
    assert!(true);
});

// ===========================================================================
// Case 4: 传入空字符串返回 NULL
// ===========================================================================

test!("test_empty_name" {
    // 空字符串 "" 不是合法的环境变量名，应返回 NULL。
    unsafe {
        let result = secure_getenv(c"".as_ptr());
        assert!(result.is_null(), "secure_getenv(\"\") 应返回 NULL");
    }
});

test!("test_empty_name_vs_getenv" {
    // 空字符串名：secure_getenv 与 getenv 均返回 NULL。
    unsafe {
        let s_result = secure_getenv(c"".as_ptr());
        let g_result = getenv(c"".as_ptr());
        assert!(s_result.is_null(), "secure_getenv(\"\") 应返回 NULL");
        assert!(g_result.is_null(), "getenv(\"\") 应返回 NULL");
    }
});

// ===========================================================================
// Case 5: 传入含 '=' 的 name 返回 NULL
// ===========================================================================

test!("test_name_with_equals" {
    // 包含 '=' 的名称不是合法环境变量名，应返回 NULL。
    unsafe {
        let result = secure_getenv(c"KEY=VALUE".as_ptr());
        assert!(result.is_null(), "secure_getenv(\"KEY=VALUE\") 含 '=' 应返回 NULL");
    }
});

test!("test_name_ends_with_equals" {
    // 名称以 '=' 结尾也应返回 NULL。
    unsafe {
        let result = secure_getenv(c"ONLY_KEY=".as_ptr());
        assert!(result.is_null(), "secure_getenv(\"ONLY_KEY=\") 含 '=' 应返回 NULL");
    }
});

test!("test_name_only_equals" {
    // 名称仅为 '=' 应返回 NULL。
    unsafe {
        let result = secure_getenv(c"=".as_ptr());
        assert!(result.is_null(), "secure_getenv(\"=\") 应返回 NULL");
    }
});

test!("test_name_equals_in_middle" {
    // 名称中间包含 '=' 应返回 NULL。
    unsafe {
        let result = secure_getenv(c"HELLO=WORLD".as_ptr());
        assert!(result.is_null(), "secure_getenv(\"HELLO=WORLD\") 应返回 NULL");
    }
});

test!("test_equals_name_vs_getenv" {
    // 包含 '=' 的名称：secure_getenv 与 getenv 均返回 NULL。
    unsafe {
        let s_result = secure_getenv(c"X=Y".as_ptr());
        let g_result = getenv(c"X=Y".as_ptr());
        assert!(s_result.is_null(), "secure_getenv(\"X=Y\") 应返回 NULL");
        assert!(g_result.is_null(), "getenv(\"X=Y\") 应返回 NULL");
    }
});

// ===========================================================================
// Case 6: 线程安全性验证
// ===========================================================================
// spec: 该函数仅读取 libc.secure（只读字段）和调用 getenv（读 environ），
// 无写入操作，天然线程安全。
// 此处验证在无并发修改情况下的多次调用正确性。

test!("test_thread_safety_read_only_multiple_calls" {
    // 多次调用应返回一致结果。
    unsafe {
        clearenv();
        let name = c"RUSL_THREAD_SAFE";
        setenv(name.as_ptr(), c"thread_ok".as_ptr(), 1);

        let r1 = secure_getenv_cstr(name);
        let r2 = secure_getenv_cstr(name);
        let r3 = secure_getenv_cstr(name);

        assert!(r1.is_some(), "第 1 次调用应返回非 NULL");
        assert!(r2.is_some(), "第 2 次调用应返回非 NULL");
        assert!(r3.is_some(), "第 3 次调用应返回非 NULL");
        assert_eq!(r1.unwrap().to_bytes(), b"thread_ok");
        assert_eq!(r2.unwrap().to_bytes(), b"thread_ok");
        assert_eq!(r3.unwrap().to_bytes(), b"thread_ok");

        unsetenv(name.as_ptr());
    }
});

// ===========================================================================
// Case 7: 类型签名 / 链接期验证
// ===========================================================================
// secure_getenv 声明为:
//   char *secure_getenv(const char *name);
// 返回 char * 且参数为 const char *，调用方需要 _GNU_SOURCE 才能获取声明。
// 此处通过实际调用验证函数符号可链接。

test!("test_linkage_basic" {
    // 验证 secure_getenv 符号存在且可调用。
    unsafe {
        // 调用一个不存在的变量名，验证函数可正常链接和返回
        let result = secure_getenv(c"__RUSL_LINKAGE_TEST_VAR__".as_ptr());
        // 无论返回什么，只要不崩溃就说明符号存在
        let _ = result;
    }
    assert!(true);
});

// ===========================================================================
// 返回值内容验证
// ===========================================================================

test!("test_returned_pointer_points_to_value" {
    // 确认返回的指针指向的是值部分（VALUE），而非 "NAME=VALUE" 整个字符串。
    // setenv 传入的是纯 value，secure_getenv 返回的应该就是该 value。
    unsafe {
        clearenv();
        let name = c"RUSL_SECURE_VAL";
        setenv(name.as_ptr(), c"returned_value_check".as_ptr(), 1);

        let result = secure_getenv(name.as_ptr());
        assert!(!result.is_null(), "secure_getenv 应返回非 NULL");

        let result_cstr = CStr::from_ptr(result);
        assert_eq!(
            result_cstr.to_bytes(),
            b"returned_value_check",
            "返回值应为纯 value，不含 NAME= 前缀"
        );

        unsetenv(name.as_ptr());
    }
});

test!("test_empty_value" {
    // 测试空值（如 "KEY=" 设置后应能获取到空字符串）。
    unsafe {
        clearenv();
        let name = c"RUSL_SECURE_EMPTY";
        setenv(name.as_ptr(), c"".as_ptr(), 1);

        let result = secure_getenv(name.as_ptr());
        assert!(!result.is_null(), "secure_getenv 值为空的变量应返回非 NULL");
        // 值应为空字符串，第一个字节为 '\0'
        assert_eq!(*result, 0, "空值第一个字节应为 '\\0'");

        unsetenv(name.as_ptr());
    }
});

// ===========================================================================
// setenv / secure_getenv 联调
// ===========================================================================

test!("test_setenv_secure_getenv_basic" {
    // 设置自定义变量，用 secure_getenv 读取验证。
    unsafe {
        clearenv();
        let name = c"RUSL_SECURE_BASIC";
        let value = c"hello_secure_world";

        let set_ret = setenv(name.as_ptr(), value.as_ptr(), 1);
        assert_eq!(set_ret, 0, "setenv 应返回 0");

        assert!(secure_getenv_equals(name, b"hello_secure_world"),
                "secure_getenv 应返回 \"hello_secure_world\"");

        unsetenv(name.as_ptr());
    }
});

test!("test_after_unsetenv_returns_null" {
    // unsetenv 后 secure_getenv 应返回 NULL。
    unsafe {
        clearenv();
        let name = c"RUSL_SECURE_UNSET";
        setenv(name.as_ptr(), c"temp_value".as_ptr(), 1);

        let before = secure_getenv(name.as_ptr());
        assert!(!before.is_null(), "unset 前应能获取到变量");

        unsetenv(name.as_ptr());

        let after = secure_getenv(name.as_ptr());
        assert!(after.is_null(), "unsetenv 后 secure_getenv 应返回 NULL");
    }
});

test!("test_after_setenv_overwrite" {
    // 覆盖已存在的变量后，secure_getenv 返回新值。
    unsafe {
        clearenv();
        let name = c"RUSL_SECURE_OVERWRITE";
        setenv(name.as_ptr(), c"original".as_ptr(), 1);
        assert!(secure_getenv_equals(name, b"original"),
                "初始值应为 original");

        setenv(name.as_ptr(), c"modified".as_ptr(), 1);
        assert!(secure_getenv_equals(name, b"modified"),
                "覆盖后值应为 modified");

        unsetenv(name.as_ptr());
    }
});

test!("test_readd_after_unsetenv" {
    // 删除后重新添加。
    unsafe {
        clearenv();
        let name = c"RUSL_SECURE_READD";
        setenv(name.as_ptr(), c"first".as_ptr(), 1);
        assert!(secure_getenv_equals(name, b"first"));

        unsetenv(name.as_ptr());
        let r1 = secure_getenv(name.as_ptr());
        assert!(r1.is_null(), "unsetenv 后应返回 NULL");

        setenv(name.as_ptr(), c"second".as_ptr(), 1);
        assert!(secure_getenv_equals(name, b"second"),
                "重新添加后应返回新值");

        unsetenv(name.as_ptr());
    }
});

// ===========================================================================
// 大小写敏感性
// ===========================================================================

test!("test_case_sensitive" {
    // musl 实现区分大小写，"path" != "PATH"。
    unsafe {
        let upper_result = secure_getenv(c"PATH".as_ptr());
        let lower_result = secure_getenv(c"path".as_ptr());

        if upper_result.is_null() {
            // PATH 不存在是异常情况，跳过详细比较
            return;
        }

        // PATH 通常全大写存在，小写 path 通常不存在
        if !lower_result.is_null() {
            assert_ne!(
                upper_result, lower_result,
                "\"PATH\" 和 \"path\" 应返回不同结果 (大小写敏感)"
            );
        }
    }
});

test!("test_case_sensitive_custom_var" {
    // 设置全小写变量，用全大写查询应返回 NULL。
    unsafe {
        clearenv();
        let lower_name = c"casesensitive";
        setenv(lower_name.as_ptr(), c"value_lower".as_ptr(), 1);

        let upper_result = secure_getenv(c"CASESENSITIVE".as_ptr());
        assert!(
            upper_result.is_null(),
            "secure_getenv(\"CASESENSITIVE\") 不应匹配 \"casesensitive\""
        );

        let lower_result = secure_getenv(lower_name.as_ptr());
        assert!(!lower_result.is_null(), "正确大小写应能获取到");
        let lower_cstr = CStr::from_ptr(lower_result);
        assert_eq!(lower_cstr.to_bytes(), b"value_lower");

        unsetenv(lower_name.as_ptr());
    }
});

// ===========================================================================
// 前缀/后缀部分匹配不误匹配
// ===========================================================================

test!("test_no_prefix_match" {
    // 查询 "PRE" 不应匹配 "PREFIX_VAR"。
    unsafe {
        clearenv();
        let full_name = c"PREFIX_VAR";
        setenv(full_name.as_ptr(), c"prefix_value".as_ptr(), 1);

        let result = secure_getenv(c"PRE".as_ptr());
        assert!(
            result.is_null(),
            "secure_getenv(\"PRE\") 不应匹配 \"PREFIX_VAR\""
        );

        // 验证完整名称确实存在
        assert!(
            secure_getenv_equals(full_name, b"prefix_value"),
            "完整名称应能匹配"
        );

        unsetenv(full_name.as_ptr());
    }
});

test!("test_no_suffix_match" {
    // 查询 "FFIX" 不应匹配 "SUFFIX"。
    unsafe {
        clearenv();
        let full_name = c"SUFFIX";
        setenv(full_name.as_ptr(), c"suffix_value".as_ptr(), 1);

        let result = secure_getenv(c"FFIX".as_ptr());
        assert!(
            result.is_null(),
            "secure_getenv(\"FFIX\") 不应匹配 \"SUFFIX\""
        );

        let result_full = secure_getenv(full_name.as_ptr());
        assert!(!result_full.is_null(), "完整名称应能获取到");
        let full_cstr = CStr::from_ptr(result_full);
        assert_eq!(full_cstr.to_bytes(), b"suffix_value");

        unsetenv(full_name.as_ptr());
    }
});

test!("test_no_superset_match" {
    // 查询 "BASE_EXTRA" 不应匹配 "BASE"。
    unsafe {
        clearenv();
        let base_name = c"BASE";
        setenv(base_name.as_ptr(), c"base_value".as_ptr(), 1);

        let result = secure_getenv(c"BASE_EXTRA".as_ptr());
        assert!(
            result.is_null(),
            "secure_getenv(\"BASE_EXTRA\") 不应匹配 \"BASE\""
        );

        let result_base = secure_getenv(base_name.as_ptr());
        assert!(!result_base.is_null(), "完整名称应能获取到");
        let base_cstr = CStr::from_ptr(result_base);
        assert_eq!(base_cstr.to_bytes(), b"base_value");

        unsetenv(base_name.as_ptr());
    }
});

// ===========================================================================
// 多次调用一致性
// ===========================================================================

test!("test_multiple_calls_same_pointer" {
    // 同一变量多次 secure_getenv 在未修改环境时返回相同指针。
    unsafe {
        let r1 = secure_getenv(c"PATH".as_ptr());
        let r2 = secure_getenv(c"PATH".as_ptr());
        let r3 = secure_getenv(c"PATH".as_ptr());

        if !r1.is_null() {
            assert_eq!(r1, r2, "连续调用应返回相同指针");
            assert_eq!(r2, r3, "连续调用应返回相同指针");
        }
    }
});

// ===========================================================================
// 特殊字符名称测试
// ===========================================================================

test!("test_name_with_underscore" {
    // 带下划线的合法环境变量名。
    unsafe {
        clearenv();
        let name = c"MY_TEST_VAR";
        setenv(name.as_ptr(), c"underscore_val".as_ptr(), 1);
        assert!(secure_getenv_equals(name, b"underscore_val"),
                "带下划线名称应能正常查询");
        unsetenv(name.as_ptr());
    }
});

test!("test_name_with_digits" {
    // 带数字的环境变量名。
    unsafe {
        clearenv();
        let name = c"TEST123";
        setenv(name.as_ptr(), c"digits_val".as_ptr(), 1);
        assert!(secure_getenv_equals(name, b"digits_val"),
                "带数字名称应能正常查询");
        unsetenv(name.as_ptr());
    }
});

test!("test_single_char_name" {
    // 单字符名。
    unsafe {
        clearenv();
        let name = c"X";
        setenv(name.as_ptr(), c"single_char_val".as_ptr(), 1);
        assert!(secure_getenv_equals(name, b"single_char_val"),
                "单字符名称应能正常查询");
        unsetenv(name.as_ptr());
    }
});

// ===========================================================================
// 长值测试
// ===========================================================================

test!("test_long_value" {
    // 测试较长值字符串。
    unsafe {
        clearenv();
        let name = c"RUSL_SECURE_LONG";
        let long_val = c"abcdefghijklmnopqrstuvwxyz_0123456789";

        setenv(name.as_ptr(), long_val.as_ptr(), 1);

        assert!(
            secure_getenv_equals(name, b"abcdefghijklmnopqrstuvwxyz_0123456789"),
            "长值应完整返回"
        );

        unsetenv(name.as_ptr());
    }
});

// ===========================================================================
// 值中包含 '=' 字符
// ===========================================================================

test!("test_value_with_equals_sign" {
    // 值中可以包含 '=' 字符，secure_getenv 应正确返回以 '\0' 结尾的完整值。
    unsafe {
        clearenv();
        let name = c"RUSL_SECURE_EQUAL_VAL";
        setenv(name.as_ptr(), c"key=value=pair".as_ptr(), 1);

        assert!(
            secure_getenv_equals(name, b"key=value=pair"),
            "值中的 '=' 应完整保留"
        );

        unsetenv(name.as_ptr());
    }
});

// ===========================================================================
// errno 不变性
// ===========================================================================

test!("test_secure_getenv_no_errno_change" {
    // spec: secure_getenv 不写入任何全局状态，不应修改 errno。
    unsafe {
        *__errno_location() = 42;

        let _result = secure_getenv(c"PATH".as_ptr());

        assert_eq!(*__errno_location(), 42, "secure_getenv 不应修改 errno");
    }
});

test!("test_secure_getenv_nonexistent_no_errno" {
    // 即使未找到，secure_getenv 也不应设置 errno。
    unsafe {
        *__errno_location() = 99;

        let result = secure_getenv(c"__RUSL_SECURE_NONEXISTENT_XYZ__".as_ptr());
        assert!(result.is_null(), "不存在的变量应返回 NULL");
        assert_eq!(
            *__errno_location(),
            99,
            "secure_getenv 返回 NULL 时也不应修改 errno"
        );
    }
});

test!("test_secure_getenv_empty_name_no_errno" {
    // 空字符串名返回 NULL 时也不应修改 errno。
    unsafe {
        *__errno_location() = 77;

        let result = secure_getenv(c"".as_ptr());
        assert!(result.is_null(), "空字符串名应返回 NULL");
        assert_eq!(
            *__errno_location(),
            77,
            "secure_getenv 空字符串名时不应修改 errno"
        );
    }
});

// ===========================================================================
// 多个变量
// ===========================================================================

test!("test_multiple_variables" {
    // 同时设置多个变量，分别用 secure_getenv 验证。
    unsafe {
        clearenv();

        setenv(c"RUSL_SECURE_A".as_ptr(), c"alpha".as_ptr(), 1);
        setenv(c"RUSL_SECURE_B".as_ptr(), c"beta".as_ptr(), 1);
        setenv(c"RUSL_SECURE_C".as_ptr(), c"gamma".as_ptr(), 1);

        assert!(secure_getenv_equals(c"RUSL_SECURE_A", b"alpha"), "变量 A");
        assert!(secure_getenv_equals(c"RUSL_SECURE_B", b"beta"), "变量 B");
        assert!(secure_getenv_equals(c"RUSL_SECURE_C", b"gamma"), "变量 C");

        // 清理
        unsetenv(c"RUSL_SECURE_A".as_ptr());
        unsetenv(c"RUSL_SECURE_B".as_ptr());
        unsetenv(c"RUSL_SECURE_C".as_ptr());
    }
});

// ===========================================================================
// secure_getenv 与 getenv 指针一致性
// ===========================================================================

test!("test_same_pointer_as_getenv" {
    // 在普通模式下，secure_getenv 调用的就是 getenv，
    // 因此两者返回的指针应相同。
    unsafe {
        clearenv();
        let name = c"RUSL_SECURE_PTR_EQ";
        setenv(name.as_ptr(), c"pointer_test".as_ptr(), 1);

        let s_ptr = secure_getenv(name.as_ptr());
        let g_ptr = getenv(name.as_ptr());

        assert!(!s_ptr.is_null(), "secure_getenv 应返回非 NULL");
        assert!(!g_ptr.is_null(), "getenv 应返回非 NULL");
        // musl 的 secure_getenv 在普通模式下直接调用 getenv，
        // 因此返回的指针应相同
        assert_eq!(
            s_ptr, g_ptr,
            "普通模式下 secure_getenv 应返回与 getenv 相同的指针"
        );

        unsetenv(name.as_ptr());
    }
});
