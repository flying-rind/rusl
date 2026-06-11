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

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rusl_core::test;
    use super::super::stdio_impl::*;
    use core::ffi::{c_char, c_int};

    /// 构造 VaList 用于测试
    unsafe fn make_va_list(gp_values: &[u64]) -> VaList {
        VaList {
            gp_offset: 0,
            fp_offset: 48,
            overflow_arg_area: core::ptr::null_mut(),
            reg_save_area: gp_values.as_ptr() as *mut core::ffi::c_void,
        }
    }

    // ---- sn_write 内部回调测试 ----

    test!("test_sn_write_basic" {
        // 前置: cookie 有足够空间
        // 后置: 数据复制到用户缓冲区，以 '\0' 结尾
        let mut out_buf = [0u8; 32];
        let mut f_buf: u8 = 0;
        unsafe {
            let mut c = Cookie {
                s: out_buf.as_mut_ptr(),
                n: 31,
            };

            let mut f: FILE = core::mem::zeroed();
            f.wpos = f_buf as *mut u8;
            f.wbase = f_buf as *mut u8;
            f.buf = f_buf as *mut u8;
            f.buf_size = 1;
            f.cookie = &mut c as *mut Cookie as *mut core::ffi::c_void;

            let data = b"hello";
            let result = sn_write(&mut f as *mut FILE, data.as_ptr(), 5);
            assert_eq!(result, 5, "sn_write 应返回 len 表示全部写入");
            assert_eq!(&out_buf[..5], b"hello");
            assert_eq!(out_buf[5], 0, "应以 '\\0' 结尾");
        }
    });

    test!("test_sn_write_truncation" {
        // 前置: cookie.n 小于写入数据长度
        // 后置: 仅写入 cookie.n 字节，以 '\0' 结尾
        let mut out_buf = [0xA5u8; 16];
        let mut f_buf: u8 = 0;
        unsafe {
            let mut c = Cookie {
                s: out_buf.as_mut_ptr(),
                n: 5, // 只能接受 5 字节
            };

            let mut f: FILE = core::mem::zeroed();
            f.wpos = f_buf as *mut u8;
            f.wbase = f_buf as *mut u8;
            f.buf = f_buf as *mut u8;
            f.buf_size = 1;
            f.cookie = &mut c as *mut Cookie as *mut core::ffi::c_void;

            let data = b"hello world"; // 11 字节
            let result = sn_write(&mut f as *mut FILE, data.as_ptr(), 11);
            assert_eq!(result, 11, "应返回 len（假装全部写入）");
            assert_eq!(&out_buf[..5], b"hello");
            assert_eq!(out_buf[5], 0, "截断后以 '\\0' 结尾");
            // c.n 应为 0（空间耗尽）
            assert_eq!(c.n, 0);
            // c.s 应超前到写入后的位置
            assert_eq!(c.s, out_buf.as_mut_ptr().add(5) as *mut u8);
        }
    });

    test!("test_sn_write_internal_buffer_has_pending_data" {
        // 前置: FILE 内部缓冲区有未刷新数据（wpos > wbase）
        // 后置: 先刷新内部缓冲数据到 cookie，再写入新数据
        let mut out_buf = [0xA5u8; 32];
        let mut internal_buf = [0u8; 16];
        // 在内部缓冲区预设一些数据
        internal_buf[0] = b'H';
        internal_buf[1] = b'i';
        unsafe {
            let mut c = Cookie {
                s: out_buf.as_mut_ptr(),
                n: 31,
            };

            let mut f: FILE = core::mem::zeroed();
            f.wbase = internal_buf.as_mut_ptr();
            f.wpos = internal_buf.as_mut_ptr().add(2); // 2 字节待刷新
            f.buf = internal_buf.as_mut_ptr();
            f.buf_size = 16;
            f.cookie = &mut c as *mut Cookie as *mut core::ffi::c_void;

            let data = b" there";
            let result = sn_write(&mut f as *mut FILE, data.as_ptr(), 6);
            assert_eq!(result, 6);
            assert_eq!(&out_buf[..2], b"Hi");
            assert_eq!(&out_buf[2..8], b" there");
            assert_eq!(out_buf[8], 0, "以 '\\0' 结尾");
        }
    });

    test!("test_sn_write_no_cookie_space" {
        // 前置: cookie.n = 0（无空间）
        // 后置: 不写入数据但依然以 '\0' 结尾
        let mut out_buf = [0xA5u8; 4];
        let mut f_buf: u8 = 0;
        unsafe {
            let mut c = Cookie {
                s: out_buf.as_mut_ptr(),
                n: 0, // 无空间
            };

            let mut f: FILE = core::mem::zeroed();
            f.wpos = f_buf as *mut u8;
            f.wbase = f_buf as *mut u8;
            f.buf = f_buf as *mut u8;
            f.buf_size = 1;
            f.cookie = &mut c as *mut Cookie as *mut core::ffi::c_void;

            let data = b"data";
            let result = sn_write(&mut f as *mut FILE, data.as_ptr(), 4);
            assert_eq!(result, 4, "sn_write 仍返回 len");
            assert_eq!(out_buf[0], 0, "应以 '\\0' 结尾");
        }
    });

    test!("test_sn_write_resets_wpos" {
        // 前置: 写入操作后
        // 后置: wpos 和 wbase 被重置到 buf 起始位置
        let mut out_buf = [0u8; 32];
        let mut internal_buf = [0u8; 8];
        unsafe {
            let mut c = Cookie {
                s: out_buf.as_mut_ptr(),
                n: 31,
            };

            let mut f: FILE = core::mem::zeroed();
            f.buf = internal_buf.as_mut_ptr();
            f.buf_size = 8;
            f.wpos = internal_buf.as_mut_ptr().add(3);
            f.wbase = internal_buf.as_mut_ptr();
            f.cookie = &mut c as *mut Cookie as *mut core::ffi::c_void;

            let _ = sn_write(&mut f as *mut FILE, b"x".as_ptr(), 1);
            // wpos 和 wbase 被重置为 buf
            assert_eq!(f.wpos, internal_buf.as_mut_ptr());
            assert_eq!(f.wbase, internal_buf.as_mut_ptr());
        }
    });

    // ---- vsnprintf 集成测试（内部函数测试角度） ----

    test!("test_vsnprintf_n_zero" {
        // 前置: n == 0，fmt 为空字符串
        // 后置: 返回 0（空字符串的长度）
        unsafe {
            let va = make_va_list(&[]);
            let ret = vsnprintf(
                core::ptr::null_mut(), // s 可为 null 当 n==0
                0,
                b"\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(ret, 0, "n=0 空格式应返回 0");
        }
    });

    test!("test_vsnprintf_truncation_short_buffer" {
        // 前置: 缓冲区小于格式化输出
        // 后置: 返回完整格式化长度，但仅写入 n-1 字节，以 '\0' 结尾
        unsafe {
            let mut buf = [0xF0u8; 6];
            let va = make_va_list(&[0xABCDu64]);
            // "%x" 格式化 0xABCD → "abcd" (4 字节)
            // n=5 → 最多写 4 字节 + '\0'
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                5, // 含 '\0'，有效空间 4
                b"%x\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(ret, 4, "应返回完整格式化长度 4");
            assert_eq!(&buf[..4], b"abcd");
            assert_eq!(buf[4], 0, "应以 '\\0' 结尾");
            assert_eq!(buf[5], 0xF0, "不应写入超出部分");
        }
    });

    test!("test_vsnprintf_exact_fit" {
        // 前置: 缓冲区刚好容纳格式输出 + '\0'
        // 后置: 完整格式化输出，以 '\0' 结尾
        unsafe {
            let mut buf = [0u8; 6]; // 5 有效字节 + '\0' = 6
            let va = make_va_list(&[]);
            // "hello" = 5 字符
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                6,
                b"hello\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(ret, 5);
            assert_eq!(&buf[..5], b"hello");
            assert_eq!(buf[5], 0);
        }
    });

    test!("test_vsnprintf_int_edge_cases" {
        // 测试特殊整数值的格式化
        unsafe {
            let mut buf = [0u8; 64];

            // 0
            let va = make_va_list(&[0u64]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                64,
                b"%d\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(ret, 1);
            assert_eq!(&buf[..1], b"0");

            // i32::MIN
            let va2 = make_va_list(&[(i32::MIN as u64)]);
            let ret2 = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                64,
                b"%d\0".as_ptr() as *const c_char,
                &va2 as *const VaList as *mut VaList,
            );
            // i32::MIN = -2147483648 → 11 字符
            assert_eq!(ret2, 11);
            assert_eq!(&buf[..11], b"-2147483648");
        }
    });

    test!("test_vsnprintf_multiple_specifiers" {
        // 测试多个格式说明符
        unsafe {
            let mut buf = [0u8; 64];
            // args: 值为 42, 字符串 "bar"
            let bar = b"bar\0";
            let va = make_va_list(&[42u64, bar.as_ptr() as u64]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                64,
                b"foo %d %s\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            // "foo 42 bar" = 10 字符
            assert_eq!(ret, 10, "ret mismatch, got {}", ret);
            assert_eq!(&buf[..10], b"foo 42 bar");
        }
    });
}
