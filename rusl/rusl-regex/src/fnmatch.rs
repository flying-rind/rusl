//! fnmatch — Shell 通配符模式匹配。对外导出 C ABI 兼容的 `fnmatch` 符号。
//!
//! 实现 "Sea of Stars" 算法（Rich Felker, 2012）。
//! 将模式分解为头部、尾部、及由 `*` 分隔的中间组件，分三阶段匹配。
//!
//! # 模块结构
//!
//! - 公开接口：`fnmatch`，以及 `FNM_*` 标志常量
//! - 内部实现：`TokenKind` 枚举、`FnmFlags` 位标志、`fnmatch_internal`、辅助函数

#![allow(unused_imports, unused_variables)]

use core::ffi::c_char;
use core::ffi::c_int;

// ============================================================================
// FNM_* 公开常量
// ============================================================================

/// 匹配成功。
pub const FNM_NOMATCH: c_int = 1;

/// 路径名匹配模式：`/` 字符不与 `*` `?` `[...]` 匹配。
pub const FNM_PATHNAME: c_int = 0x01;

/// 禁用反斜杠转义。
pub const FNM_NOESCAPE: c_int = 0x02;

/// 前导句点必须显式匹配：前导 `.` 不与 `*` `?` 匹配。
pub const FNM_PERIOD: c_int = 0x04;

/// 匹配末尾的目录前缀后即返回成功。
pub const FNM_LEADING_DIR: c_int = 0x08;

/// 大小写不敏感匹配。
pub const FNM_CASEFOLD: c_int = 0x10;

/// 系统不支持（musl 实现从不返回此值）。
pub const FNM_NOSYS: c_int = -1;

// ============================================================================
// FnmFlags — 内部位标志类型
// ============================================================================

/// fnmatch 内部使用的标志位集合。
///
/// 将 `c_int` 形式的标志位包装为类型安全的位标志类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FnmFlags(c_int);

impl FnmFlags {
    /// 空标志集。
    pub(crate) const EMPTY: FnmFlags = FnmFlags(0);
    /// 路径名模式。
    pub(crate) const PATHNAME: FnmFlags = FnmFlags(FNM_PATHNAME);
    /// 禁用反斜杠转义。
    pub(crate) const NOESCAPE: FnmFlags = FnmFlags(FNM_NOESCAPE);
    /// 前导句点必须显式匹配。
    pub(crate) const PERIOD: FnmFlags = FnmFlags(FNM_PERIOD);
    /// 前导目录匹配。
    pub(crate) const LEADING_DIR: FnmFlags = FnmFlags(FNM_LEADING_DIR);
    /// 大小写不敏感。
    pub(crate) const CASEFOLD: FnmFlags = FnmFlags(FNM_CASEFOLD);

    /// 返回底层 bits 值。
    #[inline]
    pub(crate) fn bits(self) -> c_int {
        self.0
    }

    /// 检查是否为空。
    #[inline]
    pub(crate) fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// 检查是否包含指定的标志集。
    #[inline]
    pub(crate) fn contains(self, other: FnmFlags) -> bool {
        (self.0 & other.0) != 0
    }

    /// 从原始 bits 构造（截断未知位）。
    #[inline]
    pub(crate) fn from_bits_truncate(bits: c_int) -> FnmFlags {
        FnmFlags(bits)
    }

    /// 按位或组合。
    #[inline]
    pub(crate) fn union(self, other: FnmFlags) -> FnmFlags {
        FnmFlags(self.0 | other.0)
    }
}

impl core::ops::BitOr for FnmFlags {
    type Output = FnmFlags;
    #[inline]
    fn bitor(self, rhs: FnmFlags) -> FnmFlags {
        FnmFlags(self.0 | rhs.0)
    }
}

// ============================================================================
// TokenKind — 模式/字符串 Token 枚举
// ============================================================================

/// 模式 token 类型。
///
/// C 实现使用负值宏（`END = 0`、`STAR = -5` 等）编码特殊 token 类型。
/// Rust 使用枚举替代，更类型安全且语义清晰。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TokenKind {
    /// 模式/字符串结束（对应 C 的 END = 0）。
    End,
    /// 非法多字节序列（对应 C 的 UNMATCHABLE = -2）。
    Unmatchable,
    /// 方括号表达式 `[...]`（对应 C 的 BRACKET = -3）。
    Bracket,
    /// 问号通配符 `?`（对应 C 的 QUESTION = -4）。
    Question,
    /// 星号通配符 `*`（对应 C 的 STAR = -5）。
    Star,
    /// 字面字符及其 Unicode 码点（对应 C 的非负返回值）。
    Literal(i32),
}

// ============================================================================
// 内部字符处理函数（使用 rusl tre 模块的纯 Rust 实现）
// ============================================================================

/// 多字节到宽字符转换。委托给 tre 模块的纯 Rust UTF-8 解码器。
#[inline]
unsafe fn mbtowc(pwc: &mut i32, s: *const u8, n: usize) -> i32 {
    super::tre::tre_mbtowc(pwc, s, n)
}

/// 宽字符转大写。委托给 tre 模块。
#[inline]
unsafe fn towupper(c: i32) -> i32 {
    super::tre::tre_toupper(c)
}

/// 宽字符转小写。委托给 tre 模块。
#[inline]
unsafe fn towlower(c: i32) -> i32 {
    super::tre::tre_tolower(c)
}

/// 通过名称获取宽字符类别句柄。委托给 tre 模块。
#[inline]
unsafe fn wctype(name: *const core::ffi::c_char) -> core::ffi::c_ulong {
    super::tre::tre_ctype(name)
}

/// 检查宽字符是否属于指定类别。委托给 tre 模块。
#[inline]
unsafe fn iswctype(wc: i32, desc: core::ffi::c_ulong) -> i32 {
    if super::tre::tre_isctype(wc, desc) { 1 } else { 0 }
}

// ============================================================================
// str_next — 字符串字符读取器
// ============================================================================

