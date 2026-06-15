//! vswprintf — 宽字符串格式化输出（va_list 版本）。
//! 对应 musl src/stdio/vswprintf.c

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_int, c_void};

/// sw_write 的 cookie
struct SwCookie {
    ws: *mut c_int,   // 目标 wchar_t 缓冲区
    l: usize,          // 剩余可写入的 wchar_t 数量（不含结尾 L'\0'）
}

/// sw_write 回调 — 将 multibyte (UTF-8) 转换为 wchar_t 并写入目标缓冲区。
unsafe extern "C" fn sw_write(f: *mut FILE, s: *const u8, len: usize) -> usize {
    unsafe {
        let f = &mut *f;
        let c = &mut *(f.cookie as *mut SwCookie);

        // 先刷新 FILE 写缓冲区中的待写数据
        let len2 = if f.wpos > f.wbase {
            f.wpos as usize - f.wbase as usize
        } else {
            0
        };
        if len2 > 0 {
            f.wpos = f.wbase;
            let written = sw_write(f, f.wpos, len2);
            // sw_write 可能返回 -1 on error (实际上 usize 无法表示 -1)
            if written < len2 {
                return 0; // 部分写入视为错误
            }
        }

        // 将 multibyte 字节转换为 wchar_t
        let mut src_idx: usize = 0;
        while src_idx < len && c.l > 0 {
            let byte = *s.add(src_idx);

            let (cp, advance): (u32, usize) = if byte < 0x80 {
                (byte as u32, 1)
            } else if byte < 0xC0 {
                // 非法续字节，跳过
                (0xFFFD, 1)
            } else if byte < 0xE0 {
                if src_idx + 1 < len {
                    let b1 = *s.add(src_idx + 1);
                    (((byte as u32 & 0x1F) << 6) | (b1 as u32 & 0x3F), 2)
                } else {
                    break; // 需要更多字节
                }
            } else if byte < 0xF0 {
                if src_idx + 2 < len {
                    let b1 = *s.add(src_idx + 1);
                    let b2 = *s.add(src_idx + 2);
                    (((byte as u32 & 0x0F) << 12) | ((b1 as u32 & 0x3F) << 6) | (b2 as u32 & 0x3F), 3)
                } else {
                    break;
                }
            } else {
                if src_idx + 3 < len {
                    let b1 = *s.add(src_idx + 1);
                    let b2 = *s.add(src_idx + 2);
                    let b3 = *s.add(src_idx + 3);
                    (((byte as u32 & 0x07) << 18) | ((b1 as u32 & 0x3F) << 12) | ((b2 as u32 & 0x3F) << 6) | (b3 as u32 & 0x3F), 4)
                } else {
                    break;
                }
            };

            *c.ws = cp as c_int;
            c.ws = c.ws.add(1);
            c.l -= 1;
            src_idx += advance;
            let _ = advance; // consumed in multibyte→wchar conversion
        }

        // 确保以 L'\0' 结尾
        *c.ws = 0;

        // 重置 FILE 写缓冲区
        f.wend = f.buf.add(f.buf_size);
        f.wpos = f.buf;
        f.wbase = f.buf;

        len // 假装所有输入均被消费
    }
}

/// vswprintf — 将格式化宽字符串写入缓冲区 s，最多 n 个宽字符。
///
/// [Visibility]: User — <wchar.h> 标准库函数。
#[no_mangle]
pub extern "C" fn vswprintf(
    s: *mut c_int,
    n: usize,
    fmt: *const c_int,
    ap: *mut VaList,
) -> c_int {
    unsafe {
        if n == 0 {
            return -1;
        }

        let mut buf: [u8; 256] = [0u8; 256];
        let mut c = SwCookie {
            ws: s,
            l: n - 1, // 保留 1 个位置给 L'\0'
        };

        // 构建栈上的 FILE
        let mut f: FILE = core::mem::zeroed();
        f.lbf = EOF;
        f.write = Some(sw_write);
        f.lock = -1;
        f.buf = buf.as_mut_ptr();
        f.buf_size = buf.len();
        f.cookie = &mut c as *mut SwCookie as *mut c_void;

        let r = super::vfwprintf::vfwprintf(&mut f as *mut FILE, fmt, ap);

        // 确保第二个 NULL 终止符
        if c.l > 0 {
            *c.ws = 0;
        }

        // 第二次调用 sw_write 确保缓冲区刷新
        sw_write(&mut f as *mut FILE, core::ptr::null(), 0);

        if r >= (n as c_int) {
            -1
        } else {
            r
        }
    }
}
