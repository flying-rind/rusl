//! ungetwc — 将宽字符推回 FILE 流的输入缓冲区。
//! 对应 musl src/stdio/ungetwc.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::c_int;

/// wint_t 在 musl 中定义为 int (c_int) — 与集成测试一致
pub type wint_t = c_int;

/// ungetwc — 将宽字符 c 推回流 f 的读缓冲区。
///
/// - `c`: 要推回的宽字符（wint_t），WEOF (-1) 不可推回
/// - `f`: 目标 FILE 流
///
/// 返回值：成功时返回 c；失败时返回 WEOF。
#[no_mangle]
pub extern "C" fn ungetwc(c: wint_t, f: *mut FILE) -> wint_t {
    unsafe {
        let f_ref = &mut *f;

        if f_ref.mode <= 0 {
            super::fwide::fwide(f, 1);
        }

        if f_ref.rpos.is_null() {
            super::__toread::__toread(f);
        }

        // 简单实现：只支持 ASCII 推回
        if f_ref.rpos.is_null() || c == -1 {
            return -1;
        }

        if f_ref.rpos <= f_ref.buf.sub(UNGET) {
            return -1;
        }

        f_ref.rpos = f_ref.rpos.sub(1);
        *f_ref.rpos = c as u8;
        f_ref.flags &= !F_EOF;

        c
    }
}
