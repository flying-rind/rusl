# libc_calloc.c 规约

> **源文件**: `src/malloc/libc_calloc.c`
> **编译机制**: 通过 `#define calloc __libc_calloc` / `#define malloc __libc_malloc` 对 `calloc.c` 进行符号重命名后 `#include "calloc.c"`，生成内部隐藏符号 `__libc_calloc`。公开的 `calloc` 由另一编译单元（直接编译 `calloc.c`）提供。

---

## 依赖图

```
libc_calloc.c
  ├── #define calloc __libc_calloc     (符号重命名)
  ├── #define malloc __libc_malloc     (符号重命名)
  └── #include "calloc.c"
        ├── __libc_calloc(size_t, size_t) -> void *

        │   依赖:
        │   ├── __libc_malloc(size_t)           [see mallocng/malloc.c spec / oldmalloc/malloc.c spec]
        │   ├── __malloc_replaced               [extern int, see replaced.c spec]
        │   ├── __malloc_allzerop(void *) -> int [本文件内部, weak_alias of allzerop]
        │   └── mal0_clear(char *, size_t)       [本文件内部, static]
        │
        ├── mal0_clear(char *, size_t) -> size_t  [static 内部函数]
        │   依赖:
        │   └── memset(void *, int, size_t)      [外部, <string.h>]
        │
        └── allzerop(void *) -> int              [static 内部函数]
            依赖:
            └── (无)
            导出:
            └── __malloc_allzerop               [weak_alias, 声明于 dynlink.h]
```

---

## allzerop (内部函数)

```c
static int allzerop(void *p);
```

[Visibility]: Internal — musl 内部静态函数，通过 `weak_alias(allzerop, __malloc_allzerop)` 暴露给其他 libc 内部模块，但不对用户程序导出。

### 复杂度层级: Level 2

### 意图 (Intent)

测试给定内存页是否全部为零。该函数作为**默认实现**始终返回 0（假），表示"不确定/无法确认内存页为零"。真正的 `__malloc_allzerop` 实现由具体的 malloc 后端提供，可通过 weak symbol 覆盖此默认版本。调用者（`calloc`）依据此返回值决定是否需要显式调用 `memset` 清零内存。

### 前置条件

- 无特定前置条件。`p` 可以为任意指针（包括 NULL）。

### 后置条件

- **Case 1 (默认实现)**: 始终返回 `0`，表示无法确认内存页为零。
  - 此返回值将导致 `calloc` 执行 `memset` 进行显式清零。

### 不变量

- 无。此函数为纯函数（trivial case），不修改任何全局状态。

---

## __malloc_allzerop (内部符号)

```c
int __malloc_allzerop(void *p);
```

[Visibility]: Internal — 通过 `weak_alias(allzerop, __malloc_allzerop)` 生成，声明于 `src/internal/dynlink.h` 中 `hidden extern int __malloc_allzerop(void *);`，仅对 musl 内部模块可见。默认实现为 `allzerop`（始终返回 0），但可被具体 malloc 后端的实现通过 strong symbol 覆盖。

### 复杂度层级: Level 2

### 意图 (Intent)

为 `calloc` 提供一种优化查询机制：当 `__malloc_allzerop(p)` 返回非零值时，表示地址 `p` 处的内存已知全部为零（例如由内核 `mmap` 返回的惰性分配零页），`calloc` 即可跳过显式的 `memset` 清零步骤。默认实现保守返回 0（强制 `calloc` 执行清零），由具体 malloc 后端提供优化的版本。

### 前置条件

- `p` 指向通过 `__libc_malloc`（或 `malloc`）分配的有效内存块的起始地址。

### 后置条件

- **Case 1**: 返回 `0`（默认实现或后端无法确认）— 调用者必须通过 `memset` 等途径显式清零。
- **Case 2**: 返回非零值 — 调用者可安全假设 `p` 处的内存已为零，无需额外清零操作。

### 不变量

- 该函数不得修改内存内容。
- 返回值仅影响调用路径（是否执行 `memset`），不影响最终对外语义（`calloc` 返回的内存始终保证为零）。

---

## mal0_clear (内部函数)

```c
static size_t mal0_clear(char *p, size_t n);
```

[Visibility]: Internal — `static` 函数，musl 内部辅助函数，POSIX/C 标准未定义，不对用户程序导出。

