# shgetc.h 规约

> **来源文件**: `musl/src/internal/shgetc.h`
> **复杂度层级**: Level 2 — 复杂逻辑（自定义字符扫描状态机）
> **依赖图**:
> ```
> stdio_impl.h (提供 struct _IO_FILE 定义)
>   -> __shlim() — 设置字符扫描宽度限制
>     -> __shgetc() — 从扫描流获取下一个字符
>       -> shcnt / shlim / shgetc / shunget — 内联便捷宏
>         -> sh_fromstring — 字符串伪 FILE 初始化宏
> ```

---

## 概述

`shgetc.h` 定义了 musl 内部的"扫描辅助"（scan helper）字符输入函数族。这些函数为 `scanf` 系列和 `strto*` 系列函数提供统一的字符级输入接口，支持两种底层数据源：
1. **真实 `FILE *`**：常规文件/流，通过 stdio 缓冲区读取
2. **伪 `FILE *`**：以字符串为数据源，缓冲区指针直接指向 `\0` 结尾的字符串

**不变量 (Invariants)**：
- **I1**: 伪 FILE 的 `rend` 必须被设置为 `(void *)-1`，作为"无界缓冲区"标记；此时 `rpos` 永远不应超过 `rend`。
- **I2**: `shlim` 非零值仅在真实 FILE 上有效；伪 FILE 上设置非零 `shlim` 属于未定义行为。
- **I3**: 调用 `shunget` 后 `rpos` 仍应满足 `rpos >= buf`（不跨越缓冲区边界）；在伪 FILE 下可回退到字符串起始位置，在真实 FILE 下仅支持单字节回退。

---

## 函数声明

### `void __shlim(FILE *, off_t)`

```c
void __shlim(FILE *, off_t);
```

[Visibility]: Internal — musl 内部 scanf/strto 辅助函数，声明于 `shgetc.h`，POSIX/C 标准未定义

**意图 (Intent)**：
为给定的 `FILE *` 设置字符扫描宽度限制（lim = limit）。之后 `shgetc` / `shcnt` 将受此限制约束。该函数是使用扫描辅助接口的**必须初始化步骤**。

**前置条件 (Preconditions)**：
- **P1**: `f` 非 NULL，指向一个已正确初始化的 `FILE`（真实流或伪字符串 FILE）。
- **P2**: 对伪字符串 FILE（由 `sh_fromstring` 初始化），`lim` **必须**为 0。传入非零 `lim` 导致**未定义行为**。
- **P3**: 调用 `__shlim` 应在任何 `shgetc` 调用之前。

**后置条件 (Postconditions)**：
- **Q1**: `f->shlim` 被设置为 `lim`。
- **Q2**: `f->shcnt` 被设置为 0（重置已读取计数）。
- **Q3**: 若 `f->rend` 为 `(void *)-1`（伪 FILE），内部标记该流为无界字符串源。

**系统算法 (System Algorithm)**：
```
f->shcnt = 0;
f->shlim = lim;
if (pseudo FILE):  // f->rend == (void*)-1
    f->shend = (void*)-1;  // 标记为无界
else:
    f->shend = f->rpos;    // 计算限制边界
```

---

### `int __shgetc(FILE *)`

```c
int __shgetc(FILE *);
```

[Visibility]: Internal — musl 内部 scanf 字符获取，声明于 `shgetc.h`，POSIX/C 标准未定义

**意图 (Intent)**：
从 `FILE *` 读取下一个字符，若底层缓冲区耗尽则自动填充（真实 FILE）或返回 EOF（伪 FILE 到达字符串末尾）。

**前置条件 (Preconditions)**：
- **P1**: `f` 非 NULL，且 `f->shlim` 已通过 `__shlim()` 正确设置。
- **P2**: 若 `f` 为真实流，底层文件描述符应有效；若为伪 FILE，`f->buf` 指向有效的 `\0` 结尾字符串。

**后置条件 (Postconditions)**：
- **Case 1（成功读取普通字符）**：
  - 返回值为 `(unsigned char)*f->rpos`（当前字符）
  - `f->rpos++`，`f->shcnt++`
