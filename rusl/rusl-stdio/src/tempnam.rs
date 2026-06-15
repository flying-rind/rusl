//! tempnam — 可定制临时文件名生成（POSIX XSI 扩展，已过时）。
//! 对应 musl src/stdio/tempnam.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_char, c_int};

// extern "C" 内存分配和 errno
extern "C" {
    fn malloc(size: usize) -> *mut core::ffi::c_void;
    fn __errno_location() -> *mut c_int;
    fn __randname(template: *mut u8) -> *mut u8;
}

// Linux x86_64 syscall numbers
#[cfg(target_arch = "x86_64")]
const SYS_readlink: i64 = 89;

// Linux aarch64 syscall numbers
#[cfg(target_arch = "aarch64")]
const SYS_readlinkat: i64 = 78;

const AT_FDCWD: i64 = -100;
const MAXTRIES: i32 = 100;
const PATH_MAX: usize = 4096;
const ENAMETOOLONG: c_int = 36;

/// 默认临时文件目录
const P_tmpdir: &[u8] = b"/tmp";

/// 默认前缀
const DEFAULT_PFX: &[u8] = b"temp";

/// tempnam — 生成唯一临时文件名。
///
/// [Visibility]: User — <stdio.h> 标准库函数（已过时）。
#[no_mangle]
pub extern "C" fn tempnam(dir: *const c_char, pfx: *const c_char) -> *mut c_char {
    unsafe {
        // 默认值处理
        let dir_slice: &[u8] = if dir.is_null() {
            P_tmpdir
        } else {
            // 计算 dir 长度
            let dl = crate::import::strnlen(dir, PATH_MAX);
            core::slice::from_raw_parts(dir as *const u8, dl)
        };

        let pfx_slice: &[u8] = if pfx.is_null() {
            DEFAULT_PFX
        } else {
            let pl = crate::import::strnlen(pfx, PATH_MAX);
            core::slice::from_raw_parts(pfx as *const u8, pl)
        };

        let dl = dir_slice.len();
        let pl = pfx_slice.len();
        // dir + '/' + pfx + '_' + 6 random chars + '\0'
        let l = dl + 1 + pl + 1 + 6;

        if l >= PATH_MAX {
            // errno = ENAMETOOLONG
            let errno_loc = __errno_location();
            unsafe { *errno_loc = ENAMETOOLONG; }
            return core::ptr::null_mut();
        }

        // 组装路径
        let mut s: [u8; PATH_MAX] = [0u8; PATH_MAX];
        s[..dl].copy_from_slice(dir_slice);
        s[dl] = b'/';
        s[dl + 1..dl + 1 + pl].copy_from_slice(pfx_slice);
        s[dl + 1 + pl] = b'_';

        for _ in 0..MAXTRIES {
            // 生成随机文件名
            __randname(s.as_mut_ptr().add(l - 6));

            // 检测路径是否已存在
            let dummy: u8 = 0;
            #[cfg(target_arch = "x86_64")]
            let r = rusl_core::__syscall3(
                SYS_readlink,
                s.as_ptr() as i64,
                &raw const dummy as i64,
                1,
            );

            #[cfg(target_arch = "aarch64")]
            let r = rusl_core::__syscall4(
                SYS_readlinkat,
                AT_FDCWD,
                s.as_ptr() as i64,
                &raw const dummy as i64,
                1,
            );

            // ENOENT = 2, 内核返回 -2
            if r == -2i64 {
                // strdup: malloc + copy
                let ptr = malloc(l + 1) as *mut u8;
                if ptr.is_null() {
                    return core::ptr::null_mut();
                }
                let dest = core::slice::from_raw_parts_mut(ptr, l + 1);
                dest[..=l].copy_from_slice(&s[..=l]);
                return ptr as *mut c_char;
            }
        }

        core::ptr::null_mut()
    }
}
