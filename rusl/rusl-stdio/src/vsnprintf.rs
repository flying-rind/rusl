//! vsnprintf — 格式化输出到定长缓冲区。
//! 对应 musl src/stdio/vsnprintf.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use super::vfprintf::vfprintf;
use core::ffi::{c_char, c_int};

/// 内部 cookie 结构体（musl: struct cookie）
struct Cookie {
    s: *mut u8,
    n: usize,
}

/// sn_write 回调 —— 将缓冲区内容复制到用户提供的字符串。
unsafe extern "C" fn sn_write(f: *mut FILE, buf: *const u8, len: usize) -> usize {
    let f = &mut *f;
    let c = &mut *(f.cookie as *mut Cookie);

    // 先刷新内部缓冲中已有的数据
    let k = if f.wpos > f.wbase {
        core::cmp::min(c.n, (f.wpos as usize).wrapping_sub(f.wbase as usize))
    } else {
        0
    };
    if k > 0 {
        unsafe {
            core::ptr::copy_nonoverlapping(f.wbase, c.s, k);
        }
        c.s = unsafe { c.s.add(k) };
        c.n = c.n.wrapping_sub(k);
    }

    // 再写入本次数据
    let k2 = core::cmp::min(c.n, len);
    if k2 > 0 {
        unsafe {
            core::ptr::copy_nonoverlapping(buf, c.s, k2);
        }
        c.s = unsafe { c.s.add(k2) };
        c.n = c.n.wrapping_sub(k2);
    }

    // 总是以 '\0' 结尾
    unsafe {
        *c.s = 0;
    }

    // 重置写指针（清空内部缓冲）
    f.wpos = f.buf;
    f.wbase = f.buf;

    len // 假装全部写入成功
}

/// vsnprintf — 格式化输出到定长缓冲区（musl ABI 兼容）。
///
/// - `s`: 输出缓冲区（若 n==0 可为 null）
/// - `n`: 缓冲区大小（含结尾 '\0'）
/// - `fmt`: 格式字符串
/// - `ap`: 可变参数列表
///
/// 返回若缓冲区足够大时本应写入的字节数（不含 '\0'）。
#[no_mangle]
pub extern "C" fn vsnprintf(
    s: *mut c_char,
    n: usize,
    fmt: *const c_char,
    ap: *mut VaList,
) -> c_int {
    // SAFETY: caller guarantees s (if non-null) is a valid buffer and fmt/ap are valid per C ABI contract.
    unsafe {
        let mut buf: u8 = 0;
        let mut dummy: u8 = 0;

        let mut c = Cookie {
            s: if n > 0 {
                s as *mut u8
            } else {
                &mut dummy as *mut u8
            },
            n: if n > 0 { n - 1 } else { 0 },
        };

        // 构建栈上的 FILE，lock == -1 消除所有锁操作
        let mut f: FILE = core::mem::zeroed();
        f.lbf = EOF;
        f.write = Some(sn_write);
        f.lock = -1;
        f.buf = &mut buf as *mut u8;
        f.cookie = &mut c as *mut Cookie as *mut core::ffi::c_void;

        // 确保初始以 '\0' 结尾
        *c.s = 0;

        vfprintf(&mut f as *mut FILE, fmt, ap)
    }
}
