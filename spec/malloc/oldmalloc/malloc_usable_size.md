# malloc_usable_size.c 规约

## 依赖图

```
malloc_usable_size  (对外导出)
  ├── CHUNK_SIZE     (宏) → 定义于 malloc_impl.h
  ├── MEM_TO_CHUNK   (宏) → 定义于 malloc_impl.h
  ├── OVERHEAD       (宏) → 定义于 malloc_impl.h
  └── struct chunk   (类型) → 定义于 malloc_impl.h

__realloc_dep  (内部符号)
  └── realloc        (函数) → 外部模块 (libc), 跳过
```

---

## 内部类型与宏

### `struct chunk`
[Visibility]: Internal — musl 内部分配器使用的 chunk 头部结构，POSIX/C 标准未定义

```c
struct chunk {
    size_t psize, csize;
    struct chunk *next, *prev;
};
```

用于管理堆内存块的内部元数据结构。每个通过 `malloc` 系列函数分配的堆内存块前方都有一个 `struct chunk` 头部。

- `psize`: 前一个 chunk 的大小（低位用作标记位）
- `csize`: 当前 chunk 的大小（最低位 `C_INUSE` 标记当前 chunk 是否在用）
- `next`, `prev`: 空闲链表中的前后指针（仅当 chunk 空闲时有效）

### `OVERHEAD` (宏)
[Visibility]: Internal — musl 内部使用的常量

```c
#define OVERHEAD (2*sizeof(size_t))
```

单个 chunk 的元数据开销，即 `sizeof(struct chunk)` 中 `psize + csize` 两个字段的大小（不含 `next/prev`，因为 `next/prev` 仅在空闲 chunk 中使用，可能与用户数据区域重叠）。

- 64 位系统: `OVERHEAD == 16`
- 32 位系统: `OVERHEAD == 8`

### `CHUNK_SIZE(c)` (宏)
[Visibility]: Internal

```c
#define CHUNK_SIZE(c) ((c)->csize & -2)
```

读取 chunk `c` 的有效大小。掩码 `-2`（即 `~1`）清除最低位 `C_INUSE` 标记，返回纯大小值。

**前置条件**: `c` 必须指向有效的 `struct chunk` 实例。

### `MEM_TO_CHUNK(p)` (宏)
[Visibility]: Internal

```c
#define MEM_TO_CHUNK(p) (struct chunk *)((char *)(p) - OVERHEAD)
```

将用户可见的指针 `p` 转换回内部 `struct chunk` 指针。用户指针指向 chunk 头部之后 `OVERHEAD` 字节处的数据区起始位置。

**前置条件**: `p` 必须是由 `malloc` / `calloc` / `realloc` / `aligned_alloc` 返回的有效堆指针（或为 NULL）。

---

## `__realloc_dep` (内部符号)

```c
hidden void *(*const __realloc_dep)(void *, size_t) = realloc;
```

[Visibility]: Internal (不导出) — `hidden` 属性确保该符号不出现在 ELF 动态符号表中

**意图**：链接器级依赖注入。musl 的 `realloc` 实现内部调用 `malloc_usable_size` 来获取原分配块的大小，形成循环引用：`realloc` → `malloc_usable_size`。如果 `malloc_usable_size.c` 中不引用 `realloc`，在静态链接等场景下，链接器可能因符号解析顺序问题导致 `realloc` 符号未包含在最终可执行文件中。

通过 `__realloc_dep = realloc`，强制 `malloc_usable_size.o` 的目标文件中包含对 `realloc` 的符号引用，保证链接器在处理 `malloc_usable_size.o` 时一定将 `realloc.o` 拉入链接。

`const` 限定确保该函数指针在运行时不会被修改（存储在只读数据段）。

---

## `malloc_usable_size` (对外导出)

```c
size_t malloc_usable_size(void *p);
```

[Visibility]: Public (导出) — GNU 扩展，声明于 `<malloc.h>`，用户程序可直接调用

### 功能描述

返回通过 `malloc` / `calloc` / `realloc` / `aligned_alloc` 分配的堆内存块的实际可用字节数。

返回值是 `malloc` 族函数在内部为该分配请求实际预留的内存大小——该值不小于原始 `malloc(size)` 调用时传入的 `size` 参数，但可能因内部对齐与 chunk 开销策略而略大。用户可将该返回值作为上限，安全地访问该内存。

### 前置条件

- 若 `p != NULL`，则 `p` 必须是由同一分配器实例（musl 的 `malloc` / `calloc` / `realloc` / `aligned_alloc`）返回且尚未被 `free` 的有效堆指针。
- 不得对栈变量、全局变量、`mmap` 直接返回的指针或已 `free` 的指针调用本函数，否则行为未定义。

### 后置条件

- **Case 1** (`p == NULL`): 返回 `0`。不修改任何全局或堆状态。
- **Case 2** (`p != NULL`): 返回 `CHUNK_SIZE(MEM_TO_CHUNK(p)) - OVERHEAD`。
  1. `MEM_TO_CHUNK(p)` 将用户指针偏移 `-OVERHEAD` 字节到内部 `struct chunk` 头部。
  2. `CHUNK_SIZE(c)` 读取 `c->csize` 并清除 `C_INUSE` 位得到原始 chunk 大小。
  3. 从原始 chunk 大小中减去 `OVERHEAD`，得到用户数据区实际可用字节数。

### 不变量

- 本函数为纯查询操作：不修改任何堆元数据，不持有锁，不分配或释放内存。
- 返回值始终满足 `返回值 >= 原始请求大小`（若原始请求大小已知）。
- 对于已释放的 chunk 调用本函数的结果不确定——`csize` 字段在 `free` 后可能被相邻空闲 chunk 合并逻辑覆写。

### 意图

提供一个 O(1) 的查询接口，使用户能够获知堆分配的实际可用空间上限。常用于：
- `realloc` 实现中快速判断是否需要实际搬迁数据（若原块空间足够则原地返回）。
- 用户态内存调试与统计。

### 算法复杂度

- 时间复杂度: **O(1)** — 一次指针偏移 + 一次内存读取 + 一次位掩码 + 一次减法。
- 空间复杂度: **O(1)** — 不分配额外内存。

### 错误处理

本函数**不设置 `errno`**。唯一的特殊情形是 `p == NULL`，此时直接返回 0。

### 线程安全性

**线程安全**。本函数只读取传入指针所属 chunk 的元数据字段（`csize`），不修改任何共享状态，不持有任何锁。在并发环境下与 `malloc` / `free` / `realloc` 同时调用是安全的——前提是传入的指针 `p` 在上述并发操作完成前保持有效（即未被另一个线程 `free`）。

---

*本规约通过递归依赖追踪生成。所有内部类型/宏（`struct chunk`, `CHUNK_SIZE`, `MEM_TO_CHUNK`, `OVERHEAD`）在本文件中描述；跨文件依赖（`malloc`, `realloc`）标注引用来源。*