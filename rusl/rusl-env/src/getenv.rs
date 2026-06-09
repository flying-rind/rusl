//! getenv — 在进程环境变量列表中查找指定名称的环境变量，返回其值字符串指针。
//!
//! 对应 musl `src/env/getenv.c`。
//!
//! ## 算法概述
//!
//! 1. 使用单遍扫描计算名称长度并检测 `=` 字符（等价于 musl 的 `__strchrnul`）
//! 2. 非法名称（空字符串或含 `=`）直接返回 `null_mut()`
//! 3. 环境未初始化（`environ == null`）也返回 `null_mut()`
//! 4. 线性扫描 `environ` 数组，对每个条目进行精确名称匹配：
//!    前缀 l 个字符相等 且 第 l 个字符为 `=`，则返回条目指针 + l + 1
//!
//! ## 返回值语义
//!
//! 返回的指针指向进程环境内存内部，调用者只能读取，不可修改或释放。

use core::ffi::{c_char, CStr};

// ============================================================================
// 对外 ABI: getenv
// ============================================================================

/// POSIX getenv — 在进程环境变量列表中查找指定名称的环境变量。
///
/// # Safety
///
/// - `name` 必须为非空、以 `\0` 结尾的有效 C 字符串指针。
/// - `name` 指向的内存必须在调用期间有效。
///
/// # 返回值
///
/// - 找到: 指向值字符串（`NAME=` 之后的部分）的指针，位于进程环境内存中。
/// - 未找到: `core::ptr::null_mut()`。
///
/// 调用者只能读取返回值，不得修改或释放。本函数不设置 `errno`。
#[no_mangle]
pub extern "C" fn getenv(name: *const c_char) -> *mut c_char {
    // SAFETY: 调用者确保 name 为非空、以 \0 结尾的有效 C 字符串。
    // 内部访问全局 environ 指针及环境条目内存。
    unsafe {
        // Step 1: 计算名称长度，同时检测 '=' 字符
        let eq_or_end = strchrnul_impl(name, b'=');
        let l = (eq_or_end as usize).wrapping_sub(name as usize);

        // Step 2: 非法名称 — 空字符串 或 包含 '='
        if l == 0 || *eq_or_end != 0 {
            return core::ptr::null_mut();
        }

        // Step 3: 检查 environ 是否已初始化
        let env_ptr = crate::__environ::environ;
        if env_ptr.is_null() {
            return core::ptr::null_mut();
        }

        // Step 4: 线性扫描环境变量数组
        let name_bytes = core::slice::from_raw_parts(name as *const u8, l);
        let mut i = 0;
        loop {
            let entry = *env_ptr.add(i);
            if entry.is_null() {
                break;
            }
            // 精确名称匹配: 前 l 个字符相等
            if prefix_match(name_bytes, entry, l)
                && *entry.add(l) as u8 == b'='
            {
                return entry.add(l + 1);
            }
            i += 1;
        }

        core::ptr::null_mut()
    }
}

// ============================================================================
// 内部安全抽象: find_env_value
// ============================================================================

/// 安全 Rust 封装：使用已验证的 `CStr` 名称在全局 `environ` 中查找值。
///
/// # 前置条件
///
/// - `name` 为非空、不含 `=` 字符的合法环境变量名。
///
/// 调用者负责保证前置条件。该函数内部使用 `unsafe` 块访问 `environ`，
/// 但 unsafe 范围仅限于必要的指针操作。
pub(crate) fn find_env_value(name: &CStr) -> Option<*mut c_char> {
    let name_bytes = name.to_bytes();
    if name_bytes.is_empty() {
        return None;
    }

    let env_ptr = unsafe { crate::__environ::environ };
    if env_ptr.is_null() {
        return None;
    }

    search_env_entries(name_bytes, env_ptr)
}

// ============================================================================
// 核心搜索逻辑 — 可独立测试，不依赖 global static
// ============================================================================

/// 在给定的环境条目数组中查找名称对应的值。
///
/// `env_entries` 指向以 null 指针结尾的 `*mut c_char` 数组。
/// 每个条目格式为 `"NAME=VALUE"`。
///
/// 返回指向条目中 `=` 之后部分的指针；未找到返回 `None`。
fn search_env_entries(name_bytes: &[u8], env_entries: *mut *mut c_char) -> Option<*mut c_char> {
    let l = name_bytes.len();
    if l == 0 {
        return None;
    }

    let mut i = 0;
    loop {
        let entry = unsafe { *env_entries.add(i) };
        if entry.is_null() {
            break;
        }
        if prefix_match(name_bytes, entry, l)
            && unsafe { *entry.add(l) as u8 } == b'='
        {
            return Some(unsafe { entry.add(l + 1) });
        }
        i += 1;
    }

    None
}

// ============================================================================
// 内部辅助函数
// ============================================================================

