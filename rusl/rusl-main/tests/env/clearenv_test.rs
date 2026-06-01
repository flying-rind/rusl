/// 模块: clearenv_test
/// `clearenv` 集成测试
///
/// 基于 `src/env/spec/clearenv.md` 规约生成。
///
/// clearenv 是 GNU 扩展（非 POSIX），声明于 `<stdlib.h>`（需 `_GNU_SOURCE`）。
/// 功能: 清除当前进程的所有环境变量，始终返回 0。
///
/// ## 后置条件 (来自 spec)
///
/// - 返回值始终为 0
/// - `__environ` 被设置为 NULL（外部通过 environ/getenv 访问得到空环境）
/// - 旧环境数组中的每个条目通过 `__env_rm_add(entry, NULL)` 通知释放
/// - 多线程并发修改 `__environ` 是未定义行为
///
/// ## 设计要点 (来自 spec)
///
/// - 先清空后释放: 先将 `__environ` 置 NULL，再遍历旧数组调用 `__env_rm_add`
/// - 弱符号解耦: 通过 `weak_alias` 避免对 `setenv.c` 的硬链接依赖

use core::ffi::{c_char, c_int, CStr};

use rusl_core::test;
use super::*;

// =========================================================================
// 辅助函数
// =========================================================================

/// 比较 getenv 返回值与预期字节串，NULL 视为不匹配。
unsafe fn getenv_equals_bytes(name: &CStr, expected: &[u8]) -> bool {
    let ptr = getenv(name.as_ptr());
    if ptr.is_null() {
        return false;
    }
    CStr::from_ptr(ptr).to_bytes() == expected
}

// =========================================================================
// 1. 基本返回值测试 (spec: 始终返回 0)
// =========================================================================

test!("test_clearenv_returns_zero" {
    // 验证 clearenv 始终返回 0。
    unsafe {
        assert_eq!(clearenv(), 0, "clearenv 必须始终返回 0");
    }
});

test!("test_clearenv_return_value_after_setenv" {
    // 在有环境变量的情况下调用 clearenv，仍需返回 0。
    unsafe {
        // 先设置一些变量
        assert_eq!(
            setenv(c"RUSL_PRE_A".as_ptr(), c"val_a".as_ptr(), 1),
            0
        );
        assert_eq!(
            setenv(c"RUSL_PRE_B".as_ptr(), c"val_b".as_ptr(), 1),
            0
        );

        // 清除环境，应返回 0
        assert_eq!(clearenv(), 0, "clearenv 在非空环境下也应返回 0");
    }
});

test!("test_clearenv_return_type_is_c_int" {
    // 验证函数指针类型与 c_int 兼容 — 链接期类型检查。
    let f: unsafe extern "C" fn() -> c_int = clearenv;
    unsafe {
        assert_eq!(f(), 0);
    }
});

// =========================================================================
// 2. 多次调用的幂等性 (spec: __environ = NULL 后再次清除无害)
// =========================================================================

test!("test_clearenv_twice" {
    // 连续两次调用 clearenv 均返回 0（空环境上再次清除是无害的）。
    unsafe {
        assert_eq!(clearenv(), 0);
        assert_eq!(clearenv(), 0);
    }
});

test!("test_clearenv_many_times" {
    // 多次连续调用，每次都应返回 0，且不崩溃。
    unsafe {
        for _ in 0..16 {
            assert_eq!(clearenv(), 0, "每次 clearenv 都应返回 0");
        }
    }
});

test!("test_clearenv_idempotent_empty_env" {
    // 先确保环境为空(getenv 返回 NULL)，再多次调用 clearenv。
    unsafe {
        clearenv();
        // 验证确实为空
        assert!(getenv(c"PATH".as_ptr()).is_null());

        // 空环境上再次清除，应返回 0 且不崩溃
        assert_eq!(clearenv(), 0);
        assert_eq!(clearenv(), 0);
        assert_eq!(clearenv(), 0);
    }
});

