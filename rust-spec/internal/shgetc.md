# shgetc.rs 规约 (Rust)

> **来源 C spec**: `musl/src/internal/spec/shgetc.md`
> **对应源文件**: `musl/src/internal/shgetc.h`
> **复杂度层级**: Level 2 — 复杂逻辑（自定义字符扫描状态机）

---

## 依赖图

```
stdio (提供 File 结构体定义)
  └── shlim() — 设置字符扫描宽度限制
       └── shgetc() — 从扫描流获取下一个字符
            └── ScanHelper — 封装扫描状态和操作
```

---

## 概述

`shgetc` 模块定义了 rusl 内部的"扫描辅助"（scan helper）字符输入抽象。该模块为 `scanf` 系列和 `strto*` 系列函数提供统一的字符级输入接口，支持两种底层数据源：

1. **真实 `File`**：常规文件/流，通过 stdio 缓冲区读取
2. **字符串源**：以 `\0` 结尾的字符串为数据源，直接通过指针读取

**不变量 (Invariants)**：
- **I1**: 字符串源模式下，`rend` 始终为 `usize::MAX`（标记为无界缓冲区）；此时 `rpos` 永不超过字符串末尾的 `\0`。
- **I2**: `shlim` 非零值仅在真实 File 模式上有效；字符串源模式下设置非零 `shlim` 属于未定义行为。
- **I3**: 调用 `shunget` 后 `rpos` 仍应满足 `rpos >= buf`（不跨越缓冲区边界）；在字符串模式下可回退到字符串起始位置，在真实 File 模式下仅支持单字节回退。

---

## 类型定义

### `ScanHelper` — 扫描辅助结构体

```rust
// Rust 签名
pub(crate) enum ScanSource {
    /// 真实 FILE 流
    File(*mut File),
    /// 字符串源：起始指针 + 长度（不含 '\0'）
    String {
        base: *const u8,
        len: usize,
    },
}

pub(crate) struct ScanHelper {
    source: ScanSource,
    rpos: *const u8,        // 当前读指针
    shend: *const u8,       // 扫描限制边界（字符串模式下为 usize::MAX）
    shlim: off_t,           // 扫描宽度限制
    shcnt_offset: off_t,    // shcnt 偏移量 = cnt_actual - (rpos - buf)
}
```

[Visibility]: Internal — rusl 内部 scanf/strto 扫描辅助结构体，POSIX/C 标准未定义

**设计说明**:

在 C 实现中，扫描状态被分散嵌入 `FILE` 结构体的若干字段（`buf`、`rpos`、`rend`、`shlim`、`shcnt`），并通过宏直接访问。Rust 设计中将其抽象为独立的 `ScanHelper` 结构体，实现关注点分离：

- `ScanSource` 枚举区分两种数据源，避免"伪 FILE"的 hack（`rend = (void*)-1` 哨兵值）
- `shcnt_offset` 存储实际计数与 `(rpos - buf)` 的差值，保持与 C 实现相同的"偏移约定"：快速路径的 `getc()` 只需推进 `rpos` 而不更新 `shcnt`，仅在慢速路径和 `cnt()` 方法中重新校准

---

## 方法声明

### `ScanHelper::shlim(&mut self, lim: off_t)`

```rust
// Rust 签名
pub(crate) fn shlim(&mut self, lim: off_t)
```

[Visibility]: Internal

**意图 (Intent)**：
为扫描辅助结构体设置字符扫描宽度限制。该方法是使用扫描辅助接口的**必须初始化步骤**。

**前置条件 (Preconditions)**：
- **P1**: 对字符串源模式，`lim` **必须**为 0。传入非零 `lim` 导致**未定义行为**。
- **P2**: 调用 `shlim` 应在任何 `getc` 调用之前。

**后置条件 (Postconditions)**：
- **Q1**: `self.shlim = lim`
- **Q2**: `self.shcnt_offset` 被重置（校准为当前 `rpos` 位置）
- **Q3**: 对于真实 File 模式，`self.shend` 被设置为限制边界；对于字符串模式，`self.shend` 保持不变（`usize::MAX`）

**系统算法 (System Algorithm)**：
```
self.shcnt_offset = -(rpos - buf);  // 等效于 shcnt = 0
self.shlim = lim;
if 真实 File:
    self.shend = 限制边界;
```

---