/// 从字符串字节切片中读取下一个字符（可能是多字节 UTF-8 字符），推进位置指针。
///
/// 这是 "Sea of Stars" 算法中所有字符串遍历的字符级原子操作。
///
/// # 前置条件
///
/// - `str` 为有效的字节切片
/// - `pos` 为切片中的有效位置
///
/// # 后置条件
///
/// | 条件 | 返回值 | `*pos` 变化 |
/// |------|--------|------------|
/// | `pos >= str.len()` | `TokenKind::End` | 不变 |
/// | `str[*pos] < 128`（ASCII） | `TokenKind::Literal(ch)` | `*pos += 1` |
/// | `str[*pos] >= 128`（多字节） | 由 `mbtowc` 决定 | 见下方 |
/// | `mbtowc` 成功解码 k 字节 | `TokenKind::Literal(wc)` | `*pos += k` |
/// | `mbtowc` 解码失败 | `TokenKind::Unmatchable` | `*pos += 1` |
pub(crate) fn str_next(str: &[u8], pos: &mut usize) -> TokenKind {
    // 对应 C 的 str_next(str, n, step)
    // n 是剩余可用字节数，对应 slice 长度减去当前位置
    if *pos >= str.len() {
        return TokenKind::End;
    }
    let n = str.len() - *pos;
    if str[*pos] == 0 {
        // C 返回 0 表示 END；此处返回 TokenKind::End
        return TokenKind::End;
    }
    if str[*pos] >= 128u8 {
        // 多字节 UTF-8 字符 — 调用 mbtowc 解码
        let mut wc: i32 = 0;
        let k = unsafe { mbtowc(&mut wc, str.as_ptr().add(*pos), n) };
        if k < 0 {
            // 非法多字节序列 — 跳过 1 字节
            *pos += 1;
            TokenKind::Unmatchable
        } else {
            *pos += k as usize;
            TokenKind::Literal(wc)
        }
    } else {
        // 单字节 ASCII
        *pos += 1;
        TokenKind::Literal(str[*pos - 1] as i32)
    }
}

// ============================================================================
// pat_next — 模式 Token 读取器
// ============================================================================

/// 从模式字节切片中读取下一个 token。
///
/// Token 可以是特殊通配符（`*`、`?`、`[...]`）、字面字符、或非法序列标记。
///
/// # 前置条件
///
/// - `pat` 为有效的字节切片
/// - `pos` 为切片中的有效位置
/// - `flags` 包含有效的标志位组合
pub(crate) fn pat_next(pat: &[u8], pos: &mut usize, flags: FnmFlags) -> TokenKind {
    // 对应 C 的 pat_next(pat, m, step, flags)
    // m 是剩余可用字节数，对应 slice 长度减去当前位置
    if *pos >= pat.len() {
        return TokenKind::End;
    }
    let m = pat.len() - *pos;
    if pat[*pos] == 0 {
        return TokenKind::End;
    }

    let start = *pos;
    let mut esc: usize = 0;

    // 处理反斜杠转义
    if pat[start] == b'\\'
        && start + 1 < pat.len()
        && pat[start + 1] != 0
        && !flags.contains(FnmFlags::NOESCAPE)
    {
        *pos += 1; // 跳过反斜杠
        esc = 1;
    }

    // 方括号表达式
    if pat[*pos] == b'[' && esc == 0 {
        let mut k: usize = 1;
        let bracket_start = *pos;

        // 跳过取反标记 ^ 或 !
        if k < m && (pat[bracket_start + k] == b'^' || pat[bracket_start + k] == b'!') {
            k += 1;
        }
        // 若 ] 紧跟在 [ 或 [^ 之后，当作字面 ] 处理
        if k < m && pat[bracket_start + k] == b']' {
            k += 1;
        }
        // 扫描至匹配的 ]
        while k < m && pat[bracket_start + k] != 0 && pat[bracket_start + k] != b']' {
            // 处理嵌套结构 [:class:] [.coll.] [=equiv=]
            if k + 1 < m
                && pat[bracket_start + k + 1] != 0
                && pat[bracket_start + k] == b'['
                && (pat[bracket_start + k + 1] == b':'
                    || pat[bracket_start + k + 1] == b'.'
                    || pat[bracket_start + k + 1] == b'=')
            {
                let z = pat[bracket_start + k + 1];
                k += 2;
                if k < m && pat[bracket_start + k] != 0 {
                    k += 1;
                }
                while k < m
                    && pat[bracket_start + k] != 0
                    && (pat[bracket_start + k - 1] != z || pat[bracket_start + k] != b']')
                {
                    k += 1;
                }
                if k == m || pat[bracket_start + k] == 0 {
                    break;
                }
            }
            k += 1;
        }

        if k == m || pat[bracket_start + k] == 0 {
            // 未找到匹配的 ] — 将 [ 作为字面字符处理
            *pos = start + 1; // 恢复到 [ 之后一位（跳过 [）
            return TokenKind::Literal(b'[' as i32);
        }

        // 找到匹配的 ]，整个方括号表达式的步长为 k+1
        *pos = bracket_start + k + 1;
        return TokenKind::Bracket;
    }

    // * 和 ? 通配符
    if pat[*pos] == b'*' && esc == 0 {
        *pos += 1;
        return TokenKind::Star;
    }
    if pat[*pos] == b'?' && esc == 0 {
        *pos += 1;
        return TokenKind::Question;
    }

    // 字面字符（可能为多字节）
    if pat[*pos] >= 128u8 {
        let mut wc: i32 = 0;
        let remaining = pat.len() - *pos;
        let k = unsafe { mbtowc(&mut wc, pat.as_ptr().add(*pos), remaining) };
        if k < 0 {
            // 非法多字节序列
            *pos = start; // 不推进，回到起始位置
            return TokenKind::Unmatchable;
        }
        *pos += k as usize;
        TokenKind::Literal(wc)
    } else {
        let ch = pat[*pos] as i32;
        *pos += 1;
        TokenKind::Literal(ch)
    }
}

