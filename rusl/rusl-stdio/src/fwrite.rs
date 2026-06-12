//! fwrite / __fwritex — 向 FILE 写入数据。
//! 对应 musl src/stdio/fwrite.c

#![allow(unused_imports, unused_variables)]

use super::__towrite::__towrite;
use super::stdio_impl::*;
use core::ffi::c_void;

/// fwrite — musl libc 对外导出的缓冲写入函数。
/// 计算 size*nmemb 后委托 __fwritex。
#[no_mangle]
pub extern "C" fn fwrite(
    src: *const c_void,
    size: usize,
    nmemb: usize,
    f: *mut FILE,
) -> usize {
    let l = size.wrapping_mul(nmemb);
    if l == 0 {
        return 0;
    }
    let k = unsafe { __fwritex(src as *const u8, l, f) };
    if k == l { nmemb } else { k / size }
}

/// fwrite_unlocked — fwrite 的免锁版本。
#[no_mangle]
pub extern "C" fn fwrite_unlocked(
    src: *const c_void,
    size: usize,
    nmemb: usize,
    f: *mut FILE,
) -> usize {
    let l = size.wrapping_mul(nmemb);
    if l == 0 {
        return 0;
    }
    unsafe { __fwritex(src as *const u8, l, f) }
}

/// 向 FILE 写入数据，返回成功写入的字节数。
#[no_mangle]
pub(crate) unsafe extern "C" fn __fwritex(s: *const u8, l: usize, f: *mut FILE) -> usize {
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

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rusl_core::test;
    use super::super::stdio_impl::*;

    /// 创建测试用 FILE，已开启写模式（wend 非空）
    unsafe fn make_write_file(buf: *mut u8, buf_size: usize) -> FILE {
        let mut f: FILE = core::mem::zeroed();
        f.buf = buf;
        f.buf_size = buf_size;
        f.lock = -1;
        f.lbf = EOF; // 默认全缓冲
        // 模拟 __towrite 已调用: wpos/wbase = buf, wend = buf+buf_size
        f.wpos = buf;
        f.wbase = buf;
        f.wend = if buf_size > 0 { buf.add(buf_size) } else { core::ptr::null_mut() };
        f
    }

    // ---- 基本缓冲写入 ----

    test!("test_fwritex_small_data_fits_buffer" {
        // 前置: 数据小于缓冲区剩余空间
        // 后置: 数据被复制到缓冲区，返回全部长度，wpos 前移
        let mut buf = [0u8; 32];
        let data = b"hello";
        unsafe {
            let mut f = make_write_file(buf.as_mut_ptr(), 32);
            let result = __fwritex(data.as_ptr(), 5, &mut f as *mut FILE);
            assert_eq!(result, 5, "应返回全部写入字节数");
            assert_eq!(&buf[..5], b"hello", "缓冲区应包含写入数据");
            assert_eq!(
                f.wpos as usize,
                buf.as_mut_ptr().add(5) as usize,
                "wpos 应前移 5 字节"
            );
        }
    });

    test!("test_fwritex_exact_buffer_fill" {
        // 前置: 数据正好填满缓冲区
        // 后置: 数据被复制，wpos 到达 wend
        let mut buf = [0u8; 8];
        let data = b"12345678";
        unsafe {
            let mut f = make_write_file(buf.as_mut_ptr(), 8);
            let result = __fwritex(data.as_ptr(), 8, &mut f as *mut FILE);
            assert_eq!(result, 8);
            assert_eq!(&buf[..8], b"12345678");
            assert_eq!(f.wpos as usize, buf.as_mut_ptr().add(8) as usize);
            // wpos == wend (缓冲区满了)
            assert_eq!(f.wpos, f.wend);
        }
    });

    // ---- 溢出委托 write ----

    test!("test_fwritex_exceeds_buffer_delegates_write" {
        // 前置: 数据超过缓冲区剩余空间，有 write 回调
        // 后置: 委托 write 回调，返回 write 的返回值
        unsafe extern "C" fn mock_write(_f: *mut FILE, _buf: *const u8, len: usize) -> usize {
            len // 声称全部写入
        }

        let mut buf = [0u8; 4];
        let data = b"hello world"; // 11 字节, > 4
        unsafe {
            let mut f = make_write_file(buf.as_mut_ptr(), 4);
            f.write = Some(mock_write);

            let result = __fwritex(data.as_ptr(), 11, &mut f as *mut FILE);
            assert_eq!(result, 11, "委托 write 应返回其返回值");
            // 缓冲区内容不变（wpos 未移动）
            assert_eq!(f.wpos, buf.as_mut_ptr());
        }
    });

    test!("test_fwritex_exceeds_buffer_no_write_callback" {
        // 前置: 数据超缓冲区，但 write 为 None
        // 后置: 返回 0（无回调可委托）
        let mut buf = [0u8; 4];
        let data = b"hello";
        unsafe {
            let mut f = make_write_file(buf.as_mut_ptr(), 4);
            f.write = None;

            let result = __fwritex(data.as_ptr(), 5, &mut f as *mut FILE);
            assert_eq!(result, 0, "无 write 回调时应返回 0");
        }
    });

    // ---- wend 为空 / __towrite 失败 ----

    test!("test_fwritex_wend_null_towrite_fails" {
        // 前置: wend 为空，且 __towrite 会失败（F_NOWR）
        // 后置: 返回 0
        let mut buf = [0u8; 64];
        let data = b"test";
        unsafe {
            let mut f: FILE = core::mem::zeroed();
            f.buf = buf.as_mut_ptr();
            f.buf_size = 64;
            f.lock = -1;
            f.flags = F_NOWR; // __towrite 将失败
            // wend 为空，触发 __towrite
            f.wend = core::ptr::null_mut();

            let result = __fwritex(data.as_ptr(), 4, &mut f as *mut FILE);
            assert_eq!(result, 0, "__towrite 失败应返回 0");
        }
    });

    test!("test_fwritex_wend_null_towrite_success" {
        // 前置: wend 为空但 __towrite 能成功
        // 后置: 数据被缓冲写入
        let mut buf = [0u8; 16];
        let data = b"hi";
        unsafe {
            let mut f: FILE = core::mem::zeroed();
            f.buf = buf.as_mut_ptr();
            f.buf_size = 16;
            f.lock = -1;
            f.lbf = EOF;
            // wend 为空 — __fwritex 内部会调用 __towrite
            f.wend = core::ptr::null_mut();

            let result = __fwritex(data.as_ptr(), 2, &mut f as *mut FILE);
            assert_eq!(result, 2, "应成功写入");
            // __towrite 之后 wpos=wbase=buf
            assert_eq!(&buf[..2], b"hi");
        }
    });

    // ---- 行缓冲 ----

    test!("test_fwritex_linebuf_with_newline" {
        // 前置: 行缓冲模式 (lbf >= 0)，数据包含 '\n'（非末尾）
        // 后置: '\n' 之前（含）的数据通过 write 刷新，剩余部分缓冲
        unsafe extern "C" fn mock_write(_f: *mut FILE, _buf: *const u8, len: usize) -> usize {
            len // 模拟全部写入成功
        }

        let mut buf = [0u8; 32];
        // "line1\n" = 6 bytes + "line2" = 5 bytes → total = 11
        let data = b"line1\nline2";
        unsafe {
            let mut f = make_write_file(buf.as_mut_ptr(), 32);
            f.lbf = 0; // 行缓冲
            f.write = Some(mock_write);

            let result = __fwritex(data.as_ptr(), 11, &mut f as *mut FILE);
            // 从右向左找最后一个 '\n': s[5] = '\n', i=6
            // write(f, s, 6) 刷新 "line1\n", 剩余 "line2" (5 字节) 缓冲
            assert_eq!(result, 11, "应返回全部长度");
            assert_eq!(&buf[..5], b"line2", "缓冲区应包含 line2");
            assert_eq!(
                f.wpos as usize,
                buf.as_mut_ptr().add(5) as usize,
                "wpos 应指向 line2 之后"
            );
        }
    });

    test!("test_fwritex_linebuf_no_newline" {
        // 前置: 行缓冲模式，数据无 '\n'
        // 后置: 不触发 flush，全部缓冲
        let mut buf = [0u8; 32];
        let data = b"no_newline"; // 10 bytes
        unsafe {
            let mut f = make_write_file(buf.as_mut_ptr(), 32);
            f.lbf = 0; // 行缓冲
            // 无 write 回调 — 但既然没有 '\n'，也不会调用

            let result = __fwritex(data.as_ptr(), 10, &mut f as *mut FILE);
            assert_eq!(result, 10, "应返回全部长度");
            assert_eq!(&buf[..10], b"no_newline");
        }
    });

    test!("test_fwritex_linebuf_trailing_newline" {
        // 前置: 行缓冲模式，数据最后一个字符是 '\n'
        // 后置: 全部数据通过 write 刷新，无缓冲剩余
        unsafe extern "C" fn mock_write(_f: *mut FILE, _buf: *const u8, len: usize) -> usize {
            len
        }

        let mut buf = [0u8; 32];
        let data = b"hello\n";
        unsafe {
            let mut f = make_write_file(buf.as_mut_ptr(), 32);
            f.lbf = 0;
            f.write = Some(mock_write);

            let result = __fwritex(data.as_ptr(), 6, &mut f as *mut FILE);
            // 全部通过 write 刷新，count = l - i = 6 - 6 = 0，无缓冲
            assert_eq!(result, 6);
            // wpos 未移动（无剩余数据）
            assert_eq!(f.wpos, buf.as_mut_ptr());
        }
    });

    test!("test_fwritex_linebuf_partial_write_failure" {
        // 前置: 行缓冲模式，write 回调部分写入失败（返回 < i）
        // 后置: 返回 write 的实际写入量
        unsafe extern "C" fn half_write(_f: *mut FILE, _buf: *const u8, _len: usize) -> usize {
            3 // 只写入 3 字节
        }

        let mut buf = [0u8; 32];
        let data = b"abcdef\nrest";
        unsafe {
            let mut f = make_write_file(buf.as_mut_ptr(), 32);
            f.lbf = 0;
            f.write = Some(half_write);

            let result = __fwritex(data.as_ptr(), 11, &mut f as *mut FILE);
            // i=7 (upto '\n'), write returns 3 < 7, so return 3
            assert_eq!(result, 3, "部分写入失败应返回实际写入量");
        }
    });

    // ---- 边界和零长度 ----

    test!("test_fwritex_zero_length" {
        // 前置: l = 0
        // 后置: 返回 0，不做任何操作
        let mut buf = [0u8; 16];
        unsafe {
            let mut f = make_write_file(buf.as_mut_ptr(), 16);
            let old_wpos = f.wpos;
            let result = __fwritex(core::ptr::null(), 0, &mut f as *mut FILE);
            assert_eq!(result, 0, "零长度写入应返回 0");
            assert_eq!(f.wpos, old_wpos, "wpos 不应改变");
        }
    });

    test!("test_fwritex_multiple_writes_buffer_accumulates" {
        // 前置: 多次连续小写入
        // 后置: wpos 累计前移，缓冲区正确累积数据
        let mut buf = [0u8; 16];
        unsafe {
            let mut f = make_write_file(buf.as_mut_ptr(), 16);

            let r1 = __fwritex(b"abc\0".as_ptr(), 3, &mut f as *mut FILE);
            assert_eq!(r1, 3);
            let r2 = __fwritex(b"123\0".as_ptr(), 3, &mut f as *mut FILE);
            assert_eq!(r2, 3);

            assert_eq!(&buf[..6], b"abc123");
            assert_eq!(f.wpos as usize, buf.as_mut_ptr().add(6) as usize);
        }
    });

    test!("test_fwritex_data_exactly_remaining_space" {
        // 前置: 数据 == 剩余缓冲区空间（不触发 overflow 路径）
        // 后置: 数据被缓冲写入
        let mut buf = [0xFEu8; 8];
        // 预设 wpos 在偏移 3 处（剩余 5 字节空间）
        unsafe {
            let mut f = make_write_file(buf.as_mut_ptr(), 8);
            f.wpos = buf.as_mut_ptr().add(3);

            let data = b"12345";
            let result = __fwritex(data.as_ptr(), 5, &mut f as *mut FILE);
            assert_eq!(result, 5);
            assert_eq!(&buf[3..8], b"12345");
            assert_eq!(f.wpos, f.wend, "缓冲区刚好填满");
        }
    });
}