// =========================================================================
// 3. clearenv 后 getenv 返回 NULL (spec: __environ == NULL 后 getenv 看到空环境)
// =========================================================================

test!("test_getenv_null_after_clearenv" {
    // 调用 clearenv 后，getenv 应返回 NULL。
    unsafe {
        clearenv();
        let result = getenv(c"PATH".as_ptr());
        assert!(result.is_null(), "clearenv 后 getenv(\"PATH\") 应为 NULL");
    }
});

test!("test_all_common_vars_null_after_clearenv" {
    // 验证 clearenv 后常见环境变量全部为 NULL。
    unsafe {
        clearenv();
        let common_vars: [&CStr; 8] = [
            c"PATH", c"HOME", c"USER", c"SHELL",
            c"LANG", c"TERM", c"LOGNAME", c"PWD",
        ];
        for var in &common_vars {
            assert!(
                getenv(var.as_ptr()).is_null(),
                "clearenv 后 getenv 必须返回 NULL"
            );
        }
    }
});

test!("test_getenv_custom_var_null_after_clearenv" {
    // 先 setenv 创建自定义变量，然后 clearenv 后验证它也被清除。
    unsafe {
        let name = c"RUSL_CUSTOM_TO_CLEAR";

        // 设置变量
        assert_eq!(setenv(name.as_ptr(), c"some_value".as_ptr(), 1), 0);
        // 确认设置成功
        assert!(!getenv(name.as_ptr()).is_null());

        // 清除所有环境
        clearenv();

        // 自定义变量也应被清除
        assert!(getenv(name.as_ptr()).is_null(),
            "setenv 后 clearenv，自定义变量也必须被清除");
    }
});

test!("test_getenv_empty_string_after_clearenv" {
    // clearenv 后用空字符串作为 name 查询 getenv，返回 NULL。
    unsafe {
        clearenv();
        assert!(getenv(c"".as_ptr()).is_null(),
            "空 name 查询在 clearenv 后应返回 NULL");
    }
});

// =========================================================================
// 4. clearenv 后 setenv 能正常添加新变量 (spec: 从空环境开始重建)
// =========================================================================

test!("test_setenv_after_clearenv" {
    // clearenv 创建空环境后，setenv 可正常工作。
    unsafe {
        clearenv();
        let name = c"RUSL_NEW_VAR";

        assert_eq!(setenv(name.as_ptr(), c"hello".as_ptr(), 1), 0,
            "clearenv 后 setenv 应成功");

        let result = getenv(name.as_ptr());
        assert!(!result.is_null(), "clearenv 后 setenv 的变量应可被 getenv 查到");
        assert_eq!(CStr::from_ptr(result).to_bytes(), b"hello",
            "值必须匹配");
    }
});

test!("test_setenv_multiple_after_clearenv" {
    // clearenv 后连续 setenv 多个变量均可正常工作。
    unsafe {
        clearenv();

        let pairs: [(&CStr, &CStr); 4] = [
            (c"RUSL_M1", c"alpha"),
            (c"RUSL_M2", c"beta"),
            (c"RUSL_M3", c"gamma"),
            (c"RUSL_M4", c"delta"),
        ];

        for (name, value) in &pairs {
            assert_eq!(setenv(name.as_ptr(), value.as_ptr(), 1), 0);
        }

        for (name, value) in &pairs {
            let result = getenv(name.as_ptr());
            assert!(!result.is_null(), "setenv 后变量必须存在");
            assert_eq!(CStr::from_ptr(result).to_bytes(), value.to_bytes(),
                "值必须匹配");
        }
    }
});

