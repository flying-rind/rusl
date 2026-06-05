//! intscan 模块 — 整数流解析引擎。
//!
//! 本模块定义了 `__intscan` 函数，是 rusl 中所有文本到整数转换的
//! 统一后端：`strtol`/`strtoul`/`strtoll`/`strtoull` 和 `scanf` 的
//! `%d`/`%i`/`%u`/`%x`/`%o` 转换都委托给它。
//!
//! # 模块可见性
//!
//! `pub(crate)` — 仅在 rusl crate 内部使用。

use core::ffi::{c_int, c_uint, c_ulonglong};
use rusl_core::errno::__errno_location;

/// 内部 FILE 类型的占位符声明。
///
/// 完整定义位于 `crate::stdio` 模块。此占位符允许 intscan
/// 模块在 stdio 模块尚未完全实现时即可编译。
///
/// TODO: 当 stdio 模块就绪后，替换为 `use crate::stdio::File;`
#[repr(C)]
pub struct File {
    _private: [u8; 0], // 零大小占位，实际布局由 stdio 模块定义
}

/// EINVAL — 无效参数错误码 (musl: 22)
const EINVAL: c_int = 22;

/// ERANGE — 结果超出范围错误码 (musl: 34)
const ERANGE: c_int = 34;

// ---------------------------------------------------------------------------
// 256 字节数字值查找表
// ---------------------------------------------------------------------------

/// 编译期生成 257 字节的字符分类查找表。
///
/// 索引方式：`VAL_TABLE[1 + c]`，其中 `c` 是待查询字节（0-255）。
/// * 数字字符 `'0'..='9'` → 值 `0..9`
/// * 大写字母 `'A'..='Z'` → 值 `10..35`
/// * 小写字母 `'a'..='z'` → 值 `10..35`
/// * 其他字符 → `0xFF`（无效标记）
const fn build_val_table() -> [u8; 257] {
    let mut table = [0xFFu8; 257];
    let mut d: u8 = 0;
    while d < 10 {
        table[1 + b'0' as usize + d as usize] = d;
        d += 1;
    }
    d = 0;
    while d < 26 {
        table[1 + b'A' as usize + d as usize] = 10 + d;
        table[1 + b'a' as usize + d as usize] = 10 + d;
        d += 1;
    }
    table
}

/// 静态字符分类查找表。
static VAL_TABLE: [u8; 257] = build_val_table();

/// 获取字节 `c` 对应的进制数值。无效字符返回 0xFF。
#[inline]
fn val(c: u8) -> u8 {
    // SAFETY: c 在 0..=255，索引 1+c 在 1..=256，表含 257 元素
    unsafe { *VAL_TABLE.as_ptr().add(1usize + c as usize) }
}

// ---------------------------------------------------------------------------
// C 语言 isspace 等效
// ---------------------------------------------------------------------------

#[inline]
fn is_space(c: u8) -> bool {
    c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == b'\x0c'
}

// ---------------------------------------------------------------------------
// 2 的幂次进制移位位数计算
// ---------------------------------------------------------------------------

/// 计算 2 的幂次进制的移位位数（log2(base)）。
///
/// ```c
/// int bs = "\0\1\2\4\7\3\6\5"[(0x17*base)>>5&7];
/// ```
#[inline]
fn shift_bits(base: u32) -> u32 {
    const SHIFT_MAP: [u32; 8] = [0, 1, 2, 4, 7, 3, 6, 5];
    let idx = ((0x17u32).wrapping_mul(base) >> 5) & 7;
    SHIFT_MAP[idx as usize]
}

// ---------------------------------------------------------------------------
// 字节输入游标 — 模拟 C 的 shgetc/shunget
// ---------------------------------------------------------------------------

/// 字节输入游标，追踪当前读取位置。
struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Cursor { data, pos: 0 }
    }

    /// 读取一个字节并推进；达到末尾时返回 0。
    fn getc(&mut self) -> u8 {
        if self.pos < self.data.len() {
            let c = self.data[self.pos];
            self.pos += 1;
            c
        } else {
            self.pos += 1; // 即使没有数据也推进，模拟 C 中 shgetc 越过 '\0'
            0
        }
    }

    /// 回退一个字节（不超过起点）。
    fn unget(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
        }
    }

    /// 已消费（游标已越过）的字节数。
    fn consumed(&self) -> usize {
        self.pos
    }
}

