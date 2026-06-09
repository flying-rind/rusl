//! vfprintf — 最小格式化输出引擎。
//! 对应 musl src/stdio/vfprintf.c
//!
//! 支持的格式说明符: %d %i %u %x %X %o %s %c %p %n %%
//! 支持的长度修饰符: hh h l ll z t j
//! 支持的标志: - + space # 0
//! 支持宽度和精度
//!
//! 不支持: 浮点数, 宽字符, 位置参数, %m, 千位分隔符

#![allow(unused_imports, unused_variables)]

use super::__towrite::__towrite;
use super::fwrite::__fwritex;
use super::stdio_impl::*;
use core::ffi::{c_char, c_int, c_long, c_longlong, c_void};

// ---------------------------------------------------------------------------
// 标志位
// ---------------------------------------------------------------------------
const ALT_FORM: u32 = 1u32 << (b'#' - b' ');
const ZERO_PAD: u32 = 1u32 << (b'0' - b' ');
const LEFT_ADJ: u32 = 1u32 << (b'-' - b' ');
const PAD_POS: u32  = 1u32 << (b' ' - b' ');
const MARK_POS: u32 = 1u32 << (b'+' - b' ');
const FLAGMASK: u32 = ALT_FORM | ZERO_PAD | LEFT_ADJ | PAD_POS | MARK_POS;

// ---------------------------------------------------------------------------
// 状态机常量
// ---------------------------------------------------------------------------
const BARE: u8   = 0;
const LPRE: u8   = 1;
const LLPRE: u8  = 2;
const HPRE: u8   = 3;
const HHPRE: u8  = 4;
const ZTPRE: u8  = 6;
const JPRE: u8   = 7;
const STOP: u8   = 8;

// ---------------------------------------------------------------------------
// 参数类型
// ---------------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Type {
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
    NOARG  = 27,
}

