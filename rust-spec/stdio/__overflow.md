# __overflow 函数规约

## 复杂度分级: Level 2

> musl libc 内部输出缓冲区溢出处理实现。当 `putc_unlocked` 宏检测到写缓冲区满或需要特殊处理时调用，负责将单个字符写入 FILE 流。

---

## 函数接口

```rust
use core::ffi::c_int;

extern "C" fn __overflow(f: *mut FILE, _c: c_int) -> c_int;
```

[Visibility]: Internal — musl 内部实现，但编译为 `protected` 可见性（非 hidden），因为被 `putc_unlocked` 宏直接引用。用户代码通过宏间接调用。

---

### 前置/后置条件

**[Pre-condition]:**
- `f`: `*mut FILE`，非空指针
- `_c`: 要写入的字符（作为 `c_int` 传递，内部转为 `u8`，即取低 8 位）
- `(*f).write` 已设置为有效的写函数（如 `__stdio_write` 或 `__stdout_write`）

**[Post-condition]:**

**Case 1: 成功写入**
- 字符被写入缓冲区（`*wpos++ = c`）或通过 `(*f).write` 写出
- 返回写入的字符值（`c`，即原字符转为 `u8` 后的值，范围 `0..=255`）

**Case 2: 失败**
- 若 `__towrite` 失败（写模式切换失败）：返回 `EOF`
- 若 `(*f).write` 写出的字节数不为 1：返回 `EOF`

**[Error Behavior]:**
- 写模式切换失败：返回 `EOF`，errno 由底层操作设置
- 写操作失败：返回 `EOF`，errno 由底层写函数设置
- 注意：`EOF` 在 Rust 中定义为 `-1_i32`

---

### 行缓冲触发语义

当 `(*f).lbf == b'\n'` 且 `c == b'\n'` 时，即使缓冲区有空间，触发条件 `c != f->lbf` 为假，导致走 `f->write` 路径，从而将缓冲区内容连同换行符一并刷新。这实现了 POSIX 行缓冲语义。

---

### 不变量

**[Invariant]:**
- 调用 `(*f).write` 时始终传入单字节数据（`&c` 作为首个元素指针，长度为 `1`）
- `(*f).write` 成功返回 `1` 时，内部缓冲区已由 `__stdio_write` 处理（重置 `wpos`/`wbase`）
- 仅在 `f->wend == 0` 时调用 `__towrite` 初始化写模式（仅一次）

---

### 意图

处理 stdio 输出缓冲区的"溢出"情况。分为三种场景：
1. 流尚未初始化写模式（`f->wend == 0`）：切换到写模式
2. 写缓冲区还有空间，且字符不是行缓冲字符（`c != f->lbf`）：直接写入缓冲区
3. 写缓冲区满，或字符触发行缓冲刷新：调用 `f->write` 写出

Rust 侧实现：
- `(*f).wpos`、`(*f).wend` 等字段通过 `*mut u8` 裸指针操作
- 缓冲区写入使用 `core::ptr::write` / `core::ptr::read` 进行单字节操作
- `__towrite` 调用保持为外部函数调用
- 字符类型转换：`_c as u8` 截取低 8 位
- 内部可使用 `Option<u8>` 表示 `EOF` 失败语义，但 `extern "C"` 边界仍返回 `c_int`

---

### 系统算法

```
__overflow(f, _c):
  c = _c as u8

  /* 1. 流尚未初始化写模式 */
  if (*f).wend.is_null() && __towrite(f) != 0:
    return EOF

  /* 2. 缓冲区有空间，且字符不触发行缓冲 */
  if (*f).wpos != (*f).wend && c != (*f).lbf as u8:
    *(*f).wpos = c
    (*f).wpos = (*f).wpos.add(1)
    return c as c_int

  /* 3. 缓冲区满或行缓冲触发：调用函数指针写出 */
  if (*f).write(f, &c as *const u8, 1) != 1:
    return EOF
  return c as c_int
```

时间复杂度 O(1)（不含 `f->write` 系统调用开销）。

---

## 依赖图

```
__overflow
  ├─> __towrite          (see __towrite spec)
  └─> (*f).write         (see __stdio_write / __stdout_write spec)
```

---

## [RELY]

- `__towrite()` — 将流切换为写模式（本模块）
- `(*f).write` — 流写函数指针（通常为 `__stdio_write` 或 `__stdout_write`）
- 常量: `EOF`（定义为 `-1`）

## [GUARANTEE]

Exported Interface:
  `extern "C" fn __overflow(f: *mut FILE, _c: c_int) -> c_int;`

本模块保证对外提供上述 ABI 兼容的函数符号，行为与原 C 实现完全一致：处理 stdio 输出缓冲区溢出/行缓冲刷新的三种场景，返回值语义（成功返回字符值，失败返回 `EOF`）与 C 侧完全一致。