// ---------------------------------------------------------------------------
// __intscan_bytes — 核心整数解析引擎
// ---------------------------------------------------------------------------

/// 内部整数流解析引擎（字节切片版本）。
///
/// 直接对 `&[u8]` 字节切片进行整数解析，返回 `(value, consumed)`。
/// 与 C 函数 `__intscan(FILE *f, unsigned base, int pok, unsigned long long lim)`
/// 语义完全一致。
///
/// 此函数设置全局 `errno`：`EINVAL`（非法基数/无有效数字）、`ERANGE`（溢出）。
pub(crate) fn __intscan_bytes(
    input: &[u8],
    base: c_uint,
    pok: c_int,
    lim: c_ulonglong,
) -> (c_ulonglong, usize) {
    let base_val = base as u32;

    // 非法基数
    if base_val > 36 || base_val == 1 {
        unsafe { *__errno_location() = EINVAL; }
        return (0, 0);
    }

    if input.is_empty() {
        unsafe { *__errno_location() = EINVAL; }
        return (0, 0);
    }

    let mut cur = Cursor::new(input);

    // Step 1: 跳过空白字符
    let mut c = cur.getc();
    while is_space(c) && cur.pos <= input.len() {
        c = cur.getc();
    }

    // Step 2: 正负号
    let mut neg: u64 = 0;
    if c == b'+' || c == b'-' {
        neg = if c == b'-' { u64::MAX } else { 0 };
        c = cur.getc();
    }

    // Step 3: 进制前缀检测
    let actual_base: u32;
    if (base_val == 0 || base_val == 16) && c == b'0' {
        c = cur.getc();
        if (c | 32) == b'x' {
            // "0x" / "0X"
            c = cur.getc();
            if val(c) >= 16 {
                // "0x" 后无有效十六进制数字
                cur.unget(); // 回退 'x' 后的字符
                if pok != 0 {
                    cur.unget(); // pok 模式也回退 'x'
                }
                unsafe { *__errno_location() = EINVAL; }
                return (0, cur.consumed());
            }
            actual_base = 16;
        } else if base_val == 0 {
            actual_base = 8;
        } else {
            actual_base = base_val;
        }
    } else {
        if base_val == 0 {
            actual_base = 10;
        } else {
            actual_base = base_val;
        }
        // 检查首字符是否有效
        if val(c) >= actual_base as u8 {
            cur.unget();
            unsafe { *__errno_location() = EINVAL; }
            return (0, cur.consumed());
        }
    }

    // Step 4: 按进制分派解析
    let (y, overflow) = if actual_base == 10 {
        parse_decimal(&mut cur, c)
    } else if (actual_base & (actual_base - 1)) == 0 {
        parse_power_of_two(&mut cur, c, actual_base, shift_bits(actual_base))
    } else {
        parse_generic(&mut cur, c, actual_base)
    };

    // Step 5: 溢出
    let mut y = y;
    if overflow {
        unsafe { *__errno_location() = ERANGE; }
        y = lim;
        if (lim & 1) != 0 {
            neg = 0;
        }
    }

    // Step 6: 上限检查
    let consumed = cur.consumed();
    if y >= lim {
        if (lim & 1) == 0 && neg == 0 {
            // 有符号正溢出
            unsafe { *__errno_location() = ERANGE; }
            return (lim - 1, consumed);
        } else if y > lim {
            unsafe { *__errno_location() = ERANGE; }
            return (lim, consumed);
        }
    }

    // Step 7: 取反
    let result = (y ^ neg).wrapping_sub(neg);
    (result, consumed)
}

// ---------------------------------------------------------------------------
// 各进制解析路径
// ---------------------------------------------------------------------------

