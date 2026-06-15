//! vfwprintf — 宽字符格式化输出核心引擎。
//! 对应 musl src/stdio/vfwprintf.c
//!
//! 解析宽字符格式字符串，对于字面文本逐个宽字符转换为 UTF-8 字节流写入 FILE。
//! 对于 % 格式说明符，解析标志/宽度/精度/类型后构造窄格式串委托给 vfprintf。

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use super::vfprintf::vfprintf;
use core::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// 常量：与 vfprintf 共享的标志位和状态定义
// ---------------------------------------------------------------------------
const ALT_FORM: u32 = 1u32 << (b'#' - b' ');
const ZERO_PAD: u32 = 1u32 << (b'0' - b' ');
const LEFT_ADJ: u32 = 1u32 << (b'-' - b' ');
const PAD_POS: u32  = 1u32 << (b' ' - b' ');
const MARK_POS: u32 = 1u32 << (b'+' - b' ');
const FLAGMASK: u32 = ALT_FORM | ZERO_PAD | LEFT_ADJ | PAD_POS | MARK_POS;

const BARE: u8   = 0;
const LPRE: u8   = 1;
const LLPRE: u8  = 2;
const HPRE: u8   = 3;
const HHPRE: u8  = 4;
const ZTPRE: u8  = 6;
const JPRE: u8   = 7;
const STOP: u8   = 8;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum WType {
    PTR   = 9,
    INT   = 10,
    UINT  = 11,
    LLONG = 12,
    LONG  = 13,
    ULONG = 14,
    SHORT = 15,
    USHORT = 16,
    CHAR  = 17,
    UCHAR = 18,
    ULLONG = 19,
    SIZET  = 20,
    IMAX   = 21,
    UMAX   = 22,
    PDIFF  = 23,
    UIPTR  = 24,
    DOUBLE  = 25,
    LDOUBLE = 26,
    NOARG  = 27,
}

// ---------------------------------------------------------------------------
// 宽字符 → UTF-8 编码
// ---------------------------------------------------------------------------
fn wc_to_utf8(cp: u32, out: &mut [u8; 4]) -> usize {
    if cp < 0x80 {
        out[0] = cp as u8;
        1
    } else if cp < 0x800 {
        out[0] = ((cp >> 6) as u8) | 0xC0;
        out[1] = ((cp & 0x3F) as u8) | 0x80;
        2
    } else if cp < 0x10000 {
        out[0] = ((cp >> 12) as u8) | 0xE0;
        out[1] = (((cp >> 6) & 0x3F) as u8) | 0x80;
        out[2] = ((cp & 0x3F) as u8) | 0x80;
        3
    } else {
        out[0] = ((cp >> 18) as u8) | 0xF0;
        out[1] = (((cp >> 12) & 0x3F) as u8) | 0x80;
        out[2] = (((cp >> 6) & 0x3F) as u8) | 0x80;
        out[3] = ((cp & 0x3F) as u8) | 0x80;
        4
    }
}

/// 统计多字节 UTF-8 字符串中的宽字符数量。
fn count_wide_chars(mb: *const u8, len: usize) -> usize {
    let mut count: usize = 0;
    let mut i: usize = 0;
    while i < len {
        let byte = unsafe { *mb.add(i) };
        if byte < 0x80 {
            i += 1;
        } else if byte < 0xC0 {
            i += 1; // 无效续字节
        } else if byte < 0xE0 {
            i = core::cmp::min(i + 2, len);
        } else if byte < 0xF0 {
            i = core::cmp::min(i + 3, len);
        } else {
            i = core::cmp::min(i + 4, len);
        }
        count += 1;
    }
    count
}

// ---------------------------------------------------------------------------
// 输出辅助函数
// ---------------------------------------------------------------------------

/// 向 FILE 写入字节序列（UTF-8 编码后的数据）。
unsafe fn wout_raw(f: *mut FILE, s: *const u8, len: usize) {
    if len == 0 { return; }
    let f_ref = &mut *f;
    if f_ref.write.is_none() { return; }
    let wf = f_ref.write.unwrap();
    wf(f, s, len);
}

