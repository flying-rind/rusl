/// 模块: environ_test
/// `environ` 集成测试 — 验证 musl libc 环境变量全局指针的不变量
///
/// 对应 spec: `src/env/spec/__environ.md`
///
/// ## 测试覆盖
///
/// ### 1. environ 非空 (程序启动后应有环境变量)
/// - environ 在 main() 执行后为非 NULL
/// - __environ 与 environ 共享内存位置, 亦为非 NULL
/// - 环境条目数在合理范围内
///
/// ### 2. environ 以 NULL 终止
/// - 哨兵不变量: 数组以 NULL 指针终止
/// - 哨兵位置精确性: environ[count] == NULL
/// - 哨兵后无有效条目
/// - 哨兵位置一致性 (多次计数应相同)
///
/// ### 3. 格式不变量: "NAME=VALUE"
/// - 每个非 NULL 条目包含 '='
/// - NAME 非空 ('=' 不在位置 0)
/// - 条目最小长度 >= 2
/// - 首字符不为 '\0' 或 '='
/// - NAME 仅包含可移植字符集 [A-Za-z0-9_]
/// - VALUE 可以为空
/// - VALUE 中可包含 '=' (多 '=' 条目)
///
/// ### 4. environ 可读可写
/// - 读取环境条目内容
/// - 修改 environ 指针指向自定义数组
/// - 设置 environ 为 NULL
/// - 恢复 environ 指针
/// - 多次指针赋值往返
/// - 自定义环境数组满足不变量
/// - 修改 environ 后 __environ 同步变化
/// - 修改 __environ 后 environ 同步变化
///
/// ### 5. 类型/大小验证
/// - environ 指针大小 (64 位: 8 字节; 32 位: 4 字节)
/// - environ 对齐与 usize 一致
/// - &environ 和 &__environ 地址相同 (weak_alias)
/// - environ 和 __environ 的值相等
/// - sizeof(char **) == sizeof(char *) == sizeof(usize)
/// - environ 的地址为静态 (多次获取一致)

use rusl_core::test;
use core::ffi::c_char;
use core::mem;

extern "C" {
    /// POSIX 标准环境变量全局指针 — musl libc 对外导出符号
    ///
    /// 声明于 <unistd.h>: `extern char **environ;`
    /// 通过 `weak_alias(__environ, environ)` 与内部 `__environ` 共享同一内存位置。
    /// 类型为 `char **`, 指向一个以 NULL 终止的字符串数组。
    static mut environ: *mut *mut c_char;

    /// musl 内部环境变量全局指针 — 与 environ 通过 weak_alias 共享同一内存位置。
    /// `__` 前缀保留名称, 用户程序不应直接使用。
    static mut __environ: *mut *mut c_char;
}

// =============================================================================
// 辅助函数 (no_std 兼容 — 不依赖 libc 或 std)
// =============================================================================

/// 计算以 NUL 结尾的 C 字符串长度。
/// 若 `s` 为 NULL 返回 0。
fn cstr_len(s: *const c_char) -> usize {
    if s.is_null() {
        return 0;
    }
    let mut len = 0;
    unsafe {
        while *s.add(len) != 0 {
            len += 1;
        }
    }
    len
}

/// 统计环境数组条目数, 直到 NULL 哨兵。
/// 若 `env` 为 NULL 返回 0。
fn count_env_entries(env: *const *mut c_char) -> usize {
    if env.is_null() {
        return 0;
    }
    let mut n = 0;
    unsafe {
        while !(*env.add(n)).is_null() {
            n += 1;
        }
    }
    n
}

/// 在 C 字符串中查找第一个 `'='` 的位置。
/// 返回 `None` 如果 `s` 为 NULL、为空字符串或不包含 `'='`。
fn find_equals(s: *const c_char) -> Option<usize> {
    if s.is_null() {
        return None;
    }
    let mut i = 0;
    unsafe {
        loop {
            let c = *s.add(i);
            if c == 0 {
                return None;
            }
            if c == b'=' as c_char {
                return Some(i);
            }
            i += 1;
        }
    }
}

