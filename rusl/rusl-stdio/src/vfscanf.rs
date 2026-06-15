//! vfscanf — 格式化输入核心引擎。
//! 对应 musl src/stdio/vfscanf.c
//!
//! 支持的格式说明符: %d %i %u %x %X %o %s %c %[ %n %p %%
//! 支持: %* (抑制赋值), 宽度限制, %l 长度修饰符

#![allow(unused_imports, unused_variables)]

use super::stdio_impl::*;
use core::ffi::{c_char, c_int, c_uint};

fn is_space(c: u8) -> bool {
    c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0B || c == 0x0C
}

fn is_digit(c: u8) -> bool {
    c >= b'0' && c <= b'9'
}

fn is_xdigit(c: u8) -> bool {
    is_digit(c) || (c >= b'a' && c <= b'f') || (c >= b'A' && c <= b'F')
}

fn pow10(exp: i32) -> f64 {
    let mut result: f64 = 1.0;
    let mut e = if exp < 0 { -exp } else { exp };
    while e > 0 {
        result *= 10.0;
        e -= 1;
    }
    if exp < 0 { 1.0 / result } else { result }
}

fn pow2(exp: i32) -> f64 {
    let mut result: f64 = 1.0;
    let mut e = if exp < 0 { -exp } else { exp };
    while e > 0 {
        result *= 2.0;
        e -= 1;
    }
    if exp < 0 { 1.0 / result } else { result }
}

fn xdigit_val(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    }
}

/// 从 FILE 读一个字符，内部持有一字符回退。
struct ScanState {
    f: *mut FILE,
    chars_read: usize,
    ungot: c_int, // -1 表示无回退字符
}

impl ScanState {
    unsafe fn new(f: *mut FILE) -> Self {
        ScanState { f, chars_read: 0, ungot: -1 }
    }

    unsafe fn getc(&mut self) -> c_int {
        if self.ungot >= 0 {
            let c = self.ungot;
            self.ungot = -1;
            self.chars_read += 1;
            return c;
        }
        let c = super::getc_unlocked::getc_unlocked(self.f);
        if c >= 0 {
            self.chars_read += 1;
        }
        c
    }

    unsafe fn unget(&mut self, c: c_int) {
        if c >= 0 {
            self.ungot = c;
            self.chars_read -= 1;
        }
    }

    /// 跳过空白，返回第一个非空白字符（已消费但可 unget 回退）
    unsafe fn skip_space(&mut self) -> Option<c_int> {
        loop {
            let c = self.getc();
            if c < 0 {
                return None;
            }
            if !is_space(c as u8) {
                return Some(c);
            }
        }
    }
}

