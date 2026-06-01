# memalign.c 规约

> 源文件: `src/malloc/memalign.c`
> 符号数量: 1 导出，0 内部

---

## 依赖图

```
memalign ──→ aligned_alloc (see mallocng/aligned_alloc.c or oldmalloc/aligned_alloc.c)
```

`memalign` 是 `aligned_alloc` 的薄封装，无内部依赖。所有核心逻辑由被调用的 `aligned_alloc` 承担。

---

## memalign (对外导出)

```c
void *memalign(size_t align, size_t len);
```

**[Visibility]: Public** — 声明于 `<malloc.h>` (第17行) 及 `<stdlib.h>` (第145行，条件编译)，源自 SunOS/BSD 的遗存函数，POSIX.1-2008 标记为 obsolescent。

### 意图 (Intent)

提供按指定对齐边界分配堆内存的能力。musl 将其实现为 C11 标准函数 `aligned_alloc(align, len)` 的直接委托 —— 无任何适配层或参数变换。

### 前置条件 (Preconditions)

1. **对齐参数**：`align` 必须是 2 的幂 (`(align & -align) == align`)，否则 `aligned_alloc` 返回 NULL 并设置 `errno = EINVAL`。
2. **大小参数**：`len` 必须满足 `len <= SIZE_MAX - align`，且 `align < (1ULL<<31)*UNIT`，否则 `aligned_alloc` 返回 NULL 并设置 `errno = ENOMEM`。
3. **替换检测**：若通过 `malloc` 替换机制（`__malloc_replaced` 非零）且 `aligned_alloc` 未被一同替换（`__aligned_alloc_replaced` 为零），则 `aligned_alloc` 返回 NULL 并设置 `errno = ENOMEM`。
4. **对齐下界**：若 `align <= UNIT`（malloc-ng 内部最小对齐单元），调用方传入的 `align` 值被提升至 `UNIT`（实际上等价于普通 `malloc`）。

### 后置条件 (Postconditions)

| 分支 | 条件 | 结果 |
|------|------|------|
| 成功 | `aligned_alloc(align, len)` 成功 | 返回指向至少 `len` 字节、地址对齐于 `align` 边界的内存块指针。内存内容未初始化。 |
| 失败 | `aligned_alloc(align, len)` 返回 NULL | 返回 NULL，`errno` 被设置为 `EINVAL` 或 `ENOMEM`（取决于失败原因）。 |

### 系统算法 (System Algorithm)

**Level 3** — 实现策略至关重要。

`memalign` 采用 **委托模式 (Delegation Pattern)**：

```
memalign(align, len)
  = aligned_alloc(align, len)   // C11 标准函数，定义于 src/malloc/mallocng/aligned_alloc.c
                                // 或 src/malloc/oldmalloc/aligned_alloc.c
```

此实现策略选择具有两层含义：

1. **语义收窄**：传统 BSD `memalign` 允许 `len` 不为 `align` 的整数倍，但 musl 将其委托给 C11 的 `aligned_alloc`，由后者根据具体分配器实现决定是否施加该限制。malloc-ng 版本的 `aligned_alloc` 不显式校验 `len % align == 0`，因此 musl 的 `memalign` 行为上等价于传统 BSD 版本。
2. **分配器切换透明**：musl 在编译时选择 malloc-ng（新分配器）或 oldmalloc（旧分配器），`memalign` 调用对应的 `aligned_alloc` 而无需感知差异，实现了源码级别的分配器无关性。

### 不变量 (Invariants)

无模块局部不变量。该函数为纯委托，所有不变量由 `aligned_alloc` 的内部实现维护（详见 `mallocng/aligned_alloc.c` 或 `oldmalloc/aligned_alloc.c` 规约）。

### 错误码

| errno 值 | 触发条件 |
|----------|----------|
| `EINVAL` | `align` 不是 2 的幂 |
| `ENOMEM` | `len > SIZE_MAX - align` 或 `align` 过大或分配器已被替换但 aligned_alloc 未被替换或底层 `malloc` 返回 NULL |

### 边界情况

- **align = 0**：不满足 2 的幂条件 `(0 & -0) != 0`，被视为非法参数，`aligned_alloc` 返回 NULL 并设 `errno = EINVAL`。
- **len = 0**：行为由底层 `aligned_alloc` 决定。C 标准允许 `malloc(0)` 返回 NULL 或可安全传给 `free()` 的非 NULL 指针；musl 的 `aligned_alloc` 在 `len = 0` 时将其传递给 `malloc(align - UNIT)`（若 `align <= UNIT` 则 `malloc(0)`），行为与 `malloc(0)` 一致。
- **超大对齐**：若 `align >= (1ULL<<31)*UNIT`，`aligned_alloc` 直接返回 NULL + ENOMEM，即使系统有足够内存也不尝试分配。这是 musl 对极端对齐请求的硬性拒绝。

---

## 跨文件依赖说明

| 依赖符号 | 定义位置 | 性质 |
|----------|----------|------|
| `aligned_alloc` | `src/malloc/mallocng/aligned_alloc.c` 或 `src/malloc/oldmalloc/aligned_alloc.c` | C11 标准 Public API，跨模块依赖，详见对应文件的 spec |

---

*本规约通过递归依赖追踪生成：`memalign` → `aligned_alloc`（跨文件依赖，终止追踪）。*