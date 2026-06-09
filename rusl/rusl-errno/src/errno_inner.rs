//! errno — 线程局部 errno 存储与错误消息映射。
//! 对应 musl src/errno/ + src/string/strerror_r.c
//!
//! 包含 __errno_location (errno 线程局部访问), strerror/strerror_l (错误消息映射),
//! 以及 strerror_r (线程安全错误消息缓冲区拷贝)。
//! 所有公共接口均为 `extern "C"` ABI, 与 POSIX/C 标准保持兼容。

#![allow(dead_code, unused_imports)]

mod __errno_location;
mod strerror;
mod strerror_r;

pub use __errno_location::*;
pub use strerror::*;
pub use strerror_r::*;

// 重导出依赖类型，供 crate 内部使用
pub(crate) use crate::import::__locale_struct;
