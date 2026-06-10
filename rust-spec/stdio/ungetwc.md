# ungetwc 函数规约

## 复杂度分级: Level 2

> musl libc `ungetwc` 实现 — 将一个宽字符推回 FILE 流的输入缓冲区。需要处理多字节编码转换和 locale 管理。

---

## 函数接口

```rust
use core::ffi::{c_int, c_uint};

// wint_t 在 musl 中定义为 unsigned (c_uint)，WEOF 定义为 0xFFFF_FFFFu
type wint_t = c_uint;

extern "C" fn ungetwc(c: wint_t, f: *mut FILE) -> wint_t;
```

[Visibility]: User — `<wchar.h>` 标准库函数，用户程序可直接调用（ISO C 扩展）。在 Rust 侧通过 `#[no_mangle] pub unsafe extern "C"` 导出，保持 ABI 兼容。

---

### 前置/后置条件

**[Pre-condition]:**
- `f`: 有效的 `*mut FILE` 指针，指向已打开的流（非空）
- `c`: 要推回的宽字符值（以 `wint_t` 传递），可以是 `WEOF` 以外的任何有效宽字符
- 流 `f` 必须处于读模式或有足够空间容纳推回字符的多字节表示
- 当前 locale 必须支持 `c` 的多字节编码

**[Post-condition]:**
- Case 1 成功推回（ASCII 字符，`isascii(c)` 为真）:
  - 单字节 `c as u8` 被推回读缓冲区
  - `(*f).rpos` 递减 1
  - 流的 `F_EOF` 标志被清除
  - locale 恢复为调用前的值
  - 返回 `c`

- Case 2 成功推回（非 ASCII 多字节字符）:
  - 宽字符 `c` 通过 `wcrtomb` 转换为多字节序列 `mbc`（长度 `l`）
  - 多字节序列通过 `memcpy` 写入缓冲区（`(*f).rpos` 递减 `l`）
  - 流的 `F_EOF` 标志被清除
  - locale 恢复为调用前的值
  - 返回 `c`

- Case 3 失败（任一下列条件）:
  - `c == WEOF`
  - 流无法进入读模式（`__toread` 后 `rpos` 仍为 null）
  - `wcrtomb` 转换失败（返回 < 0）
  - 推回空间不足（`(*f).rpos < (*f).buf.offset(-(UNGET as isize) + l)`）
  - locale 恢复为调用前的值
  - 返回 `WEOF`

**[Error Behavior]:**
- 推回失败时返回 `WEOF`（与 `ungetwc(WEOF, f)` 返回 `WEOF` 不可区分，符合 POSIX 定义）
- 不设置 errno（标准未要求）

---

### 不变量

**[Invariant]:**
- 无论成功或失败，调用前后的 `CURRENT_LOCALE` 值必须一致
- 推回多字节序列时，字节顺序与 `wcrtomb` 输出一致（即保持多字节序列的正确编码）
- 推回区域始终在缓冲区真实数据之前，`rpos` 可安全回退至 `(*f).buf.offset(-(UNGET as isize))`
- 推回不改变流的错误状态（仅清除 EOF 标志）

---

### 意图

将宽字符 `c` 推回 FILE 流 `f` 的读缓冲区。与 `ungetc` 的核心区别在于：

1. **宽字符支持**：推回的是 `wint_t`（宽字符类型），底层需转换为多字节序列存入字节缓冲区
2. **locale 感知**：必须使用流的 locale 进行宽字符到多字节的转换
3. **多字节序列**：宽字符 `c` 转换为多字节序列 `mbc`（长度最多 `MB_LEN_MAX`），然后推回

**关键设计点**：
- **locale 安全**：函数在入口保存当前 locale，在流的 locale 上下文中执行宽字符转换，最后在返回前恢复原始 locale。这确保了 locale 的线程安全性
- **多字节推回空间检查**：需要确保推回的多字节序列（长度 `l`）加上 `UNGET` 预留空间不超出缓冲区边界
- **宽字符模式**：若流尚未设置方向（`mode <= 0`），通过 `fwide(f, 1)` 将其设置为宽字符模式

