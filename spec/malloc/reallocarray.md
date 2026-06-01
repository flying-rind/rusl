# reallocarray

## 符号可见性

| 符号 | 可见性 | 说明 |
|------|--------|------|
| `reallocarray` | Public | BSD 扩展函数，`<stdlib.h>` 声明，用户程序可直接调用 |

## 依赖图

```
reallocarray ──→ realloc (外部 libc, 见 src/malloc/realloc.c)
```

本文件仅含一个导出函数，无内部依赖项需要递归追踪。

---

## reallocarray

### 函数签名

```c
void *reallocarray(void *ptr, size_t m, size_t n);
```

### 意图

对 `realloc(ptr, m * n)` 做安全的整数溢出检查版本。分配 `m * n` 个字节的内存（`m` 个元素，每个元素大小 `n`），并在 `m * n` 乘法溢出 `size_t` 时返回 NULL 并设置 `errno = ENOMEM`，而不是产生一个错误且可能极小的分配结果。该函数是 OpenBSD 首创的 BSD 扩展，用于防御整数溢出漏洞。

### 前置条件

- 若 `ptr` 非 NULL，则它必须是先前由 `malloc`、`calloc`、`realloc` 或 `reallocarray` 返回的有效指针，且尚未被 `free` 或 `realloc` 释放
- 无其它前置条件；`m` 和 `n` 可以为任意 `size_t` 值

### 后置条件

**Case 1: `m * n` 乘法溢出 `size_t`**

- 若 `n != 0` 且 `m > SIZE_MAX / n`（等价条件），触发溢出
- `errno` 被设置为 `ENOMEM`
- 返回 NULL
- `ptr` 指向的原始内存块保持未修改状态（未被释放）

**Case 2: 无溢出，`realloc(ptr, m * n)` 成功**

- 返回指向新分配内存块（至少 `m * n` 字节）的指针
- 若 `ptr` 非 NULL 且 `m * n > 0`，新块内容在 `min(oldsize, m * n)` 范围内与原始块一致
- 若 `ptr` 非 NULL 且 `m * n == 0`，行为等价于 `free(ptr)` 并可能返回 NULL 或唯一指针
- 若 `ptr` 为 NULL，行为等价于 `malloc(m * n)`

**Case 3: 无溢出，但 `realloc(ptr, m * n)` 失败**

- 返回 NULL
- `errno` 由 `realloc` 内部设置为 `ENOMEM`
- `ptr` 指向的原始内存块保持有效且未修改

### 不变量

- 本函数不持有任何全局锁
- 不修改任何全局或静态状态（除 `errno` 以外）

### 算法

1. 溢出检测：利用无符号整数算术的模运算特性，将 `-1`（即 `(size_t)-1` = `SIZE_MAX`）除以 `n` 得到安全上限，然后检查 `m` 是否超过该上限
   - 表达式 `if (n && m > -1 / n)` 中，先短路求值 `n` 确保避免除以零
   - 当 `n == 0` 时，直接跳过溢出检查，交由 `realloc(ptr, 0)` 处理
2. 若溢出，设置 `errno = ENOMEM` 并返回 NULL
3. 否则，调用 `realloc(ptr, m * n)` 并将结果透传给调用者

### [RELY] 依赖

```
Predefined Structures/Functions:
  void *realloc(void *ptr, size_t size);  // 依赖1: 来自 libc，执行实际的内存重分配
  extern int errno;                        // 依赖2: 来自 <errno.h>，系统错误码变量

Predefined Macros:
  _BSD_SOURCE                              // 依赖3: 启用 BSD 扩展声明（<stdlib.h> 中 reallocarray 声明受此宏保护）
  ENOMEM                                   // 依赖4: 来自 <errno.h>，表示内存不足的错误码

Predefined Types:
  size_t                                   // 依赖5: C 标准无符号整数类型，来自 <stddef.h>
```

### [GUARANTEE] 导出接口

```
Exported Interface:
  void *reallocarray(void *ptr, size_t m, size_t n);
  // 带溢出检查的安全数组内存重分配
  // 声明于 <stdlib.h>，受 _BSD_SOURCE 特性测试宏保护
```