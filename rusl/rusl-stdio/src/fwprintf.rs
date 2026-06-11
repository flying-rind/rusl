//! fwprintf — 宽字符格式化输出到 FILE 流。
//! 对应 musl src/stdio/fwprintf.c
//!
//! 由于 Rust 不支持稳定的 C 可变参数，此符号由 build.rs 编译的
//! C wrapper (fwprintf_wrapper.c) 提供，内部调用 Rust 的 vfwprintf。
