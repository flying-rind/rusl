//! __fwritex — 向 FILE 写入数据。minimal 版本省略 fwrite。
//! 对应 musl src/stdio/fwrite.c

#![allow(unused_imports, unused_variables)]

use super::__towrite::__towrite;
use super::stdio_impl::*;

/// 向 FILE 写入数据，返回成功写入的字节数。
#[no_mangle]
pub unsafe extern "C" fn __fwritex(s: *const u8, l: usize, f: *mut FILE) -> usize {
    let f = &mut *f;
    let mut i: usize = 0;

    if f.wend.is_null() && __towrite(f) != 0 {
        return 0;
    }

    // 数据超过缓冲区剩余空间，直接委托 f->write
    if l > (f.wend as usize).wrapping_sub(f.wpos as usize) {
        return f
            .write
            .map_or(0, |write| unsafe { write(f, s, l) });
    }

    // 行缓冲：找到最后一个 '\n'，刷新到该位置（含）
    if f.lbf >= 0 {
        i = l;
        while i > 0 && *s.add(i - 1) != b'\n' {
            i -= 1;
        }
        if i > 0 {
            let n = f
                .write
                .map_or(0, |write| unsafe { write(f, s, i) });
            if n < i {
                return n;
            }
            // s += i, l -= i 在后面的 memcpy 中通过偏移处理
        }
    }

    // 将剩余数据复制到缓冲区
    let count = l - i;
    if count > 0 {
        unsafe {
            core::ptr::copy_nonoverlapping(s.add(i), f.wpos, count);
        }
        f.wpos = unsafe { f.wpos.add(count) };
    }
    l
}