// ============================================================================
// casefold — 大小写折叠
// ============================================================================

/// 对单个字符进行大小写折叠。
///
/// 策略：先尝试 `towupper(k)`，若结果等于 `k`（原字符已是大写或无大写形式），
/// 则尝试 `towlower(k)`。
///
/// # 前置条件
///
/// - `k` 为有效的字符码点
///
/// # 后置条件
///
/// - 若 `towupper(k) != k`：返回 `towupper(k)`
/// - 否则返回 `towlower(k)`
pub(crate) fn casefold(k: i32) -> i32 {
    // 对应 C 的 casefold(k):
    //   int c = towupper(k);
    //   return c == k ? towlower(k) : c;
    let c = unsafe { towupper(k) };
    if c == k {
        unsafe { towlower(k) }
    } else {
        c
    }
}

// ============================================================================
// match_bracket — 方括号表达式匹配
// ============================================================================

/// 判断字符 `k`（或其大小写折叠形式 `kfold`）是否匹配方括号表达式 `[...]`。
///
/// 支持取反 `[^...]` / `[!...]`、字符范围 `a-z`、字符类 `[:class:]`。
///
/// # 前置条件
///
/// - `p` 指向 `[` 之后的字节切片
/// - `k` 和 `kfold` 为有效字符码点
///
/// # 后置条件
///
/// - 匹配成功：返回 `true`
/// - 匹配失败或模式非法：返回 `false`
///
/// # 不变量
///
/// - 函数不会读取越过首个未转义的 `]` 之后
/// - 扫描过程中若遇到多字节解码失败，返回 `false`
/// 判断字符 `k`（或其大小写折叠形式 `kfold`）是否匹配方括号表达式 `[...]`。
///
/// 支持取反 `[^...]` / `[!...]`、字符范围 `a-z`、字符类 `[:class:]`。
///
/// # 前置条件
///
/// - `p` 指向 `[` 之后的字节切片（即 `p[0]` 为方括号内第一个字符）
/// - `k` 和 `kfold` 为有效字符码点（若大小写不匹配模式下 `kfold == k`）
///
/// # 后置条件
///
/// - 匹配成功：返回 `true`
/// - 匹配失败或模式非法：返回 `false`
///
/// # 不变量
///
/// - 函数不会读取越过首个未转义的 `]` 之后
/// - 扫描过程中若遇到多字节解码失败，返回 `false`
pub(crate) fn match_bracket(
    p: &[u8],
    k: i32,
    kfold: i32,
    _eflags: i32,
) -> bool {
    // 对应 C 的 match_bracket(p, k, kfold)
    // p 指向 '[' 之后的第一个字符
    if p.is_empty() {
        return false;
    }

    let mut idx: usize = 0;
    let mut inv = false; // 取反标志
    let mut wc: i32; // 当前/前一个字符（用于范围比较）

    // 处理取反标记
    if idx < p.len() && (p[idx] == b'^' || p[idx] == b'!') {
        inv = true;
        idx += 1;
    }

    // 处理方括号开头紧跟的 ']'（字面 ']'）
    if idx < p.len() && p[idx] == b']' {
        if k == ']' as i32 || kfold == ']' as i32 {
            return !inv;
        }
        idx += 1;
    } else if idx < p.len() && p[idx] == b'-' {
        // 处理方括号开头紧跟的 '-'（字面 '-'）
        if k == '-' as i32 || kfold == '-' as i32 {
            return !inv;
        }
        idx += 1;
    }

    // 初始化 wc（第一遍循环中会被正确设置）
    wc = if idx > 0 { p[idx - 1] as i32 } else { -1 };

    // 主扫描循环
    while idx < p.len() && p[idx] != b']' {
        // 检查字符范围 a-z
        if p[idx] == b'-' && idx + 1 < p.len() && p[idx + 1] != b']' {
            let mut wc2: i32 = 0;
            let remaining = p.len() - (idx + 1);
            let l = unsafe { mbtowc(&mut wc2, p.as_ptr().add(idx + 1), remaining.min(4)) };
            if l < 0 {
                return false; // 解码失败
            }
            // 范围检查使用无符号减法技巧
            if wc <= wc2 {
                let ku = k as u32;
                let kfu = kfold as u32;
                let wcu = wc as u32;
                let wc2u = wc2 as u32;
                if (ku.wrapping_sub(wcu) <= wc2u.wrapping_sub(wcu))
                    || (kfu.wrapping_sub(wcu) <= wc2u.wrapping_sub(wcu))
                {
                    return !inv;
                }
            }
            idx += l as usize; // 指向范围终点之后（for 循环的 idx += 1 会推进）
            continue;
        }

        // 处理嵌套结构 [:class:] [.coll.] [=equiv=]
        if p[idx] == b'['
            && idx + 1 < p.len()
            && (p[idx + 1] == b':' || p[idx + 1] == b'.' || p[idx + 1] == b'=')
        {
            let z = p[idx + 1]; // ':', '.', 或 '='
            let p0 = idx + 2; // 名称/元素起始位置
            idx += 3; // 跳过 "[" + z + 下一个字符
            // 扫描至 z']
            while idx < p.len()
                && (idx == 0 || p[idx - 1] != z || p[idx] != b']')
            {
                idx += 1;
            }
            if idx >= p.len() {
                return false; // 未找到匹配的 z']
            }
            // 仅处理字符类 [:class:]
            if z == b':' && (idx - 1).wrapping_sub(p0) < 16 {
                // 构造 null 终止的类名字符串
                let class_name_len = idx - 1 - p0;
                let mut buf: [u8; 16] = [0u8; 16];
                // Safety: class_name_len < 16 已保证
                buf[..class_name_len].copy_from_slice(&p[p0..p0 + class_name_len]);
                buf[class_name_len] = 0;
                let desc = unsafe { wctype(buf.as_ptr() as *const c_char) };
                if desc != 0 {
                    if unsafe { iswctype(k, desc) != 0 || iswctype(kfold, desc) != 0 } {
                        return !inv;
                    }
                }
            }
            continue;
        }

        // 单字符匹配
        if p[idx] < 128u8 {
            wc = p[idx] as i32;
        } else {
            let mut decoded: i32 = 0;
            let remaining = p.len() - idx;
            let l = unsafe { mbtowc(&mut decoded, p.as_ptr().add(idx), remaining.min(4)) };
            if l < 0 {
                return false; // 解码失败
            }
            wc = decoded;
            idx += l as usize - 1; // 补偿 for 循环的 idx += 1
        }
        if wc == k || wc == kfold {
            return !inv;
        }
        idx += 1;
    }

    // 遍历完毕，无匹配
    inv
}

