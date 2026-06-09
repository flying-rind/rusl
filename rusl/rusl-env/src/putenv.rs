//! putenv — 进程环境变量设置 (POSIX.1-2001)
//!
//! 对应 musl `src/env/putenv.c`。
//!
//! ## 设计要点
//!
//! - `putenv` 将调用方提供的 `"NAME=VALUE"` 格式字符串**直接放入**进程环境
//!   （非拷贝），调用方不得在 `putenv` 返回后修改或释放该内存。
//! - `putenv_core` 是环境变量设置的核心实现，被 `putenv` 和 `setenv` 复用，
//!   负责管理 `__environ` 数组的插入、替换和扩容。
//! - `OLDENV` 静态变量追踪上一次由此模块分配的数组，用于判断下次插入时
//!   应使用 `realloc`（扩容自管数组）还是 `alloc`+`copy`（接管外部数组）。
//! - 通过 `RM_ADD_FN` 全局回调指针与 `setenv`/`unsetenv` 模块协同：
//!   `putenv` 路径传入 `r = null_mut()` 不注册新内存；
//!   `setenv` 路径传入 `r = s` 注册堆分配内存以便后续管理。

#![allow(dead_code, unused_imports)]

use core::ffi::{c_char, c_int};
use core::mem::transmute;
use core::ptr::null_mut;
use core::sync::atomic::Ordering;

use alloc::alloc::{alloc, dealloc, realloc, Layout};

use super::__environ::environ;
use super::__ENVIRON;
use crate::clearenv::{EnvRmAddFn, RM_ADD_FN};

// ---------------------------------------------------------------------------
// 跨模块依赖声明
// ---------------------------------------------------------------------------

// `strchrnul` — 在字符串中查找字符 `c`，若未找到返回指向终止 null 的指针。
//
// 由 `rusl_string::strchrnul` 模块提供。
// 对应 C 的 `char *__strchrnul(const char *s, int c)`。
use crate::import::strchrnul;

// `unsetenv` — 从进程环境中移除指定名称的环境变量。
//
// 由 `env::unsetenv` 模块提供。
// 对应 C 的 `int unsetenv(const char *name)`。
//
// 在测试构建中，本模块的 test stub 会提供该符号。
extern "C" {
    fn unsetenv(s: *mut c_char) -> c_int;
}

// ---------------------------------------------------------------------------
// OLDENV — 追踪上次分配的数组
// ---------------------------------------------------------------------------

/// 追踪上一次由 `putenv_core` 分配的 `__environ` 数组指针。
///
/// 对应 C 的 `static char **oldenv`。
///
/// # 不变量
///
/// - 初始值为 `null_mut()`。
/// - 每次通过分配器分配新数组后，`OLDENV` 始终等于 `__ENVIRON`，
///   确保下次插入时能正确识别为"自管理"数组而使用 `realloc` 扩容。
/// - 若 `__ENVIRON != OLDENV`，说明当前环境数组指向外部传入的内存
///   （如 `execve` 传入的 `envp`），需新分配一份并拷贝旧内容。
///
/// # Safety
///
/// 全局 `static mut`，无原子语义。并发写入是未定义行为，
/// 与 POSIX 关于 environ 的线程安全限制一致。
static mut OLDENV: *mut *mut c_char = null_mut();

// ---------------------------------------------------------------------------
// 内部辅助函数
// ---------------------------------------------------------------------------

/// 比较两个 `*const c_char` 的前 `n` 个字节是否完全相等。
///
/// 等价于 C 的 `strncmp(a, b, n) == 0`。
///
/// # Safety
///
/// - `a` 和 `b` 必须指向至少包含 `n` 个可读字节的内存区域。
#[inline]
unsafe fn nbytes_eq(a: *const c_char, b: *const c_char, n: usize) -> bool {
    for i in 0..n {
        if *a.add(i) != *b.add(i) {
            return false;
        }
    }
    true
}

/// 调用 `RM_ADD_FN` 回调，通知内存管理模块环境变量被替换或新增。
///
/// 对应 C 的 `__env_rm_add(old, new)`。
#[inline]
unsafe fn call_env_rm_add(old: *mut c_char, new: *mut c_char) {
    let ptr = RM_ADD_FN.load(Ordering::Acquire);
    if !ptr.is_null() {
        let callback: EnvRmAddFn = transmute(ptr);
        callback(old, new);
    }
}

