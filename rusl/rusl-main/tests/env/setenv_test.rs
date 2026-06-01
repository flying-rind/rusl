/// 模块: setenv_test
/// `setenv` 集成测试
///
/// 基于 `src/env/spec/setenv.md` 规约生成。
/// 测试 musl libc 的 `setenv` 函数:
/// 向进程环境变量列表中添加或更新环境变量，自行分配并复制字符串。
///
/// ## spec 约束
///
/// - 成功返回 0，失败返回 -1 并设置 errno = EINVAL
/// - `var` 不能为 NULL、空字符串、含 '=' 字符
/// - `overwrite == 0` 且变量已存在时：不修改，直接返回 0
/// - `overwrite != 0` 或变量不存在时：分配并构造 "var=value" 字符串插入环境
/// - `value` 可以为空字符串
///
/// ## 测试覆盖清单
///
/// | # | 场景 | 测试函数 |
/// |---|------|----------|
/// | 1 | 添加新变量（overwrite=1）返回 0 | `test_setenv_return_zero_on_success` |
/// | 2 | 添加新变量后 getenv 能查到 | `test_setenv_new_var` |
/// | 3 | overwrite=0 且变量已存在时不修改 | `test_setenv_overwrite_zero_existing` |
/// | 4 | overwrite=1 覆盖已存在的变量 | `test_setenv_overwrite_nonzero_existing` |
/// | 5 | var 为 NULL 时返回 -1 且 errno=EINVAL | `test_setenv_null_var` |
/// | 6 | var 为空字符串时返回 -1 且 errno=EINVAL | `test_setenv_empty_var` |
/// | 7 | var 含 '=' 时返回 -1 且 errno=EINVAL | `test_setenv_var_contains_equals` |
/// | 8 | value 为空字符串时正常工作 | `test_setenv_empty_value` |
/// | 9 | 设置后 unsetenv 能移除 | `test_setenv_then_unsetenv` |
/// | 10 | clearenv 后 setenv 可从空环境添加 | `test_setenv_after_clearenv` |
/// | 11 | 多次 setenv/unsetenv 循环 | `test_setenv_unsetenv_cycle` |

use core::ffi::{c_char, c_int, CStr};
use rusl_core::test;
use super::*;

const EINVAL: c_int = 22;