// ============================================================================
// fnmatch_internal — 核心匹配引擎
// ============================================================================

/// 使用 "Sea of Stars" 算法进行模式匹配。
///
/// 返回 `true` 表示匹配成功，`false` 表示匹配失败（对应 C 的 `FNM_NOMATCH`）。
///
/// # 系统算法
///
/// "Sea of Stars" 算法 — 将模式分解为头部、尾部和由 `*` 分隔的中间组件：
///
/// 1. **FNM_PERIOD 前缀检查**：若设置了 `FNM_PERIOD`，检查 `str` 首字符为 `.` 时
///    模式是否以 `.` 显式匹配。
/// 2. **头部匹配 (Head Match)**：从模式开头匹配到第一个 `*`，若失配则立即返回 `false`。
/// 3. **尾部收集 (Tail Collection)**：重新扫描整个模式，定位最后一个 `*` 及其后的字面 token。
/// 4. **尾部匹配 (Tail Match)**：从字符串末尾提取对应数量字符与尾部逐 token 比较。
/// 5. **星海匹配 (Sea of Stars)**：在头部和尾部之间，找到由 `*` 分隔的每个组件，
///    按顺序在字符串中搜索其首次出现处。
///
/// # 前置条件
///
/// - `pat` 和 `str` 为有效的字节切片
/// - `flags` 包含 `FNM_*` 标志位组合
///
/// # 后置条件
///
/// - 匹配成功：返回 `true`（对应 C 的 `0`）
/// - 匹配失败：返回 `false`（对应 C 的 `FNM_NOMATCH`）
/// 使用 "Sea of Stars" 算法进行模式匹配。
///
/// 返回 `true` 表示匹配成功，`false` 表示匹配失败（对应 C 的 `FNM_NOMATCH`）。
///
/// # 系统算法
///
/// "Sea of Stars" 算法 — 将模式分解为头部、尾部和由 `*` 分隔的中间组件。
pub(crate) fn fnmatch_internal(
    pat: &[u8],
    str: &[u8],
    flags: FnmFlags,
) -> bool {
    // 内联辅助：用 pat_next 返回的 TokenKind 推断 bracket 起始位置。
    // 由于 pat_next 在返回 Bracket 时已推进 pos 到 ] 之后，
    // 需要通过保存调用前位置来定位 [。
    // 为简化，所有调用 pat_next 之前保存 prev，若结果为 Bracket，
    // 则 prev 处为 [，prev+1 为方括号内部起始。

    // ========== Phase 0: FNM_PERIOD 前缀检查 ==========
    if flags.contains(FnmFlags::PERIOD) {
        if !str.is_empty() && str[0] == b'.' {
            if pat.is_empty() || pat[0] != b'.' {
                return false;
            }
        }
    }

    let mut ppos: usize = 0; // 模式扫描位置
    let mut spos: usize = 0; // 字符串扫描位置

    // ========== Phase 1: 头部匹配 ==========
    loop {
        let prev_p = ppos;
        let c = pat_next(pat, &mut ppos, flags);
        if c == TokenKind::Star {
            break;
        }
        if c == TokenKind::Unmatchable {
            return false;
        }

        let k = str_next(str, &mut spos);
        // C: k <= 0 表示字符串结束或非法字节
        let str_ended = matches!(k, TokenKind::End | TokenKind::Unmatchable);
        if str_ended {
            return c == TokenKind::End;
        }

        let kfold_val = if flags.contains(FnmFlags::CASEFOLD) {
            match k {
                TokenKind::Literal(v) => casefold(v),
                _ => match k { TokenKind::Literal(v) => v, _ => 0 },
            }
        } else {
            match k { TokenKind::Literal(v) => v, _ => 0 }
        };

        match c {
            TokenKind::End => {
                // 模式已结束，但字符串还有内容 → 不匹配
                // (两个都结束的情况在 str_ended 分支中已处理返回 true)
                return false;
            }
            TokenKind::Bracket => {
                let kv = match k { TokenKind::Literal(v) => v, _ => 0 };
                if !match_bracket(&pat[prev_p + 1..], kv, kfold_val, 0) {
                    return false;
                }
            }
            TokenKind::Question => {} // 匹配任意单字符
            TokenKind::Literal(cv) => {
                let kv = match k { TokenKind::Literal(v) => v, _ => 0 };
                if cv != kv && cv != kfold_val {
                    return false;
                }
            }
            _ => return false,
        }
    }

    // ========== Phase 2: 尾部收集 ==========
    let mut ptail: usize = ppos; // 最后一个 * 之后的位置
    let mut tailcnt: usize = 0;
    let mut ppos2: usize = 0;

    while ppos2 < pat.len() && pat[ppos2] != 0 {
        let prev_p = ppos2;
        let c = pat_next(pat, &mut ppos2, flags);
        if c == TokenKind::Unmatchable {
            return false;
        }
        if c == TokenKind::Star {
            tailcnt = 0;
            ptail = ppos2;
        } else if c != TokenKind::End {
            tailcnt += 1;
        }
        if ppos2 == prev_p {
            break;
        }
    }

    // ========== Phase 3: 尾部匹配 ==========
    // 找到字符串末尾 tailcnt 个字符，逐 token 与模式尾部比较
    if str.len() < tailcnt {
        return false;
    }

    let mut stail: usize = str.len();
    let mut rem = tailcnt;
    while stail > 0 && rem > 0 {
        stail -= 1;
        // 跳过 UTF-8 多字节字符的后续字节
        while stail > 0
            && str[stail] >= 0x80u8
            && str[stail] < 0xC0u8
        {
            stail -= 1;
        }
        rem -= 1;
    }
    if rem > 0 {
        return false;
    }

    let mut p = ptail;
    let mut s = stail;
    loop {
        let prev_p = p;
        let c = pat_next(pat, &mut p, flags);
        let k = str_next(str, &mut s);
        let str_ended = matches!(k, TokenKind::End | TokenKind::Unmatchable);
        if str_ended {
            if c != TokenKind::End {
                return false;
            }
            break;
        }
        let kfold_val = if flags.contains(FnmFlags::CASEFOLD) {
            match k { TokenKind::Literal(v) => casefold(v), _ => 0 }
        } else {
            match k { TokenKind::Literal(v) => v, _ => 0 }
        };

        match c {
            TokenKind::End => break,
            TokenKind::Bracket => {
                let kv = match k { TokenKind::Literal(v) => v, _ => 0 };
                if !match_bracket(&pat[prev_p + 1..], kv, kfold_val, 0) {
                    return false;
                }
            }
            TokenKind::Question => {}
            TokenKind::Literal(cv) => {
                let kv = match k { TokenKind::Literal(v) => v, _ => 0 };
                if cv != kv && cv != kfold_val {
                    return false;
                }
            }
            _ => return false,
        }
    }

    // 尾部匹配完成，收缩边界
    let endpat = ptail;
    let endstr = stail;

    // ========== Phase 4: 星海匹配 ==========
    let mut pat_ptr = ppos; // 第一个 * 之后
    let mut str_ptr = spos; // 头部匹配后位置

    'outer: while pat_ptr < endpat {
        let mut p2 = pat_ptr;
        let mut s2 = str_ptr;

        loop {
            let prev_p = p2;
            let c = pat_next(pat, &mut p2, flags);
            if c == TokenKind::Star {
                pat_ptr = p2;
                str_ptr = s2;
                continue 'outer;
            }
            if c == TokenKind::End {
                if s2 >= endstr {
                    pat_ptr = endpat;
                    break;
                }
                // 组件未匹配完但模式已结束 — 回退
                break;
            }

            let k = str_next(str, &mut s2);
            if matches!(k, TokenKind::End) {
                break;
            }

            let kfold_val = if flags.contains(FnmFlags::CASEFOLD) {
                match k { TokenKind::Literal(v) => casefold(v), _ => 0 }
            } else {
                match k { TokenKind::Literal(v) => v, _ => 0 }
            };

            let char_match = match c {
                TokenKind::Bracket => {
                    let kv = match k { TokenKind::Literal(v) => v, _ => 0 };
                    match_bracket(&pat[prev_p + 1..], kv, kfold_val, 0)
                }
                TokenKind::Question => true,
                TokenKind::Literal(cv) => {
                    let kv = match k { TokenKind::Literal(v) => v, _ => 0 };
                    cv == kv || cv == kfold_val
                }
                _ => false,
            };

            if !char_match {
                break;
            }
        }

        // 组件未匹配成功，推进字符串起始位置
        let k = str_next(str, &mut str_ptr);
        if matches!(k, TokenKind::End) {
            return false;
        }
        // 跳过连续的非法多字节序列
        if k == TokenKind::Unmatchable {
            while str_ptr < endstr {
                let saved = str_ptr;
                let ck = str_next(str, &mut str_ptr);
                if ck != TokenKind::Unmatchable {
                    str_ptr = saved; // 回退到第一个有效字符的开头
                    break;
                }
            }
        }
    }

    true
}

