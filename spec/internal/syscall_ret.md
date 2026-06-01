# syscall_ret.c 规约

> 源文件: `/home/mangp/桌面/OS/musl/src/internal/syscall_ret.c`
> 所属模块: musl 内部系统调用基础设施
> 复杂度层级: **Level 1** — 简单的错误码转换，仅需前置/后置条件

---

## 依赖图

```
__syscall_ret ──> errno (C 标准全局变量)
```

| 依赖项 | 来源 | 处理方式 |
|--------|------|----------|
| `errno` | `<errno.h>` (C 标准) | 标准库全局错误变量，跳过 |
| `hidden` 宏 | `features.h` | 编译器属性，跳过 |
| `syscall.h` 内的宏 | `src/internal/syscall.h` | 使用 `__syscall_ret` 的调用方宏（如 `syscall()`），但本函数自身不依赖它们 |

---

## __syscall_ret (内部函数)

### 签名

```c
long __syscall_ret(unsigned long r);
```

### 可见性

**[Visibility]: Internal (不导出)** — `hidden` 属性声明于 `syscall.h`。这是 musl 内部系统调用结果转换的核心函数，所有通过 `syscall()` / `__syscall()` 宏发起的系统调用都经过此函数将原始返回值转换为符合 POSIX 语义的返回值 + errno。POSIX/C 标准未定义此符号。

### 在 musl 中的调用路径

```
用户代码 ──> 公共 API (如 read, write, open, ...)
               └──> syscall(...) 宏
                       └──> __syscall(...) 宏 (内联汇编 syscall 指令)
                               └──> __syscall_ret(raw_result)
                                       ├── 成功: 返回 raw_result
                                       └── 失败: errno = -raw_result, return -1
```

### 意图 (Intent)

Linux 内核系统调用的错误返回约定与 POSIX C 库约定不同：

| 层面 | 错误表示 | 说明 |
|------|----------|------|
| Linux 内核 | 返回 `(unsigned long)(-errno)`，范围为 `[-4095, -1]` | 负值表示错误码 |
| POSIX libc | 返回 `-1`，设置 `errno` 为对应错误码 | 标准 C 库约定 |

`__syscall_ret` 完成此转换，是 musl 中所有系统调用的**统一错误处理瓶颈点**。

### 前置条件

- `r` 是原始系统调用返回值（通常来自内联汇编 `syscall` 指令，放在 `rax` 寄存器中的值）
- `r` 被解释为 `unsigned long` 语义（即无符号比较）

### 后置条件

**Case 1: 系统调用成功** — `r <= 4095UL`（即 Linux 内核返回非负值或小正值）

传统成功的系统调用返回值不超过 `4095`（如 `open` 返回 fd 0..4094, `read` 返回读取字节数等），或者返回 0。

特殊情况：某些系统调用可能在成功时合法返回 1..4095 范围内的值（如文件描述符）。Linux 内核保证**成功时不会返回 -4095..-1 范围内的值**（即不会返回 `[0xFFFFF001, 0xFFFFFFFF]` 区间的无符号值）。

- `errno` 保持不变（不修改）
- 返回值 = `(long)r`（原始返回值直接透传）

**Case 2: 系统调用失败** — `r > 4095UL`（即 r 在 `[0xFFFFF001, 0xFFFFFFFF]` 区间）

- `errno = -(long)r`（错误码 = 原始返回值的绝对值，如 `-EINVAL` → `errno = EINVAL`）
- 返回值 = `-1`（POSIX 标准错误返回值）

### 不变量

1. **错误传递保真性**: `errno` 被设置当且仅当返回值 `== -1`。调用方可通过检查返回值是否为 `-1` 来判断是否需要检查 `errno`。

2. **syscall → POSIX 映射可逆性**: 原始错误码 = `-errno`（失败时）。此不变量确保上层（如信号处理、线程取消点检测）可以从 `errno` 恢复原始错误码。

3. **errno 不被清除**: 成功时 `errno` 的值保持不变。调用方不能依赖"成功时 errno == 0"的假设（这是 POSIX 标准行为）。

### 系统算法 (System Algorithm)

**阈值判断算法：-4096UL 检测**

Linux 内核约定：错误返回值在 `[-4095, -1]` 范围内（Linux 定义约 200 种 errno 值，远小于 4096）。因此，任何大于 `(unsigned long)-4096 = 0xFFFFFFFFFFFFF000`（64位）或 `0xFFFFF000`（32位）的返回值被判定为错误。

阈值 `-4096UL` 的选择：
- 类型为 `unsigned long`，确保无符号比较
- `r > -4096UL` 等价于 `r >= 0xFFFFFFFFFFFFF000`（64位）或 `r >= 0xFFFFF000`（32位）
- 该阈值远大于任何合法非负返回值（文件描述符上限通常远小于 4096，即使考虑最大 fd 限制）
- Linux 内核保证不会产生 `[0x00000001, 0xFFFFEFFF]` 范围内的负 errno 编码

**线程安全性**: 写入 `errno` 是线程安全的——在现代 libc 实现中 `errno` 是线程局部变量（通过 `__errno_location()` 获取每线程地址），因此不需要加锁。

### Rust 实现注意事项

在 Rust `#![no_std]` 实现中：
- `errno` 对应 musl 内部的 `__errno_location()` → 返回 `*mut c_int` 线程局部指针
- `__syscall_ret` 应被实现为 `fn(r: usize) -> isize`，内部操作仅为无符号比较和条件赋值
- 失败情况：`*errno_ptr = -(r as isize) as c_int; return -1isize`
- 成功情况：直接返回 `r as isize`
- 零开销：此函数可以被内联而不影响正确性