extern "C" {
    /// POSIX.1-2001: 设置环境变量（拷贝语义）
    fn setenv(var: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;
    /// POSIX.1-2001: 移除环境变量
    fn unsetenv(name: *const c_char) -> c_int;
    /// POSIX.1-2001: 获取环境变量值
    fn getenv(name: *const c_char) -> *mut c_char;
    /// GNU 扩展: 清空所有环境变量
    fn clearenv() -> c_int;
}

// ===========================================================================
// 场景 1 & 2: 添加新变量（overwrite=1）返回 0，getenv 能查到
// ===========================================================================

test!("test_setenv_new_var" {
    let name = CStr::from_bytes_with_nul(b"RUSL_SENV_NEW\0").unwrap();
    let value = CStr::from_bytes_with_nul(b"hello\0").unwrap();

    unsafe {
        // 清理可能残留的旧值
        unsetenv(name.as_ptr());

        // 设置环境变量
        let ret = setenv(name.as_ptr(), value.as_ptr(), 1);
        assert_eq!(ret, 0);

        // 通过 getenv 验证
        let result = getenv(name.as_ptr());
        assert!(!result.is_null());
        let cstr = CStr::from_ptr(result);
        assert_eq!(cstr.to_bytes(), b"hello");

        // 清理
        unsetenv(name.as_ptr());
    }
});

test!("test_setenv_return_zero_on_success" {
    let name = CStr::from_bytes_with_nul(b"RUSL_SENV_RET0\0").unwrap();
    let value = CStr::from_bytes_with_nul(b"x\0").unwrap();

    unsafe {
        unsetenv(name.as_ptr());

        let ret = setenv(name.as_ptr(), value.as_ptr(), 1);
        assert_eq!(ret, 0, "setenv 成功时应返回 0");

        unsetenv(name.as_ptr());
    }
});

// ===========================================================================
// 场景 8: value 为空字符串时正常工作（变量被设为空值）
// ===========================================================================

test!("test_setenv_empty_value" {
    let name = CStr::from_bytes_with_nul(b"RUSL_SENV_EMPTY\0").unwrap();
    let value = CStr::from_bytes_with_nul(b"\0").unwrap();

    unsafe {
        unsetenv(name.as_ptr());

        let ret = setenv(name.as_ptr(), value.as_ptr(), 1);
        assert_eq!(ret, 0);

        let result = getenv(name.as_ptr());
        assert!(!result.is_null());
        let cstr = CStr::from_ptr(result);
        assert_eq!(cstr.to_bytes(), b"");

        unsetenv(name.as_ptr());
    }
});

// ===========================================================================
// 补充: value 中包含 '=' 是合法的（仅 var 不能含 '='）
// ===========================================================================

test!("test_setenv_value_with_equals" {
    let name = CStr::from_bytes_with_nul(b"RUSL_SENV_VALEQ\0").unwrap();
    let value = CStr::from_bytes_with_nul(b"a=b\0").unwrap();

    unsafe {
        unsetenv(name.as_ptr());

        let ret = setenv(name.as_ptr(), value.as_ptr(), 1);
        assert_eq!(ret, 0);

        let result = getenv(name.as_ptr());
        assert!(!result.is_null());
        let cstr = CStr::from_ptr(result);
        assert_eq!(cstr.to_bytes(), b"a=b");

        unsetenv(name.as_ptr());
    }
});

// ===========================================================================
// 场景 3: overwrite=0 且变量已存在时：不修改，直接返回 0
// ===========================================================================

test!("test_setenv_overwrite_zero_existing" {
    let name = CStr::from_bytes_with_nul(b"RUSL_SENV_NOOW\0").unwrap();
    let first = CStr::from_bytes_with_nul(b"original\0").unwrap();
    let second = CStr::from_bytes_with_nul(b"changed\0").unwrap();

    unsafe {
        unsetenv(name.as_ptr());

        // 首次设置
        let ret = setenv(name.as_ptr(), first.as_ptr(), 1);
        assert_eq!(ret, 0);

        // overwrite=0 尝试覆盖
        let ret = setenv(name.as_ptr(), second.as_ptr(), 0);
        assert_eq!(ret, 0, "overwrite=0 且变量已存在时应返回 0");

        // 值不应被修改
        let result = getenv(name.as_ptr());
        assert!(!result.is_null());
        let cstr = CStr::from_ptr(result);
        assert_eq!(cstr.to_bytes(), b"original", "overwrite=0 时值不应被修改");

        unsetenv(name.as_ptr());
    }
});

test!("test_setenv_overwrite_zero_checks_getenv" {
    // overwrite=0 时，若 getenv(var) != NULL 则直接返回 0
    // 验证这种情况下变量值确实不变
    let name = CStr::from_bytes_with_nul(b"RUSL_SENV_CHK\0").unwrap();
    let first = CStr::from_bytes_with_nul(b"keep_me\0").unwrap();
    let second = CStr::from_bytes_with_nul(b"ignore_me\0").unwrap();

    unsafe {
        unsetenv(name.as_ptr());

        // 先设置
        setenv(name.as_ptr(), first.as_ptr(), 1);

        // overwrite=0 尝试覆盖 — 应保持 first 的值
        let ret = setenv(name.as_ptr(), second.as_ptr(), 0);
        assert_eq!(ret, 0);

        let result = getenv(name.as_ptr());
        assert!(!result.is_null());
        let cstr = CStr::from_ptr(result);
        assert_eq!(cstr.to_bytes(), b"keep_me");

        unsetenv(name.as_ptr());
    }
});

// ===========================================================================
// 场景 4: overwrite=1 覆盖已存在的变量
// ===========================================================================

test!("test_setenv_overwrite_nonzero_existing" {
    let name = CStr::from_bytes_with_nul(b"RUSL_SENV_OVERW\0").unwrap();
    let first = CStr::from_bytes_with_nul(b"original\0").unwrap();
    let second = CStr::from_bytes_with_nul(b"new_value\0").unwrap();

    unsafe {
        unsetenv(name.as_ptr());

        let ret = setenv(name.as_ptr(), first.as_ptr(), 1);
        assert_eq!(ret, 0);

        // overwrite!=0 覆盖
        let ret = setenv(name.as_ptr(), second.as_ptr(), 1);
        assert_eq!(ret, 0, "overwrite!=0 时应成功覆盖");

        let result = getenv(name.as_ptr());
        assert!(!result.is_null());
        let cstr = CStr::from_ptr(result);
        assert_eq!(cstr.to_bytes(), b"new_value", "overwrite!=0 时值应被覆盖");

        unsetenv(name.as_ptr());
    }
});

test!("test_setenv_overwrite_zero_new_var" {
    // overwrite=0 且变量不存在: 应正常添加（spec Case 4 路径）
    let name = CStr::from_bytes_with_nul(b"RUSL_SENV_NOOW2\0").unwrap();
    let value = CStr::from_bytes_with_nul(b"fresh\0").unwrap();

    unsafe {
        unsetenv(name.as_ptr());

        let ret = setenv(name.as_ptr(), value.as_ptr(), 0);
        assert_eq!(ret, 0, "overwrite=0 且变量不存在时应成功添加");

        let result = getenv(name.as_ptr());
        assert!(!result.is_null());
        let cstr = CStr::from_ptr(result);
        assert_eq!(cstr.to_bytes(), b"fresh");

        unsetenv(name.as_ptr());
    }
});

// ===========================================================================
// 场景 5 & 6 & 7: 参数校验 — 非法 var
// ===========================================================================

test!("test_setenv_null_var" {
    // var == NULL: 返回 -1, errno = EINVAL
    // spec Case 1: 参数校验失败
    let value = CStr::from_bytes_with_nul(b"test\0").unwrap();

    unsafe {
        let ret = setenv(core::ptr::null(), value.as_ptr(), 1);
        assert_eq!(ret, -1, "var 为 NULL 时应返回 -1");
        assert_eq!(*__errno_location(), EINVAL, "var 为 NULL 时 errno 应设为 EINVAL");
    }
});

test!("test_setenv_empty_var" {
    // var 为空字符串: 返回 -1, errno = EINVAL
    // spec: l1 == 0 (空字符串) → goto invalid
    let var = CStr::from_bytes_with_nul(b"\0").unwrap();
    let value = CStr::from_bytes_with_nul(b"test\0").unwrap();

    unsafe {
        let ret = setenv(var.as_ptr(), value.as_ptr(), 1);
        assert_eq!(ret, -1, "var 为空字符串时应返回 -1");
        assert_eq!(*__errno_location(), EINVAL, "var 为空字符串时 errno 应设为 EINVAL");
    }
});

test!("test_setenv_var_contains_equals" {
    // var 中包含 '=': 返回 -1, errno = EINVAL
    // spec: var[l1] != '\0' (找到了 '=') → goto invalid
    let var = CStr::from_bytes_with_nul(b"BAD=NAME\0").unwrap();
    let value = CStr::from_bytes_with_nul(b"test\0").unwrap();

    unsafe {
        let ret = setenv(var.as_ptr(), value.as_ptr(), 1);
        assert_eq!(ret, -1, "var 含 '=' 时应返回 -1");
        assert_eq!(*__errno_location(), EINVAL, "var 含 '=' 时 errno 应设为 EINVAL");
    }
});

test!("test_setenv_var_equals_only" {
    // var 仅为 "=": 同时满足"空字符串"(l1==0)和"含="两个条件
    // spec: __strchrnul(var, '=') - var → l1 == 0,
    // 但 var[l1] 为 '=' (非 '\0')，任一条件都会触发 EINVAL
    let var = CStr::from_bytes_with_nul(b"=\0").unwrap();
    let value = CStr::from_bytes_with_nul(b"test\0").unwrap();

    unsafe {
        let ret = setenv(var.as_ptr(), value.as_ptr(), 1);
        assert_eq!(ret, -1, "var 为 '=' 时应返回 -1");
        assert_eq!(*__errno_location(), EINVAL, "var 为 '=' 时 errno 应设为 EINVAL");
    }
});

test!("test_setenv_var_equals_in_middle" {
    // var 中间包含 '='，如 "FOO=BAR"
    let var = CStr::from_bytes_with_nul(b"FOO=BAR\0").unwrap();
    let value = CStr::from_bytes_with_nul(b"test\0").unwrap();

    unsafe {
        let ret = setenv(var.as_ptr(), value.as_ptr(), 1);
        assert_eq!(ret, -1, "var 中间含 '=' 时应返回 -1");
        assert_eq!(*__errno_location(), EINVAL, "var 含 '=' 时 errno 应设为 EINVAL");
    }
});

// ===========================================================================
// 更新已存在变量 — 多次更新验证最终值
// ===========================================================================

test!("test_setenv_update_multiple" {
    let name = CStr::from_bytes_with_nul(b"RUSL_SENV_MULTI\0").unwrap();
    let v1 = CStr::from_bytes_with_nul(b"one\0").unwrap();
    let v2 = CStr::from_bytes_with_nul(b"two\0").unwrap();
    let v3 = CStr::from_bytes_with_nul(b"three\0").unwrap();

    unsafe {
        unsetenv(name.as_ptr());

        assert_eq!(setenv(name.as_ptr(), v1.as_ptr(), 1), 0);
        assert_eq!(setenv(name.as_ptr(), v2.as_ptr(), 1), 0);
        assert_eq!(setenv(name.as_ptr(), v3.as_ptr(), 1), 0);

        let result = getenv(name.as_ptr());
        assert!(!result.is_null());
        let cstr = CStr::from_ptr(result);
        assert_eq!(cstr.to_bytes(), b"three", "多次更新后应为最后一次设置的值");

        unsetenv(name.as_ptr());
    }
});

// ===========================================================================
// 多变量测试 — 多个变量相互独立
// ===========================================================================

test!("test_setenv_multiple_vars" {
    let name1 = CStr::from_bytes_with_nul(b"RUSL_SENV_M1\0").unwrap();
    let name2 = CStr::from_bytes_with_nul(b"RUSL_SENV_M2\0").unwrap();
    let val1 = CStr::from_bytes_with_nul(b"alpha\0").unwrap();
    let val2 = CStr::from_bytes_with_nul(b"beta\0").unwrap();

    unsafe {
        unsetenv(name1.as_ptr());
        unsetenv(name2.as_ptr());

        assert_eq!(setenv(name1.as_ptr(), val1.as_ptr(), 1), 0);
        assert_eq!(setenv(name2.as_ptr(), val2.as_ptr(), 1), 0);

        // 验证 name1
        let r1 = getenv(name1.as_ptr());
        assert!(!r1.is_null());
        assert_eq!(CStr::from_ptr(r1).to_bytes(), b"alpha");

        // 验证 name2
        let r2 = getenv(name2.as_ptr());
        assert!(!r2.is_null());
        assert_eq!(CStr::from_ptr(r2).to_bytes(), b"beta");

        // 两者应独立
        assert_ne!(r1, r2, "不同变量的环境条目指针应不同");

        unsetenv(name1.as_ptr());
        unsetenv(name2.as_ptr());
    }
});

// ===========================================================================
// 成功路径不修改 errno
// ===========================================================================

test!("test_setenv_success_does_not_set_errno" {
    // 成功调用不应修改 errno
    let name = CStr::from_bytes_with_nul(b"RUSL_SENV_ERRNO\0").unwrap();
    let value = CStr::from_bytes_with_nul(b"ok\0").unwrap();

    unsafe {
        unsetenv(name.as_ptr());

        // 将 errno 设置为一个已知值 (非 EINVAL)
        *__errno_location() = 9999;

        let ret = setenv(name.as_ptr(), value.as_ptr(), 1);
        assert_eq!(ret, 0);

        // errno 不应被修改
        // 注: POSIX 标准未规定成功时 errno 的行为, 此测试验证 musl 实现行为
        let e = *__errno_location();
        assert_eq!(e, 9999, "成功调用后 errno 应保持不变");

        unsetenv(name.as_ptr());
    }
});

// ===========================================================================
// 场景 9: 设置后 unsetenv 能移除
// ===========================================================================

test!("test_setenv_then_unsetenv" {
    // spec: setenv 分配的字符串由 __env_rm_add 追踪，
    // unsetenv 应能正常移除该变量。
    let name = CStr::from_bytes_with_nul(b"RUSL_SENV_RM\0").unwrap();
    let value = CStr::from_bytes_with_nul(b"to_be_removed\0").unwrap();

    unsafe {
        // 确保初始状态干净
        unsetenv(name.as_ptr());

        // 1. 设置变量
        let ret = setenv(name.as_ptr(), value.as_ptr(), 1);
        assert_eq!(ret, 0, "setenv 应成功");

        // 2. 验证 getenv 能查到
        let result = getenv(name.as_ptr());
        assert!(!result.is_null(), "setenv 后 getenv 应能找到变量");
        let cstr = CStr::from_ptr(result);
        assert_eq!(cstr.to_bytes(), b"to_be_removed");

        // 3. unsetenv 移除
        let ret = unsetenv(name.as_ptr());
        assert_eq!(ret, 0, "unsetenv 应成功返回 0");

        // 4. 验证 getenv 返回 NULL
        let result = getenv(name.as_ptr());
        assert!(result.is_null(), "unsetenv 后 getenv 应返回 NULL");
    }
});

test!("test_setenv_then_unsetenv_twice" {
    // 验证 setenv 后的变量可以被 unsetenv 移除，
    // 且重复 unsetenv 同一变量不会出错。
    let name = CStr::from_bytes_with_nul(b"RUSL_SENV_RM2\0").unwrap();
    let value = CStr::from_bytes_with_nul(b"remove_me\0").unwrap();

    unsafe {
        unsetenv(name.as_ptr());

        setenv(name.as_ptr(), value.as_ptr(), 1);

        // 第一次 unsetenv
        let ret = unsetenv(name.as_ptr());
        assert_eq!(ret, 0, "第一次 unsetenv 应成功");

        // 第二次 unsetenv 同一变量 — 不应报错
        let ret = unsetenv(name.as_ptr());
        assert_eq!(ret, 0, "重复 unsetenv 同一变量应返回 0");

        // getenv 确认变量不在环境中
        let result = getenv(name.as_ptr());
        assert!(result.is_null(), "unsetenv 后变量不应存在");
    }
});

// ===========================================================================
// 场景 10: clearenv 后 setenv 可从空环境添加
// ===========================================================================

test!("test_setenv_after_clearenv" {
    // spec: clearenv 清空环境后，setenv 应能正常添加新变量。
    // 这验证 setenv 不依赖预先存在的 environ 条目。
    let name = CStr::from_bytes_with_nul(b"RUSL_SENV_CLR\0").unwrap();
    let value = CStr::from_bytes_with_nul(b"from_empty\0").unwrap();

    unsafe {
        // 1. 清空所有环境变量
        let ret = clearenv();
        assert_eq!(ret, 0, "clearenv 应返回 0");

        // 2. 从空环境添加变量
        let ret = setenv(name.as_ptr(), value.as_ptr(), 1);
        assert_eq!(ret, 0, "clearenv 后 setenv 应成功");

        // 3. getenv 验证
        let result = getenv(name.as_ptr());
        assert!(!result.is_null(), "clearenv 后 setenv 的变量应可通过 getenv 查到");
        let cstr = CStr::from_ptr(result);
        assert_eq!(cstr.to_bytes(), b"from_empty");

        // 4. 再次 clearenv 清理
        clearenv();
    }
});

test!("test_setenv_after_clearenv_multiple" {
    // clearenv 后连续 setenv 多个变量
    let name1 = CStr::from_bytes_with_nul(b"RUSL_CLR_A\0").unwrap();
    let name2 = CStr::from_bytes_with_nul(b"RUSL_CLR_B\0").unwrap();
    let val1 = CStr::from_bytes_with_nul(b"first\0").unwrap();
    let val2 = CStr::from_bytes_with_nul(b"second\0").unwrap();

    unsafe {
        clearenv();

        assert_eq!(setenv(name1.as_ptr(), val1.as_ptr(), 1), 0);
        assert_eq!(setenv(name2.as_ptr(), val2.as_ptr(), 1), 0);

        let r1 = getenv(name1.as_ptr());
        assert!(!r1.is_null());
        assert_eq!(CStr::from_ptr(r1).to_bytes(), b"first");

        let r2 = getenv(name2.as_ptr());
        assert!(!r2.is_null());
        assert_eq!(CStr::from_ptr(r2).to_bytes(), b"second");

        clearenv();
    }
});

// ===========================================================================
// 场景 11: 多次 setenv/unsetenv 循环
// ===========================================================================

test!("test_setenv_unsetenv_cycle" {
    // 多次 setenv -> unsetenv -> setenv 循环，验证状态一致性
    let name = CStr::from_bytes_with_nul(b"RUSL_SENV_CYC\0").unwrap();
    let v1 = CStr::from_bytes_with_nul(b"cycle_1\0").unwrap();
    let v2 = CStr::from_bytes_with_nul(b"cycle_2\0").unwrap();
    let v3 = CStr::from_bytes_with_nul(b"cycle_3\0").unwrap();

    unsafe {
        unsetenv(name.as_ptr());

        // 第 1 轮: set -> verify -> unset -> verify gone
        assert_eq!(setenv(name.as_ptr(), v1.as_ptr(), 1), 0);
        let r = getenv(name.as_ptr());
        assert!(!r.is_null());
        assert_eq!(CStr::from_ptr(r).to_bytes(), b"cycle_1");
        assert_eq!(unsetenv(name.as_ptr()), 0);
        assert!(getenv(name.as_ptr()).is_null());

        // 第 2 轮: set with different value
        assert_eq!(setenv(name.as_ptr(), v2.as_ptr(), 1), 0);
        let r = getenv(name.as_ptr());
        assert!(!r.is_null());
        assert_eq!(CStr::from_ptr(r).to_bytes(), b"cycle_2");
        assert_eq!(unsetenv(name.as_ptr()), 0);
        assert!(getenv(name.as_ptr()).is_null());

        // 第 3 轮: set again
        assert_eq!(setenv(name.as_ptr(), v3.as_ptr(), 1), 0);
        let r = getenv(name.as_ptr());
        assert!(!r.is_null());
        assert_eq!(CStr::from_ptr(r).to_bytes(), b"cycle_3");

        // 清理
        unsetenv(name.as_ptr());
    }
});

test!("test_setenv_overwrite_then_unsetenv_cycle" {
    // overwrite=1 覆盖已存在变量后，unsetenv 再重新 setenv
    let name = CStr::from_bytes_with_nul(b"RUSL_SENV_CYC2\0").unwrap();
    let orig = CStr::from_bytes_with_nul(b"original\0").unwrap();
    let updated = CStr::from_bytes_with_nul(b"updated\0").unwrap();
    let restored = CStr::from_bytes_with_nul(b"restored\0").unwrap();

    unsafe {
        unsetenv(name.as_ptr());

        // 初始设置
        setenv(name.as_ptr(), orig.as_ptr(), 1);
        assert_eq!(CStr::from_ptr(getenv(name.as_ptr())).to_bytes(), b"original");

        // 覆盖
        setenv(name.as_ptr(), updated.as_ptr(), 1);
        assert_eq!(CStr::from_ptr(getenv(name.as_ptr())).to_bytes(), b"updated");

        // 移除
        unsetenv(name.as_ptr());
        assert!(getenv(name.as_ptr()).is_null());

        // 重新设置
        setenv(name.as_ptr(), restored.as_ptr(), 1);
        assert_eq!(CStr::from_ptr(getenv(name.as_ptr())).to_bytes(), b"restored");

        // 清理
        unsetenv(name.as_ptr());
    }
});

// ===========================================================================
// 补充: 非法参数后环境不变
// ===========================================================================

test!("test_setenv_invalid_var_does_not_change_env" {
    // spec Case 1: 参数校验失败时，环境变量列表不发生任何变化。
    // 验证非法 var 不会影响已存在的同名变量。
    let name = CStr::from_bytes_with_nul(b"RUSL_SENV_PRE\0").unwrap();
    let value = CStr::from_bytes_with_nul(b"preserved\0").unwrap();

    unsafe {
        unsetenv(name.as_ptr());

        // 先正常设置
        setenv(name.as_ptr(), value.as_ptr(), 1);
        let r = getenv(name.as_ptr());
        assert!(!r.is_null());
        assert_eq!(CStr::from_ptr(r).to_bytes(), b"preserved");

        // 用含 '=' 的同名 var 尝试 — 应失败且不改变原值
        let bad_var = CStr::from_bytes_with_nul(b"RUSL_SENV_PRE=BAD\0").unwrap();
        let new_val = CStr::from_bytes_with_nul(b"should_not_apply\0").unwrap();
        let ret = setenv(bad_var.as_ptr(), new_val.as_ptr(), 1);
        assert_eq!(ret, -1, "非法 var 应返回 -1");

        // 原变量值不应被修改
        let r = getenv(name.as_ptr());
        assert!(!r.is_null(), "非法 setenv 不应删除已存在的变量");
        assert_eq!(
            CStr::from_ptr(r).to_bytes(),
            b"preserved",
            "非法 setenv 不应修改同名变量的值"
        );

        unsetenv(name.as_ptr());
    }
});