Rust 侧实现：
- `wint_t` 定义为 `c_uint`（musl 中为 `unsigned`）
- `WEOF` 定义为 `0xFFFF_FFFFu32 as wint_t`（即 `(wint_t)(-1)`）
- `MB_LEN_MAX` 定义为常量 `const MB_LEN_MAX: usize = 4;`
- `mbc` 为栈上分配的 `[u8; MB_LEN_MAX]` 数组
- locale 切换通过内部 locale 模块的函数实现
- `isascii` 通过位运算 `c & !0x7F == 0` 实现，避免外部依赖
- `memcpy` 使用 `core::ptr::copy_nonoverlapping` 替代（更 Rust 风格）

### 系统算法

```
ungetwc(c, f):
  1. 保存当前 locale: loc = CURRENT_LOCALE
  2. FLOCK(f) — 获取流锁
  3. 若 (*f).mode <= 0，调用 fwide(f, 1) 设置为宽字符模式
  4. 切换到流的 locale: CURRENT_LOCALE = (*f).locale
  5. 若 (*f).rpos 为 null，调用 __toread(f) 初始化读模式
  6. 若以下任一条件成立，失败:
       - (*f).rpos == null（无法进入读模式）
       - c == WEOF
       - wcrtomb(mbc, c, null) 返回 < 0（转换失败）
       - (*f).rpos < (*f).buf.offset(-(UNGET as isize) + l)（空间不足）
     → 释放锁，恢复 locale = loc，返回 WEOF
  7. 若 isascii(c): 单字节推回
       (*f).rpos = (*f).rpos.offset(-1)
       *(*f).rpos = c as u8
     否则: 多字节推回
       (*f).rpos = (*f).rpos.offset(-(l as isize))
       copy_nonoverlapping(mbc.as_ptr(), (*f).rpos, l)
  8. (*f).flags &= !F_EOF — 清除 EOF 标志
  9. FUNLOCK(f) — 释放锁
  10. 恢复 locale: CURRENT_LOCALE = loc
  11. 返回 c
```

时间复杂度 O(1)（不含 `__toread` 的模式切换和 `wcrtomb` 的多字节转换）。

---

## 常量引用

| 常量 | 值 | 来源 | 说明 |
|------|-----|------|------|
| `WEOF` | `(wint_t)(-1)` 即 `0xFFFF_FFFFu` | `<wchar.h>` | 宽字符 EOF 标志，不可推回 |
| `MB_LEN_MAX` | 4（典型） | `<limits.h>` | 多字节字符最大字节数 |
| `UNGET` | 8 | `stdio_impl.h` | 字符回退预留空间大小 |
| `F_EOF` | 16 | `stdio_impl.h` | 流 EOF 状态标志位 |

---

## 依赖图

```
ungetwc (Public)
  ├── FLOCK / FUNLOCK (锁宏)
  │     ├── __lockfile (see __lockfile spec)
  │     └── __unlockfile (see __unlockfile spec)
  ├── __toread (see __toread spec)
  ├── fwide (from locale/wchar module)
  ├── wcrtomb (from locale/wchar module)
  ├── memcpy (or core::ptr::copy_nonoverlapping)
  ├── isascii (inline bit-test)
  └── CURRENT_LOCALE / f.locale (locale 管理)
```

---

## [RELY]

- `__toread(f: *mut FILE)` — 将流切换到读模式（见 `__toread` spec）
- `__lockfile(f: *mut FILE)` / `__unlockfile(f: *mut FILE)` — 流加锁/解锁（见 `__lockfile` spec）
- `fwide(f: *mut FILE, mode: c_int) -> c_int` — 设置/查询流宽窄模式
- `wcrtomb(s: *mut c_char, wc: wchar_t, ps: *mut mbstate_t) -> usize` — 宽字符到多字节转换
- `core::ptr::copy_nonoverlapping` — 内存拷贝（等价于 C 的 `memcpy`）
- `isascii(c)` — ASCII 检测，可通过位运算 `c & !0x7F == 0` 内联实现
- `CURRENT_LOCALE` / locale 管理 — 线程 locale 切换
- `FILE` 结构体定义（含 `rpos`、`buf`、`flags`、`lock`、`mode`、`locale` 字段）
- 常量 `WEOF`、`MB_LEN_MAX`、`UNGET`、`F_EOF`

## [GUARANTEE]

Exported Interface:
  `extern "C" fn ungetwc(c: wint_t, f: *mut FILE) -> wint_t;`

本模块保证对外提供 ABI 兼容的 `ungetwc` 函数符号，行为符合 POSIX/C11 标准扩展定义。保证无论成功或失败，调用前后的 locale 值一致（locale 安全性）。推回多字节序列时，字节顺序与 locale 编码规则一致。