/// 比较两个 NUL 结尾的 C 字符串是否相等。
/// 若任一指针为 NULL 返回 false。
fn cstr_eq(a: *const c_char, b: *const c_char) -> bool {
    if a.is_null() || b.is_null() {
        return false;
    }
    let mut i = 0;
    unsafe {
        loop {
            let ca = *a.add(i);
            let cb = *b.add(i);
            if ca != cb {
                return false;
            }
            if ca == 0 {
                return true;
            }
            i += 1;
        }
    }
}

// =============================================================================
// 1. environ 非空 — 程序启动后应有环境变量
// =============================================================================

test!("test_environ_non_null_after_startup" {
    // Spec: __environ 初始值为 NULL, 在 main() 执行前由 __libc_start_main
    // 或 __init_tls 赋值为实际环境指针。程序进入 main 后 environ 不应为 NULL。
    unsafe {
        assert!(!environ.is_null(),
            "environ 不应为 NULL — 程序启动后应已初始化");
    }
});

test!("test___environ_non_null_after_startup" {
    // __environ 与 environ 通过 weak_alias 共享同一内存位置。
    // 启动后同样不应为 NULL。
    unsafe {
        assert!(!__environ.is_null(),
            "__environ 不应为 NULL — 与 environ 共享内存位置, 应同步初始化");
    }
});

test!("test_environ_has_reasonable_entry_count" {
    // 验证环境条目数在合理范围内 (非零且不过大)。
    // 正常 Linux 环境通常至少有一个条目 (如 PATH)。
    // 不强制断言 n > 0, 但记录条目数以辅助诊断.
    unsafe {
        let n = count_env_entries(environ);
        assert!(n < 10000,
            "环境条目数 {} 异常, 可能 environ 未以 NULL 正确终止", n);
    }
});

// =============================================================================
// 2. environ 以 NULL 终止 (哨兵不变量)
// =============================================================================

test!("test_sentinel_null_terminated" {
    // Spec 不变量 #1: 数组必须以 NULL 指针作为终止标记。
    // 若 environ[i] 为 NULL, 则对于所有 j > i, environ[j] 应视为越界访问。
    unsafe {
        let n = count_env_entries(environ);
        let sentinel = *environ.add(n);
        assert!(sentinel.is_null(),
            "environ[{}] 应为 NULL 哨兵, 实际: {:?}", n, sentinel);
    }
});

test!("test_sentinel_position_consistent" {
    // 哨兵位置在连续多次计数中应保持一致。
    unsafe {
        let n1 = count_env_entries(environ);
        let n2 = count_env_entries(environ);
        let n3 = count_env_entries(environ);
        assert_eq!(n1, n2,
            "连续调用 count_env_entries 应返回相同结果: {} vs {}", n1, n2);
        assert_eq!(n2, n3,
            "连续调用 count_env_entries 应返回相同结果: {} vs {}", n2, n3);
    }
});

test!("test_sentinel_is_sole_terminator" {
    // Spec: NULL 哨兵是唯一的终止标记。数组边界为 [0, n), environ[n] == NULL。
    // 哨兵之后的内存不属于 environ 数组, 不应被解引用访问。
    unsafe {
        let n = count_env_entries(environ);
        // 验证哨兵位置精确: environ[n] 必须为 NULL
        assert!((*environ.add(n)).is_null(),
            "environ[{}] 应为 NULL 哨兵", n);
        // 验证哨兵之前的条目均非 NULL
        for i in 0..n {
            assert!(!(*environ.add(i)).is_null(),
                "environ[{}] (哨兵之前) 不应为 NULL", i);
        }
        // 注意: 不验证 environ[n + k] 的内容 — 哨兵之后的内存不属于
        // environ 数组, 访问其为未定义行为, 内容完全不可预测。
    }
});

// =============================================================================
// 3. 格式不变量: 每个元素格式为 "NAME=VALUE"
// =============================================================================