### 复杂度层级: Level 3

### 意图 (Intent)

将已分配的内存块清零，但利用**向后扫描**策略减少不必要的工作量：当内存由内核的零页映射返回时，页面可能是干净的全零页。`mal0_clear` 从内存块的**末尾**向**开头**扫描，跳过已为零的页面，仅对**非零**区域调用 `memset` 进行清零。这是一个针对 `calloc` 场景（大部分页面可能已经为零）的启发式优化。

### 系统算法 (System Algorithm)

算法采用**从尾到头的反向逐页扫描**策略：

1. **起点对齐**: 以 `pagesz = 4096`（任意选择的页面大小）为粒度。将指针 `pp` 初始化为 `p + n`（缓冲区末尾），将 `i` 初始化为 `pp` 在页内偏移量。

2. **循环扫描**:
   - **Step A — 清零页内尾部**: 对页面内非对齐部分执行 `memset(pp - i, 0, i)` 清零。
   - **Step B — 提前终止检查**: 若 `pp - p < pagesz`（剩余不足一页），返回 `pp - p`（剩余未处理字节数，由调用者补清零）。
   - **Step C — 整页扫描**: 从当前页末尾向开头逐 `2*sizeof(T)` 步进扫描。使用 `uint64_t`（GCC 下通过 `__may_alias__` 属性）或 `unsigned char`（非 GCC）进行逐字比较。若发现任何非零值，跳出扫描循环回到 Step A。
   - **Step D — 跳过零页**: 若扫描完整个页面未发现非零值，则该页已经是全零，直接从 `pp` 减去 `pagesz` 的剩余部分（即 `pagesz - i`），使 `pp` 跳过该页，继续检查前一页。

3. **返回**: 返回值为还需要调用者额外清零的字节数（即 `memset(p, 0, n)` 来完成工作）。

### 前置条件

- `p` 指向一个长度为 `n` 字节的内存块（通过 `malloc` 返回）。
- `n >= 0`。
- `p` 处内存可读可写。

### 后置条件

- 返回值 `r` 满足 `0 <= r < pagesz` 或 `r == n`（当 `n < pagesz` 时）。
- 所有 `pp` 至 `p + n` 之间的内存（`pp = p + r`）已被清零。
- 调用者还需对 `p[0..r-1]` 调用 `memset(p, 0, r)` 完成清零，即剩余要清零的字节数为返回值 `r`。
- 当 `n < pagesz` 时，该函数不做任何操作直接返回 `n`（全部由调用者清零）。

### 设计要点

- **页面大小的选择**: 使用常量 `4096` 而非系统页大小。这是刻意为之——即使实际页大小为 4KB 的倍数或不同值，该算法仅影响优化效果而非正确性。`4096` 作为最小公共页面大小，确保至少不会遗漏需要清零的页面。
- **类型别名 (T)**: GCC 下使用 `uint64_t __attribute__((__may_alias__))` 以允许通过 8 字节宽度扫描（不违反严格别名规则），非 GCC 编译器降级为逐字节扫描 `unsigned char`。
- **不处理最小块**: `n < pagesz` 时直接返回（不做任何优化），因为块太小不值得开销。

---

## __libc_calloc (内部符号)

```c
__libc_calloc(size_t m, size_t n) -> void *
```

> **注意**: 在源文件中通过 `#define calloc __libc_calloc` 将原始函数名 `calloc` 重命名为 `__libc_calloc`。该符号具有 `hidden` 可见性（`src/include/stdlib.h` 中声明为 `hidden void *__libc_calloc(size_t, size_t);`）。

[Visibility]: Internal — musl libc 内部实现，具有 `hidden` 可见性。对外部使用者，应使用 POSIX 标准函数 `calloc()`（由另一个编译单元提供）。`__libc_calloc` 的存在是为了允许 musl 内部其他组件（如 `atexit`, `sem_open`, `aio` 等）在替换了公共 `malloc`/`calloc` 的情况下，仍然能访问原始的内部分配器。

### 复杂度层级: Level 3

### 意图 (Intent)

