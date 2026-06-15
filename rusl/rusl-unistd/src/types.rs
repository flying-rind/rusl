//! unistd 模块公共类型和常量定义

use core::ffi::c_void;

// ========== 宏常量 ==========

/// 标准输入文件描述符
pub const STDIN_FILENO: i32 = 0;
/// 标准输出文件描述符
pub const STDOUT_FILENO: i32 = 1;
/// 标准错误输出文件描述符
pub const STDERR_FILENO: i32 = 2;

/// lseek: 从文件起始偏移
pub const SEEK_SET: i32 = 0;
/// lseek: 从当前位置偏移
pub const SEEK_CUR: i32 = 1;
/// lseek: 从文件末尾偏移
pub const SEEK_END: i32 = 2;

/// access: 测试文件存在性
pub const F_OK: i32 = 0;
/// access: 测试执行权限
pub const X_OK: i32 = 1;
/// access: 测试写权限
pub const W_OK: i32 = 2;
/// access: 测试读权限
pub const R_OK: i32 = 4;

// ========== 公共类型 ==========

/// 散布/聚集 I/O 向量
#[repr(C)]
pub struct iovec {
    pub iov_base: *mut c_void,
    pub iov_len: usize,
}