test!("test_each_entry_has_equals_sign" {
    // Spec 不变量 #2: 每个非 NULL 条目必须是 "NAME=VALUE" 格式, 必须包含 '='。
    unsafe {
        let mut i = 0;
        loop {
            let entry = *environ.add(i);
            if entry.is_null() {
                break;
            }
            let eq_pos = find_equals(entry);
            assert!(eq_pos.is_some(),
                "environ[{}] 缺少 '=' 字符, 违反 \"NAME=VALUE\" 格式不变量", i);
            i += 1;
        }
    }
});

test!("test_name_not_empty" {
    // Spec: NAME 是非空字符串。'=' 不能在位置 0 (否则 NAME 为空)。
    unsafe {
        let mut i = 0;
        loop {
            let entry = *environ.add(i);
            if entry.is_null() {
                break;
            }
            if let Some(pos) = find_equals(entry) {
                assert!(pos > 0,
                    "environ[{}]: NAME 为空 ('=' 在位置 0), 违反格式不变量", i);
            }
            i += 1;
        }
    }
});

test!("test_entry_minimum_length_two" {
    // 最小合法格式: "X=" (NAME 至少 1 字符 + '=' + 可空 VALUE)。
    // strlen 必须 >= 2。值为空时 "X=" 长度为 2, 值非空时长度 > 2。
    unsafe {
        let mut i = 0;
        loop {
            let entry = *environ.add(i);
            if entry.is_null() {
                break;
            }
            let len = cstr_len(entry);
            assert!(len >= 2,
                "environ[{}] 长度 {} 过短, 最小合法格式 'X=' 长度为 2", i, len);
            i += 1;
        }
    }
});

test!("test_first_char_not_nul_and_not_equals" {
    // 每个条目的首字符不能是 \0 (空字符串) 或 '=' (NAME 为空)。
    unsafe {
        let mut i = 0;
        loop {
            let entry = *environ.add(i);
            if entry.is_null() {
                break;
            }
            let first = *entry;
            assert_ne!(first, b'=' as c_char,
                "environ[{}] 首字符为 '=', NAME 为空", i);
            assert_ne!(first, 0,
                "environ[{}] 首字符为 NUL, 条目为空字符串", i);
            i += 1;
        }
    }
});

test!("test_name_contains_only_portable_characters" {
    // Spec: NAME 仅包含可移植字符集 — 字母 [A-Za-z]、数字 [0-9]、下划线 _。
    // POSIX 环境变量命名规范不允许其他字符。
    unsafe {
        let mut i = 0;
        loop {
            let entry = *environ.add(i);
            if entry.is_null() {
                break;
            }
            if let Some(eq_pos) = find_equals(entry) {
                for j in 0..eq_pos {
                    let c = *entry.add(j) as u8;
                    let valid = (c >= b'A' && c <= b'Z')
                        || (c >= b'a' && c <= b'z')
                        || (c >= b'0' && c <= b'9')
                        || c == b'_';
                    assert!(valid,
                        "environ[{}] NAME 位置 {} 含非法字符 '{}' (0x{:02x})",
                        i, j, c as char, c);
                }
            }
            i += 1;
        }
    }
});

test!("test_value_can_be_empty" {
    // Spec: VALUE 可以是任意字符串, 包括空字符串。
    // 值为空时 "NAME=" 后紧跟 NUL, 这符合格式不变量。
    // 本测试遍历所有条目, 检查值部分的存在性 (仅记录, 不做硬断言)。
    unsafe {
        let mut i = 0;
        loop {
            let entry = *environ.add(i);
            if entry.is_null() {
                break;
            }
            if let Some(eq_pos) = find_equals(entry) {
                // '=' 后的第一个字符 (可能是 NUL 表示空值)
                let after_eq = *entry.add(eq_pos + 1);
                // 空值是合法的, 不做断言。仅验证 NAME 和 '=' 的存在性。
                let _ = after_eq;
            }
            i += 1;
        }
    }
});

