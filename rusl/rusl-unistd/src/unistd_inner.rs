//! unistd — POSIX 系统调用封装。
//! 对应 musl src/unistd/ 目录。

#![allow(dead_code, unused_imports)]

mod write;
mod _exit;

pub use write::write;
pub use _exit::_exit;
