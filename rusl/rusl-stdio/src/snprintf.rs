//! snprintf — 格式化输出到定长缓冲区。
//! 对应 musl src/stdio/snprintf.c
//!
//! 由于 Rust 不支持稳定的 C 可变参数, 此符号由 build.rs 编译的
//! C wrapper (snprintf_wrapper.c) 提供, 内部调用 Rust 的 vsnprintf。