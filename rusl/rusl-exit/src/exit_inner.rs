//! exit — exit, abort, atexit 等进程终止函数。
//! 对应 musl src/exit/ 目录。

#![allow(dead_code, unused_imports)]
#![allow(non_snake_case)]

mod abort;
mod _Exit;
mod exit;

pub use abort::abort;
pub use _Exit::_Exit;
pub use exit::exit;