test!("test_entry_with_multiple_equals_signs" {
    // Spec: VALUE 中可以包含 '=', 例如 "LESS=--quit-if-one-screen"。
    // 验证自定义含多个 '=' 的条目不违反格式不变量。
    unsafe {
        let saved = environ;

        // 模拟一个 VALUE 中含多个 '=' 的条目
        let multi_eq = b"MULTI_EQ=key=value=pair\0".as_ptr() as *mut c_char;
        let custom_env: [*mut c_char; 2] = [
            multi_eq,
            core::ptr::null_mut(),
        ];
        environ = custom_env.as_ptr() as *mut *mut c_char;

        // 格式不变量: 必须包含 '='
        let eq_pos = find_equals(*environ.add(0));
        assert!(eq_pos.is_some(),
            "含多个 '=' 的条目应能找到第一个 '='");
        assert!(eq_pos.unwrap() > 0,
            "NAME 不应为空 (第一个 '=' 不在位置 0)");

        // 第一个 '=' 之前的字符构成 NAME = "MULTI_EQ" (8 字节)
        assert_eq!(eq_pos.unwrap(), 8,
            "第一个 '=' 应在 NAME 结束处 (位置 8)");

        // 验证完整字符串长度
        let len = cstr_len(*environ.add(0));
        let expected_len = cstr_len(multi_eq);
        assert_eq!(len, expected_len,
            "条目长度应完整包含 VALUE 中的所有 '='");

        environ = saved;
    }
});

// =============================================================================
// 4. environ 可读可写
// =============================================================================

test!("test_can_read_all_entries" {
    // 验证可以安全地遍历并读取所有环境条目的首字节。
    unsafe {
        let n = count_env_entries(environ);
        assert!(n > 0, "环境至少应有 1 个条目 (如 PATH)");
        for i in 0..n {
            let entry = *environ.add(i);
            assert!(!entry.is_null(),
                "environ[{}] 在 0..{} 范围内不应为 NULL", i, n);
            // 验证首字节可解引用
            let _first_byte = *entry;
        }
    }
});

test!("test_can_modify_environ_pointer_to_custom_array" {
    // 验证 environ 指针是可写的: 可将其重新指向一个合法的环境数组。
    unsafe {
        let saved = environ;

        // 构建最小合法环境数组: 一个条目 + NULL 哨兵
        let mini_str = b"RUSL_TEST=1\0".as_ptr() as *mut c_char;
        let mini_env: [*mut c_char; 2] = [
            mini_str,
            core::ptr::null_mut(),
        ];

        // 修改 environ 指向新数组
        environ = mini_env.as_ptr() as *mut *mut c_char;

        assert_eq!(environ, mini_env.as_ptr() as *mut *mut c_char,
            "修改后 environ 应指向新数组");
        assert!(!(*environ.add(0)).is_null(),
            "修改后 environ[0] 不应为 NULL");
        assert!((*environ.add(1)).is_null(),
            "修改后 environ[1] 应为 NULL 哨兵");

        // 恢复原 environ
        environ = saved;
        assert_eq!(environ, saved,
            "恢复后 environ 应指向原始数组");
    }
});

test!("test_can_set_environ_to_null" {
    // Spec: environ = NULL 是合法操作, 等价于清空所有环境。
    // clearenv() 在 musl 中即是将 __environ 设为 NULL。
    unsafe {
        let orig = environ;

        environ = core::ptr::null_mut();
        assert!(environ.is_null(),
            "设置 environ=NULL 后 environ 应为 NULL");
        assert_eq!(count_env_entries(environ), 0,
            "NULL environ 的条目计数应为 0");

        // 恢复
        environ = orig;
        assert!(!environ.is_null(),
            "恢复后 environ 不应为 NULL");
        assert_eq!(environ, orig,
            "恢复后 environ 应等于原始值");
    }
});