- **Case 2（达到扫描宽度限制）**：
  - 返回 `EOF`（`shcnt >= shlim && shlim > 0`）
- **Case 3（缓冲区耗尽，需要填充）**：
  - 对真实 FILE：调用底层 `__uflow` 填充缓冲区，再重新尝试读取
  - 对伪 FILE（到达 `\0`）：返回 `EOF`
- **Case 4（底层 I/O 错误）**：
  - 返回 `EOF`，FILE 的 `F_ERR` 标志可能被设置

---

## 宏定义

### `shcnt(f)`

```c
#define shcnt(f) ((f)->shcnt + ((f)->rpos - (f)->buf))
```

[Visibility]: Internal — musl 内部便捷宏

**意图**: 返回从当前扫描流中**已读取的总字符数**。

`f->shcnt` 记录填充缓冲区前已读取的数量；`(f->rpos - f->buf)` 计算当前缓冲区中已消耗的字节数。两者相加得总读数。

**用途**: `scanf` 使用此宏来实现 `%n` 转换说明符（记录当前已处理的字符数）。

---

### `shlim(f, lim)`

```c
#define shlim(f, lim) __shlim((f), (lim))
```

[Visibility]: Internal — musl 内部便捷宏

**意图**: `__shlim()` 的直接包装，设置扫描宽度限制。

---

### `shgetc(f)`

```c
#define shgetc(f) (((f)->rpos != (f)->shend) ? *(f)->rpos++ : __shgetc(f))
```

[Visibility]: Internal — musl 内部便捷宏

**意图**: 从扫描流获取下一个字符。内联快速路径：若 `rpos != shend`（缓冲区中还有字符），直接返回 `*rpos++`；否则调用 `__shgetc()` 进行慢速路径处理（填充缓冲区 / 检查限制）。

**前置条件**: `f->shend` 必须已设置（通过 `__shlim()` 初始化）。

**语义要点**:
- `shend` 在伪 FILE 下为 `(unsigned char *)-1`（最大值），此时 `rpos != shend` 始终为真，直到 `rpos` 溢出，但伪 FILE 的 `rpos` 指向 `\0` 终止字符串，最终 `__shgetc` 检测到 `\0` 后返回 EOF
- 在真实 FILE 下 `shend` 指向限制边界，`rpos == shend` 意味着达到宽度限制

---

### `shunget(f)`

```c
#define shunget(f) ((f)->shlim>=0 ? (void)(f)->rpos-- : (void)0)
```

[Visibility]: Internal — musl 内部便捷宏

**意图**: 将最近一次 `shgetc` 读取的字符"推回"，使下一次 `shgetc` 重新读取该字符。

**前置条件**：
- `f->rpos > f->buf`（不会回退到缓冲区起始位置之前）
- 仅当 `shlim >= 0` 时有效（`shlim` 被用于标记该流正在使用扫描接口）

**后置条件 (Postconditions)**：
- **Case 1 (`shlim >= 0`，正常扫描模式)**：
  - `f->rpos--`（回退一个字符位置）
  - 下一次 `shgetc(f)` 将重新返回该字符
- **Case 2 (`shlim < 0`，非扫描模式)**：
  - 无操作（`(void)0`），不会修改 FILE 状态

**实现说明**：
- 真实 FILE：仅支持单字节 pushback。后续 `shunget` 可能调整位置但不恢复数据（因为底层 stdio 的 `ungetc` 缓冲区有限）
- 伪 FILE：支持无界 pushback，可一直回退到字符串起始处（因为数据未移动，仅修改指针）

---

### `sh_fromstring(f, s)`

```c
#define sh_fromstring(f, s) \
    ((f)->buf = (f)->rpos = (void *)(s), (f)->rend = (void*)-1)
```

[Visibility]: Internal — musl 内部便捷宏

**意图**: 将字符串初始化为伪 FILE 对象，用于 `strto*` 系列函数（`strtol`、`strtod` 等）直接扫描 C 字符串。

