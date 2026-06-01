//! exit — exit, abort, atexit 等进程终止函数。
//! 对应 musl src/exit/ 目录。

#![allow(dead_code, unused_imports)]

mod abort;

pub use abort::abort;