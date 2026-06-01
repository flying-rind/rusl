//! # rusl 内部模块
//!
//! 本模块包含 rusl crate 内部使用的类型定义、辅助函数和基础设施。
//! 所有符号均为内部实现细节，不构成 rusl 稳定的公开 API。
//!
//! 对应 musl 的 `src/internal/*.h` 头文件集合。

#![allow(dead_code)] // 基础设施模块，后续阶段使用

// 原子操作 (对应 musl src/internal/atomic.h)
pub mod atomic;
// libc 全局状态 (对应 musl src/internal/libc.c + libc.h)
pub mod libc;
// 版本号 (对应 musl src/internal/version.c)
pub mod version;

// 常量定义
pub mod futex;

// 类型定义
pub mod complex_impl;
pub mod lock;

// 全局状态
pub mod defsysinfo;

// 辅助函数
pub mod aio_impl;
pub mod emulate_wait4;
pub mod fdpic_crt;
pub mod floatscan;
pub mod fork_impl;
pub mod intscan;
pub mod ksigaction;
pub mod procfdname;
pub mod shcall;

// 大型子系统
pub mod pthread_impl;
pub mod shgetc;