// ============================================================================
// fnmatch (对外导出)
// ============================================================================

/// POSIX `fnmatch()` — 测试字符串是否匹配 shell 通配符模式。
///
/// [Visibility]: Public — POSIX.1-2001 标准函数，`<fnmatch.h>` 声明。
///
/// # Safety
///
/// 调用者必须确保：
/// - `pat` 和 `str` 是以空字符结尾的有效 C 字符串
///
/// # 返回值
///
/// | 条件 | 返回值 |
/// |------|--------|
/// | 匹配成功 | `0` |
/// | 匹配失败 | `FNM_NOMATCH` (1) |
#[no_mangle]
pub extern "C" fn fnmatch(
    pat: *const c_char,
    str: *const c_char,
    flags: c_int,
) -> c_int {
    unsafe {
        // 将空指针视为空字符串
        if pat.is_null() || str.is_null() {
            return FNM_NOMATCH;
        }

        let pat_len = libc_strlen(pat);
        let str_len = libc_strlen(str);
        let pat_slice = core::slice::from_raw_parts(pat as *const u8, pat_len);
        let str_slice = core::slice::from_raw_parts(str as *const u8, str_len);
        let fnm_flags = FnmFlags::from_bits_truncate(flags);

        // ===== FNM_PATHNAME 模式 =====
        if fnm_flags.contains(FnmFlags::PATHNAME) {
            let mut p_pos: usize = 0;
            let mut s_pos: usize = 0;

            loop {
                // 在字符串中找下一个 '/'
                let s_start = s_pos;
                while s_pos < str_slice.len() && str_slice[s_pos] != b'/' {
                    s_pos += 1;
                }
                let s_seg_end = s_pos;

                // 在模式中找下一个 '/' 或 END
                let p_start = p_pos;
                let p_seg_end = loop {
                    let prev = p_pos;
                    let c = pat_next(pat_slice, &mut p_pos, fnm_flags);
                    match c {
                        TokenKind::End => break p_pos,
                        TokenKind::Literal(val) if val == b'/' as i32 => break prev, // '/' 之前
                        TokenKind::Unmatchable => return FNM_NOMATCH,
                        _ => {}
                    }
                    if p_pos == prev {
                        // 防止无限循环
                        break p_pos;
                    }
                };

                // 检查模式与字符串的终止状态一致性
                // C 代码: if (c!=*s && (!*s || !(flags & FNM_LEADING_DIR))) return FNM_NOMATCH;
                let pat_ended = p_pos >= pat_slice.len()
                    || (p_pos < pat_slice.len() && pat_slice[p_pos] == 0);
                let str_ended = s_pos >= str_slice.len();

                if pat_ended != str_ended {
                    if str_ended && fnm_flags.contains(FnmFlags::LEADING_DIR) {
                        return 0;
                    }
                    return FNM_NOMATCH;
                }

                // 匹配当前段
                if !fnmatch_internal(
                    &pat_slice[p_start..p_seg_end],
                    &str_slice[s_start..s_seg_end],
                    fnm_flags,
                ) {
                    return FNM_NOMATCH;
                }

                if pat_ended {
                    return 0;
                }

                // 跳过 '/' 分隔符
                s_pos += 1; // 跳过字符串中的 '/'
                              // p_pos 需要跳过模式中的 '/'（pat_next 已消费 '/' 字面）
                if p_pos < pat_slice.len() && pat_slice[p_pos] == b'/' {
                    p_pos += 1;
                }
                // 如果 pat_next 消费了 '/' 作为 Literal，则 p_pos 已在其后
            }
        }

        // ===== FNM_LEADING_DIR 模式 =====
        if fnm_flags.contains(FnmFlags::LEADING_DIR) {
            for s_pos in 0..str_slice.len() {
                if str_slice[s_pos] != b'/' {
                    continue;
                }
                if fnmatch_internal(pat_slice, &str_slice[..s_pos], fnm_flags) {
                    return 0;
                }
            }
        }

        // ===== 普通模式：直接全字符串匹配 =====
        if fnmatch_internal(pat_slice, str_slice, fnm_flags) {
            0
        } else {
            FNM_NOMATCH
        }
    }
}

