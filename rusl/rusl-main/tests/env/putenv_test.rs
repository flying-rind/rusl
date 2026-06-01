/// 模块: putenv_test
/// `putenv` 集成测试
///
/// 基于 `src/env/spec/putenv.md` 规约生成。
///
/// ## 测试覆盖
///
/// - putenv("NAME=VALUE") 基本设置/替换环境变量, 返回 0
/// - putenv 非拷贝行为: 直接存储传入指针, 修改缓冲区后 environ 反映变化
/// - getenv 验证 putenv 设置的值
/// - environ 数组 NULL 终止不变式
/// - 不含 '=' 时委托 unsetenv 移除变量 (返回 0)
/// - 空变量名 (=VALUE) 返回 -1, errno = EINVAL
/// - 多次替换后数组大小不变 (原地替换)
/// - 多变量设置并存
/// - putenv/unsetenv 交互: putenv 设置的变量可被 unsetenv 移除
/// - putenv/setenv 交互: 双向覆盖
/// - 变量名前缀部分匹配不误匹配
/// - environ 为 NULL/空时也能正常插入
/// - 特殊字符值


use core::ffi::{c_char, c_int, CStr};

use rusl_core::test;

// ===========================================================================
// 常量
// ===========================================================================

const EINVAL: c_int = 22;

// ===========================================================================
// C ABI 声明
// ===========================================================================
// putenv / getenv / unsetenv / setenv / clearenv 均为 POSIX <stdlib.h> 对外 API。
// environ 为 POSIX 全局变量。
// 符号在 c-test 模式下由 musl libc 提供。