/// 内部解析函数，返回 (matches, st) 以便外层做清理。
unsafe fn do_vfscanf(
    f: *mut FILE,
    fmt: *const c_char,
    ap: *mut VaList,
    mut st: ScanState,
) -> (c_int, ScanState) {
    let mut matches: c_int = 0;
    let mut fmt_idx: usize = 0;

    // 检查是否有输入可用
    let first = st.getc();
    if first < 0 {
        return (-1, st);
    }
    st.unget(first);

    loop {
        let fmt_ch = *fmt.add(fmt_idx);
        if fmt_ch == 0 {
            break;
        }

        // ---- 非 '%' : 字面量匹配 ----
        if fmt_ch as u8 != b'%' {
            if is_space(fmt_ch as u8) {
                fmt_idx += 1;
                let c = st.skip_space();
                if c.is_none() {
                    return if matches > 0 { (matches, st) } else { (-1, st) };
                }
                st.unget(c.unwrap());
            } else {
                fmt_idx += 1;
                let c = st.getc();
                if c < 0 {
                    return if matches > 0 { (matches, st) } else { (-1, st) };
                }
                if c as u8 != fmt_ch as u8 {
                    st.unget(c);
                    return (matches, st);
                }
            }
            continue;
        }

        // ---- '%' 说明符 ----
        fmt_idx += 1;
        let spec_start = *fmt.add(fmt_idx);
        if spec_start == 0 {
            break;
        }

        // %%
        if spec_start as u8 == b'%' {
            fmt_idx += 1;
            let c = st.getc();
            if c < 0 { return if matches > 0 { (matches, st) } else { (-1, st) }; }
            if c != b'%' as c_int {
                st.unget(c);
                return (matches, st);
            }
            continue;
        }

        // 抑制赋值 *
        let suppress = spec_start as u8 == b'*';
        if suppress {
            fmt_idx += 1;
        }

        // 最大宽度
        let mut max_width: usize = usize::MAX;
        let mut spec_idx = fmt_idx;
        let mut spec_ch = *fmt.add(spec_idx);
        if is_digit(spec_ch as u8) {
            max_width = 0;
            while spec_ch != 0 && is_digit(spec_ch as u8) {
                max_width = max_width * 10 + (spec_ch as u8 - b'0') as usize;
                spec_idx += 1;
                spec_ch = *fmt.add(spec_idx);
            }
        }
        if spec_ch == 0 { break; }

        // 长度修饰符
        let lflag = spec_ch as u8 == b'l';
        if lflag {
            spec_idx += 1;
            spec_ch = *fmt.add(spec_idx);
            if spec_ch == 0 { break; }
        }
        let spec = spec_ch as u8;
        fmt_idx = spec_idx + 1;

        // 跳过空白 (%c / %[ / %n 除外)
        let c: c_int;
        if spec != b'c' && spec != b'[' && spec != b'n' {
            let space_result = st.skip_space();
            if space_result.is_none() {
                return if matches > 0 { (matches, st) } else { (-1, st) };
            }
            c = space_result.unwrap();
        } else if spec == b'n' {
            // %n — 存储当前已消费字符数（不计入匹配数）
            if !suppress {
                let dest = crate::stdio_impl::va_arg_ptr(ap) as *mut c_int;
                *dest = st.chars_read as c_int;
            }
            continue;
        } else {
            // %c 或 %[ 不跳过空白
            c = st.getc();
            if c < 0 {
                return if matches > 0 { (matches, st) } else { (-1, st) };
            }
        }

        match spec {
            // ---- %d / %i / %u ----
            b'd' | b'i' | b'u' => {
                let base: i64 = if spec == b'i' { 0 } else { 10 };
                let mut val: u64 = 0;
                let mut sign: i64 = 1;
                let mut any = false;
                let mut cur = c;
                let mut width_rem = max_width;

                if cur == b'-' as c_int || cur == b'+' as c_int {
                    if cur == b'-' as c_int { sign = -1; }
                    if width_rem > 0 { width_rem -= 1; }
                    if width_rem > 0 {
                        cur = st.getc();
                    } else {
                        st.unget(cur);
                        return (matches, st);
                    }
                }

                let base_actual: i64;
                let mut consumed_prefix = false;
                if base == 0 && cur == b'0' as c_int {
                    if width_rem > 0 { width_rem -= 1; }
                    consumed_prefix = true;
                    if width_rem > 0 {
                        let next = st.getc();
                        if next >= 0 && (next as u8 == b'x' || next as u8 == b'X') {
                            if width_rem > 0 { width_rem -= 1; }
                            if width_rem > 0 {
                                cur = st.getc();
                            } else {
                                // width 耗尽在 "0x" 上，无 hex 数字 → 匹配失败
                                cur = -1;
                            }
                            base_actual = 16;
                        } else {
                            st.unget(next);
                            any = true;
                            base_actual = 8;
                        }
                    } else {
                        any = true;
                        base_actual = 8;
                    }
                } else {
                    base_actual = base;
                }

                while cur >= 0 && width_rem > 0 {
                    let digit = match base_actual {
                        8 if cur as u8 >= b'0' && cur as u8 <= b'7' =>
                            { Some((cur as u8 - b'0') as u64) }
                        10 if is_digit(cur as u8) =>
                            { Some((cur as u8 - b'0') as u64) }
                        16 if is_xdigit(cur as u8) =>
                            { Some(xdigit_val(cur as u8) as u64) }
                        _ => None,
                    };
                    if let Some(d) = digit {
                        any = true;
                        val = val.wrapping_mul(base_actual as u64).wrapping_add(d);
                        width_rem -= 1;
                        cur = st.getc();
                    } else {
                        break;
                    }
                }

                if cur >= 0 { st.unget(cur); }
                if !any {
                    if !consumed_prefix {
                        st.unget(c);
                    }
                    return (matches, st);
                }

                if !suppress {
                    let dest = crate::stdio_impl::va_arg_ptr(ap);
                    if lflag {
                        *(dest as *mut i64) = sign * val as i64;
                    } else if spec == b'u' {
                        *(dest as *mut c_uint) = val as c_uint;
                    } else {
                        *(dest as *mut c_int) = (sign * val as i64) as c_int;
                    }
                    matches += 1;
                }
            }

            // ---- %x / %X / %o ----
            b'x' | b'X' | b'o' => {
                let base_actual: i64 = if spec == b'o' { 8 } else { 16 };
                let mut val: u64 = 0;
                let mut any = false;
                let mut cur = c;
                let mut width_rem = max_width;

                let mut consumed_prefix = false;
                if spec != b'o' && cur == b'0' as c_int && width_rem > 0 {
                    width_rem -= 1;
                    consumed_prefix = true;
                    if width_rem > 0 {
                        let c2 = st.getc();
                        if c2 >= 0 && (c2 as u8 == b'x' || c2 as u8 == b'X') {
                            width_rem -= 1;
                            if width_rem > 0 {
                                cur = st.getc();
                            } else {
                                // width 耗尽在 "0x" 上，无 hex 数字 → 匹配失败
                                cur = -1;
                            }
                        } else {
                            st.unget(c2);
                            any = true;
                            cur = b'0' as c_int;
                            width_rem += 1;
                        }
                    } else {
                        any = true;
                    }
                }

                while cur >= 0 && width_rem > 0 {
                    let digit = match base_actual {
                        8 if cur as u8 >= b'0' && cur as u8 <= b'7' =>
                            { Some((cur as u8 - b'0') as u64) }
                        16 if is_xdigit(cur as u8) =>
                            { Some(xdigit_val(cur as u8) as u64) }
                        _ => None,
                    };
                    if let Some(d) = digit {
                        any = true;
                        val = val.wrapping_mul(base_actual as u64).wrapping_add(d);
                        width_rem -= 1;
                        cur = st.getc();
                    } else {
                        break;
                    }
                }

                if cur >= 0 { st.unget(cur); }
                if !any {
                    if !consumed_prefix {
                        st.unget(c);
                    }
                    return (matches, st);
                }

                if !suppress {
                    let dest = crate::stdio_impl::va_arg_ptr(ap);
                    if lflag {
                        *(dest as *mut i64) = val as i64;
                    } else {
                        *(dest as *mut c_uint) = val as c_uint;
                    }
                    matches += 1;
                }
            }

            // ---- %s ----
            b's' => {
                let mut cur = c;
                let mut width_rem = max_width;
                if suppress {
                    while cur >= 0 && width_rem > 0 && !is_space(cur as u8) {
                        width_rem -= 1;
                        cur = st.getc();
                    }
                    if cur >= 0 { st.unget(cur); }
                } else {
                    let dest = crate::stdio_impl::va_arg_ptr(ap) as *mut u8;
                    let mut i: usize = 0;
                    while cur >= 0 && width_rem > 0 && !is_space(cur as u8) {
                        *dest.add(i) = cur as u8;
                        i += 1;
                        width_rem -= 1;
                        cur = st.getc();
                    }
                    *dest.add(i) = 0;
                    if cur >= 0 { st.unget(cur); }
                    if i == 0 {
                        st.unget(c);
                        return (matches, st);
                    }
                    matches += 1;
                }
            }

            // ---- %c ----
            b'c' => {
                if max_width == usize::MAX { max_width = 1; }
                let mut cur = c;
                if suppress {
                    if cur < 0 { continue; }
                    if max_width == 1 {
                        matches += 1;
                        continue;
                    }
                    let mut width_rem = max_width - 1;
                    while cur >= 0 && width_rem > 0 {
                        width_rem -= 1;
                        cur = st.getc();
                    }
                    if width_rem > 0 { continue; } // 匹配失败
                    if cur >= 0 { st.unget(cur); }
                    matches += 1;
                } else {
                    let dest = crate::stdio_impl::va_arg_ptr(ap) as *mut u8;
                    let mut i: usize = 0;
                    *dest.add(i) = cur as u8;
                    i += 1;
                    if max_width == 1 {
                        matches += 1;
                        continue;
                    }
                    let mut width_rem = max_width - 1;
                    cur = st.getc();
                    while cur >= 0 && width_rem > 0 {
                        *dest.add(i) = cur as u8;
                        i += 1;
                        width_rem -= 1;
                        cur = st.getc();
                    }
                    if width_rem > 0 { continue; } // 匹配失败
                    if cur >= 0 { st.unget(cur); }
                    matches += 1;
                }
            }

            // ---- %[ 扫描集 ----
            b'[' => {
                let negated = *fmt.add(fmt_idx) as u8 == b'^';
                if negated { fmt_idx += 1; }

                let mut scanset: [u8; 32] = [0u8; 32];
                let first_bracket_ch = *fmt.add(fmt_idx);
                if first_bracket_ch as u8 == b']' {
                    scanset[(b']' as usize) / 8] |= 1 << ((b']' as usize) % 8);
                    fmt_idx += 1;
                }

                loop {
                    let ch = *fmt.add(fmt_idx);
                    if ch == 0 || ch as u8 == b']' { break; }
                    fmt_idx += 1;
                    if *fmt.add(fmt_idx) as u8 == b'-'
                        && *fmt.add(fmt_idx + 1) as u8 != b']'
                        && *fmt.add(fmt_idx + 1) != 0
                    {
                        fmt_idx += 1;
                        let end = *fmt.add(fmt_idx) as u8;
                        let start = ch as u8;
                        let range_start = start.min(end);
                        let range_end = start.max(end);
                        for byte in range_start..=range_end {
                            scanset[byte as usize / 8] |= 1 << (byte as usize % 8);
                        }
                    } else {
                        scanset[ch as u8 as usize / 8] |= 1 << (ch as u8 as usize % 8);
                    }
                }
                if *fmt.add(fmt_idx) == 0 { break; }
                fmt_idx += 1;

                let mut cur = c;
                let mut width_rem = max_width;
                let matched = |byte: u8| -> bool {
                    let in_set = scanset[byte as usize / 8] & (1 << (byte as usize % 8)) != 0;
                    if negated { !in_set } else { in_set }
                };

                if suppress {
                    while cur >= 0 && width_rem > 0 && matched(cur as u8) {
                        width_rem -= 1;
                        cur = st.getc();
                    }
                    if cur >= 0 { st.unget(cur); }
                } else {
                    let dest = crate::stdio_impl::va_arg_ptr(ap) as *mut u8;
                    let mut i: usize = 0;
                    while cur >= 0 && width_rem > 0 && matched(cur as u8) {
                        *dest.add(i) = cur as u8;
                        i += 1;
                        width_rem -= 1;
                        cur = st.getc();
                    }
                    *dest.add(i) = 0;
                    if cur >= 0 { st.unget(cur); }
                    if i == 0 {
                        st.unget(c);
                        return (matches, st);
                    }
                    matches += 1;
                }
            }

            // ---- %f / %e / %g / %a 浮点数 ----
            b'f' | b'e' | b'g' | b'a' => {
                let mut cur = c;
                let mut val: f64 = 0.0;
                let mut sign: f64 = 1.0;
                let mut any = false;

                // 符号
                if cur == b'-' as c_int || cur == b'+' as c_int {
                    if cur == b'-' as c_int { sign = -1.0; }
                    cur = st.getc();
                }

                // 检测 hexfloat 0x 前缀
                let is_hex = spec == b'a'
                    || (cur == b'0' as c_int && {
                        let next = st.getc();
                        let hex = next >= 0 && (next as u8 == b'x' || next as u8 == b'X');
                        st.unget(next);
                        hex
                    });

                if is_hex {
                    // 消费 0x
                    if cur == b'0' as c_int {
                        cur = st.getc(); // consume '0'
                        cur = st.getc(); // consume 'x', get first char after prefix
                    }

                    let mut hex_val: u64 = 0;
                    while cur >= 0 && is_xdigit(cur as u8) {
                        any = true;
                        hex_val = hex_val.wrapping_mul(16).wrapping_add(xdigit_val(cur as u8) as u64);
                        cur = st.getc();
                    }

                    // 可选的小数部分
                    let mut frac_digits: i32 = 0;
                    if cur == b'.' as c_int {
                        cur = st.getc();
                        while cur >= 0 && is_xdigit(cur as u8) {
                            any = true;
                            hex_val = hex_val.wrapping_mul(16).wrapping_add(xdigit_val(cur as u8) as u64);
                            frac_digits += 1;
                            cur = st.getc();
                        }
                    }

                    // 指数 p/P
                    let mut exp: i32 = 0;
                    let mut exp_sign: i32 = 1;
                    let mut has_exp_digit = false;
                    if cur == b'p' as c_int || cur == b'P' as c_int {
                        cur = st.getc();
                        if cur == b'-' as c_int { exp_sign = -1; cur = st.getc(); }
                        else if cur == b'+' as c_int { cur = st.getc(); }
                        while cur >= 0 && is_digit(cur as u8) {
                            has_exp_digit = true;
                            exp = exp * 10 + (cur as u8 - b'0') as i32;
                            cur = st.getc();
                        }
                        if !has_exp_digit {
                            // p/P 后无指数数字 → hexfloat 无效
                            // 注意: "0x1p" 前缀已被消费，不能 unget c
                            if cur >= 0 { st.unget(cur); }
                            return (matches, st);
                        }
                    }

                    if any {
                        val = (hex_val as f64)
                            * pow2(exp_sign * exp - 4 * frac_digits);
                    }
                } else {
                    // 十进制浮点
                    // 整数部分
                    let mut int_part: f64 = 0.0;
                    while cur >= 0 && is_digit(cur as u8) {
                        any = true;
                        int_part = int_part * 10.0 + (cur as u8 - b'0') as f64;
                        cur = st.getc();
                    }

                    // 小数部分
                    let mut frac_part: f64 = 0.0;
                    let mut frac_div: f64 = 1.0;
                    if cur == b'.' as c_int {
                        cur = st.getc();
                        while cur >= 0 && is_digit(cur as u8) {
                            any = true;
                            frac_part = frac_part * 10.0 + (cur as u8 - b'0') as f64;
                            frac_div *= 10.0;
                            cur = st.getc();
                        }
                    }

                    val = int_part + frac_part / frac_div;

                    // 指数 e/E
                    if cur == b'e' as c_int || cur == b'E' as c_int {
                        cur = st.getc();
                        let mut exp: i32 = 0;
                        let mut exp_sign: i32 = 1;
                        if cur == b'-' as c_int { exp_sign = -1; cur = st.getc(); }
                        else if cur == b'+' as c_int { cur = st.getc(); }
                        if cur >= 0 && is_digit(cur as u8) {
                            while cur >= 0 && is_digit(cur as u8) {
                                exp = exp * 10 + (cur as u8 - b'0') as i32;
                                cur = st.getc();
                            }
                            val *= pow10(exp_sign * exp);
                        } else {
                            // e/E 后无数字: 整个浮点匹配失败
                            if cur >= 0 { st.unget(cur); }
                            st.unget(c);
                            return (matches, st);
                        }
                    }
                }

                if cur >= 0 { st.unget(cur); }
                if !any {
                    st.unget(c);
                    return (matches, st);
                }

                if !suppress {
                    let dest = crate::stdio_impl::va_arg_ptr(ap);
                    if lflag {
                        *(dest as *mut f64) = sign * val;
                    } else {
                        *(dest as *mut f32) = (sign * val) as f32;
                    }
                    matches += 1;
                }
            }

            _ => {
                st.unget(c);
                return (matches, st);
            }
        }
    }

    (matches, st)
}

/// vfscanf — 从 FILE 流读取格式化输入。
#[no_mangle]
pub extern "C" fn vfscanf(f: *mut FILE, fmt: *const c_char, ap: *mut VaList) -> c_int {
    unsafe {
        if f.is_null() || fmt.is_null() {
            return -1;
        }
        if *fmt == 0 {
            return 0;
        }

        let st = ScanState::new(f);
        let (matches, st) = do_vfscanf(f, fmt, ap, st);

        // 将最后一个未消费字符推回 FILE (仅当有真实缓冲区时)
        if st.ungot >= 0 && (*f).buf_size > 0 {
            super::ungetc::ungetc(st.ungot, f);
        }

        matches
    }
}

/// __isoc99_vfscanf — vfscanf 的 C99 兼容弱别名。
#[no_mangle]
pub extern "C" fn __isoc99_vfscanf(f: *mut FILE, fmt: *const c_char, ap: *mut VaList) -> c_int {
    vfscanf(f, fmt, ap)
}