/// 计算 C 字符串长度（查找 null 终止符）。
unsafe fn libc_strlen(s: *const c_char) -> usize {
    let mut len: usize = 0;
    while unsafe { *s.add(len) } != 0 {
        len += 1;
    }
    len
}

// ============================================================================
// 测试模块
// ============================================================================

#[cfg(test)]
mod tests {
    use rusl_core::test;

        use alloc::format;
    use super::*;

    // ---- 常量测试 ----

    test!("test_fnm_constants_distinct" {
        // 主要标志位应互不重叠（除 NOMATCH 和 NOSYS 属于返回值）
        let flag_masks = [FNM_PATHNAME, FNM_NOESCAPE, FNM_PERIOD, FNM_LEADING_DIR, FNM_CASEFOLD];
        for i in 0..flag_masks.len() {
            for j in (i + 1)..flag_masks.len() {
                assert_eq!(
                    flag_masks[i] & flag_masks[j],
                    0,
                    "标志位 {} 和 {} 重叠",
                    flag_masks[i],
                    flag_masks[j]
                );
            }
        }
    });

    test!("test_fnm_pathname" {
        assert_eq!(FNM_PATHNAME, 0x01);
    });

    test!("test_fnm_noescape" {
        assert_eq!(FNM_NOESCAPE, 0x02);
    });

    test!("test_fnm_period" {
        assert_eq!(FNM_PERIOD, 0x04);
    });

    test!("test_fnm_leading_dir" {
        assert_eq!(FNM_LEADING_DIR, 0x08);
    });

    test!("test_fnm_casefold" {
        assert_eq!(FNM_CASEFOLD, 0x10);
    });

    test!("test_fnm_nomatch" {
        assert_eq!(FNM_NOMATCH, 1);
    });

    test!("test_fnm_nosys" {
        assert_eq!(FNM_NOSYS, -1);
    });

    // ---- FnmFlags 测试 ----

    test!("test_fnmflags_empty_has_no_bits" {
        let flags = FnmFlags::EMPTY;
        assert!(flags.is_empty());
        assert_eq!(flags.bits(), 0);
    });

    test!("test_fnmflags_pathname" {
        let flags = FnmFlags::PATHNAME;
        assert!(flags.contains(FnmFlags::PATHNAME));
        assert_eq!(flags.bits(), FNM_PATHNAME);
    });

    test!("test_fnmflags_combined" {
        let flags = FnmFlags::PATHNAME.union(FnmFlags::CASEFOLD);
        assert!(flags.contains(FnmFlags::PATHNAME));
        assert!(flags.contains(FnmFlags::CASEFOLD));
        assert!(!flags.contains(FnmFlags::NOESCAPE));
    });

    test!("test_fnmflags_from_raw_flags" {
        let raw = FNM_PATHNAME | FNM_PERIOD;
        let flags = FnmFlags::from_bits_truncate(raw);
        assert!(flags.contains(FnmFlags::PATHNAME));
        assert!(flags.contains(FnmFlags::PERIOD));
    });

    // ---- TokenKind 测试 ----

    test!("test_tokenkind_eq" {
        assert_eq!(TokenKind::End, TokenKind::End);
        assert_eq!(TokenKind::Star, TokenKind::Star);
        assert_eq!(TokenKind::Literal(65), TokenKind::Literal(65));
        assert_ne!(TokenKind::Literal(65), TokenKind::Literal(66));
        assert_ne!(TokenKind::Star, TokenKind::Question);
    });

    test!("test_tokenkind_debug_display" {
        // 验证 Debug 实现可以正常格式化
        let t = TokenKind::Literal(0x41);
        let s = format!("{:?}", t);
        assert!(s.contains("Literal"));
        assert!(s.contains("41") || s.contains("65"));
    });

