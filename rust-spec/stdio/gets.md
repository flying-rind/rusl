# gets 函数规约

## 复杂度分级: Level 1

> musl libc 标准库字符串输入（无边界检查）函数。从 stdin 读取一行到用户缓冲区。Rust 实现中，外部接口保持 ABI 兼容，内部读取逻辑用 Rust 安全重构。

**严重安全警告**: 此函数不对缓冲区进行边界检查，无法安全使用。C11 标准已移除该函数，POSIX.1-2008 标记为过时。但出于 ABI 兼容性要求，仍须提供符号。任何新代码必须使用 `fgets` 替代。

---

## 函数接口

```rust
use core::ffi::{c_char, c_int};

unsafe extern "C" fn gets(s: *mut c_char) -> *mut c_char;
```

[Visibility]: User — `<stdio.h>` 标准库函数（C99 后标记为过时，C11 已移除）。必须保持 ABI 兼容。`s` 为输出缓冲区指针（**无大小信息**，这是不安全的根源）。返回 `s` 表示成功，NULL 表示失败或 EOF。

---

### 前置/后置条件

**[Pre-condition]:**
- `s`: 非空指针，指向足够大的字符数组（**调用者负责确保缓冲区足够容纳任意输入**，此函数无法验证）。
- `stdin` 已初始化且可读。

**[Post-condition]:**
- **Case 1 成功读取（读到换行符，或在 EOF 前有数据）**
  - `s` 中包含读取的字符（去除了末尾 `\n`），以 NUL 结尾。
  - 返回 `s`（指向用户缓冲区的指针）。
  - 换行符 `\n` 被从流中消耗但**不**存入缓冲区。

- **Case 2 读取失败**
  - 遇到 EOF 且未读到任何字符，或发生 I/O 错误。
  - 返回 `core::ptr::null_mut()`。
  - `s` 的内容未定义（musl 实现中若零字节 EOF 会将 `s[0]` 设为 `\0`）。
  - `stdin` 的 EOF 或 error 标志被设置。

**[Error Behavior]:**
- 立即 EOF（未读到任何字符）-> 返回 NULL。
- I/O 读取错误 -> 返回 NULL。
- 读到数据后遇到 EOF -> 视为成功，返回 `s`。

---

### 不变量

**[Invariant]:**
- 始终从 `stdin` 读取。
- 换行符 `\n` 被消耗但不写入缓冲区。
- 缓冲区始终以 NUL 终止（在返回之前）。
- **无缓冲区边界检查** —— 这是严重安全漏洞的根源。

---

### 意图

从 `stdin` 读取字符直到遇到换行符 `\n` 或文件末尾，将读取的字符（不包括 `\n`）存入用户缓冲区 `s`，并在末尾添加 NUL 终止符。

Rust 侧实现：
- 外部接口使用 `unsafe extern "C" fn gets(s: *mut c_char) -> *mut c_char`，保持 ABI 兼容。
- 内部 I/O 读取使用 `getc_unlocked` 的安全包装函数。
- `FLOCK`/`FUNLOCK` 使用 Rust RAII 守卫管理锁。
- 缓冲区的字节写入通过 `*mut c_char` 裸指针（在 unsafe 块中）进行，与 C 行为一致——**安全的 Rust 也无法阻止缓冲区溢出，因为不持有缓冲区大小信息**。
- `feof` 检查使用内部函数的安全包装。
- 失败判断逻辑用 Rust `match` 或 `if` 清晰表达（详见下方状态表）。

### 系统算法

```
gets(s):
  1. i = 0
  2. 锁定 stdin

  3. 循环读取:
     loop {
       c = getc_unlocked(stdin)
       若 c == EOF: break
       若 c == '\n': break
       s[i] = c as u8
       i += 1
     }

  4. s[i] = 0  // NUL 终止符

  5. 失败判断:
     若 c != '\n' 且 (!feof(stdin) 或 i == 0):
       // 情况A: 读错误 或 零字节 EOF
       s = null_mut()

  6. 解锁 stdin
  7. 返回 s
```

**失败判断状态表**:

| c 最终值 | feof(stdin) | i | 返回值 | 说明 |
|----------|------------|---|--------|------|
| `'\n'` | — | 任意 | `s` | 正常遇到换行符，成功 |
| `EOF` | true | `>0` | `s` | 读到数据后遇到 EOF |
| `EOF` | true | `0` | NULL | 遇到 EOF，未读到任何数据 |
| `EOF` | false | 任意 | NULL | I/O 读取错误 |

时间复杂度 O(输入行长度)。可能因等待输入而阻塞。

---

## 依赖图

```
gets (Public, extern "C")
  ├── core::ffi::{c_char, c_int}      — Rust 内置 FFI 类型
  ├── [Internal] getc_unlocked         — 内部无锁单字符读取函数
  ├── [Internal] feof                  — 内部文件结束标志检查
  ├── [Internal] stdin                 — 标准输入流全局变量
  ├── [Internal] FLOCK / FUNLOCK       — FILE 锁操作
  └── [Internal] EOF                   — 文件结束常量
```

---

## [RELY]

- `core::ffi::{c_char, c_int}` — Rust 核心库 FFI 类型。
- 内部 `getc_unlocked` — rusl 内部无锁单字符读取。
- 内部 `feof` — rusl 内部 EOF 标志检查。
- 内部 `stdin` — rusl 内部标准输入流。
- 内部 FILE 锁定机制 — rusl 内部实现。

## [GUARANTEE]

Exported Interface:
  `unsafe extern "C" fn gets(s: *mut c_char) -> *mut c_char;`

本模块保证对外提供上述 ABI 兼容的函数符号：
- 参数 `*mut c_char` 与 C `char *` 内存布局一致。
- 返回值 `*mut c_char` 与 C `char *` 内存布局一致。
- 使用 C 调用约定 (`extern "C"`)。
- 行为符合 C89 `gets()` 语义：从 stdin 读取一行（去除 `\n`），无缓冲区边界检查。

**严重警告**: 此函数本质不安全，C11 已移除。任何新代码应使用 `fgets` 替代。本函数仅出于 ABI 兼容性的历史遗留原因存在。
