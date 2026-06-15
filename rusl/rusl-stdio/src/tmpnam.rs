//! tmpnam — 临时文件名生成（C89，已过时）。
//! 对应 musl src/stdio/tmpnam.c

#![allow(unused_imports, unused_variables)]

extern "C" {
    fn __randname(template: *mut u8) -> *mut u8;
}

use super::stdio_impl::*;
use core::ffi::c_char;

// Linux x86_64 syscall numbers
#[cfg(target_arch = "x86_64")]
const SYS_readlink: i64 = 89;

// Linux aarch64 syscall numbers
#[cfg(target_arch = "aarch64")]
const SYS_readlinkat: i64 = 78;

const AT_FDCWD: i64 = -100;
const MAXTRIES: i32 = 100;
const L_tmpnam: usize = 20;

/// 内部静态缓冲区（非线程安全，符合 POSIX 语义）
static mut TMPNAM_INTERNAL: [u8; L_tmpnam] = [0u8; L_tmpnam];

/// tmpnam — 生成唯一临时文件名。
///
/// [Visibility]: User — <stdio.h> 标准库函数（C89，已过时）。
#[no_mangle]
pub extern "C" fn tmpnam(buf: *mut c_char) -> *mut c_char {
    unsafe {
        let template_bytes = b"/tmp/tmpnam_XXXXXX\0";
        let mut template: [u8; 19] = [0u8; 19];
        template.copy_from_slice(template_bytes);
        let dest: *mut u8;

        if buf.is_null() {
            dest = TMPNAM_INTERNAL.as_ptr() as *mut u8;
        } else {
            dest = buf as *mut u8;
        }

        for _ in 0..MAXTRIES {
            // 生成随机文件名
            __randname(template.as_mut_ptr().add(12));

            // 检测路径是否已存在（readlink 对不存在路径返回 -ENOENT）
            let dummy: u8 = 0;
            #[cfg(target_arch = "x86_64")]
            let r = rusl_core::__syscall3(
                SYS_readlink,
                template.as_ptr() as i64,
                &raw const dummy as i64,
                1,
            );

            #[cfg(target_arch = "aarch64")]
            let r = rusl_core::__syscall4(
                SYS_readlinkat,
                AT_FDCWD,
                template.as_ptr() as i64,
                &raw const dummy as i64,
                1,
            );

            // ENOENT = 2, 内核返回 -2 表示文件不存在
            if r == -2 {
                // 拷贝到目标缓冲区
                let template_ptr = template.as_ptr();
                let mut i: usize = 0;
                loop {
                    let c = *template_ptr.add(i);
                    *dest.add(i) = c;
                    if c == 0 {
                        break;
                    }
                    i += 1;
                }
                return dest as *mut c_char;
            }
        }

        core::ptr::null_mut()
    }
}