extern "C" {
    /// POSIX: 将 "NAME=VALUE" 格式字符串放入进程环境 (非拷贝)
    fn putenv(s: *mut c_char) -> c_int;
    /// POSIX: 获取环境变量值
    fn getenv(name: *const c_char) -> *mut c_char;
    /// POSIX: 移除环境变量
    fn unsetenv(name: *const c_char) -> c_int;
    /// POSIX: 设置环境变量 (拷贝)
    fn setenv(name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;
    /// POSIX: 清空所有环境变量
    fn clearenv() -> c_int;
    /// musl 内部: 获取 errno 指针
    fn __errno_location() -> *mut c_int;
    /// POSIX: 环境变量数组指针
    static mut environ: *mut *mut c_char;
}

// ===========================================================================
// 辅助函数
// ===========================================================================

/// 统计 environ 中变量个数 (environ 可能为 NULL)
unsafe fn env_count() -> usize {
    if environ.is_null() {
        return 0;
    }
    let mut n = 0usize;
    let mut i = 0usize;
    loop {
        let entry = *environ.add(i);
        if entry.is_null() {
            break;
        }
        n += 1;
        i += 1;
    }
    n
}

/// 验证 environ 数组以 NULL 终止
unsafe fn env_null_terminated() -> bool {
    if environ.is_null() {
        return true;
    }
    let n = env_count();
    (*environ.add(n)).is_null()
}

/// 在 environ 中查找以 key (不含 '=') 为名字的变量条目
/// 返回条目字符串指针，未找到返回 null。
unsafe fn env_find(key: &CStr) -> *mut c_char {
    if environ.is_null() {
        return core::ptr::null_mut();
    }
    let key_bytes = key.to_bytes();
    let key_len = key_bytes.len();
    let mut i = 0usize;
    loop {
        let entry = *environ.add(i);
        if entry.is_null() {
            break;
        }
        // 检查 entry 是否以 "key=" 开头
        let mut matched = true;
        for j in 0..key_len {
            if *entry.add(j) != key_bytes[j] as c_char {
                matched = false;
                break;
            }
        }
        if matched && *entry.add(key_len) == b'=' as c_char {
            return entry;
        }
        i += 1;
    }
    core::ptr::null_mut()
}

/// 设置 errno 为指定值
unsafe fn set_errno(val: c_int) {
    *__errno_location() = val;
}

/// 获取当前 errno 值
unsafe fn get_errno() -> c_int {
    *__errno_location()
}

/// 比较 getenv 返回值与预期字节串
unsafe fn getenv_equals(name: &CStr, expected: &[u8]) -> bool {
    let ptr = getenv(name.as_ptr());
    if ptr.is_null() {
        return false;
    }
    CStr::from_ptr(ptr).to_bytes() == expected
}

// ===========================================================================
// 1. 基本 putenv 设置环境变量返回 0
// ===========================================================================

test!("test_putenv_basic_set" {
    // 规约: putenv("NAME=VALUE") 将字符串直接放入环境, 返回 0。
    unsafe {
        clearenv();
        set_errno(0);

        let ret = putenv(c"BASIC=hello".as_ptr() as *mut c_char);
        assert_eq!(ret, 0, "putenv should return 0, got {ret}");

        let entry = env_find(c"BASIC");
        assert!(!entry.is_null(), "BASIC not found in environ after putenv");
    }
});

// ===========================================================================
// 2. putenv 后 getenv 能查到
// ===========================================================================

test!("test_getenv_after_putenv" {
    // getenv 验证 putenv 设置的值。
    unsafe {
        clearenv();
        putenv(c"GETVAR=hello".as_ptr() as *mut c_char);

        let val = getenv(c"GETVAR".as_ptr());
        assert!(!val.is_null(), "getenv('GETVAR') returned NULL after putenv");

        // 使用 CStr::from_ptr 安全比较值
        let cstr = CStr::from_ptr(val);
        assert_eq!(cstr.to_bytes(), b"hello", "getenv value mismatch");
    }
});

// ===========================================================================
// 3. putenv 覆盖已存在的变量
// ===========================================================================

test!("test_putenv_replace" {
    // 规约: 变量已存在时原地替换, 不改变数组大小。
    unsafe {
        clearenv();
        putenv(c"REPLACE=old".as_ptr() as *mut c_char);
        let count_before = env_count();

        putenv(c"REPLACE=new".as_ptr() as *mut c_char);
        assert_eq!(env_count(), count_before, "env_count changed after replacement");

        assert!(
            getenv_equals(c"REPLACE", b"new"),
            "value should be 'new' after replacement"
        );
    }
});

// ===========================================================================
// 4. putenv 非拷贝行为
// ===========================================================================

test!("test_putenv_noncopy_behavior" {
    // 规约: putenv 不拷贝字符串, 直接存储传入指针。
    // 验证: 修改原始缓冲区后, environ 中的值随之改变。
    //
    // 注意: 使用栈上可变缓冲区, putenv 后此缓冲区成为环境的一部分,
    // 测试结束时通过 unsetenv 显式清理。
    unsafe {
        clearenv();

        // 在栈上分配缓冲区
        let mut buf: [c_char; 32] = [0; 32];
        let src = b"NONCOPY=original";
        let src_len = src.len();
        for i in 0..src_len {
            buf[i] = src[i] as c_char;
        }
        buf[src_len] = 0; // null terminator
        let buf_ptr = buf.as_ptr() as *mut c_char;

        let ret = putenv(buf_ptr);
        assert_eq!(ret, 0, "putenv failed");

        // 验证指针地址相同 (非拷贝)
        let entry = env_find(c"NONCOPY");
        assert!(!entry.is_null(), "NONCOPY not found in environ");
        assert_eq!(entry, buf_ptr as *mut c_char, "environ pointer != original buffer (putenv copies)");

        // 修改缓冲区内容, 验证 environ 中的值随之改变 (非拷贝的直接证据)
        let modified = b"NONCOPY=modified";
        let mod_len = modified.len();
        for i in 0..mod_len {
            buf[i] = modified[i] as c_char;
        }
        // 缓冲区已零初始化, 无需额外设置 null terminator

        // 通过 getenv 验证修改后的值
        assert!(
            getenv_equals(c"NONCOPY", b"modified"),
            "buffer modification not reflected in environ"
        );

        // 清理: 从环境中移除 NONCOPY (避免后续测试访问已被释放的栈内存)
        unsetenv(c"NONCOPY".as_ptr());
    }
});

// ===========================================================================
// 5. 插入新变量 (数组扩容)
// ===========================================================================

test!("test_putenv_insert_new_var" {
    // 规约: 新变量追加到 environ 末尾, 数组以 NULL 终止。
    unsafe {
        clearenv();
        putenv(c"FIRST=one".as_ptr() as *mut c_char);
        assert_eq!(env_count(), 1);

        putenv(c"SECOND=two".as_ptr() as *mut c_char);
        assert_eq!(env_count(), 2);

        // 验证两个变量都存在
        assert!(!env_find(c"FIRST").is_null(), "FIRST not found");
        assert!(!env_find(c"SECOND").is_null(), "SECOND not found");

        // 验证 NULL 终止
        assert!(env_null_terminated(), "environ array not NULL-terminated");
    }
});

// ===========================================================================
// 6. environ NULL 终止不变式
// ===========================================================================

test!("test_environ_null_terminated" {
    // 规约不变式: 环境数组始终以 NULL 指针终止。
    unsafe {
        clearenv();
        // clearenv 后 environ 为 NULL 或空(NULL终止)
        if !environ.is_null() {
            assert!((*environ).is_null(), "not null-terminated after clearenv");
        }

        putenv(c"NULLTERM=check".as_ptr() as *mut c_char);
        assert!(env_null_terminated(), "not null-terminated after putenv");

        putenv(c"NULLTERM2=check2".as_ptr() as *mut c_char);
        assert!(env_null_terminated(), "not null-terminated after 2nd putenv");
    }
});

// ===========================================================================
// 7. 传入不含 '=' 的字符串 -> 委托 unsetenv 移除变量
// ===========================================================================

test!("test_putenv_no_equals_removes_var" {
    // 规约: s[l] == '\0' (无 '='), 委托给 unsetenv(s) 处理, 返回 0。
    unsafe {
        clearenv();
        putenv(c"DELME=exists".as_ptr() as *mut c_char);

        // 预条件: 变量存在
        assert!(!env_find(c"DELME").is_null(), "pre-condition: DELME should exist");

        // putenv("DELME") — 无 '=', 触发 unsetenv 路径
        set_errno(0);
        let ret = putenv(c"DELME".as_ptr() as *mut c_char);
        assert_eq!(ret, 0, "putenv('DELME') should return 0, got {ret}");

        // 变量应被移除
        assert!(env_find(c"DELME").is_null(), "DELME still exists after putenv removal");
    }
});

test!("test_putenv_no_equals_nonexistent_var" {
    // 当变量不存在时, putenv 无 '=' 也返回 0 (委托 unsetenv, 未找到视为成功)
    unsafe {
        clearenv();
        set_errno(0);
        let ret = putenv(c"NOEXIST".as_ptr() as *mut c_char);
        assert_eq!(ret, 0, "putenv('NOEXIST') on nonexistent var should return 0, got {ret}");
        assert_eq!(get_errno(), 0, "errno should be 0 after removing nonexistent var");
    }
});

// ===========================================================================
// 8. 空变量名 (=VALUE) -> errno = EINVAL, 返回 -1
// ===========================================================================

test!("test_putenv_empty_name_equals_value" {
    // 规约: l == 0 时触发 unsetenv, unsetenv 检测到空变量名返回 EINVAL。
    unsafe {
        clearenv();
        set_errno(0);

        let ret = putenv(c"=novalue".as_ptr() as *mut c_char);
        assert_eq!(ret, -1, "putenv('=novalue') should return -1, got {ret}");
        assert_eq!(get_errno(), EINVAL, "errno should be EINVAL(22), got {}", get_errno());
    }
});

test!("test_putenv_equals_only" {
    // 只有等号 -> 同样 EINVAL
    unsafe {
        clearenv();
        set_errno(0);

        let ret = putenv(c"=".as_ptr() as *mut c_char);
        assert_eq!(ret, -1, "putenv('=') should return -1, got {ret}");
        assert_eq!(get_errno(), EINVAL, "errno should be EINVAL(22), got {}", get_errno());
    }
});

test!("test_putenv_empty_name_no_equals" {
    // 空字符串 -> unsetenv("") 返回 EINVAL
    unsafe {
        clearenv();
        set_errno(0);

        // 注意: putenv("") 中 s[0] == '\0', l == 0, 触发 unsetenv("")
        let ret = putenv(c"".as_ptr() as *mut c_char);
        assert_eq!(ret, -1, "putenv('') should return -1, got {ret}");
        assert_eq!(get_errno(), EINVAL, "errno should be EINVAL(22), got {}", get_errno());
    }
});

// ===========================================================================
// 9. 返回值验证
// ===========================================================================

test!("test_putenv_return_value_normal" {
    // 正常设置应返回 0, 不修改 errno
    unsafe {
        clearenv();
        set_errno(12345);

        let ret = putenv(c"RETCHECK=ok".as_ptr() as *mut c_char);
        assert_eq!(ret, 0, "putenv should return 0, got {ret}");
        // musl putenv 成功时不修改 errno
        assert_eq!(get_errno(), 12345, "errno should not be modified on success");

        assert!(
            getenv_equals(c"RETCHECK", b"ok"),
            "RETCHECK verification failed"
        );
    }
});

// ===========================================================================
// 10. 多次替换后数组大小不变
// ===========================================================================

test!("test_putenv_repeated_replace_size_stable" {
    // 规约: 替换时原地修改, 不改变数组大小。
    unsafe {
        clearenv();
        putenv(c"REPSIZE=first".as_ptr() as *mut c_char);
        let count_before = env_count();

        putenv(c"REPSIZE=second".as_ptr() as *mut c_char);
        assert_eq!(env_count(), count_before, "count changed after 1st replacement");

        putenv(c"REPSIZE=third".as_ptr() as *mut c_char);
        assert_eq!(env_count(), count_before, "count changed after 2nd replacement");

        assert!(
            getenv_equals(c"REPSIZE", b"third"),
            "REPSIZE should be 'third' after replacements"
        );
    }
});

// ===========================================================================
// 11. 多变量设置
// ===========================================================================

test!("test_putenv_multi_vars" {
    // 设置多个不同变量并全部验证。
    unsafe {
        clearenv();

        assert_eq!(putenv(c"MULTI_A=a".as_ptr() as *mut c_char), 0);
        assert_eq!(putenv(c"MULTI_B=b".as_ptr() as *mut c_char), 0);
        assert_eq!(putenv(c"MULTI_C=c".as_ptr() as *mut c_char), 0);
        assert_eq!(putenv(c"MULTI_D=d".as_ptr() as *mut c_char), 0);

        // 验证所有变量都存在
        assert!(!env_find(c"MULTI_A").is_null(), "MULTI_A not found");
        assert!(!env_find(c"MULTI_B").is_null(), "MULTI_B not found");
        assert!(!env_find(c"MULTI_C").is_null(), "MULTI_C not found");
        assert!(!env_find(c"MULTI_D").is_null(), "MULTI_D not found");
    }
});

// ===========================================================================
// 12. putenv/unsetenv 交互
// ===========================================================================

test!("test_putenv_unsetenv_interop" {
    // putenv 设置的变量可以被 unsetenv 移除。
    unsafe {
        clearenv();
        putenv(c"UNSETME=please".as_ptr() as *mut c_char);

        assert!(getenv_equals(c"UNSETME", b"please"), "pre-condition: UNSETME not set");

        set_errno(0);
        let ret = unsetenv(c"UNSETME".as_ptr());
        assert_eq!(ret, 0, "unsetenv should return 0, got {ret}");

        let val_after = getenv(c"UNSETME".as_ptr());
        assert!(val_after.is_null(), "UNSETME still present after unsetenv");

        // 重复 unsetenv 同名变量不应报错
        set_errno(0);
        let ret2 = unsetenv(c"UNSETME".as_ptr());
        assert_eq!(ret2, 0, "2nd unsetenv should return 0, got {ret2}");
        assert_eq!(get_errno(), 0, "2nd unsetenv should not set errno");
    }
});

test!("test_unsetenv_then_putenv" {
    // unsetenv 后 putenv 重新设置变量
    unsafe {
        clearenv();
        putenv(c"REBORN=first_life".as_ptr() as *mut c_char);
        unsetenv(c"REBORN".as_ptr());
        assert!(env_find(c"REBORN").is_null(), "REBORN should be gone after unsetenv");

        putenv(c"REBORN=second_life".as_ptr() as *mut c_char);
        assert!(getenv_equals(c"REBORN", b"second_life"), "REBORN re-add failed");
    }
});

// ===========================================================================
// 13. putenv/setenv 交互
// ===========================================================================

test!("test_putenv_setenv_interop" {
    // setenv 应能覆盖 putenv 设置的值。
    unsafe {
        clearenv();
        putenv(c"MIXED=putenv_val".as_ptr() as *mut c_char);

        assert!(getenv_equals(c"MIXED", b"putenv_val"), "initial putenv value not found");

        // setenv 覆盖 (overwrite=1)
        set_errno(0);
        let ret = setenv(c"MIXED".as_ptr(), c"setenv_val".as_ptr(), 1);
        assert_eq!(ret, 0, "setenv overwrite failed, ret={ret}");

        assert!(getenv_equals(c"MIXED", b"setenv_val"), "value should be setenv's after overwrite");
    }
});

test!("test_setenv_then_putenv" {
    // putenv 应能覆盖 setenv 设置的值。
    unsafe {
        clearenv();
        setenv(c"MIXED2".as_ptr(), c"setenv_first".as_ptr(), 1);
        assert!(getenv_equals(c"MIXED2", b"setenv_first"), "setenv initial value not found");

        set_errno(0);
        let ret = putenv(c"MIXED2=putenv_later".as_ptr() as *mut c_char);
        assert_eq!(ret, 0, "putenv overwrite failed, ret={ret}");

        assert!(getenv_equals(c"MIXED2", b"putenv_later"), "value should be putenv's after overwrite");
    }
});

// ===========================================================================
// 14. 变量名前缀部分匹配不误匹配
// ===========================================================================

test!("test_putenv_prefix_not_confused" {
    // 规约: strncmp(s, *e, l+1) 含 '=' 精确匹配, "FOO=x" 不被 "FO" 匹配。
    unsafe {
        clearenv();
        putenv(c"FOO=foo_val".as_ptr() as *mut c_char);
        putenv(c"FOOBAR=foobar_val".as_ptr() as *mut c_char);

        // getenv("FOO") 应返回 "foo_val"
        assert!(getenv_equals(c"FOO", b"foo_val"), "getenv('FOO') should return 'foo_val'");

        // getenv("FOOBAR") 应返回 "foobar_val"
        assert!(getenv_equals(c"FOOBAR", b"foobar_val"), "getenv('FOOBAR') should return 'foobar_val'");

        // getenv("FO") 不应返回任何值 (前缀不匹配)
        assert!(getenv(c"FO".as_ptr()).is_null(), "getenv('FO') should return NULL");
    }
});

// ===========================================================================
// 15. environ 为 NULL 时也能正常插入
// ===========================================================================

test!("test_putenv_when_environ_null" {
    // 规约: __environ 可能是 NULL (环境未初始化), putenv 必须能处理。
    unsafe {
        clearenv();
        assert_eq!(env_count(), 0, "environment not empty after clearenv");

        let ret = putenv(c"FROM_NULL=hello".as_ptr() as *mut c_char);
        assert_eq!(ret, 0, "putenv should work when environ is NULL/empty");
        assert_eq!(env_count(), 1, "expected 1 entry after putenv on NULL environ");
        assert!(!env_find(c"FROM_NULL").is_null(), "FROM_NULL not found");
        assert!(env_null_terminated(), "not null-terminated");
    }
});

// ===========================================================================
// 16. 特殊字符值
// ===========================================================================

test!("test_putenv_special_chars_in_value" {
    // 验证 putenv 能处理含特殊字符的值。
    unsafe {
        clearenv();
        let ret = putenv(c"SPECIAL=hello world!@#$".as_ptr() as *mut c_char);
        assert_eq!(ret, 0, "putenv with special chars should return 0");

        let val = getenv(c"SPECIAL".as_ptr());
        assert!(!val.is_null(), "SPECIAL not found");

        let cstr = CStr::from_ptr(val);
        assert_eq!(cstr.to_bytes(), b"hello world!@#$", "special chars value mismatch");
    }
});

test!("test_putenv_value_with_equals_sign" {
    // 值中包含 '=' 字符是合法的
    unsafe {
        clearenv();
        putenv(c"EQVAL=a=b=c".as_ptr() as *mut c_char);

        assert!(
            getenv_equals(c"EQVAL", b"a=b=c"),
            "value with equals signs mismatch"
        );
    }
});

// ===========================================================================
// 17. 大小写敏感性
// ===========================================================================

test!("test_putenv_case_sensitive" {
    // 环境变量名大小写敏感, "Key" != "key"
    unsafe {
        clearenv();
        putenv(c"Key=upper".as_ptr() as *mut c_char);

        // 小写 key 不应匹配
        let val_lower = getenv(c"key".as_ptr());
        assert!(val_lower.is_null(), "getenv('key') should return NULL (case sensitive)");

        // 精确大小写才匹配
        assert!(getenv_equals(c"Key", b"upper"), "exact case should match");
    }
});

// ===========================================================================
// 18. 连续 putenv 替换
// ===========================================================================

test!("test_putenv_chain_replace" {
    // 多次 putenv 同一变量, 最终值为最后一次设置的值
    unsafe {
        clearenv();
        putenv(c"CHAIN=val1".as_ptr() as *mut c_char);
        putenv(c"CHAIN=val2".as_ptr() as *mut c_char);
        putenv(c"CHAIN=val3".as_ptr() as *mut c_char);

        assert!(getenv_equals(c"CHAIN", b"val3"), "chain: value should be last set");
    }
});

// ===========================================================================
// 19. putenv 后 setenv overwrite=0 的行为
// ===========================================================================

test!("test_putenv_then_setenv_no_overwrite" {
    // putenv 已设置变量, setenv overwrite=0 不覆盖
    unsafe {
        clearenv();
        putenv(c"NOOW=putenv_original".as_ptr() as *mut c_char);

        let ret = setenv(c"NOOW".as_ptr(), c"setenv_ignored".as_ptr(), 0);
        assert_eq!(ret, 0, "setenv overwrite=0 should return 0");

        assert!(
            getenv_equals(c"NOOW", b"putenv_original"),
            "overwrite=0 should not change putenv value"
        );
    }
});

// ===========================================================================
// 20. 清理: 确保测试结束后环境干净 (用于测试隔离)
// ===========================================================================

test!("test_cleanup_environment" {
    // 清理环境, 确保初始状态可控。
    // 注意: 此测试虽列在前面, 但因 clearenv 被每个测试主动调用,
    // 此处的清理对后续测试无实际影响, 仅用于验证 clearenv 功能。
    unsafe {
        clearenv();
        let ret = clearenv();
        assert_eq!(ret, 0, "clearenv should return 0");

        if !environ.is_null() {
            assert!((*environ).is_null(), "environ not empty after clearenv");
        }
    }
});