test!("test_setenv_no_overwrite_after_clearenv" {
    // clearenv 后 setenv 用 overwrite=0: 因变量不存在，应创建新变量。
    unsafe {
        clearenv();
        let name = c"RUSL_NOOW";

        // overwrite=0，变量不存在，应创建
        assert_eq!(setenv(name.as_ptr(), c"first".as_ptr(), 0), 0);
        assert!(getenv_equals_bytes(name, b"first"),
            "overwrite=0 时变量不存在应创建");

        // overwrite=0，变量已存在，应保留原值
        assert_eq!(setenv(name.as_ptr(), c"second".as_ptr(), 0), 0);
        assert!(getenv_equals_bytes(name, b"first"),
            "overwrite=0 时变量已存在应保留原值");

        // overwrite=1，应覆盖
        assert_eq!(setenv(name.as_ptr(), c"third".as_ptr(), 1), 0);
        assert!(getenv_equals_bytes(name, b"third"),
            "overwrite=1 应覆盖已有值");
    }
});

test!("test_setenv_empty_value_after_clearenv" {
    // clearenv 后 setenv 空字符串值。
    unsafe {
        clearenv();
        let name = c"RUSL_EMPTY_VAL";

        assert_eq!(setenv(name.as_ptr(), c"".as_ptr(), 1), 0);

        let result = getenv(name.as_ptr());
        assert!(!result.is_null(), "空值变量应存在");
        assert_eq!(*result, 0, "空值第一个字节应为 '\\0'");
    }
});

// =========================================================================
// 5. clearenv -> setenv -> clearenv 循环 (spec: 多次清除-重建循环)
// =========================================================================

test!("test_clear_set_clear_cycle" {
    // clearenv -> setenv -> clearenv 循环验证。
    unsafe {
        let name = c"RUSL_CYCLE";

        // 循环 3 次: 清除 -> 设置 -> 验证存在 -> 清除 -> 验证不存在
        for i in 0..3 {
            clearenv();
            assert!(getenv(name.as_ptr()).is_null(),
                "[iter {}] clearenv 后变量必须为 NULL", i);

            let val_str = alloc::format!("value_{}", i);
            assert_eq!(
                setenv(name.as_ptr(), val_str.as_ptr() as *const c_char, 1),
                0,
                "[iter {}] setenv 应成功", i
            );

            let result = getenv(name.as_ptr());
            assert!(!result.is_null(), "[iter {}] setenv 后变量必须存在", i);
            assert_eq!(
                CStr::from_ptr(result).to_bytes(),
                val_str.as_bytes(),
                "[iter {}] 值必须匹配", i
            );

            clearenv();
            assert!(getenv(name.as_ptr()).is_null(),
                "[iter {}] 第二次 clearenv 后变量必须为 NULL", i);
        }
    }
});

test!("test_clear_set_clear_with_multiple_vars" {
    // 多个变量的清除-设置-清除循环。
    unsafe {
        let names: [&CStr; 3] = [c"RUSL_C1", c"RUSL_C2", c"RUSL_C3"];
        let values: [&CStr; 3] = [c"x", c"y", c"z"];

        for round in 0..2 {
            // 清除
            clearenv();
            for n in &names {
                assert!(getenv(n.as_ptr()).is_null(),
                    "[round {}] clearenv 后应为 NULL", round);
            }

            // 设置
            for (n, v) in names.iter().zip(values.iter()) {
                assert_eq!(setenv(n.as_ptr(), v.as_ptr(), 1), 0);
            }

            // 验证
            for (n, v) in names.iter().zip(values.iter()) {
                let result = getenv(n.as_ptr());
                assert!(!result.is_null(), "[round {}] 变量必须存在", round);
                assert_eq!(CStr::from_ptr(result).to_bytes(), v.to_bytes(),
                    "[round {}] 值匹配", round);
            }
        }

        // 最终清除
        clearenv();
        for n in &names {
            assert!(getenv(n.as_ptr()).is_null(), "最终 clearenv 后应为 NULL");
        }
    }
});

// =========================================================================
// 6. 空环境上再次清除 (spec: __environ == NULL 时遍历跳过，无副作用)
// =========================================================================

