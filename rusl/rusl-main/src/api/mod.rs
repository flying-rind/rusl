//! # api — musl libc 对外 API 接口
//!
//! 本模块统一管理所有 musl libc 对外导出的 C ABI 符号声明。
//!
//! 当启用 `c-test` feature 时,通过 `extern "C"` 声明 musl libc.a 中
//! 的符号;非 c-test 模式下,各符号由对应的 Rust 实现模块提供。
//!
//! ## 子模块
//!
//! - `ctype`  — 字符分类/大小写转换
//! - `string` — 内存/字符串操作
//! - `stdlib` — 标准库工具函数
//! - `malloc` — 内存分配器
//! - `search` — 搜索/哈希表/二叉树
//! - `prng`   — 伪随机数生成

#[cfg(feature = "rusl")]
pub mod types {
    pub use rusl_core::c_types::*;
}
#[cfg(not(feature = "rusl"))]
#[path ="type.rs"]
pub mod types;

#[cfg(feature = "rusl")]
pub mod ctype {
    pub use rusl_ctype::*;
}
#[cfg(not(feature = "rusl"))]
#[path = "ctype.rs"]
pub mod ctype;

#[cfg(feature = "rusl")]
pub mod malloc {
    pub use rusl_malloc::*;
}
#[cfg(not(feature = "rusl"))]
#[path = "malloc.rs"]
pub mod malloc;

#[cfg(feature = "rusl")]
pub mod prng {
    pub use rusl_prng::*;
}
#[cfg(not(feature = "rusl"))]
#[path = "prng.rs"]
pub mod prng;

#[cfg(feature = "rusl")]
pub mod search {
    pub use rusl_search::*;
}
#[cfg(not(feature = "rusl"))]
#[path = "search.rs"]
pub mod search;

#[cfg(feature = "rusl")]
pub mod stdlib {
    pub use rusl_stdlib::*;
}
#[cfg(not(feature = "rusl"))]
#[path = "stdlib.rs"]
pub mod stdlib;

#[cfg(feature = "rusl")]
pub mod string {
    pub use rusl_string::*;
}
#[cfg(not(feature = "rusl"))]
#[path = "string.rs"]
pub mod string;

#[cfg(feature = "rusl")]
pub mod exit {
    pub use rusl_exit::*;
}
#[cfg(not(feature = "rusl"))]
#[path = "exit.rs"]
pub mod exit;

#[cfg(feature = "rusl")]
pub mod unistd {
    pub use rusl_unistd::*;
}
#[cfg(not(feature = "rusl"))]
#[path = "unistd.rs"]
pub mod unistd;

#[cfg(feature = "rusl")]
pub mod regex {
    pub use rusl_regex::*;
}
#[cfg(not(feature = "rusl"))]
#[path = "regex.rs"]
pub mod regex;

#[cfg(feature = "rusl")]
pub mod stdio {
    pub use rusl_stdio::*;
}
#[cfg(not(feature = "rusl"))]
#[path = "stdio.rs"]
pub mod stdio;

#[cfg(feature = "rusl")]
pub mod env {
    pub use rusl_env::*;
}
#[cfg(not(feature = "rusl"))]
#[path = "env.rs"]
pub mod env;

#[cfg(feature = "rusl")]
pub mod errno {
    pub use rusl_errno::*;
}
#[cfg(not(feature = "rusl"))]
#[path = "errno.rs"]
pub mod errno;