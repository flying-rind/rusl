//! unsetenv — 从进程环境变量列表中移除指定名称的环境变量。
//!
//! 对应 musl `src/env/unsetenv.c`。
//!
//! ## 算法概述
//!
//! 使用**单趟双指针原地压缩**算法:
//! - `reader`: 遍历环境数组中的所有条目
//! - `writer`: 指向下一个保留条目的写入位置 (invariant: writer <= reader)
//! - 匹配条目: 通过 `__env_rm_add` 回调通知内存管理模块后跳过
//! - 不匹配条目: 若已存在间隙 (writer < reader), 前移到 writer 位置
//! - 遍历结束后: 若有条目被移除, 在 writer 位置写入 NULL 终止哨兵
//!
//! 时间复杂度 O(n), 空间复杂度 O(1)。

#![allow(dead_code)]

use core::ffi::{c_char, c_int, CStr};
use core::ptr::null_mut;

use crate::__environ::environ;

// ---------------------------------------------------------------------------
// 回调机制 — 替代 C 的 weak_alias 弱符号
// ---------------------------------------------------------------------------

/// 默认空回调函数，不执行任何内存管理操作。
///
/// 对应 musl 原始 C 代码中 `static void dummy(char *old, char *new) {}`。
/// 当程序未使用 `setenv`/`putenv` 时，环境变量字符串来自内核传递
/// 的原始内存区域，无需(也不能)释放。
///
/// 作为 `__env_rm_add` 全局函数指针的默认值。
unsafe extern "C" fn dummy(_old: *mut c_char, _new: *mut c_char) {}

/// 环境变量删除/添加回调的函数指针类型。
///
/// 参数:
/// - `old`: 被替换或移除的旧环境变量字符串指针（可为 null）
/// - `new`: 新分配的环境变量字符串指针（可为 null）
///
/// 对应 C 中 `__env_rm_add` 的函数签名。
pub(crate) type EnvRmAddFn = unsafe extern "C" fn(*mut c_char, *mut c_char);

/// 全局可替换回调，默认指向无操作实现 `dummy`。
///
/// 等效于 C 实现中通过 ELF `weak_alias` 链接覆盖的 `__env_rm_add` 弱符号。
/// Rust 中无法直接使用弱符号，改用运行时可替换的函数指针实现等价语义:
///
/// - **默认**: 指向 `dummy`，不对被移除的环境字符串做任何操作
///   （适用于未链接 `setenv` 的程序，环境字符串来自内核原始内存区域）
/// - **setenv 模块加载后**: 指向 setenv 提供的真实内存管理回调，
///   负责释放堆分配的旧环境字符串并维护登记表
///
/// # Safety
///
/// - 读写 `static mut` 为 unsafe 操作，调用者需用 `unsafe` 块包裹。
/// - 所有环境变量修改操作（setenv/unsetenv/putenv/clearenv）之间无需同步，
///   因为 POSIX 规定这些函数本身非线程安全。
///
/// # Visibility
///
/// `pub(crate)` — 允许 `setenv` 模块在初始化时替换此指针。
#[allow(non_upper_case_globals)]
pub(crate) static mut __env_rm_add: EnvRmAddFn = dummy;

// ---------------------------------------------------------------------------
// unsetenv_impl — 内部安全抽象
// ---------------------------------------------------------------------------

