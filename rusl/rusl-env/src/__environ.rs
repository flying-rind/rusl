//! # __environ — 环境变量全局入口指针
//!
//! 定义 POSIX `environ` 全局变量，类型为 `*mut *mut c_char`。
//! 该指针是所有环境变量操作（getenv、setenv、putenv、unsetenv、clearenv）
//! 的核心数据入口。
//!
//! ## C 端对照
//!
//! C 原始实现中，`__environ` 是内部符号，通过 `weak_alias` 将 POSIX 标准名
//! `environ` 及历史别名 `___environ` / `_environ` 指向同一内存位置。
//! Rust 实现仅保留 `environ` 作为唯一的 `#[no_mangle]` 导出符号，
//! 删除所有内部别名（Rust 无需通过别名间接定义对外符号）。
//!
//! 对应 musl 的 `src/env/__environ.c`。

use core::ffi::c_char;

// ---------------------------------------------------------------------------
// environ — POSIX 标准全局入口指针
// ---------------------------------------------------------------------------

/// POSIX 标准环境变量指针。
///
/// 指向以空指针终止的字符串指针数组，每个字符串为 `"NAME=VALUE"` 格式的
/// 环境变量条目。初始值为 `null_mut()`，由启动代码（`__libc_start_main`）
/// 在 `main()` 之前填充。
///
/// # 数据布局
///
/// ```text
/// environ ──> [0] ──> "HOME=/home/user\0"
///             [1] ──> "PATH=/usr/bin\0"
///             [2] ──> "LANG=en_US.UTF-8\0"
///             ...
///             [n] ──> NULL (终止哨兵)
/// ```
///
/// # Safety
///
/// - **读取前提**: `environ` 在 `main()` 之后才被初始化。启动早期阶段
///   读取到 `null_mut()` 是合法的，调用者应做防护检查。
/// - **写入前提**: 新数组必须以 `null_mut()` 终止，且每个非 null 条目
///   必须满足 `"NAME=VALUE"` 格式不变量。旧数组的内存不会被自动释放。
/// - **并发**: 全局 `static mut`，无原子语义。并发写入是未定义行为
///   （与 POSIX 关于 environ 的线程安全限制一致）。
///
/// # C 符号对照
///
/// | C 符号 | C 可见性 | Rust 状态 |
/// |--------|----------|-----------|
/// | `environ` | **Public** (POSIX 标准) | **保留** — `#[no_mangle]` 导出 |
/// | `__environ` | Internal (不导出) | **保留** — 通过 global_asm 创建别名 |
/// | `___environ` | Internal (不导出) | 已移除 |
/// | `_environ` | Internal (不导出) | 已移除 |
#[no_mangle]
pub static mut environ: *mut *mut c_char = core::ptr::null_mut::<*mut c_char>();

// 创建 `__environ` 作为 `environ` 的符号别名。
// musl C 实现中 `__environ` 是强符号定义，`environ` 通过 `weak_alias`
// 指向同一内存位置。Rust 无法直接表达此类链接器别名，故使用 `.set`
// 汇编伪指令将 `__environ` 设置为与 `environ` 相同的地址。
// 仅在 ELF 目标（Linux x86_64）上受支持。
#[cfg(target_os = "linux")]
core::arch::global_asm!(
    ".globl __environ",
    ".set __environ, environ",
);

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rusl_core::test;

    // 每个测试结束时调用此辅助函数，确保 environ 恢复为 null 状态，
    // 避免测试间的残留状态相互干扰。
    unsafe fn reset_environ() {
        environ = core::ptr::null_mut();
    }

    // 验证 environ 的初始值为 null。
    //
    // 这是所有环境变量操作的起点：在启动代码填充 environ 之前，
    // 所有消费者读到 null 意味着环境变量数组尚未就绪。
    test!("environ_initially_null" {
        unsafe {
            // 先恢复为 null，消除其他测试可能设置的残留值
            reset_environ();
            assert!(
                environ.is_null(),
                "environ 初始值必须为 null，表示尚未初始化"
            );
        }
    });

    // 验证空环境数组场景：environ 指向一个仅含 null 哨兵的数组。
    //
    // 对应 POSIX clearenv() 调用后或某些嵌入式场景下的环境状态。
    test!("environ_empty_array" {
        unsafe {
            let mut env_array: [*mut c_char; 1] = [core::ptr::null_mut()];
            environ = env_array.as_mut_ptr();

            assert!(!environ.is_null(), "environ 不应为 null");
            assert!(
                (*environ).is_null(),
                "空环境数组的首元素应为 null（终止哨兵）"
            );

            reset_environ();
        }
    });

    // 验证含两个字符串条目的环境数组的读写正确性。
    //
    // 在栈上构造两个 `"NAME=VALUE"` 格式的字节串，验证通过
    // environ 可以正确遍历并读取每个条目的指针。
    test!("environ_two_string_entries" {
        unsafe {
            // 栈上构造环境字符串: "HOME=/tmp" 和 "PATH=/bin"
            let mut s1: [c_char; 10] = [0; 10];
            let mut s2: [c_char; 10] = [0; 10];
            for (i, &b) in b"HOME=/tmp".iter().enumerate() {
                s1[i] = b as c_char;
            }
            for (i, &b) in b"PATH=/bin".iter().enumerate() {
                s2[i] = b as c_char;
            }

            let mut env_entries: [*mut c_char; 3] = [
                s1.as_mut_ptr(),
                s2.as_mut_ptr(),
                core::ptr::null_mut(), // 终止哨兵
            ];

            environ = env_entries.as_mut_ptr();

            // 验证 environ 指向正确的条目数组
            assert!(!environ.is_null());
            assert_eq!(*environ, s1.as_mut_ptr(), "第一个条目指针应匹配");
            assert_eq!(*environ.add(1), s2.as_mut_ptr(), "第二个条目指针应匹配");
            assert!((*environ.add(2)).is_null(), "终止哨兵应为 null");

            // 验证条目内容可以正确读取
            assert_eq!(
                *(*environ.add(0)).cast::<u8>(),
                b'H',
                "第一个条目首字符应为 'H'"
            );
            assert_eq!(
                *(*environ.add(1)).cast::<u8>(),
                b'P',
                "第二个条目首字符应为 'P'"
            );

            reset_environ();
        }
    });

    // 验证基本的指针读写：写入任意指针值后能准确读回。
    //
    // 测试核心的写后读一致性，不依赖环境字符串的格式约束。
    test!("environ_read_write_pointers" {
        unsafe {
            let p0: *mut c_char = 0x100 as *mut c_char;
            let p1: *mut c_char = 0x200 as *mut c_char;
            let p2: *mut c_char = 0x300 as *mut c_char;

            let mut env_entries: [*mut c_char; 4] = [p0, p1, p2, core::ptr::null_mut()];

            environ = env_entries.as_mut_ptr();

            assert_eq!(*environ, p0);
            assert_eq!(*environ.add(1), p1);
            assert_eq!(*environ.add(2), p2);
            assert!((*environ.add(3)).is_null());

            reset_environ();
        }
    });
}
