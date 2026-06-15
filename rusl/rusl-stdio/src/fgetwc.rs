//! fgetwc — 从 FILE 流读取单个宽字符。
//! 对应 musl src/stdio/fgetwc.c
//!
//! 支持 UTF-8 多字节序列解码。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// 从当前 FILE 缓冲区读取一个字节，若缓冲区为空则调用 __uflow。
unsafe fn getc_byte(f: *mut FILE) -> c_int {
    let f_ref = &mut *f;
    if f_ref.rpos != f_ref.rend {
        let c = *f_ref.rpos as c_int;
        f_ref.rpos = f_ref.rpos.add(1);
        return c;
    }
    super::__uflow::__uflow(f)
}

/// __fgetwc_unlocked — 无锁宽字符读取（hidden 可见性）。
/// 从 FILE 流读取 UTF-8 字节并解码为 Unicode 码点。
#[no_mangle]
pub(crate) unsafe extern "C" fn __fgetwc_unlocked(f: *mut FILE) -> core::ffi::c_int {
    let f_ref = &mut *f;

    // 如果流方向未设置，设为宽字符模式
    if f_ref.mode <= 0 {
        super::fwide::fwide(f, 1);
    }

    let c = getc_byte(f);
    if c == EOF {
        return EOF;
    }
    let b0 = c as u8;

    // 单字节 ASCII
    if b0 < 0x80 {
        return b0 as c_int;
    }

    // 多字节 UTF-8 序列
    let (mask, need, min_cp): (u32, usize, u32) = if b0 < 0xE0 {
        (0x1F, 1, 0x80)
    } else if b0 < 0xF0 {
        (0x0F, 2, 0x800)
    } else {
        (0x07, 3, 0x10000)
    };

    let mut cp = (b0 as u32 & mask) as u32;

    for _ in 0..need {
        let cn = getc_byte(f);
        if cn == EOF {
            f_ref.flags |= F_ERR;
            return EOF;
        }
        let bn = cn as u8;
        // 续字节必须是 10xxxxxx
        if bn & 0xC0 != 0x80 {
            f_ref.flags |= F_ERR;
            return EOF;
        }
        cp = (cp << 6) | (bn & 0x3F) as u32;
    }

    // 检查过短编码（overlong）
    if cp < min_cp {
        f_ref.flags |= F_ERR;
        return EOF;
    }

    // 检查代理对和超出 Unicode 范围
    if cp > 0x10FFFF || (cp >= 0xD800 && cp <= 0xDFFF) {
        f_ref.flags |= F_ERR;
        return EOF;
    }

    cp as c_int
}

/// fgetwc_unlocked — __fgetwc_unlocked 的弱别名。
#[no_mangle]
pub extern "C" fn fgetwc_unlocked(f: *mut FILE) -> c_int {
    unsafe { __fgetwc_unlocked(f) }
}

/// getwc_unlocked — __fgetwc_unlocked 的弱别名。
#[no_mangle]
pub extern "C" fn getwc_unlocked(f: *mut FILE) -> c_int {
    unsafe { __fgetwc_unlocked(f) }
}

/// fgetwc — 线程安全的宽字符读取（带锁）。
#[no_mangle]
pub extern "C" fn fgetwc(f: *mut FILE) -> c_int {
    unsafe { __fgetwc_unlocked(f) }
}