/// 内部实现：从 `environ` 中移除匹配 `name` 的环境变量条目。
///
/// 使用双指针原地压缩算法，一次遍历完成匹配和数组压缩。
///
/// # Safety
///
/// 调用者必须保证:
/// - `name` 为非空、不含 `=` 字符、以 NUL 终止的合法 C 字符串。
/// - 在调用期间没有其他线程并发修改 `environ`（POSIX 非线程安全语义）。
/// - `environ` 若非 null，则指向有效的以 NULL 终止的 `*mut c_char` 数组。
///
/// # 返回值
///
/// - `0`: 成功（环境变量已移除或本就不存在）
/// - `-1`: 参数无效（errno 设为 `EINVAL`）
pub(crate) unsafe fn unsetenv_impl(name: &CStr) -> c_int {
    let name_bytes = name.to_bytes_with_nul();

    // 查找 '=' 的位置，若不存在则返回 name 末尾 NUL 的索引（即键名长度）
    let l = name_bytes
        .iter()
        .position(|&b| b == b'=')
        .unwrap_or(name_bytes.len() - 1);

    // 验证 name: 非空 且 不包含 '='
    // l == 0: 空字符串
    // name_bytes[l] != 0: 包含 '=', 即 name_bytes[l] == b'='
    if l == 0 || name_bytes[l] != 0 {
        rusl_core::errno::set_errno(rusl_core::errno::EINVAL);
        return -1;
    }

    // 若环境数组未初始化, 无操作返回成功（Case 5: environ 为 NULL 且 name 合法）
    // SAFETY: 读取全局静态变量，单线程环境，符合 POSIX 语义
    if environ.is_null() {
        return 0;
    }

    // 单趟双指针原地压缩
    // SAFETY: environ 非 null，指向有效的以 NULL 终止的指针数组
    let mut reader = environ;
    let mut writer = environ;

    loop {
        let entry_ptr = *reader;
        if entry_ptr.is_null() {
            break;
        }

        // SAFETY: entry_ptr 来自 environ 数组，musl 保证每个条目是有效的 NUL 终止字符串
        let entry_cstr = CStr::from_ptr(entry_ptr as *const _);
        let entry_bytes = entry_cstr.to_bytes();

        // 判断条目是否匹配要删除的键名:
        // 1. entry 长度 > l（否则无法包含 "NAME=VALUE" 格式的最短形式 "N="）
        // 2. 前 l 个字节与 name 的键名部分完全相等
        // 3. 第 l 个字节为 '='
        let is_match = entry_bytes.len() > l
            && &entry_bytes[..l] == &name_bytes[..l]
            && entry_bytes[l] == b'=';

        if is_match {
            // 通知内存管理模块: 该条目即将被移除（第二个参数为 null 表示无替换字符串）
            // SAFETY: __env_rm_add 指向有效函数（默认 dummy 或 setenv 注册的实现）
            __env_rm_add(entry_ptr, null_mut());
            // writer 不动（跳过此项，产生间隙），reader 继续前进
        } else {
            // 保留此条目，若存在间隙则前移
            if writer != reader {
                *writer = entry_ptr;
            }
            writer = writer.add(1);
        }
        reader = reader.add(1);
    }

    // 若有条目被移除，用 NULL 重新终止数组
    if writer != reader {
        *writer = null_mut();
    }

    0
}

// ---------------------------------------------------------------------------
// unsetenv — 对外导出 ABI
// ---------------------------------------------------------------------------