/// 将一组宽字符转换为 UTF-8 并写入 FILE。
unsafe fn wout(f: *mut FILE, ws: *const c_int, l: usize) {
    let mut i = 0;
    let mut utf8_buf: [u8; 4] = [0; 4];
    let mut byte_buf: [u8; 256] = [0; 256];
    let mut bi: usize = 0;

    while i < l {
        let cp = *ws.add(i) as u32;
        let n = wc_to_utf8(cp, &mut utf8_buf);
        if bi + n > 256 {
            wout_raw(f, byte_buf.as_ptr(), bi);
            bi = 0;
        }
        for j in 0..n {
            byte_buf[bi] = utf8_buf[j];
            bi += 1;
        }
        i += 1;
    }
    if bi > 0 {
        wout_raw(f, byte_buf.as_ptr(), bi);
    }
}

/// 填充空格（右对齐/左对齐）。
unsafe fn wpad(f: *mut FILE, pad_char: u8, w: i32, l: i32, fl: u32) {
    if (fl & (LEFT_ADJ | ZERO_PAD) != 0) || l >= w {
        return;
    }
    let n = (w - l) as usize;
    let pad_buf = [pad_char; 256];
    let mut remaining = n;
    while remaining >= 256 {
        wout_raw(f, pad_buf.as_ptr(), 256);
        remaining -= 256;
    }
    wout_raw(f, pad_buf.as_ptr(), remaining);
}

/// 检查宽字符是否为数字。
fn iswdigit(wc: c_int) -> bool {
    (wc as u32) >= b'0' as u32 && (wc as u32) <= b'9' as u32
}

/// 从宽字符串中解析整数（宽度/精度）。
unsafe fn wgetint(s: &mut *const c_int) -> i32 {
    let mut i: i32 = 0;
    loop {
        let wc = **s;
        if !iswdigit(wc) {
            break;
        }
        let d = (wc - b'0' as c_int) as i32;
        if i > i32::MAX / 10 || (i == i32::MAX / 10 && d > i32::MAX % 10) {
            i = -1;
        } else {
            i = i * 10 + d;
        }
        *s = s.add(1);
    }
    i
}

/// 检查是否为有效标志字符。
fn is_wflag(ch: c_int) -> bool {
    let cu = ch as u32;
    cu >= b' ' as u32 && (cu - b' ' as u32) < 32 && (FLAGMASK & (1u32 << (cu - b' ' as u32))) != 0
}

/// 宽字符状态机 — 与 vfprintf 的 next_state 类似。
fn wnext_state(st: u8, ch: u8) -> u8 {
    match (st, ch) {
        (BARE, b'd') | (BARE, b'i') => WType::INT as u8,
        (BARE, b'o') | (BARE, b'u') | (BARE, b'x') | (BARE, b'X') => WType::UINT as u8,
        (BARE, b'f') | (BARE, b'F') | (BARE, b'e') | (BARE, b'E')
            | (BARE, b'g') | (BARE, b'G') | (BARE, b'a') | (BARE, b'A') => WType::DOUBLE as u8,
        (BARE, b'c') => WType::INT as u8,
        (BARE, b'C') => WType::UINT as u8,
        (BARE, b's') => WType::PTR as u8,
        (BARE, b'S') => WType::PTR as u8,
        (BARE, b'p') => WType::UIPTR as u8,
        (BARE, b'n') => WType::PTR as u8,
        (BARE, b'm') => WType::NOARG as u8,
        (BARE, b'l') => LPRE,
        (BARE, b'h') => HPRE,
        (BARE, b'z') | (BARE, b't') => ZTPRE,
        (BARE, b'j') => JPRE,

        (LPRE, b'd') | (LPRE, b'i') => WType::LONG as u8,
        (LPRE, b'o') | (LPRE, b'u') | (LPRE, b'x') | (LPRE, b'X') => WType::ULONG as u8,
        (LPRE, b'f') | (LPRE, b'F') | (LPRE, b'e') | (LPRE, b'E')
            | (LPRE, b'g') | (LPRE, b'G') | (LPRE, b'a') | (LPRE, b'A') => WType::DOUBLE as u8,
        (LPRE, b'c') => WType::UINT as u8,
        (LPRE, b's') => WType::PTR as u8,
        (LPRE, b'n') => WType::PTR as u8,
        (LPRE, b'l') => LLPRE,

        (LLPRE, b'd') | (LLPRE, b'i') => WType::LLONG as u8,
        (LLPRE, b'o') | (LLPRE, b'u') | (LLPRE, b'x') | (LLPRE, b'X') => WType::ULLONG as u8,
        (LLPRE, b'f') | (LLPRE, b'F') | (LLPRE, b'e') | (LLPRE, b'E')
            | (LLPRE, b'g') | (LLPRE, b'G') | (LLPRE, b'a') | (LLPRE, b'A') => WType::DOUBLE as u8,
        (LLPRE, b'n') => WType::PTR as u8,

        (HPRE, b'd') | (HPRE, b'i') => WType::SHORT as u8,
        (HPRE, b'o') | (HPRE, b'u') | (HPRE, b'x') | (HPRE, b'X') => WType::USHORT as u8,
        (HPRE, b'n') => WType::PTR as u8,
        (HPRE, b'h') => HHPRE,

        (HHPRE, b'd') | (HHPRE, b'i') => WType::CHAR as u8,
        (HHPRE, b'o') | (HHPRE, b'u') | (HHPRE, b'x') | (HHPRE, b'X') => WType::UCHAR as u8,
        (HHPRE, b'n') => WType::PTR as u8,

        (ZTPRE, b'd') | (ZTPRE, b'i') => WType::PDIFF as u8,
        (ZTPRE, b'o') | (ZTPRE, b'u') | (ZTPRE, b'x') | (ZTPRE, b'X') => WType::SIZET as u8,
        (ZTPRE, b'n') => WType::PTR as u8,

        (JPRE, b'd') | (JPRE, b'i') => WType::IMAX as u8,
        (JPRE, b'o') | (JPRE, b'u') | (JPRE, b'x') | (JPRE, b'X') => WType::UMAX as u8,
        (JPRE, b'n') => WType::PTR as u8,

        _ => 0,
    }
}