为 musl libc 内部提供**不可被替换的** `calloc` 实现。这是 musl 的关键架构设计——通过为 `malloc` 系列函数同时提供 weakly-linked 的公开符号（可被用户替换）和 hidden 的内部符号（`__libc_*`），使得 libc 内部代码始终使用原始分配器，即使应用程序已通过 `LD_PRELOAD` 或静态链接替换了 `malloc`/`calloc`/`free`。

### 系统算法 (System Algorithm)

该函数实现标准的 `calloc` 语义：分配 `m * n` 字节的零初始化内存。实现通过以下三步完成：

**阶段 1 — 溢出检测**:
```
if (n && m > (size_t)-1 / n):
    errno = ENOMEM; return NULL;
```
检测 `m * n` 是否溢出 `size_t`。标准等效写法为 `SIZE_MAX / n < m`。

**阶段 2 — 分配**:
```
n *= m;
void *p = __libc_malloc(n);
```
调用 musl 内部 malloc 分配 `n` 字节内存。

**阶段 3 — 清零优化**:
```
if (!p || (!__malloc_replaced && __malloc_allzerop(p)))
    return p;
n = mal0_clear(p, n);
return memset(p, 0, n);
```
- 若分配失败（`!p`），直接返回 NULL。
- 若 `__malloc_replaced == 0`（公共 malloc 未被替换）且 `__malloc_allzerop(p)` 返回非零（内存已知全零），则直接返回指针（跳过清零）。
- 否则，调用 `mal0_clear` 反向扫描清零，再对剩余部分补 `memset`。

**关键设计决策**:
- `__malloc_replaced` 检查确保仅在内部分配器仍被使用时才启用零页优化。若用户替换了 `malloc`，无法信任自定义分配器的行为，必须强制调用 `memset` 清零。

### 前置条件

- `m` 和 `n` 为非负 `size_t` 值。
- 若 `m * n` 不溢出且成功分配，则返回有效指针。
- 调用者不持有任何 malloc 锁。

### 后置条件

- **Case 1 (成功)**: 返回一个指向 `m * n` 字节连续内存块的指针，其中所有字节初始化为 0。返回的指针满足与 `malloc` 相同的对齐要求。
- **Case 2 (溢出)**: 若 `m * n` 溢出 `size_t`（即 `n != 0 && m > SIZE_MAX / n`），`errno` 被设置为 `ENOMEM`，返回 NULL。
- **Case 3 (分配失败)**: 若 `__libc_malloc(n)` 返回 NULL（内存不足），返回 NULL。
- **Case 4 (malloc 已被替换)**: 若 `__malloc_replaced` 为非零值，不使用零页优化，强制通过 `memset`/`mal0_clear` 清零所有字节。

### 不变量

- `__libc_calloc` 始终返回零初始化内存（或 NULL），无论 `__malloc_replaced` 的状态如何。
- `__libc_calloc` 的实现不得依赖于 `__libc_calloc` 之外的任何可替换的函数——它使用 `__libc_malloc`（内部版）而非 `malloc`（可替换版）。

---

## 对 musl 内部调用者的说明

musl 内部模块通过 `#define calloc __libc_calloc` 在包含 `<stdlib.h>` 之前重定向 `calloc` 符号，从而透明地使用内部分配器。这意味着以下 musl 组件始终使用内部 `calloc` 实现：

| 使用模块 | 源文件 |
|---------|-------|
| atexit 处理 | `src/exit/atexit.c` |
| 命名信号量 | `src/thread/sem_open.c` |
| 异步 I/O | `src/aio/aio.c` |
| 动态链接器错误处理 | `src/ldso/dlerror.c` |
| 进程 fd 操作 | `src/process/fdop.h` |
| NLS / gettext | `src/locale/dcngettext.c` |

## 外部依赖说明

| 依赖符号 | 来源 | 说明 |
|---------|------|------|
| `__libc_malloc` | `src/malloc/libc_calloc.c` 内 `#define malloc __libc_malloc` | musl 内部 malloc，不可被替换 |
| `__malloc_replaced` | `src/malloc/replaced.c` | `hidden extern int`，标记公开 malloc 是否被替换 |
| `__malloc_allzerop` | 本文件 `weak_alias(allzerop, ...)` | 可被 malloc 后端覆盖的零页检测 |
| `memset` | `<string.h>` / libc | 标准内存设置函数 |
| `errno` / `ENOMEM` | `<errno.h>` | 错误报告机制 |