/// 释放调用方传入的堆分配字符串 `r`（仅 OOM 路径使用）。
///
/// 计算 `r` 的 C 字符串长度以构造 `Layout`。由于本项目的全局分配器
/// (`RuslAlloc`) 的 `dealloc` 方法会忽略 layout 参数并直接调用 `free()`，
/// 此处构造的 layout 仅需满足 API 契约的最小要求。
unsafe fn free_r(r: *mut c_char) {
    // 计算 r 的 C 字符串长度
    let mut len = 0usize;
    while *r.add(len) != 0 {
        len += 1;
    }
    // len + 1 包含终止 null 字节
    let layout = Layout::from_size_align(len + 1, 1).unwrap();
    dealloc(r as *mut u8, layout);
}

// ---------------------------------------------------------------------------
// putenv_core — 内部核心函数
// ---------------------------------------------------------------------------

/// 环境变量设置核心实现。
///
/// 将字符串 `s` 插入进程的 `__environ` 数组。若 `s` 对应的变量已存在则
/// 原地替换，否则在必要时扩增 `__environ` 数组的容量。
///
/// # 参数
///
/// | 参数 | 类型                     | 含义 |
/// |------|--------------------------|------|
/// | `s`  | `*mut c_char`            | 指向 `"NAME=VALUE"` 格式的字符串，将被直接放入环境数组（非拷贝） |
/// | `l`  | `usize`                  | 环境变量名的长度（不含 `=`），即 `=` 在 `s` 中的偏移量 |
/// | `r`  | `*mut c_char`            | 调用方在堆上分配的字符串；`putenv` 路径传入 `null_mut()` |
///
/// # 返回值
///
/// - `0`: 成功（插入或替换）
/// - `-1`: OOM 失败（原环境不变，若 `r` 非空则已释放）
///
/// # Safety
///
/// - `s` 必须指向有效的 `"NAME=VALUE"` 格式 C 字符串，且 `*s.add(l) == b'='`。
/// - `l` 必须 `> 0`（变量名非空）。
/// - `s` 指向的内存生命期不短于其在环境数组中的存留时间。
pub(crate) unsafe fn putenv_core(
    s: *mut c_char,
    l: usize,
    r: *mut c_char,
) -> c_int {
    let mut i: usize = 0;

    // Step 1: 遍历现有环境数组，查找同名变量
    let env_ptr = __ENVIRON.load(Ordering::Acquire);
    if !env_ptr.is_null() {
        let mut e = env_ptr;
        loop {
            let entry = *e;
            if entry.is_null() {
                break;
            }
            // 比较前 l+1 个字节（变量名 + '='）
            if nbytes_eq(s as *const c_char, entry as *const c_char, l + 1) {
                // 找到同名变量，原地替换
                let tmp = entry;
                *e = s;
                call_env_rm_add(tmp, r);
                return 0;
            }
            e = e.add(1);
            i += 1;
        }
    }

    // Step 2: 变量不存在，需要插入
    //   i = 现有变量数量
    let oldenv = OLDENV;
    let newenv: *mut *mut c_char;

    // Step 2a: 判断 realloc 还是 alloc+copy
    if env_ptr == oldenv && !oldenv.is_null() {
        // 当前数组由此模块管理，使用 realloc 扩容
        // oldenv 指向大小为 (i+1) 个指针的数组，新大小为 (i+2)
        let old_layout = Layout::array::<*mut c_char>(i + 1).unwrap();
        let new_byte_size = (i + 2) * core::mem::size_of::<*mut c_char>();
        let new_ptr = realloc(oldenv as *mut u8, old_layout, new_byte_size);
        if new_ptr.is_null() {
            // OOM: 释放调用方传入的 r（若存在）
            if !r.is_null() {
                free_r(r);
            }
            return -1;
        }
        newenv = new_ptr as *mut *mut c_char;
    } else {
        // 需要全新分配（接管外部数组或首次分配）
        let new_layout = Layout::array::<*mut c_char>(i + 2).unwrap();
        let new_ptr = alloc(new_layout);
        if new_ptr.is_null() {
            // OOM: 释放调用方传入的 r（若存在）
            if !r.is_null() {
                free_r(r);
            }
            return -1;
        }
        newenv = new_ptr as *mut *mut c_char;

        // 复制现有条目到新数组
        if i > 0 && !env_ptr.is_null() {
            core::ptr::copy_nonoverlapping(env_ptr, newenv, i);
        }

        // 释放旧的自管数组（若存在）
        if !oldenv.is_null() {
            let old_layout = Layout::array::<*mut c_char>(i + 1).unwrap();
            dealloc(oldenv as *mut u8, old_layout);
        }
    }

    // Step 2b: 设置新条目和终止哨兵
    *newenv.add(i) = s;
    *newenv.add(i + 1) = null_mut();

    // Step 2c: 更新全局指针
    __ENVIRON.store(newenv, Ordering::Release);
    // SAFETY: 单线程环境操作，与 POSIX 一致
    environ = newenv;
    OLDENV = newenv;

    // Step 2d: 若调用方传入了堆分配字符串，通知内存管理模块注册
    if !r.is_null() {
        call_env_rm_add(null_mut(), r);
    }

    0
}