unsafe fn w_oob(ch: u8) -> bool {
    ch < b'A' || ch > b'z'
}

// ---------------------------------------------------------------------------
// 参数提取 (x86_64 System V ABI)
// ---------------------------------------------------------------------------

unsafe fn w_pop_arg_int(ap: *mut VaList, wt: WType) -> u64 {
    match wt {
        WType::INT   => va_arg_int(ap) as i32 as u64,
        WType::UINT  => va_arg_uint(ap) as u64,
        WType::LONG  => va_arg_long(ap) as u64,
        WType::ULONG => va_arg_ulong(ap),
        WType::LLONG => va_arg_longlong(ap) as u64,
        WType::ULLONG=> va_arg_ulonglong(ap),
        WType::SHORT => va_arg_int(ap) as i16 as u64,
        WType::USHORT=> va_arg_int(ap) as u16 as u64,
        WType::CHAR  => va_arg_int(ap) as i8 as u64,
        WType::UCHAR => va_arg_int(ap) as u8 as u64,
        WType::SIZET => va_arg_ulong(ap),
        WType::IMAX  => va_arg_longlong(ap) as u64,
        WType::UMAX  => va_arg_ulonglong(ap),
        WType::PDIFF => va_arg_long(ap) as u64,
        WType::UIPTR => va_arg_ptr(ap) as u64,
        _ => 0,
    }
}

unsafe fn w_pop_arg_double(ap: *mut VaList) -> f64 {
    va_arg_double(ap)
}

unsafe fn w_pop_arg_ptr(ap: *mut VaList) -> *mut c_void {
    va_arg_ptr(ap)
}

// ---------------------------------------------------------------------------
// 构建窄 VaList 并调用 vfprintf 进行数字格式化
// ---------------------------------------------------------------------------

/// 用两个 int 和一个 int 值构建 VaList，调用 vfprintf。
/// 只有当 p >= 0（用户指定了精度）时，precision 才会出现在 va_list 中，
/// 此时格式串中才包含 ".*"。
unsafe fn w_call_vfprintf_int(f: *mut FILE, narrow_fmt: *const u8, w: i32, p: i32, val: u64) -> i32 {
    // 构建寄存器保存区: 6×8 GP + XMM 区
    let mut reg_area = [0u64; 16]; // 前 6 个 GP, 后 8 个 XMM
    let mut idx: usize = 0;
    reg_area[idx] = w as u32 as u64; idx += 1;
    if p >= 0 {
        reg_area[idx] = p as u32 as u64; idx += 1;
    }
    reg_area[idx] = val;

    let mut vl = VaList {
        gp_offset: 0,
        fp_offset: 48,
        overflow_arg_area: core::ptr::null_mut(),
        reg_save_area: reg_area.as_mut_ptr() as *mut c_void,
    };
    vfprintf(f, narrow_fmt as *const i8, &mut vl as *mut VaList)
}

