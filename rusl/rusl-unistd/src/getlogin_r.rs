//! getlogin_r — 获取当前登录用户名（线程安全）。
//! 对应 musl src/unistd/getlogin_r.c
//!
//! 线程安全版本，通过 getlogin 获取后拷贝到用户缓冲区。

use core::ffi::{c_char, c_int};
use core::ptr;

/// POSIX `getlogin_r` — 将登录用户名写入用户提供的缓冲区 `name`。
///
/// 线程安全版本。先通过 [`getlogin`] 获取登录名，检查长度后拷贝到用户缓冲区。
/// 成功返回 0，失败返回非零错误码。
#[no_mangle]
/// [Visibility]: External
pub extern "C" fn getlogin_r(name: *mut c_char, size: usize) -> c_int {
    if name.is_null() || size == 0 {
        return c_int::from(22i8); // EINVAL
    }
    let login = super::getlogin();
    if login.is_null() {
        return 6; // ENXIO (没有关联的登录名)
    }
    // 计算登录名长度
    let mut login_len: usize = 0;
    unsafe {
        while *login.add(login_len) != 0 {
            login_len += 1;
        }
    }
    // 检查缓冲区大小
    if login_len + 1 > size {
        return 34; // ERANGE
    }
    // 拷贝字符串
    unsafe {
        ptr::copy_nonoverlapping(login as *const u8, name as *mut u8, login_len + 1);
    }
    0
}

// 注意: 单元测试不在此文件中，因为 getlogin_r 依赖于 getlogin，
// 而 getlogin 调用 extern getenv，该符号在 rusl-unistd 的独立测试二进制中不可用。
