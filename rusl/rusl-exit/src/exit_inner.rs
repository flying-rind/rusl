//! exit — exit, abort, atexit 等进程终止函数。
//! 对应 musl src/exit/ 目录全部文件。

#![allow(dead_code, unused_imports)]
#![allow(non_snake_case)]

mod sys_consts;

mod _Exit;
pub(crate) use _Exit::_Exit;

mod abort;
pub use abort::abort;

mod exit;
pub use exit::exit;

pub(crate) mod atexit;

mod quick_exit;

mod abort_lock;

mod assert;