/// 用两个 int 和一个 double 值构建 VaList，调用 vfprintf。
unsafe fn w_call_vfprintf_double(f: *mut FILE, narrow_fmt: *const u8, w: i32, p: i32, val: f64) -> i32 {
    let mut reg_area = [0u64; 16];
    let mut idx: usize = 0;
    reg_area[idx] = w as u32 as u64; idx += 1;
    if p >= 0 {
        reg_area[idx] = p as u32 as u64; idx += 1;
    }
    // double 放在第一个 XMM 寄存器位置 (offset 48)
    let fp_ptr = (reg_area.as_mut_ptr() as *mut u8).add(48) as *mut f64;
    *fp_ptr = val;

    let mut vl = VaList {
        gp_offset: 0,
        fp_offset: 48,
        overflow_arg_area: core::ptr::null_mut(),
        reg_save_area: reg_area.as_mut_ptr() as *mut c_void,
    };
    vfprintf(f, narrow_fmt as *const i8, &mut vl as *mut VaList)
}

// ---------------------------------------------------------------------------
// wprintf_core — 核心格式化引擎
// ---------------------------------------------------------------------------

unsafe fn wprintf_core(f: *mut FILE, fmt: *const c_int, ap: *mut VaList) -> i32 {
    let mut s = fmt;
    let mut cnt: i32 = 0;
    let mut l: i32;

    loop {
        // 溢出保护
        if cnt > i32::MAX / 2 {
            return -1;
        }

        // 扫描字面文本直到 '%' 或结尾
        let a = s;
        while *s != 0 && *s != b'%' as c_int {
            s = s.add(1);
        }
        // 处理 %% → 合并相邻百分号
        let mut z = s;
        while *s == b'%' as c_int && *s.add(1) == b'%' as c_int {
            z = z.add(1);
            s = s.add(2);
        }
        l = (z as usize).wrapping_sub(a as usize) as i32;
        if f.is_null() {
            cnt += l;
        } else if l > 0 {
            wout(f, a, l as usize);
            cnt += l;
        }
        if l > 0 {
            continue;
        }
        if *z == 0 {
            break;
        }

        // 跳过 '%'
        s = z.add(1);

        // 读取标志
        let mut fl: u32 = 0;
        while is_wflag(*s) {
            fl |= 1u32 << ((*s as u32) - b' ' as u32);
            s = s.add(1);
        }

        // 读取宽度
        let w: i32;
        if *s == b'*' as c_int {
            s = s.add(1);
            w = if f.is_null() { 0 } else { va_arg_int(ap) };
            if w < 0 {
                fl |= LEFT_ADJ;
            }
        } else {
            w = wgetint(&mut s);
            if w < 0 { return -1; }
        }
        let w_abs = if w < 0 { -w } else { w };

        // 读取精度
        let p: i32;
        let xp: bool;
        if *s == b'.' as c_int {
            s = s.add(1);
            if *s == b'*' as c_int {
                s = s.add(1);
                p = if f.is_null() { 0 } else { va_arg_int(ap) };
                xp = p >= 0;
            } else {
                p = wgetint(&mut s);
                xp = true;
            }
        } else {
            p = -1;
            xp = false;
        }

        // 状态机
        let mut st: u8 = BARE;
        let mut ps: u8;
        loop {
            if w_oob(*s as u8) {
                return -1;
            }
            let ch = *s as u8;
            s = s.add(1);
            ps = st;
            st = wnext_state(st, ch);
            if st == 0 { return -1; }
            if st >= STOP { break; }
        }

        let wt = match st {
            9 => WType::PTR, 10 => WType::INT, 11 => WType::UINT,
            12 => WType::LLONG, 13 => WType::LONG, 14 => WType::ULONG,
            15 => WType::SHORT, 16 => WType::USHORT, 17 => WType::CHAR,
            18 => WType::UCHAR, 19 => WType::ULLONG, 20 => WType::SIZET,
            21 => WType::IMAX, 22 => WType::UMAX, 23 => WType::PDIFF,
            24 => WType::UIPTR, 25 => WType::DOUBLE, 26 => WType::LDOUBLE,
            27 => WType::NOARG,
            _ => return -1,
        };

        if wt == WType::NOARG {
            continue;
        }

        if f.is_null() {
            continue;
        }

        // 检查 FILE 错误状态
        if ferror(f) { return -1; }

        let terminal = *s.sub(1) as u8;

        // LEFT_ADJ 与 ZERO_PAD 互斥
        if fl & LEFT_ADJ != 0 {
            fl &= !ZERO_PAD;
        }

        // 根据终端类型分发
        match (if ps != 0 && (terminal & 15) == 3 { terminal & !32 } else { terminal }) as u8 {
            b'n' => {
                let ptr = w_pop_arg_ptr(ap);
                match ps {
                    BARE  => { *(ptr as *mut c_int) = cnt; }
                    LPRE  => { *(ptr as *mut i64) = cnt as i64; }
                    LLPRE => { *(ptr as *mut i64) = cnt as i64; }
                    HPRE  => { *(ptr as *mut i16) = cnt as i16; }
                    HHPRE => { *(ptr as *mut i8) = cnt as i8; }
                    ZTPRE => { *(ptr as *mut u64) = cnt as u64; }
                    JPRE  => { *(ptr as *mut u64) = cnt as u64; }
                    _     => { *(ptr as *mut c_int) = cnt; }
                }
                continue;
            }

            b'c' | b'C' => {
                let wc_val = if terminal != b'C' {
                    w_pop_arg_int(ap, wt)
                } else {
                    w_pop_arg_int(ap, WType::UINT)
                };
                let pw = if w_abs < 1 { 1 } else { w_abs };
                wpad(f, b' ', pw, 1, fl);
                let cp = wc_val as u32;
                let mut utf8_buf: [u8; 4] = [0; 4];
                let utf8_len = wc_to_utf8(cp, &mut utf8_buf);
                wout_raw(f, utf8_buf.as_ptr(), utf8_len);
                wpad(f, b' ', pw, 1, fl ^ LEFT_ADJ);
                cnt += pw;
                continue;
            }

            // %s (multibyte string) 或 %S (wide string)
            b's' | b'S' => {
                if wt != WType::PTR {
                    // 如果参数根本不是指针，返回错误
                    return -1;
                }
                let arg_ptr = w_pop_arg_ptr(ap);
                if arg_ptr.is_null() {
                    // 空指针 → "(null)"
                    let null_str = b"(null)";
                    let max_len = if p < 0 { 6 } else { (p as usize).min(6) };
                    let out_len: i32 = max_len as i32;
                    let pw = if w_abs < out_len { out_len } else { w_abs };
                    wpad(f, b' ', pw, out_len, fl);
                    wout_raw(f, null_str.as_ptr(), max_len);
                    wpad(f, b' ', pw, out_len, fl ^ LEFT_ADJ);
                    cnt += pw;
                    continue;
                }

                if terminal == b'S' {
                    // 宽字符串: 直接作为 wchar_t* 处理
                    let ws = arg_ptr as *const c_int;
                    // 计算宽字符串长度
                    let mut ws_len: usize = 0;
                    while *ws.add(ws_len) != 0 {
                        if p >= 0 && ws_len >= p as usize { break; }
                        ws_len += 1;
                    }
                    if p < 0 && *ws.add(ws_len) != 0 {
                        // 实际长度超过 INT_MAX
                        return -1;
                    }
                    let char_len = if p >= 0 && (p as usize) < ws_len { p as usize } else { ws_len };
                    let out_len = char_len as i32;
                    let pw = if w_abs < out_len { out_len } else { w_abs };
                    wpad(f, b' ', pw, out_len, fl);
                    wout(f, ws, char_len);
                    wpad(f, b' ', pw, out_len, fl ^ LEFT_ADJ);
                    cnt += pw;
                } else {
                    // %s — 多字节字符串: 直接写入字节
                    let mb = arg_ptr as *const u8;
                    let mb_len = crate::import::strnlen(mb as *const i8, usize::MAX);
                    if p < 0 && *mb.add(mb_len) != 0 {
                        return -1;
                    }
                    let out_len;
                    let wc_count;
                    if p >= 0 {
                        // 精度 p 限制输出的宽字符数，截断到 p 个完整宽字符的字节偏移
                        let mut byte_off: usize = 0;
                        let mut wc_n: usize = 0;
                        while byte_off < mb_len && wc_n < p as usize {
                            let byte = *mb.add(byte_off);
                            let adv = if byte < 0x80 {
                                1
                            } else if byte < 0xC0 {
                                1
                            } else if byte < 0xE0 {
                                if byte_off + 2 <= mb_len { 2 } else { break; }
                            } else if byte < 0xF0 {
                                if byte_off + 3 <= mb_len { 3 } else { break; }
                            } else {
                                if byte_off + 4 <= mb_len { 4 } else { break; }
                            };
                            byte_off += adv;
                            wc_n += 1;
                        }
                        out_len = byte_off;
                        wc_count = wc_n;
                    } else {
                        out_len = mb_len;
                        wc_count = count_wide_chars(mb, mb_len);
                    }
                    let wc_count_i32 = wc_count as i32;
                    let pw = if w_abs < wc_count_i32 { wc_count_i32 } else { w_abs };
                    wpad(f, b' ', pw, wc_count_i32, fl);
                    wout_raw(f, mb, out_len);
                    wpad(f, b' ', pw, wc_count_i32, fl ^ LEFT_ADJ);
                    cnt += pw;
                }
                continue;
            }

            // 用于数字和浮点的尺寸前缀
            _ => {}
        }

        // ---- 构建窄格式串并委托给 vfprintf ----
        // 确定尺寸前缀（对应 snprintf 中的 sizeprefix）
        let size_prefix: u8 = match (terminal | 32, wt) {
            (b'a' | b'e' | b'f' | b'g', WType::LDOUBLE) => b'L',
            (_, _) => match wt {
                WType::LLONG | WType::ULLONG => b'l',
                WType::LONG | WType::ULONG => b'l',
                WType::SIZET | WType::PDIFF => b'l',
                WType::IMAX | WType::UMAX => b'l',
                WType::SHORT | WType::USHORT => b'h',
                WType::CHAR | WType::UCHAR => b'h',
                _ => 0,
            }
        };

        // 构建窄格式字符串
        let mut charfmt: [u8; 32] = [0; 32];
        let mut ci: usize = 0;
        charfmt[ci] = b'%'; ci += 1;
        // 标志
        if fl & ALT_FORM != 0 { charfmt[ci] = b'#'; ci += 1; }
        if fl & MARK_POS != 0 { charfmt[ci] = b'+'; ci += 1; }
        if fl & LEFT_ADJ != 0 { charfmt[ci] = b'-'; ci += 1; }
        if fl & PAD_POS != 0 { charfmt[ci] = b' '; ci += 1; }
        if fl & ZERO_PAD != 0 && !xp { charfmt[ci] = b'0'; ci += 1; }
        // 宽度
        charfmt[ci] = b'*'; ci += 1;
        // 精度
        if xp {
            charfmt[ci] = b'.'; ci += 1;
            charfmt[ci] = b'*'; ci += 1;
        }
        // 尺寸前缀和转换字符
        if size_prefix != 0 { charfmt[ci] = size_prefix; ci += 1; }
        charfmt[ci] = terminal; ci += 1;
        charfmt[ci] = 0;

        let eff_p = if xp { p } else { -1 };
        let ret: i32;

        match terminal | 32 {
            b'a' | b'e' | b'f' | b'g' => {
                let val = w_pop_arg_double(ap);
                let eff_w = if w_abs > INT_MAX { INT_MAX } else { w_abs };
                ret = w_call_vfprintf_double(f, charfmt.as_ptr(), eff_w, eff_p, val);
            }
            b'd' | b'i' | b'o' | b'u' | b'x' | b'p' => {
                let val = w_pop_arg_int(ap, wt);
                let eff_w = if w_abs > INT_MAX { INT_MAX } else { w_abs };
                // 对于 %p, 改变转换字符为 p
                if terminal == b'p' {
                    charfmt[ci - 1] = b'p';
                }
                ret = w_call_vfprintf_int(f, charfmt.as_ptr(), eff_w, eff_p, val);
            }
            _ => {
                return -1;
            }
        }

        if ret < 0 { return -1; }
        cnt += ret;
    }

    if f.is_null() {
        cnt
    } else {
        cnt
    }
}

// ---------------------------------------------------------------------------
// vfwprintf — 公共入口
// ---------------------------------------------------------------------------

/// vfwprintf — 向 FILE 流写入宽字符格式化输出。
///
/// 当前实现：解析宽格式串，对于字面文本逐 wchar_t → UTF-8 写入，
/// 对于 % 格式说明符委托给 vfprintf 进行数字/浮点格式化。
///
/// [Visibility]: User — <wchar.h> 标准库函数。
#[no_mangle]
pub extern "C" fn vfwprintf(f: *mut FILE, fmt: *const c_int, ap: *mut VaList) -> c_int {
    unsafe {
        if f.is_null() || fmt.is_null() {
            return -1;
        }
        wprintf_core(f, fmt, ap)
    }
}
