# donate.c 规约

## 依赖图

```
__malloc_donate
  └── donate                            (static, 当前文件)
        ├── memset                       (外部: <string.h> — 跳过)
        ├── alloc_meta()                 (内部: malloc.c, see malloc.c spec)
        ├── queue()                      (内部: meta.h inline 函数)
        ├── ctx                          (全局上下文: malloc.c)
        ├── size_classes[]               (全局表: malloc.c)
        ├── struct meta                  (meta.h)
        ├── struct group                 (meta.h)
        └── UNIT 宏                      (meta.h)
```

---

## donate (内部函数)

### 函数签名
```c
static void donate(unsigned char *base, size_t len);
```

[Visibility]: Internal — musl mallocng 内部辅助函数，`static` 修饰，不对外导出

### 意图
将一段已清零的连续内存区域拆分为多个大小类（size class）的单槽（single-slot）内存组，并将它们逐个加入全局分配器上下文 `ctx.active[]` 链表，使之可被后续 `malloc` 分配使用。调用方（动态链接器）通过此函数将其不使用的内存页"捐献"给堆分配器。

### 系统算法
采用**从大到小贪心拆分**策略：

1. **对齐边界**：将起始地址 `base` 向上对齐到 `UNIT`（16 字节）边界，将结束地址向下对齐到 `UNIT` 边界。这确保后续插入的 `struct group` 和 `struct meta` 自然对齐。
2. **全区域清零**：对整个 `[base, base+len)` 区域调用 `memset(base, 0, len)`，确保被捐献内存中所有 header 字节初始为 0。
3. **逆序遍历大小类**：从最大 size class（47）开始，以步长 4 递减遍历（即 47, 43, 39, 35, 31, 27, 23, 19, 15, 11, 7, 3）。对每个 size class `sc`：
   - 若剩余空间 `b - a` 不足以容纳 `(size_classes[sc] + 1) * UNIT` 字节的 group，跳过该 class。
   - 调用 `alloc_meta()` 分配一个 `struct meta` 元数据对象。
   - 将 group 的起始地址 `a` 作为 `struct group *`，初始化其元数据和 slot 内部结构。
   - 将 group 加入 `ctx.active[sc]` 循环双向链表。
   - 将指针 `a` 向前推进 `(size_classes[sc] + 1) * UNIT` 字节。
4. **剩余碎片**：遍历结束后，剩余不足任何 class 最小尺寸的空间被丢弃（不再使用）。

### 单槽 Group 初始化

对于每个被捐献的 group（仅含 1 个 slot，`last_idx = 0`）：

```c
m->avail_mask = 0;        // 无可用 slot（等待 free 后产生 freed_mask → avail_mask）
m->freed_mask = 1;        // slot 0 标记为已释放，表示该 slot 内存在可用内存
m->mem = (void *)a;       // group 首地址
m->mem->meta = m;         // 反向指针，从 group 到 meta
m->last_idx = 0;          // 仅有 slot 0
m->freeable = 0;          // 标记为不可被 munmap/madvise 释放（捐献内存不可回收）
m->sizeclass = sc;        // 绑定大小类
m->maplen = 0;            // 非 mmap 分配
```

设置 slot header 字节（位于 slot 用户数据起始位置的前 4 字节）：

| 偏移 | 字节 | 含义                                                        |
|------|------|-------------------------------------------------------------|
| `UNIT-4` (=12) | `0` | check byte，设为 0 表示无扩展 offset（此 slot 从 group 起始处偏移 0） |
| `UNIT-3` (=13) | `255` (=0xFF) | header byte：`idx = 255 & 31 = 31`，`reserved = 255 >> 5 = 7` |
| `UNIT-2` (=14) | `0` | offset 低字节（由 memset 已归零）                            |
| `UNIT-1` (=15) | `0` | offset 高字节（由 memset 已归零）                            |

设置 slot 结束标记字节：`m->mem->storage[size_classes[sc] * UNIT - 4] = 0`，此字节位于 slot 末尾之后，用于 `get_nominal_size` 中的 `assert(!*end)` 校验。

### 前置条件
- `base != NULL`
- `len > 0`
- `[base, base+len)` 所在内存页为可读写（PROT_READ | PROT_WRITE）
- 全局分配器上下文 `ctx` 已初始化（`ctx.init_done == 1`）
- `alloc_meta()` 必须能成功分配（即存在可用的 meta 区域或能通过 brk/mmap 扩展）
- 调用方持有 malloc 写入锁，或此时为单线程环境（早期初始化阶段）