**前置条件 (Preconditions)**：
- **P1**: `f` 指向一个已分配但内容可覆盖的 `FILE` 结构体（不需要完整初始化）。
- **P2**: `s` 指向以 `\0` 结尾的 C 字符串（可从头开始读取）。

**后置条件 (Postconditions)**：
- **Q1**: `f->buf = f->rpos = (void *)s` — 读指针和缓冲区起始指针均指向字符串开头
- **Q2**: `f->rend = (void *)-1` — 标记为无界伪 FILE（`rend` 永远大于 `rpos`）
- **Q3**: `FILE` 结构的其余字段**未被初始化**（不应被 `shgetc` 之外的操作访问）

**使用注意**: `sh_fromstring` 之后必须调用 `__shlim(f, 0)` 来最终完成扫描初始化。

---

## 使用场景与生命周期

### 真实 FILE 使用模式
```c
shlim(f, width);         // 1. 设置扫描宽度
while ((c = shgetc(f)) != EOF) { ... }  // 2. 读取字符
nread = shcnt(f);        // 3. 获取已读字符数
```

### 字符串伪 FILE 使用模式
```c
sh_fromstring(&f, s);    // 1. 初始化伪 FILE
shlim(&f, 0);            // 2. 设置无限制扫描
while ((c = shgetc(&f)) != EOF) { ... }  // 3. 读取（直到 '\0'）
nread = shcnt(&f);       // 4. 获取已读字符数
```

---

## 跨文件依赖

| 依赖符号 | 来源 | 处理方式 |
|---------|------|---------|
| `struct _IO_FILE` | `stdio_impl.h`（musl 内部） | 跨文件定义，`shgetc.h` 通过 `#include "stdio_impl.h"` 引入 |
| `__shlim()` | `src/stdio/__shlim.c` | 跨文件实现 |
| `__shgetc()` | `src/stdio/__shgetc.c` | 跨文件实现 |
| `<stdio.h>` | 标准 C 库 | 外部依赖（提供 `FILE` 类型名和 `EOF`） |
| `off_t` | POSIX `<sys/types.h>` 等 | 外部类型依赖 |

---

## 实现指南 (rusl/Rust)

- `sh_fromstring` → 构造一个包含字符串引用和指针的结构体，而非复用 FILE
  ```rust
  struct ScanHelper<'a> {
      buf: *const u8,
      rpos: *const u8,
      rend: *const u8,  // 伪 FILE 时设为 usize::MAX
      shlim: isize,
      shcnt: isize,
  }
  ```
- `shgetc` → 快速路径 `if rpos < rend { let c = *rpos; rpos = rpos.add(1); c } else { shgetc_slow() }`
- `shunget` → `rpos = rpos.offset(-1)`
- `shcnt` → `shcnt + (rpos - buf)`
- 伪 FILE 标记 `rend = (void*)-1` → 在 Rust 中用 `Option<NonNull<u8>>` 区分"有界"和"无界"模式
- 内联宏在 Rust 中使用 `#[inline(always)]` 函数或泛型实现

---

## 实现文件补充说明

`__shlim()` 和 `__shgetc()` 的具体实现在 `src/stdio/__shlim.c` 和 `src/stdio/__shgetc.c` 中。关键设计细节：

- **shcnt 偏移约定**: `shcnt` 字段存储的是 `cnt_actual - (rpos - buf)` 而非直接存储实际计数。这是为了允许内联快速路径 `shgetc` 宏仅通过 `*rpos++` 推进读指针而不额外更新 `shcnt`；`shcnt(f)` 宏通过加上偏移自动修正。仅在 `__shlim` 和 `__shgetc` 慢速路径中重新校准 `shcnt` 存储值。
- **两阶段读取**: 阶段 1 为内联快速路径（`rpos != shend` 时直接读取），阶段 2 为函数慢速路径（缓冲区耗尽或宽度限制触发时调用 `__shgetc`）。
- **rpos[-1] 回存**: 真实 FILE 中，当缓冲区耗尽需填充新数据时，将刚读到的字符存入 `rpos[-1]` 供 `shunget` 的单字节 pushback 使用。