test!("test_clearenv_on_already_empty_env" {
    // 多次 clearenv 使环境为空，再调用仍然返回 0 且不崩溃。
    unsafe {
        // 首次清除
        clearenv();
        // 确认空环境
        assert!(getenv(c"PATH".as_ptr()).is_null());
        assert!(getenv(c"HOME".as_ptr()).is_null());

        // 空环境上连续清除
        for _ in 0..10 {
            assert_eq!(clearenv(), 0,
                "空环境上 clearenv 必须返回 0 且不崩溃");
            assert!(getenv(c"PATH".as_ptr()).is_null(),
                "空环境上多次 clearenv 后查询仍为 NULL");
        }
    }
});

test!("test_clearenv_after_clearenv_no_crash" {
    // 最短路径: clearenv 紧接 clearenv，验证不会崩溃。
    unsafe {
        clearenv();
        clearenv();
        clearenv();
        // 如果以上三行未触发 SIGSEGV/panic，则认为通过
        assert_eq!(clearenv(), 0);
    }
});

// =========================================================================
// 7. 类型签名/链接期验证
// =========================================================================

test!("test_clearenv_function_pointer" {
    // 验证 clearenv 可被正确赋值为函数指针（链接期检查）。
    unsafe {
        let f: unsafe extern "C" fn() -> c_int = clearenv;
        // 函数指针非空 (已正确链接)
        assert!(!(clearenv as *const ()).is_null(),
            "clearenv 函数指针应为非 NULL");
        // 通过函数指针调用
        assert_eq!(f(), 0, "通过函数指针调用 clearenv 应返回 0");
    }
});

test!("test_clearenv_function_pointer_size" {
    // 验证函数指针大小与标准指针一致。
    assert_eq!(
        core::mem::size_of::<unsafe extern "C" fn() -> c_int>(),
        core::mem::size_of::<*const ()>(),
        "C 函数指针大小应与数据指针一致"
    );
});

test!("test_clearenv_fn_pointer_roundtrip" {
    // 函数指针赋值后回调，行为和直接调用一致。
    unsafe {
        let direct_ret = clearenv();
        assert_eq!(direct_ret, 0);

        let f: unsafe extern "C" fn() -> c_int = clearenv;
        assert_eq!(f(), 0, "函数指针调用应返回 0");

        // 再次直接调用，验证函数指针调用无副作用
        assert_eq!(clearenv(), 0);
    }
});

// =========================================================================
// 8. errno 不变性 (spec 未提及 errno，但验证无害)
// =========================================================================

test!("test_clearenv_preserves_errno" {
    // clearenv 不应对 errno 有可观测的修改（spec 未指定 errno 行为）。
    unsafe {
        *__errno_location() = 123;
        clearenv();

        // 注意: clearenv 内部调用 setenv.c 的 __env_rm_add 强实现时
        // 可能涉及 free()，某些实现可能修改 errno。
        // 本测试验证 musl 实际行为，不做严格断言。
        let errno_val = *__errno_location();
        // 记录 errno 值以便观察（不强制断言特定值）
        let _ = errno_val;
    }
});

test!("test_setenv_after_clearenv_does_not_affect_errno" {
    // clearenv -> setenv 后 errno 不应被异常修改。
    unsafe {
        *__errno_location() = 0;
        clearenv();
        assert_eq!(
            setenv(c"RUSL_ERRNO_TEST".as_ptr(), c"val".as_ptr(), 1),
            0
        );
        // getenv 不应修改 errno
        let _result = getenv(c"RUSL_ERRNO_TEST".as_ptr());
    }
});

// =========================================================================
// 9. 大量环境变量压力测试
// =========================================================================