/// 十进制：u32 快路径 → u64 慢路径。返回 (累加值, 溢出标记)。
fn parse_decimal(cur: &mut Cursor, first_c: u8) -> (u64, bool) {
    let mut c = first_c;
    let mut x: u32 = 0;

    // u32 阶段
    while c.wrapping_sub(b'0') < 10 && x <= u32::MAX / 10 - 1 {
        x = x * 10 + (c - b'0') as u32;
        c = cur.getc();
    }

    let mut y = x as u64;

    // u64 阶段
    while c.wrapping_sub(b'0') < 10
        && y <= u64::MAX / 10
        && 10u64 * y <= u64::MAX - (c - b'0') as u64
    {
        y = y * 10 + (c - b'0') as u64;
        c = cur.getc();
    }

    if c.wrapping_sub(b'0') < 10 {
        // 溢出：还有更多合法数字但无法容纳
        while c.wrapping_sub(b'0') < 10 {
            c = cur.getc();
        }
        cur.unget();
        return (y, true);
    }

    cur.unget();
    (y, false)
}

/// 2 的幂次进制：移位路径。返回 (累加值, 溢出标记)。
fn parse_power_of_two(cur: &mut Cursor, first_c: u8, base: u32, bs: u32) -> (u64, bool) {
    let mut c = first_c;
    let mut x: u32 = 0;

    while val(c) < base as u8 && x <= u32::MAX / 32 {
        x = x << bs | (val(c) as u32);
        c = cur.getc();
    }

    let mut y = x as u64;

    while val(c) < base as u8 && y <= u64::MAX >> bs {
        y = y << bs | (val(c) as u64);
        c = cur.getc();
    }

    if val(c) < base as u8 {
        while val(c) < base as u8 {
            c = cur.getc();
        }
        cur.unget();
        return (y, true);
    }

    cur.unget();
    (y, false)
}

/// 通用进制：乘法路径。返回 (累加值, 溢出标记)。
fn parse_generic(cur: &mut Cursor, first_c: u8, base: u32) -> (u64, bool) {
    let mut c = first_c;
    let mut x: u32 = 0;

    while val(c) < base as u8 && x <= u32::MAX / 36 - 1 {
        x = x * base + (val(c) as u32);
        c = cur.getc();
    }

    let mut y = x as u64;

    while val(c) < base as u8
        && y <= u64::MAX / (base as u64)
        && (base as u64) * y <= u64::MAX - (val(c) as u64)
    {
        y = y * (base as u64) + (val(c) as u64);
        c = cur.getc();
    }

    if val(c) < base as u8 {
        while val(c) < base as u8 {
            c = cur.getc();
        }
        cur.unget();
        return (y, true);
    }

    cur.unget();
    (y, false)
}

// ---------------------------------------------------------------------------
// __intscan — File 流版本（占位实现）
// ---------------------------------------------------------------------------