    test!("test_tokenkind_clone_copy" {
        let t1 = TokenKind::Star;
        let t2 = t1; // Copy
        assert_eq!(t1, t2);
        let t3 = t1.clone(); // Clone
        assert_eq!(t1, t3);
    });

    // ---- str_next 边界测试 ----

    test!("test_str_next_empty" {
        let s: &[u8] = &[];
        let mut pos: usize = 0;
        let result = str_next(s, &mut pos);
        assert_eq!(result, TokenKind::End);
        assert_eq!(pos, 0); // 位置不变
    });

    test!("test_str_next_pos_past_end" {
        let s: &[u8] = b"abc";
        let mut pos: usize = 10; // 超出长度
        let result = str_next(s, &mut pos);
        assert_eq!(result, TokenKind::End);
        assert_eq!(pos, 10); // 位置不变
    });

    test!("test_str_next_ascii_single_byte" {
        let s: &[u8] = b"A";
        let mut pos: usize = 0;
        let result = str_next(s, &mut pos);
        // 'A' 的码点为 65
        assert_eq!(result, TokenKind::Literal(65));
        // pos 应推进 1
        // （注意：此测试在实现完成前验证接口签名）
    });

    test!("test_str_next_ascii_multiple" {
        let s: &[u8] = b"abc";
        let mut pos: usize = 0;
        let r1 = str_next(s, &mut pos);
        assert_eq!(r1, TokenKind::Literal(97)); // 'a'
        let r2 = str_next(s, &mut pos);
        assert_eq!(r2, TokenKind::Literal(98)); // 'b'
        let r3 = str_next(s, &mut pos);
        assert_eq!(r3, TokenKind::Literal(99)); // 'c'
        let r4 = str_next(s, &mut pos);
        assert_eq!(r4, TokenKind::End);
    });

    test!("test_str_next_mixed_bytes" {
        let s: &[u8] = b"ab";
        let mut pos: usize = 0;
        let r1 = str_next(s, &mut pos);
        assert_eq!(r1, TokenKind::Literal(b'a' as i32));
        let r2 = str_next(s, &mut pos);
        assert_eq!(r2, TokenKind::Literal(b'b' as i32));
        let r3 = str_next(s, &mut pos);
        assert_eq!(r3, TokenKind::End);
    });

    // ---- pat_next 边界测试 ----

    test!("test_pat_next_empty" {
        let s: &[u8] = &[];
        let mut pos: usize = 0;
        let result = pat_next(s, &mut pos, FnmFlags::EMPTY);
        assert_eq!(result, TokenKind::End);
    });

    test!("test_pat_next_star" {
        let s: &[u8] = b"*";
        let mut pos: usize = 0;
        let result = pat_next(s, &mut pos, FnmFlags::EMPTY);
        assert_eq!(result, TokenKind::Star);
    });

    test!("test_pat_next_question" {
        let s: &[u8] = b"?";
        let mut pos: usize = 0;
        let result = pat_next(s, &mut pos, FnmFlags::EMPTY);
        assert_eq!(result, TokenKind::Question);
    });

    test!("test_pat_next_literal" {
        let s: &[u8] = b"X";
        let mut pos: usize = 0;
        let result = pat_next(s, &mut pos, FnmFlags::EMPTY);
        assert_eq!(result, TokenKind::Literal(b'X' as i32));
    });

    test!("test_pat_next_escaped_star" {
        // 默认 NOESCAPE 未设置时，\* 应作为字面 '*' 处理
        let s: &[u8] = b"\\*";
        let mut pos: usize = 0;
        let result = pat_next(s, &mut pos, FnmFlags::EMPTY);
        // 非 NOESCAPE 模式下，\* 转义为字面 '*'
        assert_eq!(result, TokenKind::Literal(b'*' as i32));
    });

    test!("test_pat_next_noescape_mode" {
        // NOESCAPE 模式下，\* 的 \ 是字面反斜杠
        let s: &[u8] = b"\\*";
        let mut pos: usize = 0;
        let result = pat_next(s, &mut pos, FnmFlags::NOESCAPE);
        // \ 是字面字符
        assert_eq!(result, TokenKind::Literal(b'\\' as i32));
    });

    test!("test_pat_next_pos_past_end" {
        let s: &[u8] = b"a";
        let mut pos: usize = 10;
        let result = pat_next(s, &mut pos, FnmFlags::EMPTY);
        assert_eq!(result, TokenKind::End);
    });

    // ---- casefold 测试 ----

    test!("test_casefold_ascii_lower_to_upper" {
        // 小写字母应折叠为大写
        let result = casefold('a' as i32);
        // 'a' 折叠后应为 'A'(65)
        assert_eq!(result, 'A' as i32);
    });

    test!("test_casefold_ascii_upper_stays" {
        // 大写字保持大写
        let result = casefold('A' as i32);
        assert!(result == 'A' as i32 || result == 'a' as i32);
    });

    test!("test_casefold_digit_unchanged" {
        // 数字不区分大小写
        let result = casefold('5' as i32);
        assert_eq!(result, '5' as i32);
    });

    test!("test_casefold_symbol_unchanged" {
        // 符号不变
        let result = casefold('.' as i32);
        assert_eq!(result, '.' as i32);
    });

    // ---- match_bracket 测试 ----

    test!("test_match_bracket_simple_char" {
        // [a] 匹配 'a'
        let result = match_bracket(b"a]", 'a' as i32, 'A' as i32, 0);
        assert!(result);
    });

    test!("test_match_bracket_simple_no_match" {
        // [a] 不匹配 'b'
        let result = match_bracket(b"a]", 'b' as i32, 'B' as i32, 0);
        assert!(!result);
    });

    test!("test_match_bracket_range" {
        // [a-z] 应匹配 'm'
        let result = match_bracket(b"a-z]", 'm' as i32, 'M' as i32, 0);
        assert!(result);
    });

    test!("test_match_bracket_range_no_match" {
        // [a-z] 不应匹配 '0'
        let result = match_bracket(b"a-z]", '0' as i32, '0' as i32, 0);
        assert!(!result);
    });