// ---------------------------------------------------------------------------
// putenv — 对外导出（POSIX 标准）
// ---------------------------------------------------------------------------

/// POSIX putenv — 将 `"NAME=VALUE"` 格式的字符串直接放入进程环境。
///
/// 声明于 `<stdlib.h>`（POSIX.1-2001, SVr4, 4.3BSD）。
///
/// **重要**: 此函数不拷贝 `s` 指向的内存，而是直接将该指针存入环境数组。
/// 调用方必须保证 `s` 指向的内存在整个环境存续期间有效且不被修改。
///
/// # 算法
///
/// 1. 调用 `strchrnul(s, '=')` 查找 `=` 的位置，计算 `l = 偏移量`
/// 2. 若 `l == 0`（空变量名）或 `s[l] == '\0'`（无 `=`），
///    委托给 `unsetenv(s)` 处理（视为移除同名环境变量）
/// 3. 否则调用 `putenv_core(s, l, null_mut())` 执行实际插入/替换
///
/// # 返回值
///
/// - `0`: 成功
/// - `-1`: OOM 失败（原环境不变）
///
/// # Safety
///
/// - `s` 必须是有效的、以 `\0` 结尾的 C 字符串指针。
/// - 调用方在 `putenv` 返回后不得修改或释放 `s` 指向的内存。
///
/// # 与 setenv 的区别
///
/// - `putenv`: 不拷贝字符串，不拥有内存所有权。`r = null_mut()`。
/// - `setenv`: 在堆上构造 `"NAME=VALUE"` 字符串，通过 `putenv_core`
///   传入 `r = s` 并注册到内存管理模块。
#[no_mangle]
pub extern "C" fn putenv(s: *mut c_char) -> c_int {
    // SAFETY: caller guarantees s is a valid null-terminated C string per C ABI contract.
    unsafe {
        // Step 1: 查找 '=' 的位置
        let eq = strchrnul(s as *const c_char, b'=' as c_int);
        let l = (eq as usize).wrapping_sub(s as usize);

        // Step 2: 检查有效性 — 空变量名或无 '='
        if l == 0 || *s.add(l) == 0 {
            // 不含 '=' 或变量名为空：委托给 unsetenv
            return unsetenv(s);
        }

        // Step 3: 委托核心逻辑
        // r = null_mut() 表示 putenv 未在堆上分配字符串，无需 ENV_RM_ADD 注册
        putenv_core(s, l, null_mut())
    }
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use rusl_core::test;
    use super::*;
    use alloc::vec::Vec;
    use core::ffi::c_char;
    use core::ptr;
    use core::sync::atomic::Ordering;

    // ========================================================================
    // 测试辅助函数
    // ========================================================================

    // 在每个测试前重置全局状态，避免测试间残留相互干扰。
    unsafe fn reset_state() {
        __ENVIRON.store(ptr::null_mut(), Ordering::Release);
        environ = ptr::null_mut();
        OLDENV = ptr::null_mut();
        // 重置回调为默认 no_op
        RM_ADD_FN.store(
            crate::clearenv::no_op as *mut EnvRmAddFn,
            Ordering::Release,
        );
    }

    // 在栈上构造一个 C 字符串，返回其可变指针。
    // 传入的字节序列不要求含终止 null，由本函数自动追加。
    fn stack_cstr<const N: usize>(bytes: &[u8]) -> [c_char; N] {
        let mut buf: [c_char; N] = [0; N];
        for (i, &b) in bytes.iter().enumerate() {
            buf[i] = b as c_char;
        }
        buf
    }

    // 计算 `strchrnul(s, c)` 返回位置相对于 `s` 的偏移量。
    unsafe fn strchrnul_offset(s: *mut c_char, c: c_int) -> usize {
        let eq = strchrnul(s as *const c_char, c);
        (eq as usize).wrapping_sub(s as usize)
    }

    // 从 `__ENVIRON` 加载当前环境数组，收集所有条目指针到 Vec。
    unsafe fn collect_environ() -> Vec<*mut c_char> {
        let mut result = Vec::new();
        let env_ptr = __ENVIRON.load(Ordering::Acquire);
        if !env_ptr.is_null() {
            let mut i = 0usize;
            loop {
                let entry = *env_ptr.add(i);
                if entry.is_null() {
                    break;
                }
                result.push(entry);
                i += 1;
            }
        }
        result
    }

    // 测试用回调: 记录被调用的 old/new 参数。
    static mut CB_OLD: *mut c_char = ptr::null_mut();
    static mut CB_NEW: *mut c_char = ptr::null_mut();
    static mut CB_COUNT: usize = 0;

    unsafe extern "C" fn test_callback(old: *mut c_char, new: *mut c_char) {
        CB_OLD = old;
        CB_NEW = new;
        CB_COUNT += 1;
    }

    unsafe fn reset_cb_state() {
        CB_OLD = ptr::null_mut();
        CB_NEW = ptr::null_mut();
        CB_COUNT = 0;
    }

    // ========================================================================
    // nbytes_eq 测试
    // ========================================================================

    test!("test_nbytes_eq_identical" {
        let s1 = stack_cstr::<8>(b"HELLO=\0");
        let s2 = stack_cstr::<8>(b"HELLO=\0");
        unsafe {
            assert!(nbytes_eq(s1.as_ptr(), s2.as_ptr(), 6));
        }
    });

    test!("test_nbytes_eq_different" {
        let s1 = stack_cstr::<8>(b"HELLO=\0");
        let s2 = stack_cstr::<8>(b"HELLX=\0");
        unsafe {
            assert!(!nbytes_eq(s1.as_ptr(), s2.as_ptr(), 6));
        }
    });

    test!("test_nbytes_eq_differ_after_n" {
        let s1 = stack_cstr::<8>(b"HELLO=A\0");
        let s2 = stack_cstr::<8>(b"HELLO=B\0");
        unsafe {
            // 前 6 个字节 (HELLO=) 相同
            assert!(nbytes_eq(s1.as_ptr(), s2.as_ptr(), 6));
            // 前 7 个字节不同
            assert!(!nbytes_eq(s1.as_ptr(), s2.as_ptr(), 7));
        }
    });

    test!("test_nbytes_eq_zero" {
        let s1 = stack_cstr::<4>(b"ABC\0");
        let s2 = stack_cstr::<4>(b"XYZ\0");
        unsafe {
            // n = 0 时应始终返回 true
            assert!(nbytes_eq(s1.as_ptr(), s2.as_ptr(), 0));
        }
    });

    // ========================================================================
    // strchrnul_offset 测试
    // ========================================================================

    test!("test_strchrnul_finds_equals" {
        let mut s = stack_cstr::<10>(b"HOME=/tmp\0");
        let off = unsafe { strchrnul_offset(s.as_mut_ptr(), b'=' as c_int) };
        assert_eq!(off, 4); // "HOME" 长度 4
    });

    test!("test_strchrnul_no_equals" {
        let mut s = stack_cstr::<10>(b"NOVALUE\0\0\0");
        let off = unsafe { strchrnul_offset(s.as_mut_ptr(), b'=' as c_int) };
        assert_eq!(off, 7); // 指向终止 null，即完整长度
    });

    test!("test_strchrnul_empty_string" {
        let s: [c_char; 1] = [0];
        let off = unsafe { strchrnul_offset(s.as_ptr() as *mut c_char, b'=' as c_int) };
        assert_eq!(off, 0); // 空字符串，终止 null 在位置 0
    });

    // ========================================================================
    // putenv_core 测试 — 插入到空环境
    // ========================================================================

    // 首个变量插入：`__ENVIRON` 为 null，`OLDENV` 为 null。
    // 应触发 alloc 路径。
    test!("test_putenv_core_first_insert" {
        unsafe {
            reset_state();
            let mut s = stack_cstr::<10>(b"HOME=/tmp\0");
            let s_ptr = s.as_mut_ptr();

            let ret = putenv_core(s_ptr, 4, ptr::null_mut());
            assert_eq!(ret, 0, "首次插入应返回 0");

            // 验证环境数组状态
            let entries = collect_environ();
            assert_eq!(entries.len(), 1, "应有 1 个条目");
            assert_eq!(entries[0], s_ptr, "条目应指向传入的 s");

            // 验证 OLDENV 追踪
            let env = __ENVIRON.load(Ordering::Acquire);
            assert!(!env.is_null());
            assert_eq!(env, OLDENV, "首次分配后 OLDENV 应等于 __ENVIRON");
            assert_eq!(env, environ, "environ 应与 __ENVIRON 同步");

            // 清理: 释放自管数组
            dealloc(
                env as *mut u8,
                Layout::array::<*mut c_char>(2).unwrap(),
            );
            reset_state();
        }
    });

    // 第二个不同变量的插入：应触发 realloc 路径（`__ENVIRON == OLDENV`）。
    test!("test_putenv_core_second_insert_uses_realloc" {
        unsafe {
            reset_state();

            // 插入第一个变量
            let mut s1 = stack_cstr::<10>(b"HOME=/tmp\0");
            let r1 = putenv_core(s1.as_mut_ptr(), 4, ptr::null_mut());
            assert_eq!(r1, 0);

            // 插入第二个不同变量
            let mut s2 = stack_cstr::<12>(b"PATH=/bin\0\0\0");
            let r2 = putenv_core(s2.as_mut_ptr(), 4, ptr::null_mut());
            assert_eq!(r2, 0);

            let entries = collect_environ();
            assert_eq!(entries.len(), 2, "应有 2 个条目");
            assert_eq!(entries[0], s1.as_mut_ptr());
            assert_eq!(entries[1], s2.as_mut_ptr());

            // OLDENV 应仍等于 __ENVIRON
            assert_eq!(OLDENV, __ENVIRON.load(Ordering::Acquire));

            // 清理
            dealloc(
                OLDENV as *mut u8,
                Layout::array::<*mut c_char>(3).unwrap(),
            );
            reset_state();
        }
    });

    // 替换已存在的变量：原地替换，不改变数组大小。
    test!("test_putenv_core_replace_existing" {
        unsafe {
            reset_state();

            // 插入初始变量
            let mut s1 = stack_cstr::<12>(b"HOME=/old\0\0\0");
            let r1 = putenv_core(s1.as_mut_ptr(), 4, ptr::null_mut());
            assert_eq!(r1, 0);

            // 用新值替换同一变量
            let mut s2 = stack_cstr::<12>(b"HOME=/new\0\0\0");
            let r2 = putenv_core(s2.as_mut_ptr(), 4, ptr::null_mut());
            assert_eq!(r2, 0);

            let entries = collect_environ();
            assert_eq!(entries.len(), 1, "替换后仍应为 1 个条目");
            assert_eq!(entries[0], s2.as_mut_ptr(), "应指向新值 s2");

            // 清理
            dealloc(
                OLDENV as *mut u8,
                Layout::array::<*mut c_char>(2).unwrap(),
            );
            reset_state();
        }
    });

    // 替换已存在变量时触发回调 (old, r)。
    test!("test_putenv_core_replace_calls_callback" {
        unsafe {
            reset_state();
            reset_cb_state();

            // 注册测试回调
            RM_ADD_FN.store(
                test_callback as *mut EnvRmAddFn,
                Ordering::Release,
            );

            let mut s1 = stack_cstr::<12>(b"FOO=old\0\0\0\0");
            let r1 = putenv_core(s1.as_mut_ptr(), 3, ptr::null_mut());
            assert_eq!(r1, 0);

            let mut s2 = stack_cstr::<12>(b"FOO=new\0\0\0\0");
            let r2 = putenv_core(s2.as_mut_ptr(), 3, ptr::null_mut());
            assert_eq!(r2, 0);

            // 回调应被调用: old=s1, new=null (putenv 路径)
            assert_eq!(CB_COUNT, 1, "替换时应触发回调 1 次");
            assert_eq!(CB_OLD, s1.as_mut_ptr(), "回调 old 应为旧值 s1");
            assert!(CB_NEW.is_null(), "putenv 路径 new 应为 null");

            // 清理
            dealloc(
                OLDENV as *mut u8,
                Layout::array::<*mut c_char>(2).unwrap(),
            );
            reset_state();
        }
    });

    // 插入新变量且 `r != null_mut()` 时触发回调 (null, r)。
    test!("test_putenv_core_insert_calls_callback_with_r" {
        unsafe {
            reset_state();
            reset_cb_state();

            // 注册测试回调
            RM_ADD_FN.store(
                test_callback as *mut EnvRmAddFn,
                Ordering::Release,
            );

            let mut s1 = stack_cstr::<12>(b"VAR=value\0\0\0");
            // 模拟 setenv 路径: r = s1
            let ret = putenv_core(s1.as_mut_ptr(), 3, s1.as_mut_ptr());
            assert_eq!(ret, 0);

            // 回调应被调用: old=null, new=s1
            assert_eq!(CB_COUNT, 1, "插入 r 非空时应触发回调");
            assert!(CB_OLD.is_null(), "插入时 old 应为 null");
            assert_eq!(CB_NEW, s1.as_mut_ptr(), "new 应为 r=s1");

            // 清理
            dealloc(
                OLDENV as *mut u8,
                Layout::array::<*mut c_char>(2).unwrap(),
            );
            reset_state();
        }
    });

    // ========================================================================
    // putenv_core 测试 — OLDENV 追踪机制
    // ========================================================================

    // 测试接管外部数组场景：构造一个外部环境数组，通过 `__ENVIRON` 指向它。
    // `putenv_core` 应识别到 `__ENVIRON != OLDENV`，触发 alloc+copy 路径。
    test!("test_putenv_core_external_env_array" {
        unsafe {
            reset_state();

            // 构造一个"外部"环境数组（模拟 execve 传入）
            let mut ext_s1 = stack_cstr::<12>(b"EXT=one\0\0\0\0\0");
            let mut ext_array: [*mut c_char; 2] = [
                ext_s1.as_mut_ptr(),
                ptr::null_mut(),
            ];
            let ext_ptr = ext_array.as_mut_ptr();

            // 将 __ENVIRON 指向外部数组
            __ENVIRON.store(ext_ptr, Ordering::Release);
            environ = ext_ptr;
            // 注意: OLDENV 仍为 null (初始值)

            // 插入新变量: 应触发 alloc 路径（非 realloc）
            let mut s2 = stack_cstr::<10>(b"NEW=two\0\0");
            let ret = putenv_core(s2.as_mut_ptr(), 3, ptr::null_mut());
            assert_eq!(ret, 0);

            // 验证新数组包含两个条目
            let entries = collect_environ();
            assert_eq!(entries.len(), 2);

            // 验证 OLDENV 指向新分配的数组（不等于外部数组 ext_ptr）
            let new_env = __ENVIRON.load(Ordering::Acquire);
            assert!(!new_env.is_null());
            assert_ne!(new_env, ext_ptr, "新数组应不同于外部数组");
            assert_eq!(new_env, OLDENV, "OLDENV 应指向新数组");
            assert_eq!(new_env, environ);

            // 清理新分配的数组
            dealloc(
                new_env as *mut u8,
                Layout::array::<*mut c_char>(3).unwrap(),
            );
            reset_state();
        }
    });

    // ========================================================================
    // putenv_core 测试 — OOM 路径
    // ========================================================================

    // 验证 `free_r` 辅助函数对有效堆指针无 panic。
    // （真实 OOM 路径由集成测试覆盖）
    test!("test_free_r_valid_pointer" {
        unsafe {
            reset_state();

            let layout = Layout::from_size_align(10, 1).unwrap();
            let ptr = alloc(layout) as *mut c_char;
            assert!(!ptr.is_null());

            *ptr.add(0) = b'H' as c_char;
            *ptr.add(1) = b'I' as c_char;
            *ptr.add(2) = 0; // null terminator, len = 2

            free_r(ptr);
            // 若无 panic 则成功

            reset_state();
        }
    });

    // ========================================================================
    // putenv 测试 — 正常路径
    // ========================================================================

    // putenv 设置新环境变量。
    test!("test_putenv_set_new_variable" {
        unsafe {
            reset_state();

            let mut s = stack_cstr::<16>(b"LANG=C.UTF-8\0\0\0\0");
            let ret = putenv(s.as_mut_ptr());
            assert_eq!(ret, 0);

            let entries = collect_environ();
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0], s.as_mut_ptr());

            // 清理
            dealloc(
                OLDENV as *mut u8,
                Layout::array::<*mut c_char>(2).unwrap(),
            );
            reset_state();
        }
    });

    // putenv 替换已有环境变量。
    test!("test_putenv_replace_variable" {
        unsafe {
            reset_state();

            let mut s1 = stack_cstr::<16>(b"TERM=xterm\0\0\0\0\0\0");
            let r1 = putenv(s1.as_mut_ptr());
            assert_eq!(r1, 0);

            let mut s2 = stack_cstr::<16>(b"TERM=linux\0\0\0\0\0\0");
            let r2 = putenv(s2.as_mut_ptr());
            assert_eq!(r2, 0);

            let entries = collect_environ();
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0], s2.as_mut_ptr(), "应指向新值");

            // 清理
            dealloc(
                OLDENV as *mut u8,
                Layout::array::<*mut c_char>(2).unwrap(),
            );
            reset_state();
        }
    });

    // putenv 设置多个不同环境变量。
    test!("test_putenv_multiple_variables" {
        unsafe {
            reset_state();

            let mut s1 = stack_cstr::<12>(b"A=1\0\0\0\0\0\0\0\0");
            let mut s2 = stack_cstr::<12>(b"B=2\0\0\0\0\0\0\0\0");
            let mut s3 = stack_cstr::<12>(b"C=3\0\0\0\0\0\0\0\0");

            assert_eq!(putenv(s1.as_mut_ptr()), 0);
            assert_eq!(putenv(s2.as_mut_ptr()), 0);
            assert_eq!(putenv(s3.as_mut_ptr()), 0);

            let entries = collect_environ();
            assert_eq!(entries.len(), 3);
            assert_eq!(entries[0], s1.as_mut_ptr());
            assert_eq!(entries[1], s2.as_mut_ptr());
            assert_eq!(entries[2], s3.as_mut_ptr());

            // 清理
            dealloc(
                OLDENV as *mut u8,
                Layout::array::<*mut c_char>(4).unwrap(),
            );
            reset_state();
        }
    });

    // putenv 含 '=' 但变量名为空（`=VALUE`）委托给 unsetenv。
    // 注意: 此测试依赖 `env::unsetenv` 模块提供的真实实现。
    test!("test_putenv_empty_name_delegates_to_unsetenv" {
        unsafe {
            reset_state();

            let mut s = stack_cstr::<10>(b"=VALUE\0\0\0\0");
            // l == 0 (空变量名), putenv 委托给 unsetenv
            let ret = putenv(s.as_mut_ptr());
            // unsetenv 拒绝含 '=' 的无效变量名，返回 -1 + EINVAL
            assert_eq!(ret, -1);

            reset_state();
        }
    });

    // putenv 不含 '=' 的字符串委托给 unsetenv。
    // 注意: 此测试依赖 `env::unsetenv` 模块提供的真实实现。
    test!("test_putenv_no_equals_delegates_to_unsetenv" {
        unsafe {
            reset_state();

            let mut s = stack_cstr::<10>(b"REMOVEME\0\0");
            // 不含 '=', putenv 委托给 unsetenv
            let ret = putenv(s.as_mut_ptr());
            // unsetenv 由 env::unsetenv 模块提供
            assert_eq!(ret, 0);

            reset_state();
        }
    });

    // ========================================================================
    // putenv_core 测试 — 边界情况
    // ========================================================================

    // 插入变量名仅 1 个字符长的变量。
    test!("test_putenv_core_single_char_name" {
        unsafe {
            reset_state();

            let mut s = stack_cstr::<9>(b"X=1\0\0\0\0\0\0");
            let ret = putenv_core(s.as_mut_ptr(), 1, ptr::null_mut());
            assert_eq!(ret, 0);

            let entries = collect_environ();
            assert_eq!(entries.len(), 1);

            dealloc(
                OLDENV as *mut u8,
                Layout::array::<*mut c_char>(2).unwrap(),
            );
            reset_state();
        }
    });

    // 替换变量时变量名仅部分匹配不应误判。
    test!("test_putenv_core_partial_name_no_match" {
        unsafe {
            reset_state();

            // 插入 "LONGNAME=1"
            let mut s1 = stack_cstr::<16>(b"LONGNAME=1\0\0\0\0\0\0");
            let r1 = putenv_core(s1.as_mut_ptr(), 8, ptr::null_mut());
            assert_eq!(r1, 0);

            // 插入 "LONG=2" — 不同变量
            let mut s2 = stack_cstr::<16>(b"LONG=2\0\0\0\0\0\0\0\0\0");
            let r2 = putenv_core(s2.as_mut_ptr(), 4, ptr::null_mut());
            assert_eq!(r2, 0);

            let entries = collect_environ();
            assert_eq!(entries.len(), 2, "应有两个不同变量");

            dealloc(
                OLDENV as *mut u8,
                Layout::array::<*mut c_char>(3).unwrap(),
            );
            reset_state();
        }
    });

    // 插入后 __ENVIRON 和 environ 保持一致。
    test!("test_putenv_core_environ_sync" {
        unsafe {
            reset_state();

            let mut s = stack_cstr::<12>(b"KEY=val\0\0\0\0\0");
            let ret = putenv_core(s.as_mut_ptr(), 3, ptr::null_mut());
            assert_eq!(ret, 0);

            let atomic_env = __ENVIRON.load(Ordering::Acquire);
            assert!(!atomic_env.is_null());
            assert_eq!(atomic_env, environ,
                "__ENVIRON 和 environ 应指向同一数组");

            dealloc(
                OLDENV as *mut u8,
                Layout::array::<*mut c_char>(2).unwrap(),
            );
            reset_state();
        }
    });

    // ========================================================================
    // OLDENV 静态变量测试
    // ========================================================================

    // OLDENV 初始值为 null。
    test!("test_oldenv_initial_null" {
        unsafe {
            assert!(OLDENV.is_null(), "OLDENV 初始值应为 null");
        }
    });

    // OLDENV 在首次插入后被正确设置。
    test!("test_oldenv_set_after_first_insert" {
        unsafe {
            reset_state();
            assert!(OLDENV.is_null());

            let mut s = stack_cstr::<8>(b"K=V\0\0\0\0\0");
            let ret = putenv_core(s.as_mut_ptr(), 1, ptr::null_mut());
            assert_eq!(ret, 0);
            assert!(!OLDENV.is_null(), "首次插入后 OLDENV 不应为 null");
            assert_eq!(OLDENV, __ENVIRON.load(Ordering::Acquire));

            dealloc(
                OLDENV as *mut u8,
                Layout::array::<*mut c_char>(2).unwrap(),
            );
            reset_state();
        }
    });

    // ========================================================================
    // call_env_rm_add 测试
    // ========================================================================

    // 默认 no_op 回调不执行任何操作。
    test!("test_call_env_rm_add_default_no_op" {
        unsafe {
            reset_state();
            reset_cb_state();

            call_env_rm_add(0x1000usize as *mut c_char, 0x2000usize as *mut c_char);
            // 不应有副作用，不应 panic
            assert_eq!(CB_COUNT, 0, "默认 no_op 不应调用测试回调");

            reset_state();
        }
    });

    // 注册自定义回调后 call_env_rm_add 调用它。
    test!("test_call_env_rm_add_registered_callback" {
        unsafe {
            reset_state();
            reset_cb_state();

            RM_ADD_FN.store(
                test_callback as *mut EnvRmAddFn,
                Ordering::Release,
            );

            let old_ptr = 0x3000usize as *mut c_char;
            let new_ptr = 0x4000usize as *mut c_char;
            call_env_rm_add(old_ptr, new_ptr);

            assert_eq!(CB_COUNT, 1);
            assert_eq!(CB_OLD, old_ptr);
            assert_eq!(CB_NEW, new_ptr);

            reset_state();
        }
    });
}