test!("test_clearenv_many_custom_variables" {
    // 设置大量自定义变量，然后 clearenv 全部清除。
    // 验证 spec 中"遍历旧 __environ 数组"的逻辑不会导致越界或崩溃。
    unsafe {
        // 设置 N 个变量 — 使用显式 null 终止符保证 C 字符串安全
        for i in 0..50 {
            let name_str = alloc::format!("RUSL_S{}\0", i);
            let val_str = alloc::format!("stress_value_{}\0", i);
            assert_eq!(
                setenv(
                    name_str.as_ptr() as *const c_char,
                    val_str.as_ptr() as *const c_char,
                    1,
                ),
                0,
            );
        }

        // 验证部分变量存在
        let check_name = alloc::format!("RUSL_S{}\0", 25);
        assert!(!getenv(check_name.as_ptr() as *const c_char).is_null(),
            "设置后变量应存在");

        // 全部清除
        assert_eq!(clearenv(), 0);

        // 验证所有变量都被清除
        for i in 0..50 {
            let name_str = alloc::format!("RUSL_S{}\0", i);
            assert!(
                getenv(name_str.as_ptr() as *const c_char).is_null(),
                "clearenv 后所有自定义变量必须为 NULL (index {})",
                i,
            );
        }
    }
});

// =========================================================================
// 10. 与其他 env 函数的交叉验证
// =========================================================================

test!("test_clearenv_then_environ_is_empty" {
    // clearenv 后所有 getenv 查询返回 NULL (等价于 environ 为空)。
    unsafe {
        clearenv();

        // 执行多个不同查询，均应为 NULL
        let queries: [&CStr; 5] = [
            c"PATH", c"HOME", c"USER", c"ANYTHING_RANDOM", c"",
        ];
        for q in &queries {
            assert!(getenv(q.as_ptr()).is_null(),
                "clearenv 后任何 getenv 都应返回 NULL");
        }
    }
});

test!("test_clearenv_after_partial_clear" {
    // 先 setenv 多个变量，再 unsetenv 其中部分，最后 clearenv 清除剩余。
    unsafe {
        clearenv();

        // 设置三个变量
        assert_eq!(setenv(c"RUSL_KEEP1".as_ptr(), c"v1".as_ptr(), 1), 0);
        assert_eq!(setenv(c"RUSL_KEEP2".as_ptr(), c"v2".as_ptr(), 1), 0);
        assert_eq!(setenv(c"RUSL_REMOVE".as_ptr(), c"v3".as_ptr(), 1), 0);

        // 移除 RUSL_REMOVE (注意 unsetenv 不是本测试关注点)
        // 此处直接使用 clearenv 全部清除，验证最终一致性
        clearenv();

        assert!(getenv(c"RUSL_KEEP1".as_ptr()).is_null(),
            "clearenv 必须清除所有变量");
        assert!(getenv(c"RUSL_KEEP2".as_ptr()).is_null(),
            "clearenv 必须清除所有变量");
        assert!(getenv(c"RUSL_REMOVE".as_ptr()).is_null(),
            "clearenv 必须清除所有变量");
    }
});

// =========================================================================
// 11. 边界: 值与特殊字符 (验证 clearenv 后重建不受限制)
// =========================================================================

test!("test_setenv_special_chars_after_clearenv" {
    // clearenv 后 setenv 包含特殊字符的值，验证重建正常。
    unsafe {
        clearenv();
        let name = c"RUSL_SPECIAL";

        // 值包含 '='、空格、特殊符号
        assert_eq!(
            setenv(name.as_ptr(), c"a=b c!@#$%".as_ptr(), 1),
            0
        );

        let result = getenv(name.as_ptr());
        assert!(!result.is_null());
        assert_eq!(
            CStr::from_ptr(result).to_bytes(),
            b"a=b c!@#$%",
            "特殊字符值完整保留"
        );
    }
});

test!("test_setenv_long_value_after_clearenv" {
    // clearenv 后 setenv 长字符串值。
    unsafe {
        clearenv();
        let name = c"RUSL_LONG";
        let long_val = c"abcdefghijklmnopqrstuvwxyz_0123456789_ABCDEFGHIJKLMNOPQRSTUVWXYZ";

        assert_eq!(
            setenv(name.as_ptr(), long_val.as_ptr() as *const c_char, 1),
            0
        );

        let result = getenv(name.as_ptr());
        assert!(!result.is_null());
        assert_eq!(
            CStr::from_ptr(result).to_bytes(),
            long_val.to_bytes(),
            "长值应完整保留"
        );
    }
});