/// POSIX unsetenv — 从进程环境变量列表中移除指定名称的环境变量。
///
/// 声明于 `<stdlib.h>`, POSIX.1-2001 标准函数。
///
/// # Safety
///
/// - `name` 必须为非 NULL、以 NUL 终止的有效 C 字符串指针。
/// - `name` 指向的内存必须在调用期间有效。
/// - 多线程下并发调用 setenv/unsetenv/putenv 是未定义行为
///   （符合 POSIX 线程安全限制）。
///
/// # 返回值
///
/// - `0`: 成功（环境变量已移除或原本就不存在）
/// - `-1`: 参数无效，errno 设为 `EINVAL`（空字符串 或 name 包含 '='）
///
/// # 错误码
///
/// | errno | 条件 |
/// |-------|------|
/// | `EINVAL` | `name` 为 NULL、空字符串或包含 `=` 字符 |
#[no_mangle]
pub extern "C" fn unsetenv(name: *const c_char) -> c_int {
    // NULL 指针检查：等价于空 name 的 EINVAL 处理
    if name.is_null() {
        // SAFETY: set_errno 在单线程环境下安全
        unsafe { rusl_core::errno::set_errno(rusl_core::errno::EINVAL); }
        return -1;
    }

    // SAFETY: 调用者保证 name 是有效的以 NUL 终止的 C 字符串。
    // 即使在 name 为空或含 '=' 的情况下，CStr::from_ptr 也是安全的——
    // CStr 只检查 NUL 终止符，不验证环境变量名的语义合法性。
    // 语义验证在 unsetenv_impl 内部进行。
    let name_cstr = unsafe { CStr::from_ptr(name) };
    unsafe { unsetenv_impl(name_cstr) }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;
    use rusl_core::test;
    use core::ffi::CStr;

    // ---- 测试辅助 ----

    // 在每个测试后重置全局状态，避免测试间状态残留。
    unsafe fn reset_state() {
        environ = null_mut();
        __env_rm_add = dummy;
    }

    // 在栈上构造以 NUL 终止的 C 字符串字节数组。
    fn make_cstr<const N: usize>(s: &[u8]) -> [c_char; N] {
        let mut buf: [c_char; N] = [0; N];
        let len = s.len().min(N - 1);
        for i in 0..len {
            buf[i] = s[i] as c_char;
        }
        buf
    }

    // 构造一个测试用环境数组。
    //
    // `strings` 是 `(key, value)` 对的列表, 自动构造成 `"KEY=VALUE"` 格式。
    // 返回:
    // - `_bufs`: 持有环境字符串数据的缓冲区（保持存活）
    // - `env_ptr`: 指向环境条目指针数组的指针
    struct TestEnv {
        _bufs: Vec<Vec<u8>>,
        env_ptr: *mut *mut c_char,
    }

    impl TestEnv {
        fn from_pairs(pairs: &[(&str, &str)]) -> Self {
            let mut bufs: Vec<Vec<u8>> = Vec::new();
            let mut ptrs: Vec<*mut c_char> = Vec::new();

            for &(k, v) in pairs {
                let mut s = Vec::new();
                s.extend_from_slice(k.as_bytes());
                s.push(b'=');
                s.extend_from_slice(v.as_bytes());
                s.push(0);
                ptrs.push(s.as_mut_ptr() as *mut c_char);
                bufs.push(s);
            }
            ptrs.push(null_mut());

            let env_ptr = ptrs.as_mut_ptr();
            // 泄露 ptrs 以便返回的指针在测试期间有效
            core::mem::forget(ptrs);

            Self {
                _bufs: bufs,
                env_ptr,
            }
        }
    }

    // ========================================================================
    // dummy / __env_rm_add 测试
    // ========================================================================

    // 验证 dummy 回调不会 panic: 任意参数组合均安全返回。
    test!("test_dummy_does_nothing" {
        unsafe {
            dummy(null_mut(), null_mut());
            dummy(0x1000usize as *mut c_char, null_mut());
            dummy(null_mut(), 0x2000usize as *mut c_char);
            dummy(0x1000usize as *mut c_char, 0x2000usize as *mut c_char);
        }
    });

    // 验证 __env_rm_add 默认指向 dummy。
    test!("test_default_callback_is_dummy" {
        unsafe {
            reset_state();
            // 通过比较函数指针地址验证默认值
            let default_ptr = __env_rm_add as *const EnvRmAddFn;
            let dummy_ptr = dummy as *const EnvRmAddFn;
            // 函数指针地址比较: 二者应指向同一函数
            assert_eq!(default_ptr, dummy_ptr,
                "__env_rm_add 默认应指向 dummy");
        }
    });

    // ========================================================================
    // unsetenv_impl / unsetenv 行为测试 — Case 3: 空字符串
    // ========================================================================

    // Case 3: name 为空字符串 — 返回 -1, errno = EINVAL。
    test!("test_unset_impl_empty_name" {
        unsafe {
            reset_state();
            let name_cstr = CStr::from_ptr(b"\0".as_ptr() as *const c_char);
            let result = unsetenv_impl(name_cstr);
            assert_eq!(result, -1, "空字符串应返回 -1");
            assert_eq!(*rusl_core::errno::__errno_location(), rusl_core::errno::EINVAL,
                "errno 应设为 EINVAL");
        }
    });

    // ========================================================================
    // Case 4: name 包含 '='
    // ========================================================================

    // Case 4: name 包含 '=' — 返回 -1, errno = EINVAL。
    test!("test_unset_impl_name_with_equals" {
        unsafe {
            reset_state();
            let name_cstr = CStr::from_ptr(b"FOO=BAR\0".as_ptr() as *const c_char);
            let result = unsetenv_impl(name_cstr);
            assert_eq!(result, -1, "含 '=' 的名称应返回 -1");
            assert_eq!(*rusl_core::errno::__errno_location(), rusl_core::errno::EINVAL,
                "errno 应设为 EINVAL");
        }
    });

    // ========================================================================
    // Case 5: environ 为 NULL 且 name 合法
    // ========================================================================

    // Case 5: environ 为 NULL, 合法名称 — 返回 0（无操作）。
    test!("test_unset_impl_null_environ" {
        unsafe {
            reset_state();
            assert!(environ.is_null(), "前置: environ 应为 null");

            let name_cstr = CStr::from_ptr(b"ANYTHING\0".as_ptr() as *const c_char);
            let result = unsetenv_impl(name_cstr);
            assert_eq!(result, 0, "environ 为 null 时应返回 0");
        }
    });

    // ========================================================================
    // Case 2: environ 中有条目但无匹配项
    // ========================================================================

    // Case 2: environ 中有条目但 name 不匹配任何条目 — 返回 0, environ 不变。
    test!("test_unset_no_match" {
        unsafe {
            reset_state();

            let env = TestEnv::from_pairs(&[("PATH", "/usr/bin"), ("HOME", "/root")]);
            environ = env.env_ptr;

            let name_cstr = CStr::from_ptr(b"USER\0".as_ptr() as *const c_char);
            let result = unsetenv_impl(name_cstr);
            assert_eq!(result, 0, "无匹配时应返回 0");

            // 环境数组应保持不变
            assert_eq!(*environ, *env.env_ptr, "第一个条目应不变");
            assert_eq!(*environ.add(1), *env.env_ptr.add(1), "第二个条目应不变");
            assert!((*environ.add(2)).is_null(), "终止哨兵应不变");
        }
    });

    // ========================================================================
    // Case 1: 成功移除匹配项
    // ========================================================================

    // Case 1: 移除数组中间的条目。
    test!("test_unset_remove_middle" {
        unsafe {
            reset_state();

            let env = TestEnv::from_pairs(&[
                ("PATH", "/usr/bin"),
                ("HOME", "/root"),
                ("SHELL", "/bin/sh"),
            ]);
            environ = env.env_ptr;

            // 移除中间的 "HOME"
            let name_cstr = CStr::from_ptr(b"HOME\0".as_ptr() as *const c_char);
            let result = unsetenv_impl(name_cstr);
            assert_eq!(result, 0, "移除应返回 0");

            // 验证: environ[0] 仍是 PATH, environ[1] 是原来的 SHELL（前移）
            let entry0 = *environ;
            let entry1 = *environ.add(1);
            let term = *environ.add(2);

            assert!(!entry0.is_null(), "第一个条目不应为 null");
            assert!(!entry1.is_null(), "第二个条目不应为 null");
            assert!(term.is_null(), "第三个条目应为 null 终止哨兵");

            // 验证 PATH 和 SHELL 的内容
            let c0 = CStr::from_ptr(entry0 as *const _);
            assert!(c0.to_bytes().starts_with(b"PATH="), "第一个条目应为 PATH");
            let c1 = CStr::from_ptr(entry1 as *const _);
            assert!(c1.to_bytes().starts_with(b"SHELL="), "第二个条目应为 SHELL（已前移）");
        }
    });

    // 移除数组第一个条目（头部）。
    test!("test_unset_remove_first" {
        unsafe {
            reset_state();

            let env = TestEnv::from_pairs(&[
                ("PATH", "/usr/bin"),
                ("HOME", "/root"),
                ("SHELL", "/bin/sh"),
            ]);
            environ = env.env_ptr;

            // 移除第一个 "PATH"
            let name_cstr = CStr::from_ptr(b"PATH\0".as_ptr() as *const c_char);
            let result = unsetenv_impl(name_cstr);
            assert_eq!(result, 0);

            // 验证: 原来的第二个和第三个条目前移
            let entry0 = *environ;
            let entry1 = *environ.add(1);
            let term = *environ.add(2);

            assert!(!entry0.is_null(), "第一个条目应为原第二个");
            assert!(!entry1.is_null(), "第二个条目应为原第三个");
            assert!(term.is_null(), "第三个应为 null");

            let c0 = CStr::from_ptr(entry0 as *const _);
            assert!(c0.to_bytes().starts_with(b"HOME="), "第一个条目应为 HOME");
            let c1 = CStr::from_ptr(entry1 as *const _);
            assert!(c1.to_bytes().starts_with(b"SHELL="), "第二个条目应为 SHELL");
        }
    });

    // 移除数组最后一个条目（尾部）。
    test!("test_unset_remove_last" {
        unsafe {
            reset_state();

            let env = TestEnv::from_pairs(&[
                ("PATH", "/usr/bin"),
                ("HOME", "/root"),
                ("SHELL", "/bin/sh"),
            ]);
            environ = env.env_ptr;

            // 移除最后一个 "SHELL"
            let name_cstr = CStr::from_ptr(b"SHELL\0".as_ptr() as *const c_char);
            let result = unsetenv_impl(name_cstr);
            assert_eq!(result, 0);

            // 验证: 前两个条目不变, 第三个为 null
            let entry0 = *environ;
            let entry1 = *environ.add(1);
            let term = *environ.add(2);

            assert!(!entry0.is_null());
            assert!(!entry1.is_null());
            assert!(term.is_null(), "第三个条目应为 null");

            let c0 = CStr::from_ptr(entry0 as *const _);
            assert!(c0.to_bytes().starts_with(b"PATH="));
            let c1 = CStr::from_ptr(entry1 as *const _);
            assert!(c1.to_bytes().starts_with(b"HOME="));
        }
    });

    // 移除数组中唯一的条目。
    test!("test_unset_remove_only_entry" {
        unsafe {
            reset_state();

            let env = TestEnv::from_pairs(&[("HOME", "/root")]);
            environ = env.env_ptr;

            let name_cstr = CStr::from_ptr(b"HOME\0".as_ptr() as *const c_char);
            let result = unsetenv_impl(name_cstr);
            assert_eq!(result, 0);

            // 验证: 数组现在仅含 null 哨兵
            assert!((*environ).is_null(), "唯一条目被移除后应为 null");
        }
    });

    // ========================================================================
    // 幂等性测试
    // ========================================================================

    // 连续两次移除同一变量: 第二次也是无操作返回 0。
    test!("test_unset_idempotent" {
        unsafe {
            reset_state();

            let env = TestEnv::from_pairs(&[("HOME", "/root"), ("PATH", "/bin")]);
            environ = env.env_ptr;

            let name_cstr = CStr::from_ptr(b"HOME\0".as_ptr() as *const c_char);

            let r1 = unsetenv_impl(name_cstr);
            assert_eq!(r1, 0, "第一次移除应返回 0");

            let r2 = unsetenv_impl(name_cstr);
            assert_eq!(r2, 0, "第二次移除（幂等）应返回 0");

            // 第二次后 PATH 仍在
            let entry0 = *environ;
            let c0 = CStr::from_ptr(entry0 as *const _);
            assert!(c0.to_bytes().starts_with(b"PATH="), "PATH 应仍在");
            assert!((*environ.add(1)).is_null(), "终止哨兵应为 null");
        }
    });

    // ========================================================================
    // 键名前缀匹配测试
    // ========================================================================

    // 确保 "HOME" 不会误匹配 "HOMEOTHER"。
    test!("test_unset_prefix_match" {
        unsafe {
            reset_state();

            let env = TestEnv::from_pairs(&[("HOME", "/root"), ("HOMEOTHER", "/other")]);
            environ = env.env_ptr;

            let name_cstr = CStr::from_ptr(b"HOME\0".as_ptr() as *const c_char);
            let result = unsetenv_impl(name_cstr);
            assert_eq!(result, 0);

            // 只有 HOME 被移除, HOMEOTHER 保留
            let entry0 = *environ;
            let term = *environ.add(1);
            let c0 = CStr::from_ptr(entry0 as *const _);
            assert!(c0.to_bytes().starts_with(b"HOMEOTHER="),
                "HOMEOTHER 不应被误匹配移除");
            assert!(term.is_null());
        }
    });

    // 确保 "HOMEOTHER" 不会误匹配 "HOME"。
    test!("test_unset_longer_name_does_not_match_shorter" {
        unsafe {
            reset_state();

            let env = TestEnv::from_pairs(&[("HOME", "/root"), ("HOMEOTHER", "/other")]);
            environ = env.env_ptr;

            let name_cstr = CStr::from_ptr(b"HOMEOTHER\0".as_ptr() as *const c_char);
            let result = unsetenv_impl(name_cstr);
            assert_eq!(result, 0);

            // HOMEOTHER 被移除, HOME 保留
            let entry0 = *environ;
            let c0 = CStr::from_ptr(entry0 as *const _);
            assert!(c0.to_bytes().starts_with(b"HOME="),
                "HOME 不应被误匹配移除");
        }
    });

    // 确保 "HOM" 不会匹配 "HOME"。
    test!("test_unset_shorter_name_does_not_match" {
        unsafe {
            reset_state();

            let env = TestEnv::from_pairs(&[("HOME", "/root")]);
            environ = env.env_ptr;

            let name_cstr = CStr::from_ptr(b"HOM\0".as_ptr() as *const c_char);
            let result = unsetenv_impl(name_cstr);
            assert_eq!(result, 0);

            // 无匹配, HOME 保留
            let entry0 = *environ;
            let c0 = CStr::from_ptr(entry0 as *const _);
            assert!(c0.to_bytes().starts_with(b"HOME="),
                "HOME 不应被 'HOM' 误匹配");
        }
    });

    // 确保 "HO" 不会匹配 "H"。
    test!("test_unset_entry_value_equals" {
        unsafe {
            reset_state();

            // 构造 "KEY=val=ue" 条目, 确保值中的 '=' 不影响匹配
            let env = TestEnv::from_pairs(&[("KEY", "val=ue")]);
            environ = env.env_ptr;

            let name_cstr = CStr::from_ptr(b"KEY\0".as_ptr() as *const c_char);
            let result = unsetenv_impl(name_cstr);
            assert_eq!(result, 0);

            // KEY 被正确移除
            assert!((*environ).is_null(), "含 '=' 值的条目应被正确移除");
        }
    });

    // ========================================================================
    // unsetenv 外部 ABI 测试
    // ========================================================================

    // 外部 ABI: NULL 参数 — 返回 -1 + EINVAL。
    test!("test_unsetenv_null_pointer" {
        unsafe {
            reset_state();
            let result = unsetenv(null_mut());
            assert_eq!(result, -1, "NULL 参数应返回 -1");
            assert_eq!(*rusl_core::errno::__errno_location(), rusl_core::errno::EINVAL,
                "errno 应设为 EINVAL");
        }
    });

    // 外部 ABI: 空字符串 — 返回 -1 + EINVAL。
    test!("test_unsetenv_empty_name" {
        unsafe {
            reset_state();
            let result = unsetenv(b"\0".as_ptr() as *const c_char);
            assert_eq!(result, -1);
        }
    });

    // 外部 ABI: 合法参数正常移除。
    test!("test_unsetenv_normal_removal" {
        unsafe {
            reset_state();

            let env = TestEnv::from_pairs(&[("MYVAR", "hello_world"), ("OTHER", "value")]);
            environ = env.env_ptr;

            let result = unsetenv(b"MYVAR\0".as_ptr() as *const c_char);
            assert_eq!(result, 0);

            // 验证 OTHER 仍在
            let entry0 = *environ;
            let c0 = CStr::from_ptr(entry0 as *const _);
            assert!(c0.to_bytes().starts_with(b"OTHER="));
            assert!((*environ.add(1)).is_null());
        }
    });

    // ========================================================================
    // __env_rm_add 回调调用验证
    // ========================================================================

    // 测试用回调: 记录被移除条目的指针。
    static mut RM_CALLED: bool = false;
    static mut RM_OLD_PTR: *mut c_char = null_mut();

    unsafe extern "C" fn test_rm_callback(old: *mut c_char, new: *mut c_char) {
        RM_CALLED = true;
        RM_OLD_PTR = old;
        // 验证 new 为 null（unsetenv 不提供替换字符串）
        assert!(new.is_null(), "unsetenv 回调的 new 参数应为 null");
    }

    // 验证匹配项被移除时 __env_rm_add 回调被调用。
    test!("test_rm_callback_called_on_match" {
        unsafe {
            reset_state();
            RM_CALLED = false;
            RM_OLD_PTR = null_mut();

            // 注册测试回调
            __env_rm_add = test_rm_callback;

            let env = TestEnv::from_pairs(&[("HOME", "/root"), ("PATH", "/bin")]);
            environ = env.env_ptr;

            let name_cstr = CStr::from_ptr(b"HOME\0".as_ptr() as *const c_char);
            let result = unsetenv_impl(name_cstr);
            assert_eq!(result, 0);
            assert!(RM_CALLED, "匹配项被移除时回调应被调用");
            assert!(!RM_OLD_PTR.is_null(), "回调的 old 参数应为被移除的条目指针");
        }
    });

    // 验证无匹配项时 __env_rm_add 回调不被调用。
    test!("test_rm_callback_not_called_on_no_match" {
        unsafe {
            reset_state();
            RM_CALLED = false;

            __env_rm_add = test_rm_callback;

            let env = TestEnv::from_pairs(&[("HOME", "/root")]);
            environ = env.env_ptr;

            let name_cstr = CStr::from_ptr(b"USER\0".as_ptr() as *const c_char);
            let result = unsetenv_impl(name_cstr);
            assert_eq!(result, 0);
            assert!(!RM_CALLED, "无匹配项时回调不应被调用");
        }
    });

    // 验证移除多个匹配项（虽然正常环境不应有重复键名，但测试算法行为）。
    test!("test_unset_remove_multiple_same_key" {
        unsafe {
            reset_state();

            let env = TestEnv::from_pairs(&[
                ("HOME", "/first"),
                ("HOME", "/second"),
                ("PATH", "/bin"),
            ]);
            environ = env.env_ptr;

            let name_cstr = CStr::from_ptr(b"HOME\0".as_ptr() as *const c_char);
            let result = unsetenv_impl(name_cstr);
            assert_eq!(result, 0);

            // 两个 HOME 都被移除, 只有 PATH 保留
            let entry0 = *environ;
            assert!(!entry0.is_null());
            let c0 = CStr::from_ptr(entry0 as *const _);
            assert!(c0.to_bytes().starts_with(b"PATH="),
                "多次匹配时所有匹配项都应被移除");
            assert!((*environ.add(1)).is_null(), "PATH 之后应为 null 终止");
        }
    });

    // ========================================================================
    // 循环不变量验证
    // ========================================================================

    // 验证 writer <= reader 不变量: writer 绝不会超过 reader。
    // 此测试通过构造特定场景确保 compaction 算法的正确性。
    test!("test_invariant_writer_not_exceed_reader" {
        unsafe {
            reset_state();

            // 构造场景: 移除第一个条目后 writer 停留在原地，reader 前进
            let env = TestEnv::from_pairs(&[("A", "1"), ("B", "2"), ("C", "3")]);
            environ = env.env_ptr;

            let name_cstr = CStr::from_ptr(b"A\0".as_ptr() as *const c_char);
            let result = unsetenv_impl(name_cstr);
            assert_eq!(result, 0);

            // 验证最终状态: B 和 C 前移到位置 0 和 1
            let e0 = CStr::from_ptr(*environ as *const _);
            assert!(e0.to_bytes().starts_with(b"B="));
            let e1 = CStr::from_ptr(*environ.add(1) as *const _);
            assert!(e1.to_bytes().starts_with(b"C="));
            assert!((*environ.add(2)).is_null());
        }
    });

    // 验证空环境数组的行为。
    test!("test_unset_on_empty_array" {
        unsafe {
            reset_state();

            // 仅含 null 哨兵的空环境
            let mut empty_array: [*mut c_char; 1] = [null_mut()];
            environ = empty_array.as_mut_ptr();

            let name_cstr = CStr::from_ptr(b"HOME\0".as_ptr() as *const c_char);
            let result = unsetenv_impl(name_cstr);
            assert_eq!(result, 0);
            assert!((*environ).is_null(), "空数组应保持 null");
        }
    });

    // 验证单字符键名的正确匹配。
    test!("test_unset_single_char_name" {
        unsafe {
            reset_state();

            let env = TestEnv::from_pairs(&[("A", "1"), ("B", "2")]);
            environ = env.env_ptr;

            let name_cstr = CStr::from_ptr(b"A\0".as_ptr() as *const c_char);
            let result = unsetenv_impl(name_cstr);
            assert_eq!(result, 0);

            let e0 = CStr::from_ptr(*environ as *const _);
            assert!(e0.to_bytes().starts_with(b"B="));
            assert!((*environ.add(1)).is_null());
        }
    });

    // 验证键名大小写敏感。
    test!("test_unset_case_sensitive" {
        unsafe {
            reset_state();

            let env = TestEnv::from_pairs(&[("MyVar", "upper")]);
            environ = env.env_ptr;

            // 尝试用小写移除
            let name_lower = CStr::from_ptr(b"myvar\0".as_ptr() as *const c_char);
            let result = unsetenv_impl(name_lower);
            assert_eq!(result, 0, "大小写敏感: 不匹配也应返回 0");

            // 原条目应仍在
            let e0 = CStr::from_ptr(*environ as *const _);
            assert!(e0.to_bytes().starts_with(b"MyVar="),
                "大小写不同的键名不应匹配");

            // 用正确的大小写移除
            let name_correct = CStr::from_ptr(b"MyVar\0".as_ptr() as *const c_char);
            let result2 = unsetenv_impl(name_correct);
            assert_eq!(result2, 0);
            assert!((*environ).is_null(), "正确大小写的键名应匹配");
        }
    });
}