/// 模拟 musl 的 `__strchrnul(s, c)`: 在字符串 s 中查找字符 c 首次出现的位置。
///
/// 若找到返回指向该字符的指针，否则返回指向终止 `\0` 的指针。
///
/// # Safety
/// - `s` 必须以 `\0` 结尾。
unsafe fn strchrnul_impl(s: *const c_char, c: u8) -> *const c_char {
    let p = s as *const u8;
    let mut i = 0;
    loop {
        let byte = *p.add(i);
        if byte == c || byte == 0 {
            return p.add(i) as *const c_char;
        }
        i += 1;
    }
}

/// 比较 `name_bytes` 与 `entry` 的前 l 个字节是否完全相等。
///
/// `entry` 指向 `"NAME=VALUE"` 格式的 C 字符串。
/// `name_bytes` 为要匹配的名称（不含 `=`，不含终止符）。
fn prefix_match(name_bytes: &[u8], entry: *mut c_char, l: usize) -> bool {
    let entry_bytes = entry as *const u8;
    for i in 0..l {
        if unsafe { *entry_bytes.add(i) } != name_bytes[i] {
            return false;
        }
    }
    true
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;
    use rusl_core::test;

    // ---- strchrnul_impl 测试 ----

    test!("test_strchrnul_no_equals_returns_nul" {
        let s = b"FOO\0";
        let r = unsafe { strchrnul_impl(s.as_ptr() as *const c_char, b'=') };
        assert_eq!(unsafe { *r as u8 }, 0);
        let len = (r as usize) - (s.as_ptr() as usize);
        assert_eq!(len, 3);
    });

    test!("test_strchrnul_finds_equals" {
        let s = b"FOO=BAR\0";
        let r = unsafe { strchrnul_impl(s.as_ptr() as *const c_char, b'=') };
        assert_eq!(unsafe { *r as u8 }, b'=');
        let len = (r as usize) - (s.as_ptr() as usize);
        assert_eq!(len, 3);
    });

    test!("test_strchrnul_empty_string" {
        let s = b"\0";
        let r = unsafe { strchrnul_impl(s.as_ptr() as *const c_char, b'=') };
        assert_eq!(unsafe { *r as u8 }, 0);
        let len = (r as usize) - (s.as_ptr() as usize);
        assert_eq!(len, 0);
    });

    test!("test_strchrnul_no_match_long" {
        let s = b"LONG_VARIABLE_NAME_WITHOUT_EQUALS\0";
        let r = unsafe {
            strchrnul_impl(s.as_ptr() as *const c_char, b'=')
        };
        assert_eq!(unsafe { *r as u8 }, 0);
    });

    // ---- prefix_match 测试 ----

    test!("test_prefix_match_full_equal" {
        let entry = b"PATH=/usr/bin\0";
        assert!(prefix_match(b"PATH", entry.as_ptr() as *mut c_char, 4));
    });

    test!("test_prefix_match_diff" {
        let entry = b"PATH=/usr/bin\0";
        assert!(!prefix_match(b"PATT", entry.as_ptr() as *mut c_char, 4));
    });

    test!("test_prefix_match_longer_name" {
        let entry = b"FOO=bar\0";
        assert!(!prefix_match(b"FOOBAR", entry.as_ptr() as *mut c_char, 6));
    });

    // ---- search_env_entries 测试 ----

    /// 在堆上构造测试用环境字符串数组。
    struct TestEnv {
        _bufs: Vec<Vec<u8>>,
        ptrs: Vec<*mut c_char>,
    }

    impl TestEnv {
        fn new(entries: &[&str]) -> Self {
            let bufs: Vec<Vec<u8>> = entries
                .iter()
                .map(|s| {
                    let mut v = Vec::new();
                    v.extend_from_slice(s.as_bytes());
                    v.push(0);
                    v
                })
                .collect();
            let mut ptrs: Vec<*mut c_char> = bufs
                .iter()
                .map(|b| b.as_ptr() as *mut c_char)
                .collect();
            ptrs.push(core::ptr::null_mut());
            Self {
                _bufs: bufs,
                ptrs,
            }
        }

        fn as_mut_ptr(&mut self) -> *mut *mut c_char {
            self.ptrs.as_mut_ptr()
        }
    }

    test!("test_search_found_simple" {
        let mut env = TestEnv::new(&["PATH=/usr/bin", "HOME=/root", "SHELL=/bin/sh"]);
        let r = search_env_entries(b"HOME", env.as_mut_ptr());
        assert!(r.is_some());
        let val_str = unsafe { CStr::from_ptr(r.unwrap()) };
        assert_eq!(val_str.to_bytes(), b"/root");
    });

    test!("test_search_not_found" {
        let mut env = TestEnv::new(&["PATH=/usr/bin", "HOME=/root"]);
        let r = search_env_entries(b"USER", env.as_mut_ptr());
        assert!(r.is_none());
    });

    test!("test_search_empty_env" {
        let empty: [*mut c_char; 1] = [core::ptr::null_mut()];
        let r = search_env_entries(b"HOME", empty.as_ptr() as *mut *mut c_char);
        assert!(r.is_none());
    });

    test!("test_search_empty_name_returns_none" {
        let mut env = TestEnv::new(&["PATH=/usr/bin"]);
        let r = search_env_entries(b"", env.as_mut_ptr());
        assert!(r.is_none());
    });

    test!("test_search_prefix_exact_match" {
        // 确保 "HOME" 不会匹配 "HOMEOTHER"
        let mut env = TestEnv::new(&["HOME=/root", "HOMEOTHER=/other"]);
        let r = search_env_entries(b"HOME", env.as_mut_ptr());
        assert!(r.is_some());
        let val_str = unsafe { CStr::from_ptr(r.unwrap()) };
        assert_eq!(val_str.to_bytes(), b"/root");
    });

    test!("test_search_first_match_returned" {
        let mut env = TestEnv::new(&["PATH=/first", "PATH=/second"]);
        let r = search_env_entries(b"PATH", env.as_mut_ptr());
        assert!(r.is_some());
        let val_str = unsafe { CStr::from_ptr(r.unwrap()) };
        assert_eq!(val_str.to_bytes(), b"/first");
    });

    test!("test_search_value_empty" {
        let mut env = TestEnv::new(&["EMPTY="]);
        let r = search_env_entries(b"EMPTY", env.as_mut_ptr());
        assert!(r.is_some());
        let val_str = unsafe { CStr::from_ptr(r.unwrap()) };
        assert_eq!(val_str.to_bytes(), b"");
    });

    test!("test_search_single_char_name" {
        let mut env = TestEnv::new(&["A=1", "B=2"]);
        let r = search_env_entries(b"A", env.as_mut_ptr());
        assert!(r.is_some());
        let val_str = unsafe { CStr::from_ptr(r.unwrap()) };
        assert_eq!(val_str.to_bytes(), b"1");
    });

    test!("test_search_value_contains_equals" {
        // 值中包含 '=' 的情况（如 BASE64 编码值）
        let mut env = TestEnv::new(&["KEY=val=ue"]);
        let r = search_env_entries(b"KEY", env.as_mut_ptr());
        assert!(r.is_some());
        let val_str = unsafe { CStr::from_ptr(r.unwrap()) };
        assert_eq!(val_str.to_bytes(), b"val=ue");
    });

    // ---- find_env_value 测试 ----

    test!("test_find_env_value_null_environ" {
        let name = unsafe { CStr::from_ptr(b"HOME\0".as_ptr() as *const c_char) };
        let result = find_env_value(name);
        assert!(result.is_none());
    });

    test!("test_find_env_value_empty_name" {
        let name = unsafe { CStr::from_ptr(b"\0".as_ptr() as *const c_char) };
        let result = find_env_value(name);
        assert!(result.is_none());
    });

    // ---- getenv 集成风格测试 ----

    test!("test_getenv_null_name_logically_empty" {
        let r = getenv(b"\0".as_ptr() as *const c_char) ;
        assert!(r.is_null());
    });

    test!("test_getenv_equals_in_name" {
        let r = getenv(b"FOO=BAR\0".as_ptr() as *const c_char);
        assert!(r.is_null());
    });

    test!("test_getenv_not_found_when_environ_null" {
        let r = getenv(b"ANYTHING\0".as_ptr() as *const c_char);
        assert!(r.is_null());
    });

    test!("test_getenv_with_test_env" {
        unsafe {
            let mut env = TestEnv::new(&["MYVAR=hello_world", "OTHER=value"]);
            crate::__environ::environ = env.as_mut_ptr();

            let r = getenv(b"MYVAR\0".as_ptr() as *const c_char);
            assert!(!r.is_null());
            let val = CStr::from_ptr(r);
            assert_eq!(val.to_bytes(), b"hello_world");

            let r2 = getenv(b"NOVAR\0".as_ptr() as *const c_char);
            assert!(r2.is_null());

            crate::__environ::environ = core::ptr::null_mut();
        }
    });

    test!("test_getenv_case_sensitive" {
        unsafe {
            let mut env = TestEnv::new(&["MyVar=upper"]);
            crate::__environ::environ = env.as_mut_ptr();

            let r1 = getenv(b"MyVar\0".as_ptr() as *const c_char);
            assert!(!r1.is_null());

            let r2 = getenv(b"myvar\0".as_ptr() as *const c_char);
            assert!(r2.is_null());

            let r3 = getenv(b"MYVAR\0".as_ptr() as *const c_char);
            assert!(r3.is_null());

            crate::__environ::environ = core::ptr::null_mut();
        }
    });
}
