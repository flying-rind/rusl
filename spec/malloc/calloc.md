# calloc.c 规约

> 源文件: `src/malloc/calloc.c`
> 功能: 实现 `calloc` 及相关的内部零填充优化辅助函数

---

## 依赖图

```
calloc (Public)
├── malloc                 → see lite_malloc.c / mallocng/  (外部模块)
├── __malloc_replaced      → defined in replaced.c          (外部模块)
├── __malloc_allzerop(p)
│   └── allzerop(p)        → [本文件] 默认实现, 可被 oldmalloc 覆盖
└── mal0_clear(p, n)       → [本文件] 内部辅助函数
    └── memset             → <string.h>  (libc 外部)
```

---

## 内部结构体/类型

本文件未定义新的结构体。依赖的类型：

| 类型 | 来源 | 用途 |
|------|------|------|
| `size_t` | `<stdint.h>` | 内存尺寸 |
| `uint64_t` | `<stdint.h>` | 快速零检测的别名类型（GCC下） |
| `uintptr_t` | `<stdint.h>` | 地址对齐计算 |

---

## Layer 1: 底层内部函数

---

### mal0_clear (内部函数)

**[Visibility]**: Internal — `static` 函数，musl 内部零填充优化辅助函数，POSIX/C 标准未定义

**[Complexity]**: Level 2 — 含优化策略，需 Intent 描述

#### C 签名

```c
static size_t mal0_clear(char *p, size_t n);
```

#### Intent (意图描述)

对已分配的 `n` 字节内存块 `p` 进行高效的尾部清零。利用内核 mmap 页面初始即为零的语义特性，从块尾部向头部逐页"探测"：若连续整页已全为零，则跳过 memset，仅返回仍需清零的起始偏移量。这是一种面向大块内存分配的 memset 加速策略。

#### 前置条件 (Preconditions)

- `p` 指向一块有效的、可写入的 `n` 字节内存区域（来自 `malloc` 返回值或等同来源）
- `n >= 0`
- 调用者持有对 `p` 所指向内存的所有权，无并发写入者

#### 后置条件 (Postconditions)

**Case 1 — 正常路径 (n < 4096)**:
- 不做任何清零操作，原样返回 `n`
- 语义: 小块内存不满足优化阈值，交给调用者全量 memset

**Case 2 — 大块优化路径 (n >= 4096)**:
- 从 `p + n` 向 `p` 方向逐页扫描
- 首先将尾部未对齐到页边界的部分 `memset` 清零
- 然后向前逐页检查：若页内第一个和第二个 `sizeof(T)` 字均为零（暗示整页可能为零），则跳过该页继续向前；若发现非零字，则停止探测，将该非零页清零后终止
- 返回值 `r = pp - p`，满足 `0 <= r < 4096`：表示剩余需要调用者补齐清零的前缀字节数
- **保证**: 从 `p + r` 到 `p + n` 的全部字节已被清零

#### 系统算法 (System Algorithm)

```
Input: p (char*), n (size_t), pagesz = 4096
Output: remaining_bytes (size_t)

1. if n < pagesz → return n  // 不满足优化阈值
2. pp = p + n
3. i = (uintptr_t)pp & (pagesz - 1)  // 尾部未对齐字节数
4. loop:
   a. pp = memset(pp - i, 0, i)      // 清零尾部片段
   b. if pp - p < pagesz → return pp - p  // 剩余不足一页
   c. for i = pagesz down to 0 step 2*sizeof(T):
        if *(T*)(pp - sizeof(T)) != 0
        or *(T*)(pp - 2*sizeof(T)) != 0:
          break  // 发现非零页，需清零
   d. if i == 0: 整个页已为零，pp -= pagesz 继续向前
      else: 进入下一次迭代清零该非零页
```

**类型 T 选择策略**:
- GCC 编译时 (`#ifdef __GNUC__`): `T = uint64_t __attribute__((__may_alias__))`，一次检查 16 字节
- 其他编译器: `T = unsigned char`，逐字节检查（无优化）

#### 不变量 (Invariants)

- **方向不变量**: 清零始终从高地址向低地址进行 (`pp` 单调递减)
- **页对齐不变量**: 每次 `memset(pp - i, 0, i)` 清零的部分，起始地址必为页对齐地址（因 `i = (uintptr_t)pp & (pagesz - 1)`, `pp - i` 向下取整到页边界）
- **终止保证**: 每轮循环 `pp` 严格递减（至少减少 `pagesz`），因 `n` 有限，循环必定终止

---

### allzerop (内部函数)

**[Visibility]**: Internal — `static` 函数，通过 `weak_alias` 暴露为 `__malloc_allzerop`，为 musl 内部符号，POSIX/C 标准未定义

#### C 签名

```c
static int allzerop(void *p);
```

#### Intent (意图描述)

默认的空实现。总是返回 `0`（即"非全零"），迫使 `calloc` 走完整的 `mal0_clear` + `memset` 清零路径。当链接到 `oldmalloc` 实现时，此弱符号会被 `oldmalloc/malloc.c` 中的同名强符号覆盖，该强符号会真实检测 `p` 是否来自 mmap 分配区（mmap 页面初始为零）。

