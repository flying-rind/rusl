# __fmodeflags 函数规约

## 复杂度分级: Level 1

> musl libc 内部工具函数。将 `fopen` 风格的 mode 字符串转换为 `open()` 系统调用的标志位。

---

## 函数接口

```rust
use core::ffi::{c_char, c_int};

extern "C" fn __fmodeflags(mode: *const c_char) -> c_int;
```

[Visibility]: Internal (hidden) — musl 内部实现，不直接对外暴露。由 `fopen`、`freopen` 等内部实现调用。

---

### 前置/后置条件

**[Pre-condition]:**
- `mode`: 非空（non-null）的合法 mode 字符串，以 `'\0'` 结尾，首字符为 `'r'`、`'w'` 或 `'a'`
- mode 字符串仅包含合法修饰字符：`'+'`、`'x'`、`'e'`

**[Post-condition]:**
- 返回组合了对应 `open()` 系统调用标志的 `c_int` 值
- 不修改全局状态
- 不设置 errno

**[Error Behavior]:**
- 本函数不产生错误。对于合法输入始终返回有效标志组合。对非法输入（如 `mode` 为 null 或首字符非法）行为未定义（由调用方保证输入合法性）。

---

### 不变量

**[Invariant]:**
- 纯函数（无副作用）：输入相同 mode 字符串则输出相同标志
- 返回值始终为正数（标志位组合）
- 不访问任何外部状态

---

### 意图

将 C 标准库的 `fopen` mode 字符串（如 `"r"`、`"w+"`、`"a+xe"`）转换为底层 `open()`/`openat()` 系统调用所需的 `O_RDONLY`/`O_WRONLY`/`O_RDWR` 等标志位组合。

Rust 侧实现：
- 输入参数可内部转换为 `&[u8]` 切片进行安全遍历（通过 `core::slice::from_raw_parts`，但需先验证 null 终止）
- 字符查找使用安全的字节切片遍历替代 `strchr`
- 核心逻辑保持纯函数风格：无 unsafe 的副作用
- 可使用 Rust 的 `match` 表达式清晰地表达标志组合逻辑

---

### 系统算法

```
__fmodeflags(mode):
  /* 1. 确定基本访问模式 */
  flags = if '+' in mode_slice:
    O_RDWR                    // 读写模式
  else if mode_slice[0] == b'r':
    O_RDONLY                  // 只读模式
  else:
    O_WRONLY                  // 只写模式

  /* 2. 附加修饰符 */
  if 'x' in mode_slice:  flags |= O_EXCL
  if 'e' in mode_slice:  flags |= O_CLOEXEC
  if mode_slice[0] != b'r': flags |= O_CREAT
  if mode_slice[0] == b'w': flags |= O_TRUNC
  if mode_slice[0] == b'a': flags |= O_APPEND

  return flags
```

---

### Mode 字符含义速查

| 字符 | 标志 | 含义 |
|------|------|------|
| `'r'` | `O_RDONLY` | 只读（文件必须存在） |
| `'w'` | `O_WRONLY \| O_CREAT \| O_TRUNC` | 只写（创建/截断） |
| `'a'` | `O_WRONLY \| O_CREAT \| O_APPEND` | 追加写 |
| `'+'` | 覆盖为 `O_RDWR` | 同时读写 |
| `'x'` | `O_EXCL` | 独占创建（与 `O_CREAT` 一起时，文件已存在则失败） |
| `'e'` | `O_CLOEXEC` | close-on-exec |

---

## 依赖图

```
__fmodeflags
  └─> (core 字符串操作，无外部模块依赖)
```

---

## [RELY]

- 无外部模块依赖。内部使用 Rust `core` 的切片/字节操作。

## [GUARANTEE]

Exported Interface:
  `extern "C" fn __fmodeflags(mode: *const c_char) -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号，行为与原 C 实现完全一致：将 fopen mode 字符串转换为 open() 系统调用标志位。
