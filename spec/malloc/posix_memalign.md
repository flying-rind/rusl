# posix_memalign.c 规约

## 依赖图

```
posix_memalign → aligned_alloc (C11, see src/malloc/mallocng/aligned_alloc.c)
               → errno (POSIX, <errno.h>)
               → EINVAL (POSIX, <errno.h>)
```

本文件仅包含一个导出函数 `posix_memalign`，无内部辅助函数、无 `static` 变量、无内部数据结构。核心分配逻辑委托给 `aligned_alloc`（C11 标准函数，musl 内由 `src/malloc/mallocng/aligned_alloc.c` 实现），本规约仅描述 `posix_memalign` 自身的封装契约。

---

## posix_memalign (对外导出)

```c
int posix_memalign(void **res, size_t align, size_t len);
```

[Visibility]: Public — POSIX.1-2001 标准函数，`<stdlib.h>` 声明

### Intent（意图）

提供一个符合 POSIX 标准的对齐内存分配接口。该函数是 `aligned_alloc`（C11）的 POSIX 兼容封装，二者核心差异在于：

- `aligned_alloc` 返回 `void *`，失败时返回 NULL 并设置 `errno`（C11 惯例）。
- `posix_memalign` 通过输出参数 `res` 返回指针，并以**返回值**直接传递错误码（POSIX 惯例），不设置 `errno`。

musl 的实现策略是将所有参数校验与分配逻辑委托给 `aligned_alloc`，`posix_memalign` 仅负责参数转发和错误码格式转换（从 `errno` 读取转换为直接返回）。这种设计避免了代码重复，使得 `malloc`、`memalign`、`aligned_alloc`、`posix_memalign` 共享同一分配引擎。

### 前置条件（Preconditions）

| 条件 | 描述 |
|------|------|
| P1 | `res` 必须为非 NULL 的有效指针，指向一个 `void *` 类型的可写内存位置 |
| P2 | `align` 和 `len` 可以为任意 `size_t` 值（包括 0）；参数合法性由本函数及 `aligned_alloc` 内部校验 |
| P3 | 无外部锁或全局状态依赖；函数为线程安全（依赖于底层 `aligned_alloc`/`malloc` 的线程安全性） |

### 后置条件（Postconditions）

#### Case 1：分配成功（返回值 = 0）

| 条件 | 描述 |
|------|------|
| Q1.1 | `*res` 指向一块大小为 `len` 字节的对齐内存区域 |
| Q1.2 | 返回地址 `*res` 满足对齐要求：`(*res) % align == 0`，且 `align >= sizeof(void *)` 且 `align` 为 2 的幂 |
| Q1.3 | 分配的内存未初始化（内容不确定） |
| Q1.4 | 分配的内存可安全读写 `len` 字节 |
| Q1.5 | 可通过 `free(*res)` 释放（与 `malloc`/`aligned_alloc` 共享同一堆） |

#### Case 2：分配失败（返回值 != 0）

| 条件 | 描述 |
|------|------|
| Q2.1 | `*res` **未被修改**（保持调用前的值） |
| Q2.2 | 无内存被分配，无堆状态变更 |
| Q2.3 | 返回值是以下错误码之一：`EINVAL` 或 `ENOMEM` |

### 错误码语义

| 返回值 | 触发条件 | 检测位置 |
|--------|----------|----------|
| `EINVAL` | `align < sizeof(void *)` | `posix_memalign` 自身检测 |
| `EINVAL` | `align` 不是 2 的幂 | `aligned_alloc` 内部检测，透传 `errno` |
| `ENOMEM` | `len == 0` | `aligned_alloc` 内部处理，透传 `errno` |
| `ENOMEM` | 内存不足或 `len` 溢出 (`len > SIZE_MAX - align`) | `aligned_alloc` 内部检测，透传 `errno` |
| `ENOMEM` | `malloc` 底层分配失败 | `aligned_alloc` → `malloc` 链路，透传 `errno` |

> **注**：POSIX 标准规定 `size == 0` 时的行为是实现定义的。musl（通过 `aligned_alloc`）此时返回 `ENOMEM`。

### 系统算法（System Algorithm）

```
posix_memalign(res, align, len):
1. if align < sizeof(void *) → return EINVAL
2. mem := aligned_alloc(align, len)
3. if mem == NULL → return errno    // 错误码透传
4. *res := mem
5. return 0
```

- **步骤 1** 是一个快速路径优化：当 `align` 小于指针宽度时直接拒绝，避免进入 `aligned_alloc`。
- **步骤 2** 委托给 `aligned_alloc`，该函数内部完成：2 的幂校验、溢出检测、替换检测（`__aligned_alloc_replaced`）、实际对齐分配。
- **步骤 3** 实现了从 C11 错误报告惯例（`errno`）到 POSIX 错误报告惯例（返回值）的转换。由于 `aligned_alloc` 在返回 NULL 前必然设置 `errno` 为 `EINVAL` 或 `ENOMEM`，此转换是可靠的。

### 不变量（Invariants）

- **错误码一致性**：`posix_memalign` 的返回值始终来自 `errno` 或 `EINVAL`，不会返回非 POSIX 定义的错误码。
- **参数保护**：失败时 `*res` 保持不变（POSIX 要求），调用者无需在失败分支中释放 `*res`。