test!("test_multiple_pointer_reassignments" {
    // 验证多次指针赋值后 environ 行为正确。
    unsafe {
        let orig = environ;

        // 第 1 轮: 设为 NULL
        environ = core::ptr::null_mut();
        assert!(environ.is_null());

        // 第 2 轮: 恢复
        environ = orig;
        assert_eq!(environ, orig);

        // 第 3 轮: 再次设为 NULL
        environ = core::ptr::null_mut();
        assert!(environ.is_null());

        // 第 4 轮: 再次恢复
        environ = orig;
        assert_eq!(environ, orig);
        assert!(!environ.is_null());
    }
});

test!("test_write_then_read_custom_array" {
    // 验证修改 environ 后能正确读取自定义数组的内容。
    unsafe {
        let saved = environ;

        let str_a = b"ALPHA=1\0".as_ptr() as *mut c_char;
        let str_b = b"BETA=2\0".as_ptr() as *mut c_char;
        let custom_env: [*mut c_char; 3] = [
            str_a,
            str_b,
            core::ptr::null_mut(),
        ];

        environ = custom_env.as_ptr() as *mut *mut c_char;

        // 通过 environ 读取内容
        assert!(cstr_eq(*environ.add(0), str_a),
            "environ[0] 内容应为 ALPHA=1");
        assert!(cstr_eq(*environ.add(1), str_b),
            "environ[1] 内容应为 BETA=2");
        assert!((*environ.add(2)).is_null(),
            "environ[2] 应为 NULL 哨兵");

        // 哨兵不变量
        assert_eq!(count_env_entries(environ), 2,
            "自定义数组应有 2 个条目");

        environ = saved;
    }
});

test!("test_custom_environ_array_preserves_invariants" {
    // 验证自定义环境数组满足所有不变量 (哨兵终止 + NAME=VALUE 格式)。
    unsafe {
        let saved = environ;

        let entry0 = b"FOO=BAR\0".as_ptr() as *mut c_char;
        let entry1 = b"ANSWER=42\0".as_ptr() as *mut c_char;
        let custom_env: [*mut c_char; 3] = [
            entry0,
            entry1,
            core::ptr::null_mut(),
        ];

        environ = custom_env.as_ptr() as *mut *mut c_char;

        // 哨兵不变量
        assert_eq!(count_env_entries(environ), 2,
            "自定义环境应有 2 个条目");
        assert!((*environ.add(2)).is_null(),
            "environ[2] 应为 NULL 哨兵");

        // 格式不变量
        for i in 0..2 {
            let eq_pos = find_equals(*environ.add(i));
            assert!(eq_pos.is_some(),
                "自定义 environ[{}] 应包含 '='", i);
            assert!(eq_pos.unwrap() > 0,
                "自定义 environ[{}] NAME 不应为空", i);
        }

        environ = saved;
    }
});

test!("test_empty_array_setting" {
    // 验证可以设置 environ 指向一个仅含 NULL 哨兵的空数组。
    // 这在语义上表示有环境变量数组但没有条目。
    unsafe {
        let saved = environ;

        let empty_env: [*mut c_char; 1] = [core::ptr::null_mut()];
        environ = empty_env.as_ptr() as *mut *mut c_char;

        assert_eq!(count_env_entries(environ), 0,
            "纯哨兵数组的条目数应为 0");
        assert!((*environ.add(0)).is_null(),
            "environ[0] 应为 NULL");

        environ = saved;
    }
});

// =============================================================================
// 4b. weak_alias 双向同步验证 — 修改任一方, 另一方应同步
// =============================================================================

