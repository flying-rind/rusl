# perror 函数规约

## 复杂度分级: Level 1

> musl libc 标准库错误信息打印函数。向 stderr 输出与当前 errno 对应的错误消息。Rust 实现中，外部接口保持 ABI 兼容，内部 I/O 调用使用 Rust 安全抽象。

---

## 函数接口

```rust
use core::ffi::{c_char, c_int};

unsafe extern "C" fn perror(msg: *const c_char);
```

[Visibility]: User — `<stdio.h>` 标准库函数。必须保持 ABI 兼容。`msg` 可为 NULL（不输出前缀），若非 NULL 则指向以 NUL 结尾的字符串。无返回值。

---

### 前置/后置条件

**[Pre-condition]:**
- `msg`: 可为 NULL（表示不输出前缀）；若非 NULL，指向以 NUL 结尾的字符串。
- `stderr` 已初始化且可写。
- `errno` 包含一个有效错误码（由调用之前的错误系统调用设置）。

**[Post-condition]:**
- **Case 1 输出成功**
  - 错误消息已写入 `stderr`，格式为 `<msg>: <error_message>\n` 或 `<error_message>\n`（若 msg 为空）。
  - `stderr` 的 locale 和 mode 保持不变。
  - `errno` 可能被内部 I/O 操作修改。

- **Case 2 输出失败**
  - 部分或全部消息写入失败（不检查 fwrite/fputc 返回值，perror 无返回值）。

**[Error Behavior]:**
- `perror` 为 `void` 返回，不报告 I/O 错误。调用者无法通过返回值判断是否成功。

---

### 不变量

**[Invariant]:**
- `perror` 不会改变 `stderr` 的 orientation（字节/宽字符方向）和 locale 设置。
- 输出始终以换行符 `\n` 结束。
- `perror` 无返回值，不报告 I/O 错误。
- 输出格式固定：`[msg: ]error_message\n`。

---

### 意图

向 `stderr` 输出描述最后一个系统调用错误的字符串。错误消息由 `strerror(errno)` 获得。

Rust 侧实现：
- 外部接口使用 `unsafe extern "C" fn perror(msg: *const c_char)`，保持 ABI 兼容。
- 内部获取 `stderr` 全局变量（通过 `*mut FILE` 不透明指针访问）。
- `strerror(errno)` 调用 rusl 内部的错误描述函数，返回字节切片引用。
- `FLOCK`/`FUNLOCK` 对应内部 FILE 锁获取/释放操作，使用安全包装（如用 RAII 守卫自动管理锁）。
- 保存/恢复 `stderr` 的 locale 和 mode 字段：使用 Rust 的局部变量保存后恢复，或使用作用域守卫（scope guard）。
- `fwrite` 和 `fputc` 调用内部安全 I/O 函数，不检查返回值（与 C 语义一致）。
- 输出内容先通过 `strlen` 获取长度后再 `fwrite` 写入。

### 系统算法

```
perror(msg):
  1. f = stderr
  2. errstr = strerror(errno)          // 获取错误描述字符串

  3. 锁定 stderr (FLOCK)

  4. 保存上下文:
     old_locale = f.locale             // 保存 locale
     old_mode = f.mode                 // 保存 orientation

  5. 输出:
     若 msg != NULL 且 *msg != 0:
       fwrite(msg, strlen(msg), 1, f)  // 写入用户消息
       fputc(':', f)                   // 冒号分隔符
       fputc(' ', f)                   // 空格
     fwrite(errstr, strlen(errstr), 1, f)  // 写入错误描述
     fputc('\n', f)                        // 换行符

  6. 恢复上下文:
     f.mode   = old_mode
     f.locale = old_locale

  7. 解锁 stderr (FUNLOCK)
```

时间复杂度 O(len(msg) + len(errstr))。

---

## 依赖图

```
perror (Public, extern "C")
  ├── core::ffi::{c_char, c_int}           — Rust 内置 FFI 类型
  ├── [Internal] stderr                    — 标准错误流全局变量
  ├── [Internal] strerror(errno)           — 错误码转描述字符串
  ├── [Internal] strlen                    — 字符串长度计算 (可用 Rust 替代)
  ├── [Internal] fwrite / fputc            — 内部安全 I/O 函数
  ├── [Internal] FLOCK / FUNLOCK           — FILE 锁操作 (可用 Rust Mutex 替代)
  ├── [Internal] __errno_location()        — 读取 errno
  └── [Internal] FILE 类型 (locale, mode 字段) — stdio_impl 模块定义
```

---

## [RELY]

- `core::ffi::{c_char, c_int}` — Rust 核心库 FFI 类型。
- 内部 `stderr` — rusl 内部标准错误流。
- 内部 `strerror` — rusl 内部错误描述函数。
- 内部 `fwrite` / `fputc` — rusl 内部安全 I/O 函数。
- 内部 FILE 锁定机制 — rusl 内部实现（可用 Rust 同步原语替代）。

## [GUARANTEE]

Exported Interface:
  `unsafe extern "C" fn perror(msg: *const c_char);`

本模块保证对外提供上述 ABI 兼容的函数符号：
- 参数 `*const c_char` 与 C `const char *` 内存布局一致。
- 无返回值（`void`），与 C 完全一致。
- 使用 C 调用约定 (`extern "C"`)。
- 行为符合 POSIX `perror()` 语义：向 stderr 输出 `msg: error_message\n` 格式的错误信息。
- 不改变 stderr 的 locale 和 orientation 属性。