    test!("test_match_bracket_negate" {
        // [^a] 不应匹配 'a'
        let result = match_bracket(b"^a]", 'a' as i32, 'A' as i32, 0);
        assert!(!result);
    });

    test!("test_match_bracket_negate_match_other" {
        // [^a] 应匹配 'b'
        let result = match_bracket(b"^a]", 'b' as i32, 'B' as i32, 0);
        assert!(result);
    });

    test!("test_match_bracket_exclamation_negate" {
        // [!a] 等价于 [^a]
        let result = match_bracket(b"!a]", 'b' as i32, 'B' as i32, 0);
        assert!(result);
    });

    // ---- fnmatch_internal 测试 ----

    test!("test_fnmatch_internal_exact_match" {
        let pat = b"abc";
        let str = b"abc";
        assert!(fnmatch_internal(pat, str, FnmFlags::EMPTY));
    });

    test!("test_fnmatch_internal_no_match" {
        let pat = b"abc";
        let str = b"abd";
        assert!(!fnmatch_internal(pat, str, FnmFlags::EMPTY));
    });

    test!("test_fnmatch_internal_star_match" {
        let pat = b"a*c";
        let str = b"abc";
        assert!(fnmatch_internal(pat, str, FnmFlags::EMPTY));
    });

    test!("test_fnmatch_internal_star_match_long" {
        let pat = b"a*c";
        let str = b"aXYZc";
        assert!(fnmatch_internal(pat, str, FnmFlags::EMPTY));
    });

    test!("test_fnmatch_internal_question_match" {
        let pat = b"a?c";
        let str = b"abc";
        assert!(fnmatch_internal(pat, str, FnmFlags::EMPTY));
    });

    test!("test_fnmatch_internal_question_no_match" {
        let pat = b"a?c";
        let str = b"ac"; // 缺少一个字符
        assert!(!fnmatch_internal(pat, str, FnmFlags::EMPTY));
    });

    test!("test_fnmatch_internal_empty_pat_empty_str" {
        assert!(fnmatch_internal(b"", b"", FnmFlags::EMPTY));
    });

    test!("test_fnmatch_internal_empty_pat_nonempty_str" {
        assert!(!fnmatch_internal(b"", b"abc", FnmFlags::EMPTY));
    });

    test!("test_fnmatch_internal_star_only" {
        // 仅 * 的模式匹配任何字符串
        let pat = b"*";
        let str = b"anything";
        assert!(fnmatch_internal(pat, str, FnmFlags::EMPTY));
    });

    test!("test_fnmatch_internal_multiple_stars" {
        let pat = b"a*b*c";
        let str = b"aXbYc";
        assert!(fnmatch_internal(pat, str, FnmFlags::EMPTY));
    });

    test!("test_fnmatch_internal_casefold" {
        let pat = b"ABC";
        let str = b"abc";
        assert!(fnmatch_internal(pat, str, FnmFlags::CASEFOLD));
    });

    test!("test_fnmatch_internal_period" {
        // FNM_PERIOD: 前导 . 必须显式匹配
        let pat = b"*";
        let str = b".hidden";
        // 没有 FNM_PERIOD 时，* 应匹配 .hidden
        assert!(fnmatch_internal(pat, str, FnmFlags::EMPTY));
    });

    test!("test_fnmatch_internal_period_match" {
        // FNM_PERIOD: 模式以 . 开头时应显式匹配开头的 .
        let pat = b".*";
        let str = b".hidden";
        assert!(fnmatch_internal(pat, str, FnmFlags::PERIOD));
    });

    // ---- fnmatch 公开 API 测试 ----

    test!("test_fnmatch_basic_match" {
        unsafe {
            let pat = b"abc\0" as *const u8 as *const c_char;
            let s = b"abc\0" as *const u8 as *const c_char;
            let result = fnmatch(pat, s, 0);
            assert_eq!(result, 0);
        }
    });

    test!("test_fnmatch_basic_no_match" {
        unsafe {
            let pat = b"abc\0" as *const u8 as *const c_char;
            let s = b"xyz\0" as *const u8 as *const c_char;
            let result = fnmatch(pat, s, 0);
            assert_eq!(result, FNM_NOMATCH);
        }
    });

    test!("test_fnmatch_pathname_slash_not_matched" {
        unsafe {
            let pat = b"a?b\0" as *const u8 as *const c_char;
            let s = b"a/b\0" as *const u8 as *const c_char;
            let result = fnmatch(pat, s, FNM_PATHNAME);
            // ? 不应匹配 /
            assert_eq!(result, FNM_NOMATCH);
        }
    });

    test!("test_fnmatch_leading_dir" {
        unsafe {
            let pat = b"a*\0" as *const u8 as *const c_char;
            let s = b"abc/xyz\0" as *const u8 as *const c_char;
            let result = fnmatch(pat, s, FNM_LEADING_DIR);
            // "a*" 匹配 "abc" 前缀，FNM_LEADING_DIR 应返回成功
            assert_eq!(result, 0);
        }
    });

    test!("test_fnmatch_noescape" {
        unsafe {
            let pat = b"a\\*b\0" as *const u8 as *const c_char;
            let s = b"a*b\0" as *const u8 as *const c_char;
            // NOESCAPE: \ 是字面字符，\* 是两个字符 '\' 和 '*'
            let result = fnmatch(pat, s, FNM_NOESCAPE);
            // "a\*b" vs "a*b": \ 对应 *，* 对应 b(?) — 通常不匹配
            assert_eq!(result, FNM_NOMATCH);
        }
    });

    test!("test_fnmatch_zero_flags_empty_strings" {
        unsafe {
            let pat = b"\0" as *const u8 as *const c_char;
            let s = b"\0" as *const u8 as *const c_char;
            let result = fnmatch(pat, s, 0);
            assert_eq!(result, 0);
        }
    });
}