#### 前置条件

- `p` 是 `malloc` 返回的合法指针

#### 后置条件

- 始终返回 `0`，无副作用

---

### __malloc_allzerop (内部符号，weak alias 导出)

**[Visibility]**: Internal — 通过 `weak_alias(allzerop, __malloc_allzerop)` 暴露，musl 内部链接符号，POSIX/C 标准未定义

**说明**: 这是一个通过弱符号机制提供的可覆盖内部接口。`calloc.c` 提供默认的 `allzerop` → `__malloc_allzerop` (始终返回 0)；当链接到 `oldmalloc` 时，`oldmalloc/malloc.c` 中定义的强符号 `__malloc_allzerop` 会覆盖此弱符号，提供真实的 mmap 区域零检测逻辑。

#### C 签名

```c
int __malloc_allzerop(void *p);
```

#### 前置条件

- `p` 是由 `malloc` 分配的有效指针（或 NULL，此时不应被调用）

#### 后置条件

- 返回值语义:
  - `0`: 内存块不确定为零，需要显式清零
  - `非0`: 内存块确定已全零，可跳过清零 (仅 oldmalloc 覆盖版本返回非零)

---

## Layer 2: 对外导出函数

---

### calloc (对外导出)

**[Visibility]**: Public — POSIX.1-2001 标准函数，`<stdlib.h>` 声明，用户程序可直接调用

**[Complexity]**: Level 2 — 含溢出检测和零填充优化路径

#### C 签名

```c
void *calloc(size_t m, size_t n);
```

#### Intent (意图描述)

分配一个包含 `m` 个元素、每个元素 `n` 字节的数组，并将分配的内存全部清零后返回指针。相比 `malloc(m*n) + memset`，`calloc` 有两项优势：

1. **乘法溢出检测**: 在分配前检查 `m * n` 是否溢出 `size_t` 范围，溢出则返回 NULL 并设置 `errno = ENOMEM`
2. **零填充优化**: 对于大块内存，利用内核 mmap 页初始为零的特性，通过 `mal0_clear` 跳过已零页的显式 `memset`

#### 前置条件 (Preconditions)

- `m` 和 `n` 为任意 `size_t` 值
- 无内部状态要求（线程安全，可重入）

#### 后置条件 (Postconditions)

**Case 1 — 乘法溢出**:
- **触发条件**: `n != 0` 且 `m > (size_t)-1 / n`（即 `m * n > SIZE_MAX`）
- **效果**:
  - `errno = ENOMEM`
  - 返回 `NULL` (空指针)
  - 不分配任何内存
  - **保证**: 无内存泄漏

**Case 2 — 底层分配失败**:
- **触发条件**: 乘法未溢出，但内部 `malloc(n)` 返回 `NULL`
- **效果**:
  - `errno` 由 `malloc` 设置 (通常为 `ENOMEM`)
  - 返回 `NULL`
  - 不执行清零操作

**Case 3 — 分配成功且内存已全零**:
- **触发条件**: `malloc(n)` 成功 且 `!__malloc_replaced` 且 `__malloc_allzerop(p) != 0`
- **效果**:
  - 返回 `p`，指向 `n` 字节全零内存
  - 不执行额外的 `memset`
  - **前提**: 仅当使用 musl 内置 malloc（`__malloc_replaced == 0`）且底层分配器确认内存已零时才走到此分支

**Case 4 — 分配成功但需显式清零**:
- **触发条件**: `malloc(n)` 成功 且 (用户替换了 malloc (`__malloc_replaced != 0`) 或 `__malloc_allzerop(p) == 0`)
- **效果**:
  - 调用 `mal0_clear(p, n)` 从尾部高效清零，获得剩余需清零前缀长度 `r`
  - 调用 `memset(p, 0, r)` 清零剩余前缀
  - 返回 `p`，指向 `n` 字节全零内存

#### 外部依赖

| 依赖 | 来源 | 角色 |
|------|------|------|
| `__malloc_replaced` | `replaced.c` (定义), `internal/dynlink.h` (声明) | 标记用户是否替换了 malloc 实现 |
| `__malloc_allzerop` | 本文件 `weak_alias` / `oldmalloc/malloc.c` (覆盖) | 检测分配块是否已全零 |
| `malloc` | `lite_malloc.c` 或 `mallocng/` | 底层原始内存分配 |
| `memset` | `<string.h>` | 显式字节清零 |

#### 线程安全

- `calloc` 本身无内部静态状态
- 线程安全由底层 `malloc` 实现提供
- `errno` 的设置遵循 C11 线程安全语义（每线程 errno）

#### 与 malloc(0) 的兼容性

- 若 `m == 0 || n == 0`，则 `n = m * n = 0`
- 结果行为取决于底层 `malloc(0)` 的实现策略（返回 NULL 或唯一指针）
- 使用前检查返回值是否为 NULL 即可处理所有情况