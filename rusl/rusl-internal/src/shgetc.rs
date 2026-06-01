//! shgetc 模块 — 扫描辅助字符输入抽象。
//!
//! 本模块定义了 `ScanHelper` 类型，为 `scanf` 系列和 `strto*`
//! 系列函数提供统一的字符级输入接口。支持两种底层数据源：
//! 真实 `File` 流和以 `\0` 结尾的字符串。
//!
//! # 模块可见性
//!
//! `pub(crate)` — 仅在 rusl crate 内部使用。

use core::ffi::c_int;

/// 内部 FILE 类型的占位符声明（从 intscan 模块导入，避免重复定义）。
///
/// 完整定义位于 `crate::stdio` 模块。
pub use crate::intscan::File;

/// EOF 常量，与 musl 一致。
const EOF: c_int = -1;

/// 扫描数据源类型。
pub enum ScanSource {
    /// 真实 FILE 流
    File(*mut File),
    /// 字符串源：起始指针 + 长度（不含 '\0'）
    String {
        base: *const u8,
        len: usize,
    },
}

/// 扫描辅助结构体。
///
/// 封装了字符扫描的状态和操作，为 `scanf`/`strto*` 提供统一的
/// 字符级输入接口。支持两种数据源（File 流和字符串），通过内联
/// 快速路径 `getc()` 实现高效字符读取。
///
/// # 设计说明
///
/// 在 C 实现中，扫描状态被分散嵌入 `FILE` 结构体。Rust 设计中将
/// 其抽象为独立的 `ScanHelper` 结构体，实现关注点分离：
/// - `ScanSource` 枚举区分两种数据源
/// - `shcnt_offset` 存储计数偏移量，保持与 C 兼容的快速路径约定
pub struct ScanHelper {
    /// 数据源类型
    source: ScanSource,

    /// 当前读指针
    rpos: *const u8,

    /// 扫描限制边界（字符串模式下为字符串末尾 + 1）
    shend: *const u8,

    /// 扫描宽度限制
    shlim: isize,

    /// 计数字段偏移量：`cnt_actual = shcnt_offset + (rpos - base)`
    shcnt_offset: isize,
}

impl ScanHelper {
    /// 获取数据源的基指针（用于计数计算）。
    fn base_ptr(&self) -> *const u8 {
        match self.source {
            ScanSource::String { base, .. } => base,
            ScanSource::File(_) => core::ptr::null(),
        }
    }

    /// 从 C 字符串构造扫描辅助结构体。
    ///
    /// 用于 `strto*` 系列函数直接扫描 C 字符串。
    ///
    /// # 前置条件
    ///
    /// * `s` 指向以 `\0` 结尾的有效 C 字符串
    ///
    /// # 后置条件
    ///
    /// * `rpos` 指向字符串第一个字符
    /// * `shend` 设置为字符串末尾（`base + len`），空终止符不会被读取
    /// * 调用者必须随后调用 `shlim(0)` 完成初始化（若不设置限制）
    pub fn from_string(s: &[u8]) -> Self {
        let base = s.as_ptr();
        let slen = s.len();
        ScanHelper {
            source: ScanSource::String { base, len: slen },
            rpos: base,
            // shend 初始指向字符串末尾边界，getc 的快速路径会在此停止。
            // 当 shlim(0) 调用后，若 lim != 0，shend 会被进一步收紧。
            shend: unsafe { base.add(slen) },
            shlim: -1,
            shcnt_offset: 0,
        }
    }

    /// 从真实 FILE 流构造扫描辅助结构体。
    ///
    /// 用于 `fscanf` 等基于流的扫描。
    ///
    /// # 前置条件
    ///
    /// * `f` 非空，指向一个已初始化的 `File`
    ///
    /// TODO: 当 stdio 模块完整实现后，此函数将从真实 FILE 流读取数据。
    pub fn from_file(_f: *mut File) -> Self {
        // 目前 File 是占位符（零大小），返回一个空的 ScanHelper
        ScanHelper {
            source: ScanSource::File(_f),
            rpos: core::ptr::null(),
            shend: core::ptr::null(),
            shlim: 0,
            shcnt_offset: 0,
        }
    }

