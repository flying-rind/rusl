//! env 模块 — 环境变量与运行时初始化支持。
//!
//! 对应 musl `src/env/` 目录。
//!
//! # 子模块
//!
//! | 模块 | 说明 |
//! |------|------|
//! | `__environ` | POSIX environ 全局变量 — 环境变量数组入口指针 |
//! | `clearenv` | GNU clearenv — 清除所有环境变量 |
//! | `getenv` | POSIX getenv — 在环境变量列表中查找值 |
//! | `__stack_chk_fail` | GCC/Clang 栈保护器 (SSP) 运行时支持 |
//! | `secure_getenv` | GNU secure_getenv — 安全模式下的环境变量访问 |
//! | `putenv` | POSIX putenv — 将"NAME=VALUE"格式字符串放入进程环境 |
//! | `setenv` | POSIX setenv — 分配并设置环境变量（拷贝语义） |
//! | `unsetenv` | POSIX unsetenv — 从环境变量列表中移除指定变量 |
//! | `__reset_tls` | musl 内部 — fork() 后/信号线程 TLS 重置 |
//! | `__libc_start_main` | musl 内部 — CRT 启动入口, 包含 `_start` → `__libc_start_main` → `__init_libc` |
//! | `exit` | POSIX exit / _Exit / _exit — 进程终止 |

#![allow(dead_code, unused_imports)]

use core::ffi::c_char;
use core::sync::atomic::AtomicPtr;

// ---------------------------------------------------------------------------
// 模块内部共享全局变量
// ---------------------------------------------------------------------------

/// 内部环境变量数组指针（原子访问）。
///
/// 替代 C 的 `extern char **__environ` 裸指针。所有内部环境操作
/// (clearenv, getenv, setenv 等) 均通过此 `AtomicPtr` 读写环境数组，
/// 使用 Acquire/Release 内存顺序保证跨线程可见性。
///
/// 外部 C ABI 兼容的 `environ` 符号由 `__environ` 模块提供，
/// 启动代码需同时初始化两者。
pub(crate) static __ENVIRON: AtomicPtr<*mut c_char> =
    AtomicPtr::new(core::ptr::null_mut());

// ---------------------------------------------------------------------------
// 子模块声明
// ---------------------------------------------------------------------------

pub mod __environ;
pub(crate) mod __init_tls;
pub(crate) mod __reset_tls;
pub mod clearenv;
pub mod getenv;
pub mod __stack_chk_fail;
pub mod secure_getenv;
pub mod unsetenv;
pub mod putenv;
pub mod setenv;
pub mod __libc_start_main;

pub use clearenv::clearenv;
pub use getenv::getenv;
pub use putenv::putenv;
pub use setenv::setenv;
pub use unsetenv::unsetenv;

// CRT 启动入口 — 对应 musl src/env/__libc_start_main.c
pub use __libc_start_main::{_start_c, __libc_start_main, __init_libc, exit, _Exit, _exit};

// 公开导出 LIBC_SECURE 供 crate 内部使用（如 __init_libc 在启动时设置）
pub(crate) use secure_getenv::LIBC_SECURE;

// 内部 TLS 重置函数导出
pub(crate) use __reset_tls::__reset_tls;
