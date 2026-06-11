//! fscanf — 从 FILE 流格式化输入。
//! 对应 musl src/stdio/fscanf.c
//!
//! 由于 Rust 不支持稳定的 C 可变参数，此符号由 build.rs 编译的
//! C wrapper (fscanf_wrapper.c) 提供，内部调用 Rust 的 vfscanf。
//! __isoc99_fscanf 为 fscanf 的 C99 兼容弱别名。