impl Type {
    #[inline]
    fn from_u8(v: u8) -> Option<Type> {
        match v {
            9  => Some(Type::PTR),
            10 => Some(Type::INT),
            11 => Some(Type::UINT),
            12 => Some(Type::LLONG),
            13 => Some(Type::LONG),
            14 => Some(Type::ULONG),
            15 => Some(Type::SHORT),
            16 => Some(Type::USHORT),
            17 => Some(Type::CHAR),
            18 => Some(Type::UCHAR),
            19 => Some(Type::ULLONG),
            20 => Some(Type::SIZET),
            21 => Some(Type::IMAX),
            22 => Some(Type::UMAX),
            23 => Some(Type::PDIFF),
            24 => Some(Type::UIPTR),
            27 => Some(Type::NOARG),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// 参数值联合体
// ---------------------------------------------------------------------------
union Arg {
    i: u64,
    p: *mut c_void,
}

// ---------------------------------------------------------------------------
// 常量
// ---------------------------------------------------------------------------
const XDIGITS: &[u8; 16] = b"0123456789ABCDEF";

// ===========================================================================
// 内部辅助函数
// ===========================================================================

fn next_state(st: u8, ch: u8) -> u8 {
    match (st, ch) {
        (BARE, b'd') | (BARE, b'i') => Type::INT as u8,
        (BARE, b'o') | (BARE, b'u') | (BARE, b'x') | (BARE, b'X') => Type::UINT as u8,
        (BARE, b'c') => Type::INT as u8,
        (BARE, b's') => Type::PTR as u8,
        (BARE, b'p') => Type::UIPTR as u8,
        (BARE, b'n') => Type::PTR as u8,
        (BARE, b'l') => LPRE,
        (BARE, b'h') => HPRE,
        (BARE, b'z') | (BARE, b't') => ZTPRE,
        (BARE, b'j') => JPRE,

        (LPRE, b'd') | (LPRE, b'i') => Type::LONG as u8,
        (LPRE, b'o') | (LPRE, b'u') | (LPRE, b'x') | (LPRE, b'X') => Type::ULONG as u8,
        (LPRE, b'c') => Type::UINT as u8,
        (LPRE, b's') => Type::PTR as u8,
        (LPRE, b'n') => Type::PTR as u8,
        (LPRE, b'l') => LLPRE,

        (LLPRE, b'd') | (LLPRE, b'i') => Type::LLONG as u8,
        (LLPRE, b'o') | (LLPRE, b'u') | (LLPRE, b'x') | (LLPRE, b'X') => Type::ULLONG as u8,
        (LLPRE, b'n') => Type::PTR as u8,

        (HPRE, b'd') | (HPRE, b'i') => Type::SHORT as u8,
        (HPRE, b'o') | (HPRE, b'u') | (HPRE, b'x') | (HPRE, b'X') => Type::USHORT as u8,
        (HPRE, b'n') => Type::PTR as u8,
        (HPRE, b'h') => HHPRE,

        (HHPRE, b'd') | (HHPRE, b'i') => Type::CHAR as u8,
        (HHPRE, b'o') | (HHPRE, b'u') | (HHPRE, b'x') | (HHPRE, b'X') => Type::UCHAR as u8,
        (HHPRE, b'n') => Type::PTR as u8,

        (ZTPRE, b'd') | (ZTPRE, b'i') => Type::PDIFF as u8,
        (ZTPRE, b'o') | (ZTPRE, b'u') | (ZTPRE, b'x') | (ZTPRE, b'X') => Type::SIZET as u8,
        (ZTPRE, b'n') => Type::PTR as u8,

        (JPRE, b'd') | (JPRE, b'i') => Type::IMAX as u8,
        (JPRE, b'o') | (JPRE, b'u') | (JPRE, b'x') | (JPRE, b'X') => Type::UMAX as u8,
        (JPRE, b'n') => Type::PTR as u8,

        _ => 0,
    }
}

unsafe fn pop_arg(arg: &mut Arg, t: Type, ap: *mut VaList) {
    match t {
        Type::PTR   => { arg.p = va_arg_ptr(ap); }
        Type::INT   => { arg.i = va_arg_int(ap) as i32 as u64; }
        Type::UINT  => { arg.i = va_arg_uint(ap) as u64; }
        Type::LONG  => { arg.i = va_arg_long(ap) as u64; }
        Type::ULONG => { arg.i = va_arg_ulong(ap); }
        Type::LLONG => { arg.i = va_arg_longlong(ap) as u64; }
        Type::ULLONG=> { arg.i = va_arg_ulonglong(ap); }
        Type::SHORT => { arg.i = va_arg_int(ap) as i16 as u64; }
        Type::USHORT=> { arg.i = va_arg_int(ap) as u16 as u64; }
        Type::CHAR  => { arg.i = va_arg_int(ap) as i8 as u64; }
        Type::UCHAR => { arg.i = va_arg_int(ap) as u8 as u64; }
        Type::SIZET => { arg.i = va_arg_ulong(ap); }
        Type::IMAX  => { arg.i = va_arg_longlong(ap) as u64; }
        Type::UMAX  => { arg.i = va_arg_ulonglong(ap); }
        Type::PDIFF => { arg.i = va_arg_long(ap) as u64; }
        Type::UIPTR => { arg.i = va_arg_ptr(ap) as u64; }
        Type::NOARG => {}
    }
}

unsafe fn out(f: *mut FILE, s: *const u8, l: usize) {
    if !ferror(f) {
        __fwritex(s as *const u8, l, f);
    }
}

unsafe fn pad(f: *mut FILE, c: u8, w: i32, l: i32, fl: u32) {
    if fl & (LEFT_ADJ | ZERO_PAD) != 0 || l >= w {
        return;
    }
    let mut remaining = (w - l) as usize;
    let pad_buf = [c; 256];
    while remaining >= 256 {
        out(f, pad_buf.as_ptr(), 256);
        remaining -= 256;
    }
    out(f, pad_buf.as_ptr(), remaining);
}

unsafe fn fmt_u(x: u64, s: *mut u8) -> *mut u8 {
    let mut p = s;
    let mut y = x;
    loop {
        p = p.sub(1);
        *p = b'0' + (y % 10) as u8;
        y /= 10;
        if y == 0 {
            break;
        }
    }
    p
}

unsafe fn fmt_x(x: u64, s: *mut u8, lower: bool) -> *mut u8 {
    let mut p = s;
    let mut v = x;
    loop {
        p = p.sub(1);
        *p = XDIGITS[(v & 15) as usize] | if lower { 32 } else { 0 };
        v >>= 4;
        if v == 0 {
            break;
        }
    }
    p
}

unsafe fn fmt_o(x: u64, s: *mut u8) -> *mut u8 {
    let mut p = s;
    let mut v = x;
    loop {
        p = p.sub(1);
        *p = b'0' + (v & 7) as u8;
        v >>= 3;
        if v == 0 {
            break;
        }
    }
    p
}

unsafe fn getint(s: &mut *const u8) -> i32 {
    let mut i: i32 = 0;
    loop {
        let ch = **s;
        if !ch.is_ascii_digit() {
            break;
        }
        let d = (ch - b'0') as i32;
        if i >= 0 {
            if i > i32::MAX / 10 || (i == i32::MAX / 10 && d > i32::MAX % 10) {
                i = -1;
            } else {
                i = i * 10 + d;
            }
        }
        *s = s.add(1);
    }
    i
}

fn is_flag(ch: u8) -> bool {
    (ch >= b' ' && (ch - b' ') < 32) && (FLAGMASK & (1u32 << (ch - b' '))) != 0
}

fn oob(ch: u8) -> bool {
    ch < b'A' || ch > b'z'
}

// ===========================================================================
// FmtResult — match 分支产出
// ===========================================================================

struct FmtResult {
    a: *const u8,
    z: i32,
    prefix: *const u8,
    pl: i32,
}

// ===========================================================================
// printf_core — 格式化状态机
// ===========================================================================

unsafe fn printf_core(f: *mut FILE, fmt: *const u8, ap: *mut VaList) -> i32 {
    let mut s = fmt;
    let mut cnt: i32 = 0;
    let mut buf = [0u8; 32];
    let z_buf = buf.as_mut_ptr().add(buf.len());

    loop {
        // --- 扫描字面文本 ---
        let a_lit = s;
        while *s != 0 && *s != b'%' {
            s = s.add(1);
        }
        // 处理 %%: z 逐字节前进, s 每次跳过两个 '%'
        let mut z_lit = s;
        while *s == b'%' && *s.add(1) == b'%' {
            z_lit = z_lit.add(1);
            s = s.add(2);
        }
        let l_lit = (z_lit as usize) - (a_lit as usize);
        if l_lit > 0 {
            if l_lit as i32 > i32::MAX - cnt {
                return -1;
            }
            if !f.is_null() {
                out(f, a_lit, l_lit);
            }
            cnt += l_lit as i32;
            if *z_lit == 0 {
                break;
            }
            // 与 musl 一致: 输出字面文本后 continue, s 保持在原位置
            // (不跳过 '%'), 下一轮迭代会以 l_lit=0 进入格式符解析
            continue;
        }
        // l_lit == 0: s 和 z_lit 都指向独立的 '%', 开始解析格式说明符
        if *z_lit == 0 {
            break;
        }
        s = z_lit.add(1); // 跳过 '%'

        // --- 读取标志 ---
        let mut fl: u32 = 0;
        while is_flag(*s) {
            fl |= 1u32 << (*s - b' ');
            s = s.add(1);
        }

        // --- 读取宽度 ---
        let w: i32;
        if *s == b'*' {
            s = s.add(1);
            w = if f.is_null() { 0 } else { va_arg_int(ap) };
            if w < 0 {
                fl |= LEFT_ADJ;
            }
        } else {
            w = getint(&mut s);
            if w < 0 {
                return -1;
            }
        }
        let w = if w < 0 { -w } else { w };

        // --- 读取精度 ---
        let p: i32;
        let xp: bool;
        if *s == b'.' {
            s = s.add(1);
            if *s == b'*' {
                s = s.add(1);
                p = if f.is_null() { 0 } else { va_arg_int(ap) };
                xp = p >= 0;
            } else {
                p = getint(&mut s);
                xp = true;
            }
        } else {
            p = -1;
            xp = false;
        }

        // --- 状态机 ---
        let mut st: u8 = BARE;
        let mut ps: u8;
        loop {
            if oob(*s) {
                return -1;
            }
            let ch = *s;
            s = s.add(1);
            ps = st;
            st = next_state(st, ch);
            if st == 0 {
                return -1;
            }
            if st >= STOP {
                break;
            }
        }

        let t = match Type::from_u8(st) {
            Some(t) => t,
            None => return -1,
        };

        if t == Type::NOARG {
            if f.is_null() {
                continue;
            }
            continue;
        }

        if f.is_null() {
            continue;
        }

        let mut arg = Arg { i: 0 };
        pop_arg(&mut arg, t, ap);

        if ferror(f) {
            return -1;
        }

        let terminal = *s.sub(1);
        if fl & LEFT_ADJ != 0 {
            fl &= !ZERO_PAD;
        }

        let res: FmtResult = match terminal {
            b'n' => {
                let ptr = arg.p;
                match ps {
                    BARE  => { *(ptr as *mut c_int) = cnt; }
                    LPRE  => { *(ptr as *mut c_long) = cnt as c_long; }
                    LLPRE => { *(ptr as *mut c_longlong) = cnt as c_longlong; }
                    HPRE  => { *(ptr as *mut i16) = cnt as i16; }
                    HHPRE => { *(ptr as *mut i8) = cnt as i8; }
                    ZTPRE => { *(ptr as *mut isize) = cnt as isize; }
                    JPRE  => { *(ptr as *mut i64) = cnt as i64; }
                    _     => { *(ptr as *mut c_int) = cnt; }
                }
                continue;
            }

            b'p' => {
                let a = fmt_x(arg.i, z_buf, false);
                let prefix = b"0X\0".as_ptr();
                FmtResult { a, z: (z_buf as usize - a as usize) as i32, prefix, pl: 2 }
            }

            b'x' | b'X' => {
                let lower = terminal & 32 != 0;
                let a = fmt_x(arg.i, z_buf, lower);
                let (prefix, pl) = if arg.i != 0 && (fl & ALT_FORM != 0) {
                    // musl: prefix += t>>4
                    // 'X'>>4=5 starts "0X", 'x'>>4=7 starts "0x"
                    let prefix_ptr = if lower {
                        b"0x\0".as_ptr()
                    } else {
                        b"0X\0".as_ptr()
                    };
                    (prefix_ptr, 2i32)
                } else {
                    (core::ptr::null(), 0i32)
                };
                let z = (z_buf as usize - a as usize) as i32;
                FmtResult { a, z, prefix, pl }
            }

            b'o' => {
                let a = fmt_o(arg.i, z_buf);
                let z = (z_buf as usize - a as usize) as i32;
                let mut z_adj = z;
                if (fl & ALT_FORM != 0) && p < z + 1 {
                    z_adj = z + 1;
                }
                FmtResult { a, z: z_adj, prefix: core::ptr::null(), pl: 0 }
            }

            b'd' | b'i' => {
                let (prefix, pl): (*const u8, i32) = if arg.i > i64::MAX as u64 {
                    arg.i = 0u64.wrapping_sub(arg.i);
                    (b"-\0".as_ptr(), 1)
                } else if fl & MARK_POS != 0 {
                    (b"+\0".as_ptr(), 1)
                } else if fl & PAD_POS != 0 {
                    (b" \0".as_ptr(), 1)
                } else {
                    (core::ptr::null(), 0)
                };
                let a = fmt_u(arg.i, z_buf);
                let z = (z_buf as usize - a as usize) as i32;
                FmtResult { a, z, prefix, pl }
            }

            b'u' => {
                let a = fmt_u(arg.i, z_buf);
                let z = (z_buf as usize - a as usize) as i32;
                FmtResult { a, z, prefix: core::ptr::null(), pl: 0 }
            }

            b'c' => {
                let ptr = z_buf.sub(1);
                *ptr = arg.i as u8;
                FmtResult { a: ptr, z: 1, prefix: core::ptr::null(), pl: 0 }
            }

            b's' => {
                let s_ptr: *const u8 = if arg.p.is_null() {
                    b"(null)\0".as_ptr()
                } else {
                    arg.p as *const u8
                };
                let max_len = if p < 0 { i32::MAX as usize } else { p as usize };
                let len = crate::import::strnlen(s_ptr as *const c_char, max_len);
                if p < 0 && *s_ptr.add(len) != 0 {
                    return -1;
                }
                let z = if p >= 0 && p < len as i32 { p } else { len as i32 };
                FmtResult { a: s_ptr, z, prefix: core::ptr::null(), pl: 0 }
            }

            _ => return -1,
        };

        // --- 统一输出 ---
        let mut z = res.z;
        let pl = res.pl;

        if xp {
            fl &= !ZERO_PAD;
        }

        if arg.i == 0 && p == 0 && matches!(terminal, b'd' | b'i' | b'u' | b'x' | b'X' | b'o') {
            z = 0;
        }

        if terminal == b'o' {
            z = p.max(z);
        } else if !matches!(terminal, b's' | b'c') {
            z = p.max(z + if arg.i == 0 && z == 0 && p > 0 { 1 } else { 0 });
        }

        if z > i32::MAX - pl {
            return -1;
        }
        let w_final = if w < pl + z { pl + z } else { w };
        if w_final > i32::MAX - cnt {
            return -1;
        }

        pad(f, b' ', w_final, pl + z, fl);
        if !res.prefix.is_null() && pl > 0 {
            out(f, res.prefix, pl as usize);
        }
        pad(f, b'0', w_final, pl + z, fl ^ ZERO_PAD);
        // 精度零填充 (在数据之前, 与 musl 一致)
        if z > 0 {
            if z > res.z {
                pad(f, b'0', z, res.z, 0);
            }
            let data_len = res.z.min(z);
            if data_len > 0 {
                out(f, res.a, data_len as usize);
            }
        }
        pad(f, b' ', w_final, pl + z, fl ^ LEFT_ADJ);

        cnt += w_final;
    }

    cnt
}

// ===========================================================================
// vfprintf — 公共接口
// ===========================================================================

#[no_mangle]
pub extern "C" fn vfprintf(
    f: *mut FILE,
    fmt: *const c_char,
    ap: *mut VaList,
) -> c_int {
    // SAFETY: caller guarantees f, fmt, ap are valid pointers per C ABI contract.
    unsafe {
        let f_ref = &mut *f;
        let mut saved_buf: *mut u8 = core::ptr::null_mut();
        let mut internal_buf = [0u8; 80];
        let olderr = f_ref.flags & F_ERR;
        let mut ret: c_int;

        f_ref.flags &= !F_ERR;

        if f_ref.buf_size == 0 {
            saved_buf = f_ref.buf;
            f_ref.buf = internal_buf.as_mut_ptr();
            f_ref.buf_size = internal_buf.len();
            f_ref.wpos = core::ptr::null_mut();
            f_ref.wbase = core::ptr::null_mut();
            f_ref.wend = core::ptr::null_mut();
        }

        if f_ref.wend.is_null() {
            if __towrite(f) != 0 {
                ret = -1;
                if !saved_buf.is_null() {
                    f_ref.buf = saved_buf;
                    f_ref.buf_size = 0;
                    f_ref.wpos = core::ptr::null_mut();
                    f_ref.wbase = core::ptr::null_mut();
                    f_ref.wend = core::ptr::null_mut();
                }
                f_ref.flags |= olderr;
                return ret;
            }
        }

        ret = printf_core(f, fmt as *const u8, ap);

        if !saved_buf.is_null() {
            if let Some(write_fn) = f_ref.write {
                write_fn(f, core::ptr::null(), 0);
            }
            if f_ref.wpos.is_null() {
                ret = -1;
            }
            f_ref.buf = saved_buf;
            f_ref.buf_size = 0;
            f_ref.wpos = core::ptr::null_mut();
            f_ref.wbase = core::ptr::null_mut();
            f_ref.wend = core::ptr::null_mut();
        }

        if ferror(f) {
            ret = -1;
        }
        f_ref.flags |= olderr;
        ret
    }
}

// ===========================================================================
// 测试
// ===========================================================================

#[cfg(test)]
mod tests {
    use rusl_core::test;

        use alloc::boxed::Box;
        use alloc::vec::Vec;
        use alloc::vec;
        use alloc::format;
        use alloc::string::ToString;
        use alloc::string::String;
    use super::vfprintf;
    use super::super::vsnprintf::vsnprintf;
    use super::super::stdio_impl::*;
    use core::ffi::{c_char, c_int, c_void};

    unsafe fn make_va_list(gp_values: &[u64]) -> VaList {
        VaList {
            gp_offset: 0,
            fp_offset: 48,
            overflow_arg_area: core::ptr::null_mut(),
            reg_save_area: gp_values.as_ptr() as *mut c_void,
        }
    }

    test!("empty_direct" {
        // Direct test: no alloc, no format specifiers
        unsafe {
            let mut buf = [0u8; 100];
            let va = make_va_list(&[]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                100,
                b"hello\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(ret, 5);
            assert_eq!(&buf[..5], b"hello");
        }
    });

    unsafe fn fmt(fmt_str: &str, args: &[u64], n: usize) -> String {
        let mut buf = vec![0u8; n];
        let mut va = make_va_list(args);
        let ret = vsnprintf(
            buf.as_mut_ptr() as *mut c_char,
            n,
            fmt_str.as_ptr() as *const c_char,
            &mut va as *mut VaList,
        );
        assert!(ret >= 0, "vsnprintf error: {}", ret);
        let len = (ret as usize).min(n.saturating_sub(1));
        String::from_utf8_lossy(&buf[..len]).to_string()
    }

    test!("empty" {
        unsafe {
            let mut buf = [0u8; 100];
            let va = make_va_list(&[]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                100,
                b"hello\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(ret, 5);
            assert_eq!(&buf[..5], b"hello");
        }
    });

    test!("percent_ab" {
        unsafe {
            let mut buf = [0u8; 100];
            let va = make_va_list(&[]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                100,
                b"ab%%cd\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(ret, 5, "ret = {}", ret);
            assert_eq!(&buf[..5], b"ab%cd");
        }
    });

    test!("percent_trailing" {
        unsafe {
            let mut buf = [0u8; 100];
            let va = make_va_list(&[]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                100,
                b"100%%\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(ret, 4, "ret = {}", ret);
            assert_eq!(&buf[..4], b"100%");
        }
    });

    test!("percent" {
        unsafe {
            let mut buf = [0u8; 100];
            let va = make_va_list(&[]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                100,
                b"100%% done\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            // "100% done" = 9 chars
            assert_eq!(ret, 9, "ret mismatch");
            assert_eq!(&buf[..9], b"100% done");
        }
    });

    test!("string_basic" {
        unsafe {
            let world = b"world\0";
            let mut buf = [0xFEu8; 100];
            let va = make_va_list(&[world.as_ptr() as u64]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                100,
                b"hello %s\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            if ret != 11 {
                panic!("ret = {}, buf[..15] = {:?}", ret, &buf[..15]);
            }
            assert_eq!(ret, 11);
            assert_eq!(&buf[..11], b"hello world");
        }
    });

    test!("int_positive" {
        unsafe {
            let mut buf = [0u8; 100];
            let va = make_va_list(&[42]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                100,
                b"%d\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(ret, 2);
            assert_eq!(&buf[..2], b"42");
        }
    });

    test!("int_negative" {
        unsafe {
            let mut buf = [0u8; 100];
            let va = make_va_list(&[(-1i32) as u64]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                100,
                b"%d\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(ret, 2);
            assert_eq!(&buf[..2], b"-1");
        }
    });

    test!("unsigned_val" {
        unsafe {
            let mut buf = [0u8; 100];
            let va = make_va_list(&[0xFFFFFFFFu64]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                100,
                b"%u\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(ret, 10);
            assert_eq!(&buf[..10], b"4294967295");
        }
    });

    test!("hex_lower" {
        unsafe {
            let mut buf = [0u8; 100];
            let va = make_va_list(&[255u64]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                100,
                b"%x\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(ret, 2);
            assert_eq!(&buf[..2], b"ff");
        }
    });

    test!("hex_upper" {
        unsafe {
            let mut buf = [0u8; 100];
            let va = make_va_list(&[255u64]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                100,
                b"%X\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(ret, 2);
            assert_eq!(&buf[..2], b"FF");
        }
    });

    test!("hex_prefix" {
        unsafe {
            let mut buf = [0u8; 100];
            let va = make_va_list(&[255u64]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                100,
                b"%#x\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(ret, 4);
            assert_eq!(&buf[..4], b"0xff");
        }
    });

    test!("octal" {
        unsafe {
            let mut buf = [0u8; 100];
            let va = make_va_list(&[8u64]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                100,
                b"%o\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(ret, 2);
            assert_eq!(&buf[..2], b"10");
        }
    });

    test!("char_basic" {
        unsafe {
            let mut buf = [0u8; 100];
            let va = make_va_list(&[b'A' as u64]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                100,
                b"%c\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(ret, 1);
            assert_eq!(buf[0], b'A');
        }
    });

    test!("width_right_align" {
        unsafe {
            let mut buf = [0u8; 100];
            let va = make_va_list(&[42]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                100,
                b"%5d\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(ret, 5);
            assert_eq!(&buf[..5], b"   42");
        }
    });

    test!("zero_pad" {
        unsafe {
            let mut buf = [0u8; 100];
            let va = make_va_list(&[42]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                100,
                b"%05d\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(ret, 5);
            assert_eq!(&buf[..5], b"00042");
        }
    });

    test!("left_align" {
        unsafe {
            let mut buf = [0u8; 100];
            let va = make_va_list(&[42]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                100,
                b"%-5d\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(ret, 5);
            assert_eq!(&buf[..5], b"42   ");
        }
    });

    test!("precision_int" {
        unsafe {
            let mut buf = [0u8; 100];
            let va = make_va_list(&[42]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                100,
                b"%.5d\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(ret, 5);
            assert_eq!(&buf[..5], b"00042");
        }
    });

    test!("string_precision" {
        unsafe {
            let mut buf = [0u8; 100];
            let s = b"hello\0".as_ptr() as u64;
            let va = make_va_list(&[s]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                100,
                b"%.3s\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(ret, 3);
            assert_eq!(&buf[..3], b"hel");
        }
    });

    test!("plus_flag" {
        unsafe {
            let mut buf = [0u8; 100];
            let va = make_va_list(&[42]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                100,
                b"%+d\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(ret, 3);
            assert_eq!(&buf[..3], b"+42");
        }
    });

    test!("space_flag" {
        unsafe {
            let mut buf = [0u8; 100];
            let va = make_va_list(&[42]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                100,
                b"% d\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(ret, 3);
            assert_eq!(&buf[..3], b" 42");
        }
    });

    test!("long_modifier" {
        unsafe {
            let mut buf = [0u8; 100];
            let va = make_va_list(&[(-1i64) as u64]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                100,
                b"%ld\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(ret, 2);
            assert_eq!(&buf[..2], b"-1");
        }
    });

    test!("ulong_hex" {
        unsafe {
            let mut buf = [0u8; 100];
            let va = make_va_list(&[0xDEADBEEFu64]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                100,
                b"%lx\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(ret, 8);
            assert_eq!(&buf[..8], b"deadbeef");
        }
    });

    test!("null_string" {
        unsafe {
            let mut buf = [0u8; 100];
            let va = make_va_list(&[0u64]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                100,
                b"%s\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(ret, 6);
            assert_eq!(&buf[..6], b"(null)");
        }
    });

    test!("n_specifier" {
        unsafe {
            let mut count: c_int = 0;
            let mut buf = [0u8; 100];
            let va = make_va_list(&[&mut count as *mut c_int as u64, 42]);
            let ret = vsnprintf(
                buf.as_mut_ptr() as *mut c_char,
                100,
                b"abc%nd\0".as_ptr() as *const c_char,
                &va as *const VaList as *mut VaList,
            );
            assert_eq!(count, 3);
        }
    });
}
#[cfg(test)]
mod tests_basic {
    use rusl_core::test;

    test!("sanity_check" {
        assert_eq!(1 + 1, 2);
    });
}