### `ScanHelper::getc(&mut self) -> c_int`

```rust
// Rust 签名
pub(crate) fn getc(&mut self) -> c_int
```

[Visibility]: Internal

**意图 (Intent)**：
从扫描源读取下一个字符。该函数内联快速路径：若 `rpos != shend`（还有字符可在缓冲区中读取），直接返回 `*rpos++`；否则调用慢速路径 `shgetc_slow` 处理缓冲区耗尽或宽度限制。

**前置条件 (Preconditions)**：
- **P1**: 已通过 `shlim()` 正确设置扫描限制。
- **P2**: 若为真实 File 模式，底层文件描述符应有效。

**后置条件 (Postconditions)**：
- **Case 1（成功读取普通字符）**：
  - 返回值为当前 `*rpos` 所指向的字节（转为 `c_int`，无符号扩展）
  - `rpos` 前进一个字节
- **Case 2（达到扫描宽度限制）**：
  - 返回 `EOF`
- **Case 3（缓冲区耗尽，需要填充 — 真实 File）**：
  - 调用底层 `__uflow` 填充缓冲区，再重新尝试读取
- **Case 4（到达字符串末尾 `\0`）**：
  - 返回 `EOF`
- **Case 5（底层 I/O 错误）**：
  - 返回 `EOF`

**快速路径内联伪代码**：
```rust
if self.rpos != self.shend {
    let c = unsafe { *self.rpos };
    self.rpos = unsafe { self.rpos.add(1) };
    c as c_int & 0xFF
} else {
    self.shgetc_slow()
}
```

---

### `ScanHelper::unget(&mut self)`

```rust
// Rust 签名
pub(crate) fn unget(&mut self)
```

[Visibility]: Internal

**意图 (Intent)**：
将最近一次 `getc` 读取的字符"推回"，使下一次 `getc` 重新读取该字符。

**前置条件**：
- `self.rpos > self.source_base_ptr()`（不会回退到缓冲区起始位置之前）
- 仅当 `self.shlim >= 0` 时有效（标记扫描模式激活）

**后置条件 (Postconditions)**：
- **Case 1 (`shlim >= 0`，正常扫描模式)**：
  - `self.rpos` 回退一个字节
  - 下一次 `getc()` 将重新返回该字符
- **Case 2 (`shlim < 0`，非扫描模式)**：
  - 无操作，不会修改任何状态

---

### `ScanHelper::cnt(&self) -> off_t`

```rust
// Rust 签名
pub(crate) fn cnt(&self) -> off_t
```

[Visibility]: Internal

**意图 (Intent)**：
返回从当前扫描流中**已读取的总字符数**。

在 C 实现中，`shcnt` 存储的是偏移量而非实际计数，以允许内联快速路径 `shgetc` 宏仅推进 `rpos` 而不额外更新计数器。`shcnt(f)` 宏通过 `shcnt + (rpos - buf)` 自动修正。

Rust 实现保持相同设计：`shcnt_offset` 存储 `cnt_actual - (rpos - buf)`。

**用途**: `scanf` 使用此方法实现 `%n` 转换说明符。

---

### `ScanHelper::from_string(s: &[u8]) -> ScanHelper`

```rust
// Rust 签名
pub(crate) fn from_string(s: &[u8]) -> ScanHelper
```

[Visibility]: Internal

**意图 (Intent)**：
从 C 字符串（以 `\0` 结尾的字节序列）构造扫描辅助结构体，用于 `strto*` 系列函数（`strtol`、`strtod` 等）直接扫描 C 字符串。

**前置条件 (Preconditions)**：
- **P1**: `s` 指向以 `\0` 结尾的有效 C 字符串（可从头开始读取）。

**后置条件 (Postconditions)**：
- **Q1**: 返回的 `ScanHelper` 的 `rpos` 指向字符串第一个字符
- **Q2**: `source` 设置为 `ScanSource::String { base, len }`
- **Q3**: `shend` 设置为 `usize::MAX`（标记无界模式）
- **Q4**: 调用者必须随后调用 `shlim(0)` 完成初始化（这与 C 的 `sh_fromstring` + `__shlim(f, 0)` 两步骤模式一致）

**使用注意**: `from_string` 之后必须调用 `shlim(&mut self, 0)` 来最终完成扫描初始化。

---