/// 内部整数流解析引擎（File 流版本）。
pub fn __intscan(
    _f: *mut File,
    base: c_uint,
    _pok: c_int,
    lim: c_ulonglong,
) -> c_ulonglong {
    let _ = (_f, _pok);
    if (base as u32) > 36 || base == 1 {
        unsafe { *__errno_location() = EINVAL; }
        return 0;
    }
    let _ = lim;
    unsafe { *__errno_location() = EINVAL; }
    0
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use rusl_core::test;
    use super::*;

    fn check(input: &[u8], base: c_uint, expected_val: c_ulonglong, expected_consumed: usize) {
        let (val, consumed) = __intscan_bytes(input, base, 0, u64::MAX);
        assert_eq!(val, expected_val,
            "Value mismatch for {:?} base {}: got {}, expected {}",
            input, base, val, expected_val);
        assert_eq!(consumed, expected_consumed,
            "Consumed mismatch for {:?} base {}: got {}, expected {}",
            input, base, consumed, expected_consumed);
    }

    test!("intscan_base10_positive" {
        check(b"123", 10, 123, 3);
        check(b"0", 10, 0, 1);
        check(b"42", 10, 42, 2);
        check(b"999999", 10, 999999, 6);
    });

    test!("intscan_base10_negative" {
        let (val, consumed) = __intscan_bytes(b"-123", 10, 0, 1u64 << 63);
        assert_eq!(consumed, 4);
        assert_eq!(val as i64, -123);
    });

    test!("intscan_base10_positive_sign" {
        check(b"+42", 10, 42, 3);
    });

    test!("intscan_hexadecimal" {
        check(b"ff", 16, 255, 2);
        check(b"FF", 16, 255, 2);
        check(b"1a", 16, 26, 2);
        check(b"A", 16, 10, 1);
    });

    test!("intscan_hex_prefix" {
        check(b"0xff", 0, 255, 4);
        check(b"0x1A", 0, 26, 4);
        check(b"0X2B", 0, 43, 4);
    });

    test!("intscan_octal" {
        check(b"77", 8, 63, 2);
        check(b"10", 8, 8, 2);
        check(b"077", 0, 63, 3);
        check(b"010", 0, 8, 3);
    });

    test!("intscan_auto_decimal" {
        check(b"123", 0, 123, 3);
        check(b"42", 0, 42, 2);
    });

    test!("intscan_whitespace_skip" {
        check(b"  42", 10, 42, 4);
        check(b"\t\n 123", 10, 123, 6);
        check(b" \t 0xff", 0, 255, 7);
    });

    test!("intscan_invalid_base" {
        assert_eq!(__intscan_bytes(b"123", 1, 0, u64::MAX).0, 0);
        assert_eq!(__intscan_bytes(b"123", 37, 0, u64::MAX).0, 0);
    });

    test!("intscan_no_digits" {
        let (val, consumed) = __intscan_bytes(b"abc", 10, 0, u64::MAX);
        assert_eq!(val, 0);
        assert_eq!(consumed, 0);
    });

    test!("intscan_partial_number" {
        check(b"123abc", 10, 123, 3);
        check(b"42 hello", 10, 42, 2);
        check(b"0xffz", 0, 255, 4);
    });

    test!("intscan_empty_input" {
        assert_eq!(__intscan_bytes(b"", 10, 0, u64::MAX).0, 0);
    });

    test!("intscan_power_of_two_bases" {
        check(b"1010", 2, 10, 4);
        check(b"23", 4, 11, 2);
        check(b"17", 8, 15, 2);
    });

    test!("intscan_strange_base" {
        check(b"z", 36, 35, 1);
        check(b"10", 36, 36, 2);
        check(b"1z", 36, 71, 2);
    });

    test!("intscan_value_table" {
        assert_eq!(val(b'0'), 0);
        assert_eq!(val(b'9'), 9);
        assert_eq!(val(b'A'), 10);
        assert_eq!(val(b'Z'), 35);
        assert_eq!(val(b'a'), 10);
        assert_eq!(val(b'z'), 35);
        assert_eq!(val(b'+'), 0xFF);
        assert_eq!(val(b'\0'), 0xFF);
    });

    test!("intscan_shift_bits_calc" {
        assert_eq!(shift_bits(2), 1);
        assert_eq!(shift_bits(4), 2);
        assert_eq!(shift_bits(8), 3);
        assert_eq!(shift_bits(16), 4);
        assert_eq!(shift_bits(32), 5);
    });

    test!("intscan_just_zero" {
        check(b"0", 0, 0, 1);
        check(b"0", 10, 0, 1);
        check(b"0", 16, 0, 1);
    });

    test!("intscan_overflow_detection" {
        // lim=100 (even) → signed overflow → 返回 lim-1 = 99
        let (val, _) = __intscan_bytes(b"1000", 10, 0, 100);
        assert_eq!(val, 99);
    });

    test!("intscan_cursor_basic" {
        let mut cur = Cursor::new(b"abc");
        assert_eq!(cur.getc(), b'a');
        assert_eq!(cur.getc(), b'b');
        assert_eq!(cur.getc(), b'c');
        assert_eq!(cur.consumed(), 3);
        cur.unget();
        assert_eq!(cur.consumed(), 2);
        assert_eq!(cur.getc(), b'c');
    });

    test!("intscan_empty_input_returns_zero" {
        let (val, consumed) = __intscan_bytes(b"", 10, 0, u64::MAX);
        assert_eq!(val, 0);
        assert_eq!(consumed, 0);
    });
}