### 后置条件
- `[base, base+len)` 范围内的所有字节被清零。
- 在可用空间内，从大到小依次建立了若干单槽 groups，每个 group 的 `struct meta` 被链表化到 `ctx.active[sc]` 上。
- 每个被捐献 group 的 `freed_mask = 1`、`avail_mask = 0`、`freeable = 0`。
- `ctx.usage_by_class[sc]` 因 `alloc_group` 被 `alloc_meta` 之外的其他路径间接更新——实际上 `donate` 本身不更新 `usage_by_class`。（该统计仅在 `alloc_group` 中更新，donate 不走 alloc_group 路径。）
- 遍历结束后，未使用的尾部碎片（`b - a < min_group_size`）被丢弃，不再被追踪。
- 该函数无返回值，无法向调用方报告部分失败。

### 不变量
- 每个被创建的 group，其 `meta->mem->meta == meta`（group 与 meta 互为反向指针）。
- 每个被创建的 group，其 `meta->last_idx == 0`（单槽）。
- 捐献内存被标记为 `freeable = 0`，确保 `free()` 路径不会尝试 munmap/madvise 回收这些页面。
- `maplen = 0`，确保 `get_stride()` 使用 `UNIT * size_classes[sc]` 计算 stride 而非 mmap 路径。

### 性能特性
- 时间复杂度 O(N)，其中 N 为可容纳的 group 数量上限（最多约 `len / (最小 group 尺寸)` 次迭代）。
- 空间开销：每个 group 需要一个 `struct meta`（典型 32 字节），以及每个 group 的一个 UNIT（16 字节）group header。对于大的捐献区域，开销可忽略不计。

---

## __malloc_donate (对外导出)

### 函数签名
```c
void __malloc_donate(char *start, char *end);
```

[Visibility]: Internal — musl 内部接口，声明于 `src/internal/dynlink.h`（`hidden` 可见性），仅供动态链接器 `ldso/dynlink.c:reclaim()` 调用，用于将共享库可写段之间的对齐间隙内存"捐献"给 malloc 堆。POSIX/C 标准未定义此接口，用户程序不应调用。

### 意图
对外部调用者（动态链接器）提供一个简洁的接口：接收任意地址区间 `[start, end)`，直接委托给内部函数 `donate()` 完成实际的拆分和并入操作。

### 前置条件
- `start != NULL`
- `end > start`（区间非空）
- `[start, end)` 为可读写内存页
- 调用发生在动态链接器初始化阶段（`reclaim_gaps` → `reclaim`），此时为单线程环境，无锁竞争
- 全局 malloc 上下文 `ctx` 已初始化

### 后置条件
- 等价于执行 `donate((unsigned char *)start, (size_t)(end - start))`。
- 区间 `[start, end)` 内的可用部分被按大小类拆分为可分配的单槽 groups。
- 函数无返回值，无法向调用方报告错误。

### 不变量
- 函数本身无副作用，所有状态变更由内部 `donate()` 完成，参见 `donate` 的不变量。

---

## 依赖符号汇总

### 来自 meta.h（当前模块内部，inline 函数）
- `queue(struct meta **phead, struct meta *m)`: 将 meta 节点插入双向循环链表

### 来自 malloc.c（当前模块内部，跨文件）
- `alloc_meta()`: 分配并返回一个新的 `struct meta` 对象。see `malloc.c` spec
- `ctx` (`struct malloc_context`): 全局分配器上下文，包含所有 active lists、usage 统计等
- `size_classes[]` (`const uint16_t[48]`): 查找表，将 size class 索引映射到 slot 大小（以 UNIT 为单位）

### 来自 meta.h（类型定义）
- `struct meta`: 内存组元数据，包含 prev/next 双向链表指针、avail_mask/freed_mask 位掩码、sizeclass、maplen 等字段
- `struct group`: 内存组 header，包含指向 meta 的反向指针、active_idx 及紧接的 slot 存储区
- `UNIT` 宏: 值为 16，最小对齐单位

### 外部依赖（跳过，不生成 spec）
- `memset()`: 来自 `<string.h>`，标准库函数
- `uintptr_t`, `size_t`: C 语言内建类型