test!("test_modify_environ_reflected_in___environ" {
    // Spec: 别名共享同一内存位置。修改 environ 后 __environ 应同步变化。
    unsafe {
        let saved = environ;

        let mini_str = b"SYNC_ALPHA=1\0".as_ptr() as *mut c_char;
        let mini_env: [*mut c_char; 2] = [
            mini_str,
            core::ptr::null_mut(),
        ];

        // 通过 environ 修改
        environ = mini_env.as_ptr() as *mut *mut c_char;

        // __environ 的值应同步变化
        assert_eq!(__environ, environ,
            "修改 environ 后 __environ 应与 environ 值相同");
        assert_eq!(__environ, mini_env.as_ptr() as *mut *mut c_char,
            "修改 environ 后 __environ 应指向同一新数组");

        // 通过 __environ 读取内容
        assert!(!(*__environ.add(0)).is_null(),
            "通过 __environ 读取 environ[0] 不应为 NULL");
        assert!((*__environ.add(1)).is_null(),
            "通过 __environ 读取 environ[1] 应为 NULL");

        environ = saved;
    }
});

test!("test_modify___environ_reflected_in_environ" {
    // 反向验证: 修改 __environ 后 environ 也应同步变化。
    unsafe {
        let saved = __environ;

        let mini_str = b"REVERSE_SYNC=OK\0".as_ptr() as *mut c_char;
        let mini_env: [*mut c_char; 2] = [
            mini_str,
            core::ptr::null_mut(),
        ];

        // 通过 __environ 修改
        __environ = mini_env.as_ptr() as *mut *mut c_char;

        // environ 应同步
        assert_eq!(environ, __environ,
            "修改 __environ 后 environ 应与 __environ 值相同");
        assert_eq!(environ, mini_env.as_ptr() as *mut *mut c_char,
            "修改 __environ 后 environ 应指向新数组");

        // 通过 environ 读取内容
        assert!(!(*environ.add(0)).is_null(),
            "通过 environ 读取 __environ[0] 不应为 NULL");
        assert!((*environ.add(1)).is_null(),
            "通过 environ 读取 __environ[1] 应为 NULL");

        __environ = saved;
    }
});

// =============================================================================
// 5. 类型/大小验证 — 指针大小一致
// =============================================================================

test!("test_pointer_size_matches_usize" {
    // environ 的类型为 char **, 在 x86_64 上 sizeof(char **) == 8 字节。
    // 指针的大小应与 usize 一致。
    let ptr_size = mem::size_of::<*mut *mut c_char>();
    let usize_size = mem::size_of::<usize>();
    assert_eq!(ptr_size, usize_size,
        "sizeof(char **) ({}) 应与 sizeof(usize) ({}) 一致",
        ptr_size, usize_size);
});

test!("test_pointer_size_on_current_platform" {
    // 根据目标指针宽度验证具体字节数。
    let ptr_size = mem::size_of::<*mut *mut c_char>();
    if cfg!(target_pointer_width = "64") {
        assert_eq!(ptr_size, 8,
            "在 64 位平台上 sizeof(char **) 应为 8 字节, 实际: {}", ptr_size);
    } else if cfg!(target_pointer_width = "32") {
        assert_eq!(ptr_size, 4,
            "在 32 位平台上 sizeof(char **) 应为 4 字节, 实际: {}", ptr_size);
    }
});

test!("test_pointer_alignment_matches_usize" {
    // char ** 的对齐应与 usize 一致。
    let ptr_align = mem::align_of::<*mut *mut c_char>();
    let usize_align = mem::align_of::<usize>();
    assert_eq!(ptr_align, usize_align,
        "alignof(char **) ({}) 应与 alignof(usize) ({}) 一致",
        ptr_align, usize_align);
});

test!("test_sizeof_char_ptr_equals_char_ptr_ptr" {
    // sizeof(char *) == sizeof(char **) — 两者都是指针, 大小相同。
    let char_ptr_size = mem::size_of::<*mut c_char>();
    let char_ptr_ptr_size = mem::size_of::<*mut *mut c_char>();
    assert_eq!(char_ptr_size, char_ptr_ptr_size,
        "sizeof(char *) ({}) 应等于 sizeof(char **) ({})",
        char_ptr_size, char_ptr_ptr_size);
});