    /// 设置字符扫描宽度限制。
    ///
    /// 这是使用扫描辅助接口的必需初始化步骤。
    ///
    /// # 前置条件
    ///
    /// * 字符串源模式下，`lim` 可为零（表示无宽度限制）
    /// * 调用 `shlim` 应在任何 `getc` 调用之前
    ///
    /// # 后置条件
    ///
    /// * `self.shlim = lim`
    /// * `self.shcnt_offset` 被重置，使已读计数归零
    /// * 若 `lim != 0` 且剩余字节数 `> lim`：`self.shend` 被收紧到 `rpos + lim`
    /// * 否则 `shend` 保持不变
    pub fn shlim(&mut self, lim: isize) {
        self.shlim = lim;

        // 复位计数基础：使 cnt() 返回 0。
        // C 等效: f->shcnt = f->buf - f->rpos;  使得 shcnt(f) = 0
        // Rust: cnt() = shcnt_offset + (rpos - base)，因此需要 shcnt_offset = -(rpos - base)
        // 初始状态下 rpos == base，所以 shcnt_offset = 0
        let base = self.base_ptr();
        if base.is_null() {
            self.shcnt_offset = 0;
        } else {
            // SAFETY: base 和 rpos 都指向同一分配的范围内（或 rpos == shend 在边界上）
            self.shcnt_offset = -unsafe { self.rpos.offset_from(base) } as isize;
        }

        // 若设置了宽度限制且当前剩余可读字节超过该限制，则收紧 shend
        if lim != 0 && !self.shend.is_null() {
            // SAFETY: rpos 和 shend 来自同一分配，offset_from 是安全的
            let remaining = unsafe { self.shend.offset_from(self.rpos) } as isize;
            if remaining > lim {
                // SAFETY: lim >= 0 已通过 lim != 0 && remaining > lim 隐式保证
                self.shend = unsafe { self.rpos.offset(lim) };
            }
            // 如果 remaining <= lim，shend 保持不变（字符串末尾边界）
        }
    }

    /// 从扫描源读取下一个字符。
    ///
    /// 内联快速路径：若 `rpos != shend`，直接返回 `*rpos` 并推进 rpos；
    /// 否则调用慢速路径处理缓冲区耗尽或宽度限制。
    ///
    /// # 返回值
    ///
    /// * 成功读取：返回当前字节（转为 `c_int`）
    /// * 达到宽度限制/字符串末尾/IO 错误：返回 `EOF`
    pub fn getc(&mut self) -> c_int {
        // 快速路径：尚有可读字节
        if self.rpos != self.shend {
            // SAFETY: rpos 指向有效分配的范围内（< shend）
            let c = unsafe { *self.rpos };
            // SAFETY: rpos + 1 仍在有效范围内（rpos != shend，所以前进后不会越界）
            self.rpos = unsafe { self.rpos.add(1) };
            return c as c_int;
        }

        // 慢速路径：已到达 shend 边界（字符串末尾或 shlim 限制点）
        self.shgetc_slow()
    }

    /// 慢速路径：处理 getc 无法通过快速路径服务的情况。
    ///
    /// 当 rpos == shend 时调用，检查是否因 shlim 限制而停止。
    fn shgetc_slow(&mut self) -> c_int {
        // 若处于扫描模式（shlim >= 0），检查是否达到宽度限制
        if self.shlim >= 0 {
            let base = self.base_ptr();
            if !base.is_null() {
                // SAFETY: rpos 和 base 指向同一分配
                let cnt = self.shcnt_offset + unsafe { self.rpos.offset_from(base) } as isize;

                // 达到扫描限制 → 标记 EOF 并返回
                if self.shlim != 0 && cnt >= self.shlim {
                    // C 等效: f->shcnt = f->buf - f->rpos + cnt;
                    //        f->shend = f->rpos; f->shlim = -1; return EOF;
                    // 更新 shcnt_offset 使 cnt() 返回最终计数
                    // 新 cnt = self.shcnt_offset + (rpos - base)
                    // 我们希望 cnt() 返回 cnt（当前已读字节数）
                    // SAFETY: rpos 和 base 在同一分配内
                    self.shcnt_offset = cnt - unsafe { self.rpos.offset_from(base) } as isize;
                    self.shend = self.rpos;
                    self.shlim = -1;
                    return EOF;
                }
            }
        }

        // 到达数据源末尾（字符串模式）或 IO 错误（文件模式）
        // 对于字符串模式，shend 在 from_string 中设置为字符串末尾，rpos == shend 意味着已读完
        EOF
    }

    /// 将最近一次 `getc` 读取的字符"推回"。
    ///
    /// # 前置条件
    ///
    /// * `rpos` 未越过缓冲区起始位置
    /// * 仅在 `shlim >= 0` 时有效
    ///
    /// # 后置条件
    ///
    /// * `shlim >= 0` 时：`rpos` 回退一个字节
    /// * `shlim < 0` 时：无操作
    pub fn unget(&mut self) {
        if self.shlim >= 0 {
            // SAFETY: rpos 不能跨越缓冲区起始位置（由调用者保证）
            // 对于字符串模式，只要 rpos > base 就是安全的
            self.rpos = unsafe { self.rpos.offset(-1) };
        }
    }

    /// 返回从当前扫描流中已读取的总字符数。
    ///
    /// 用于 `scanf` 的 `%n` 转换说明符。
    ///
    /// C 等效: `shcnt(f) = f->shcnt + (f->rpos - f->buf)`
    /// Rust: `cnt() = shcnt_offset + (rpos - base)`
    pub fn cnt(&self) -> isize {
        let base = self.base_ptr();
        if base.is_null() {
            self.shcnt_offset
        } else {
            // SAFETY: rpos 和 base 指向同一分配
            self.shcnt_offset + unsafe { self.rpos.offset_from(base) } as isize
        }
    }
}

#[cfg(test)]
mod tests {
    use rusl_core::test;
    use super::*;

