//! fwscanf — 从 FILE 流宽字符格式化输入。
//! 对应 musl src/stdio/fwscanf.c
//!
//! 由于 Rust 不支持稳定的 C 可变参数，此符号由 build.rs 编译的
//! C wrapper (fwscanf_wrapper.c) 提供，内部调用 Rust 的 vfwscanf。
//! __isoc99_fwscanf 为 fwscanf 的 C99 兼容弱别名。