test!("test_sizeof_environ_entry_equals_ptr" {
    // environ[i] 的类型为 char *, 其大小应等于 usize。
    let entry_size = mem::size_of::<*mut c_char>();
    let usize_size = mem::size_of::<usize>();
    assert_eq!(entry_size, usize_size,
        "sizeof(char *) ({}) 应与 sizeof(usize) ({}) 一致",
        entry_size, usize_size);
});

test!("test_environ_and___environ_same_address" {
    // Spec: __environ 和 environ 通过 weak_alias 共享同一内存位置。
    // 即 &environ == &__environ (静态变量的地址相同)。
        let addr_environ = core::ptr::addr_of!(environ) as usize;
        let addr_inner = core::ptr::addr_of!(__environ) as usize;

        assert_eq!(addr_environ, addr_inner,
            "&environ (0x{:x}) 和 &__environ (0x{:x}) 应指向同一内存位置",
            addr_environ, addr_inner);
});

test!("test_environ_and___environ_same_value" {
    // 验证 environ 和 __environ 存储的值相等 (即指向同一环境数组)。
    unsafe {
        assert_eq!(environ, __environ,
            "environ ({:?}) 和 __environ ({:?}) 存储的指针值应相等",
            environ, __environ);
    }
});

test!("test_environ_address_is_static" {
    // environ 是全局静态变量, 其地址在程序生命周期内不变。
        let addr1 = core::ptr::addr_of!(environ) as usize;
        let addr2 = core::ptr::addr_of!(environ) as usize;
        let addr3 = core::ptr::addr_of!(environ) as usize;
        assert_eq!(addr1, addr2,
            "environ 的地址在连续获取中应保持不变");
        assert_eq!(addr2, addr3,
            "environ 的地址在连续获取中应保持不变");
});

// =============================================================================
// 6. 边界/极端情况
// =============================================================================

test!("test_null_environ_safe_helpers" {
    // 当 environ 为 NULL 时, 辅助函数应安全处理 (不应解引用 NULL)。
    unsafe {
        let saved = environ;
        environ = core::ptr::null_mut();

        assert_eq!(count_env_entries(environ), 0,
            "NULL environ 的条目计数应为 0");

        // 辅助函数也应对 NULL 参数安全
        assert_eq!(cstr_len(core::ptr::null()), 0,
            "cstr_len(NULL) 应返回 0");
        assert_eq!(find_equals(core::ptr::null()), None,
            "find_equals(NULL) 应返回 None");

        environ = saved;
    }
});

test!("test_empty_string_entry_invalid" {
    // 空字符串 "" 作为环境条目是无效的 (缺少 '=' 且长度为 0)。
    // 验证自定义空字符串条目不满足格式不变量。
    unsafe {
        let saved = environ;

        // 故意构造非法数组: 空字符串条目
        let empty_str = b"\0".as_ptr() as *mut c_char;
        let bad_env: [*mut c_char; 2] = [empty_str, core::ptr::null_mut()];
        environ = bad_env.as_ptr() as *mut *mut c_char;

        // 验证不变量检查能检测到非法格式
        let eq_pos = find_equals(*environ.add(0));
        assert!(eq_pos.is_none(),
            "空字符串不应包含 '=', 格式非法");

        let len = cstr_len(*environ.add(0));
        assert_eq!(len, 0, "空字符串长度应为 0");

        environ = saved;
    }
});

test!("test_equals_only_entry_invalid" {
    // 仅含 '=' 的条目不满足格式不变量 (NAME 为空)。
    // 验证此类条目不通过格式检查。
    unsafe {
        let saved = environ;

        let bad_str = b"=value\0".as_ptr() as *mut c_char;
        let bad_env: [*mut c_char; 2] = [bad_str, core::ptr::null_mut()];
        environ = bad_env.as_ptr() as *mut *mut c_char;

        let eq_pos = find_equals(*environ.add(0));
        assert!(eq_pos.is_some(), "应能找到 '='");
        assert_eq!(eq_pos.unwrap(), 0,
            "'=' 在位置 0, 表示 NAME 为空 — 格式非法");

        environ = saved;
    }
});