    test!("from_string_basic" {
        let s = b"hello\0";
        let sh = ScanHelper::from_string(&s[..]);
        assert_eq!(sh.rpos, s.as_ptr());
        assert_eq!(sh.shend, unsafe { s.as_ptr().add(s.len()) });
        assert_eq!(sh.shlim, -1);
        assert_eq!(sh.shcnt_offset, 0);
    });

    test!("getc_reads_characters" {
        let s = b"abc\0";
        let mut sh = ScanHelper::from_string(&s[..s.len() - 1]); // exclude null
        sh.shlim(0);
        assert_eq!(sh.getc(), 'a' as c_int);
        assert_eq!(sh.getc(), 'b' as c_int);
        assert_eq!(sh.getc(), 'c' as c_int);
        assert_eq!(sh.getc(), EOF);
    });

    test!("getc_with_shlim_restricts_reads" {
        let s = b"12345\0";
        let mut sh = ScanHelper::from_string(&s[..s.len() - 1]);
        sh.shlim(3);
        assert_eq!(sh.getc(), '1' as c_int);
        assert_eq!(sh.getc(), '2' as c_int);
        assert_eq!(sh.getc(), '3' as c_int);
        assert_eq!(sh.getc(), EOF);
    });

    test!("unget_puts_back_character" {
        let s = b"xyz\0";
        let mut sh = ScanHelper::from_string(&s[..s.len() - 1]);
        sh.shlim(0);
        let first = sh.getc();
        assert_eq!(first, 'x' as c_int);
        sh.unget();
        let second = sh.getc();
        assert_eq!(second, 'x' as c_int);
    });

    test!("unget_noop_when_shlim_negative" {
        let s = b"test\0";
        let mut sh = ScanHelper::from_string(&s[..s.len() - 1]);
        sh.shlim(0);
        let c1 = sh.getc();
        // 手动设置 shlim = -1 模拟非扫描模式
        sh.shlim = -1;
        let rpos_before = sh.rpos;
        sh.unget(); // 应无操作
        assert_eq!(sh.rpos, rpos_before);
    });

    test!("cnt_counts_read_bytes" {
        let s = b"abcdef\0";
        let mut sh = ScanHelper::from_string(&s[..s.len() - 1]);
        sh.shlim(0);
        assert_eq!(sh.cnt(), 0);
        sh.getc(); // 'a'
        assert_eq!(sh.cnt(), 1);
        sh.getc(); // 'b'
        sh.getc(); // 'c'
        assert_eq!(sh.cnt(), 3);
        sh.getc(); // 'd'
        sh.getc(); // 'e'
        sh.getc(); // 'f'
        assert_eq!(sh.cnt(), 6);
    });

    test!("cnt_with_shlim" {
        let s = b"1234567890\0";
        let mut sh = ScanHelper::from_string(&s[..s.len() - 1]);
        sh.shlim(5);
        assert_eq!(sh.cnt(), 0);
        sh.getc(); // '1'
        sh.getc(); // '2'
        sh.getc(); // '3'
        assert_eq!(sh.cnt(), 3);
        sh.getc(); // '4'
        sh.getc(); // '5'
        assert_eq!(sh.cnt(), 5);
        assert_eq!(sh.getc(), EOF); // 限制已达
    });

    test!("getc_returns_eof_at_end" {
        let s = b"a\0";
        let mut sh = ScanHelper::from_string(&s[..s.len() - 1]);
        sh.shlim(0);
        assert_eq!(sh.getc(), 'a' as c_int);
        assert_eq!(sh.getc(), EOF);
        assert_eq!(sh.getc(), EOF); // 重复读仍为 EOF
    });

    test!("empty_string_returns_eof" {
        let s: &[u8] = &[];
        let mut sh = ScanHelper::from_string(s);
        sh.shlim(0);
        assert_eq!(sh.getc(), EOF);
    });

    test!("from_file_placeholder" {
        let f: *mut File = core::ptr::null_mut();
        let sh = ScanHelper::from_file(f);
        assert!(sh.shend.is_null());
        assert_eq!(sh.shlim, 0);
    });

    test!("shlim_zero_keeps_shend_at_string_end" {
        let s = b"hello\0";
        let mut sh = ScanHelper::from_string(&s[..s.len() - 1]);
        let expected_end = sh.shend;
        sh.shlim(0);
        assert_eq!(sh.shend, expected_end);
        assert_eq!(sh.shlim, 0);
    });

    test!("shlim_nonzero_tightens_shend" {
        let s = b"hello\0";
        let mut sh = ScanHelper::from_string(&s[..s.len() - 1]);
        sh.shlim(3);
        // shend 应被收紧到 rpos + 3
        let expected = unsafe { s.as_ptr().add(3) };
        assert_eq!(sh.shend, expected);
    });

    test!("unget_then_read_again" {
        let s = b"abc\0";
        let mut sh = ScanHelper::from_string(&s[..s.len() - 1]);
        sh.shlim(0);
        assert_eq!(sh.getc(), 'a' as c_int);
        assert_eq!(sh.getc(), 'b' as c_int);
        sh.unget();
        assert_eq!(sh.getc(), 'b' as c_int);
        assert_eq!(sh.getc(), 'c' as c_int);
    });
}