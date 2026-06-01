# secure_getenv.c 规约

## 依赖图

```
secure_getenv
  ├── libc (__libc.secure)  — 来自 src/internal/libc.h，musl 内部全局状态
  └── getenv(const char *)  — POSIX 标准函数，来自 <stdlib.h>，外部模块
```

---

## __libc 结构体及 secure 字段 (内部依赖)

```c
struct __libc {
    // ...其他字段...
    char secure;   // 第 23 行: 进程安全模式标志
    // ...其他字段...
};
```

**[Visibility]: Internal (不导出)** — musl 内部使用的全局库状态结构体，定义于 `src/internal/libc.h`，用户代码不可直接访问。

**语义**:
- `__libc`（宏定义为 `libc`）是 musl 的全局内部状态容器。
- `secure` 字段为 `char` 类型，取值为 `0` 或 `1`。
- 当进程以特权模式（如 setuid/setgid 二进制）运行时，musl 启动代码将 `libc.secure` 设置为 `1`。
- 该字段用于控制是否允许危险的环境变量操作（如 `LD_PRELOAD`、`LD_LIBRARY_PATH`），以防止特权升级攻击。

**不变量 (Invariant)**:
- `libc.secure` 一旦在进程启动期被设置（从 `__init_libc` 中依据 AT_SECURE 辅助向量或 uid/gid 检测），在整个进程生命周期内保持不变。
- `libc.secure` 仅取 `0` 或 `1`。

---

## secure_getenv (对外导出)

```c
char *secure_getenv(const char *name);
```

**[Visibility]: Public** — GNU 扩展函数（需 `_GNU_SOURCE`），声明于 `<stdlib.h>`。用户程序可直接调用。

### 意图 (Intent)

提供安全的环境变量访问：在特权执行上下文（如 setuid/setgid 进程）中，拒绝返回环境变量值以防止环境变量注入攻击。在普通上下文中，等价于 `getenv`。

### 前置条件 (Precondition)

- `name` 为指向以 NUL 结尾的 C 字符串的有效指针。
- `name` 不得为 `NULL`。
- 全局 `libc.secure` 的值已在进程启动时正确初始化（由 `__init_libc` 设置）。

### 后置条件 (Postcondition)

**Case 1 — 安全模式（`libc.secure == 1`）**:
- 直接返回 `NULL`，不访问环境变量列表。
- 无论 `name` 是否匹配任何环境变量，均返回 `NULL`。
- 无任何副作用，不修改全局状态。

**Case 2 — 普通模式（`libc.secure == 0`）**:
- 等价于调用 `getenv(name)`。
- 返回值与 `getenv(name)` 完全一致:
  - 若 `name` 匹配某个环境变量，返回指向该环境变量值字符串的指针（如 `"PATH=/usr/bin"` 中 `=` 之后的部分）。
  - 若 `name` 不匹配任何环境变量，返回 `NULL`。

### 设计说明

1. **安全性优先**: 该函数在条件判断中使用 `libc.secure`（而非执行额外的 uid/gid 检查），因为 `libc.secure` 的初始化已经综合了 AT_SECURE 辅助向量和实际 uid/gid 比较。这避免了竞态条件和冗余系统调用。

2. **线程安全**: 该函数仅读取 `libc.secure`（只读字段）和调用 `getenv`（读 `environ`），无写入操作，天然线程安全。

3. **GNU 兼容性**: `secure_getenv` 是 glibc 引入的 GNU 扩展，musl 提供兼容实现。代码开头的 `#define _GNU_SOURCE` 确保 GNU 扩展声明可见。