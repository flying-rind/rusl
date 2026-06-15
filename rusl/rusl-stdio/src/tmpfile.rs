//! tmpfile — 创建临时文件，关闭或程序退出时自动删除。
//! 对应 musl src/stdio/tmpfile.c

#![allow(unused_imports, unused_variables)]

extern "C" {
    fn __randname(template: *mut u8) -> *mut u8;
}

use super::stdio_impl::*;
use core::ffi::c_int;

// Linux x86_64 syscall numbers and flags
#[cfg(target_arch = "x86_64")]
const SYS_open: i64 = 2;
#[cfg(target_arch = "x86_64")]
const SYS_close: i64 = 3;
#[cfg(target_arch = "x86_64")]
const SYS_unlink: i64 = 87;

#[cfg(target_arch = "aarch64")]
const SYS_openat: i64 = 1024;
#[cfg(target_arch = "aarch64")]
const SYS_close: i64 = 57;
#[cfg(target_arch = "aarch64")]
const SYS_unlinkat: i64 = 35;

const O_RDWR: c_int = 2;
const O_CREAT: c_int = 0o100;
const O_EXCL: c_int = 0o200;
const AT_FDCWD: i64 = -100;
const MAXTRIES: c_int = 100;

/// 创建临时文件，返回 FILE 指针。
///
/// [Visibility]: User — <stdio.h> 标准库函数。
#[no_mangle]
pub extern "C" fn tmpfile() -> *mut FILE {
    unsafe {
        // 使用静态字节数组作为模板
        static TEMPLATE: &[u8] = b"/tmp/tmpfile_XXXXXX";
        let mut s: [u8; 20] = [0u8; 20];
        // 手动复制（比 copy_from_slice 更宽容，避免长度不匹配 panic）
        let tmpl_len = TEMPLATE.len();
        for i in 0..tmpl_len {
            s[i] = TEMPLATE[i];
        }

        for _ in 0..MAXTRIES {
            // 生成随机文件名（替换末尾 6 个字符）
            __randname(s.as_mut_ptr().add(13));

            // 原子创建 + 打开文件
            #[cfg(target_arch = "x86_64")]
            let fd = rusl_core::__syscall3(
                SYS_open,
                s.as_ptr() as i64,
                (O_RDWR | O_CREAT | O_EXCL) as i64,
                0o600,
            ) as c_int;

            #[cfg(target_arch = "aarch64")]
            let fd = rusl_core::__syscall4(
                SYS_openat,
                AT_FDCWD,
                s.as_ptr() as i64,
                (O_RDWR | O_CREAT | O_EXCL) as i64,
                0o600,
            ) as c_int;

            if fd >= 0 {
                // 立即删除目录项（文件通过 fd 仍可访问）
                #[cfg(target_arch = "x86_64")]
                { rusl_core::__syscall1(SYS_unlink, s.as_ptr() as i64); }
                #[cfg(target_arch = "aarch64")]
                { rusl_core::__syscall2(SYS_unlinkat, AT_FDCWD, s.as_ptr() as i64); }

                // 从 fd 构造 FILE 流
                let mode = b"w+\0".as_ptr() as *const core::ffi::c_char;
                let f = super::__fdopen::__fdopen(fd, mode);
                if f.is_null() {
                    #[cfg(target_arch = "x86_64")]
                    { rusl_core::__syscall1(SYS_close, fd as i64); }
                    #[cfg(target_arch = "aarch64")]
                    { rusl_core::__syscall1(SYS_close, fd as i64); }
                }
                return f;
            }
        }

        core::ptr::null_mut()
    }
}
