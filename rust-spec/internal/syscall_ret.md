# syscall_ret 规约 (Rust)

> **来源文件**: `musl/src/internal/syscall_ret.c`
> **目标模块**: `rusl/src/internal/syscall_ret.rs`
> **复杂度层级**: Level 1 — 简单的错误码转换

---

## 概述

`__syscall_ret` 函数是 musl/rusl 中所有系统调用的**统一错误处理瓶颈点**。它将 Linux 内核系统调用的原始返回值（负值 = `-errno`）转换为符合 POSIX 语义的返回值（`-1` + 设置 `errno`）。

Linux 内核与 POSIX libc 的约定差异：

| 层面 | 错误表示 | 说明 |
|------|----------|------|
| Linux 内核 | 返回 `[-4095, -1]` 范围内的负值 | 负值表示错误码 |
| POSIX libc | 返回 `-1`，设置 `errno` 为对应错误码 | 标准 C 库约定 |

---

## [RELY]

```
Predefined Functions:
  fn errno_location() -> *mut c_int;   // 依赖: 线程局部 errno 指针获取
                                        // 来源: errno 模块
```

## [GUARANTEE]

```
Exported Interface:
  fn __syscall_ret(r: c_ulong) -> c_long;  // [Visibility]: Internal (不导出)
                                            // musl 内部系统调用返回值处理器
                                            // 被所有 syscall()/__syscall() 宏使用
```

---

## 函数签名

```rust
/// 将 Linux 内核系统调用的原始返回值转换为符合 POSIX 语义的返回值
///
/// Linux 内核错误返回值约定为 `[-4095, -1]`，即 `r > -4096UL` 为错误。
///
/// # Safety
///
/// `r` 必须是某次 Linux 系统调用的原始返回值。
///
/// # Returns
///
/// - 成功: 返回 `r as c_long`（原始返回值直接透传）
/// - 失败: `errno = -(r as c_int)`，返回 `-1`
#[no_mangle]
pub unsafe extern "C" fn __syscall_ret(r: c_ulong) -> c_long;
```

`[Visibility]: Internal — musl 内部系统调用返回值处理器`

---

### 意图 (Intent)

完成 Linux 内核系统调用返回值到 POSIX libc 返回值的转换。

---

### 前置条件

- `r` 是原始系统调用返回值（通常来自内联汇编 `syscall` 指令，放在 `rax` 寄存器中的值）
- `r` 被解释为 `c_ulong` 语义（即无符号比较）

---

### 后置条件

**Case 1: 系统调用成功** — `r <= -4096isize as c_ulong`

- `errno` 保持不变（不修改）
- 返回值 = `r as c_long`（原始返回值直接透传）

**Case 2: 系统调用失败** — `r > -4096isize as c_ulong`

- `errno = -(r as c_int)`（错误码 = 原始返回值的绝对值）
- 返回值 = `-1`（POSIX 标准错误返回值）

---

### 不变量

1. **错误传递保真性**: `errno` 被设置当且仅当返回值 `== -1`。调用方可通过检查返回值是否为 `-1` 来判断是否需要检查 `errno`。

2. **syscall -> POSIX 映射可逆性**: 原始错误码 = `-errno`（失败时）。此不变量确保上层可以从 `errno` 恢复原始错误码。

3. **errno 不被清除**: 成功时 `errno` 的值保持不变。调用方不能依赖"成功时 errno == 0"的假设（这是 POSIX 标准行为）。

---

### 系统算法 (System Algorithm)

**阈值判断算法: -4096 检测**

Linux 内核约定：错误返回值在 `[-4095, -1]` 范围内。因此，任何大于 `(c_ulong)(-4096)` 的返回值被判定为错误。

阈值 `-4096isize as c_ulong` 的选择：
- 类型为 `c_ulong`，确保无符号比较
- `r > -4096isize as c_ulong` 等价于 64 位下的 `r >= 0xFFFFFFFFFFFFF000` 或 32 位下的 `r >= 0xFFFFF000`
- Linux 内核保证产生错误码不超过 4095，合法成功返回值（如 fd）远小于此阈值

**线程安全性**: 写入 `errno` 是线程安全的——在 libc 实现中 `errno` 是线程局部变量（通过 `errno_location()` 获取每线程地址），因此不需要加锁。

---

### 实现伪代码

```rust
/// 系统调用原始返回值转换为 POSIX 语义
///
/// # Safety
/// r 必须是有效的系统调用原始返回值。
#[no_mangle]
pub unsafe extern "C" fn __syscall_ret(r: c_ulong) -> c_long {
    if r > (-4096isize) as c_ulong {
        // 错误路径
        let errno_val = -(r as c_int);
        unsafe {
            *errno_location() = errno_val;
        }
        -1
    } else {
        // 成功路径
        r as c_long
    }
}
```

---

### Rust 实现注意事项

在 Rust `#![no_std]` 实现中：
- `errno` 对应 rusl 内部的 `errno_location()` -> 返回 `*mut c_int` 线程局部指针
- `__syscall_ret` 实现为 `pub unsafe extern "C" fn(r: c_ulong) -> c_long`
- 内部操作仅为无符号比较和条件赋值，零开销
- 此函数可以被安全内联而不影响正确性
- 使用 `#[no_mangle]` 保持与 C 链接器的符号兼容
- `c_ulong` / `c_long` / `c_int` 使用 `core::ffi::` 或自定义的类型别名