### `ScanHelper::from_file(f: *mut File) -> ScanHelper`

```rust
// Rust 签名
pub(crate) fn from_file(f: *mut File) -> ScanHelper
```

[Visibility]: Internal

**意图 (Intent)**：
从真实 FILE 流构造扫描辅助结构体，用于 `fscanf` 等基于流的扫描。

**前置条件 (Preconditions)**：
- **P1**: `f` 非空，指向一个已正确初始化的 `File`。

---

## 使用场景与生命周期

### 真实 File 使用模式

```rust
let mut sh = ScanHelper::from_file(file_ptr);
sh.shlim(width);                          // 1. 设置扫描宽度
while { let c = sh.getc(); c != EOF } {   // 2. 读取字符
    // 处理字符 c
}
let nread = sh.cnt();                     // 3. 获取已读字符数
```

### 字符串源使用模式

```rust
let s = b"12345\0";
let mut sh = ScanHelper::from_string(s);  // 1. 初始化
sh.shlim(0);                               // 2. 设置无限制扫描
while { let c = sh.getc(); c != EOF } {   // 3. 读取（直到 '\0'）
    // 处理字符 c
}
let nread = sh.cnt();                     // 4. 获取已读字符数
```

---

## 慢速路径内部函数

以下函数为 `ScanHelper` 的内部实现细节，不对外暴露：

### `shgetc_slow(sh: &mut ScanHelper) -> c_int`

[Visibility]: Internal (模块私有)

**意图**: 处理 `getc` 快速路径无法服务的情况：
- 对真实 File：缓冲区耗尽，需调用底层 I/O 填充
- 对字符串源：已达到 `\0` 终止符
- 达到扫描宽度限制

### `shlim_real_file(sh: &mut ScanHelper, f: &File)`

[Visibility]: Internal (模块私有)

**意图**: 为真实 File 模式计算 `shend` 限制边界。

---

## 跨文件依赖

| 依赖符号 | 来源 | 处理方式 |
|---------|------|---------|
| `File` | `stdio` 模块（rusl 内部） | 跨文件定义 |
| `__uflow` | `stdio` 模块（rusl 内部） | 底层缓冲区填充函数 |
| `c_int`, `off_t` | `core::ffi` | Rust core FFI 类型 |
| `EOF` | `stdio` 模块（rusl 内部） | EOF 常量 |

---

## Rust 设计要点

- `ScanSource` 枚举替代 C 的"伪 FILE" hack（`rend = (void*)-1`），提高代码可读性和类型安全
- `ScanHelper` 封装所有扫描状态，关注点分离于 `File` 结构体之外
- `getc()` 方法内联快速路径，`shgetc_slow()` 处理慢速路径（缓冲区填充/宽度限制/EOF）
- `unget()` 仅当 `shlim >= 0` 时有效（通过检查而非宏条件编译）
- `cnt()` 方法使用 `shcnt_offset + (rpos - base) ` 计算，与 C 偏移约定一致
- 字符串源模式下 `from_string` 接受 `&[u8]`，内部存储裸指针（因 `ScanHelper` 需跨函数调用）
- 模块整体 `#![no_std]` 兼容

---

## RELY / GUARANTEE

```
[RELY]
Rust Core 内建类型:
  core::ffi::c_int             // 依赖1: C int 类型（字符返回值）
  core::ffi::c_void            // 依赖2: C void 指针

rusl 内部模块:
  stdio::File                  // 依赖1: 文件流结构体定义
  stdio::__uflow               // 依赖2: 底层缓冲区填充（真实 File 慢速路径）
  stdio::EOF                   // 依赖3: EOF 常量

[GUARANTEE]
pub(crate) 接口:
  enum ScanSource              // 扫描数据源类型（File / String）
  struct ScanHelper            // 扫描辅助结构体
  fn ScanHelper::from_string(s: &[u8]) -> ScanHelper    // 从字符串构造
  fn ScanHelper::from_file(f: *mut File) -> ScanHelper  // 从 FILE 构造
  fn ScanHelper::shlim(&mut self, lim: off_t)           // 设置扫描宽度限制
  fn ScanHelper::getc(&mut self) -> c_int               // 获取下一个字符
  fn ScanHelper::unget(&mut self)                       // 回退一个字符
  fn ScanHelper::cnt(&self) -> off_t                    // 获取已读取的字符数
```