//! stdio 集成测试子模块
//!
//! 根据 rusl feature 选择导入源:
//! - rusl 模式: 直接从 rusl_stdio crate 导入
//! - 非 rusl 模式: 从 api::stdio 模块导入 (musl libc 符号)

// 标准库 API 导入
#[allow(unused)]
pub use rusl::api::stdio::*;

// 测试导入 — 根据 feature 选择后端
#[cfg(feature = "rusl")]
mod imports {
    pub use rusl_stdio::*;
    // 可变参数函数由 C wrapper 提供, 需单独声明
    use core::ffi::{c_char, c_int};
    extern "C" {
        pub fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
        pub fn fprintf(f: *mut FILE, fmt: *const c_char, ...) -> c_int;
    }
}
#[cfg(not(feature = "rusl"))]
mod imports {
    pub use rusl::api::stdio::*;
}

// ---------------------------------------------------------------------------
// 测试子模块
// ---------------------------------------------------------------------------

mod fwrite_tests;
mod vfprintf_tests;
mod vsnprintf_tests;
mod snprintf_tests;
mod fopen_fclose_tests;
mod file_io_tests;
mod char_io_tests;
mod stream_state_tests;
mod printf_family_tests;
mod stdio_vars_tests;

// ---- 新增测试模块 ----
mod gets_tests;
mod scanf_tests;
mod vscanf_tests;
mod vasprintf_vdprintf_tests;
mod dprintf_asprintf_tests;
mod file_pos_ext_tests;
mod buffer_ctl_tests;
mod fs_ops_tests;
mod advanced_streams_tests;
mod ungetc_tests;
mod misc_stream_tests;
mod getw_putw_tests;
mod unlocked_tests;
mod lockfile_tests;
mod wide_char_io_tests;
mod wide_char_unlocked_tests;
mod wide_format_tests;
mod wide_scan_tests;
mod gnu_ext_tests;
