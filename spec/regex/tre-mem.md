# tre-mem 内存分配器规约

## 复杂度分级: Level 2

---

## 依赖图

```
tre_mem_new ──→ tre_mem_new_impl ──→ calloc / memset (外部 libc)
tre_mem_alloc ──→ tre_mem_alloc_impl ──→ malloc / free / memset (外部 libc)
tre_mem_calloc ──→ tre_mem_alloc_impl
tre_mem_destroy ──→ free (外部 libc)

tre_mem_new_impl, tre_mem_alloc_impl, tre_mem_destroy 共享:
  tre_mem_struct, tre_list_t (本模块内部数据结构, 定义于 tre.h)
  ALIGN, TRE_MEM_BLOCK_SIZE (本模块内部宏, 定义于 tre.h)
```

---

## 内部数据结构

### tre_list_t

**[Visibility]:** Internal — musl TRE 内部链表节点类型，定义于 `tre.h`，标准未定义

```c
typedef struct tre_list {
  void *data;              // 指向分配的内存块
  struct tre_list *next;   // 链表中的下一个节点
} tre_list_t;
```

**语义**: 单向链表节点，用于链接分配器中所有动态分配的内存块。`data` 指向块的实际数据区，`next` 指向下一个块节点。链表以 `NULL` 终结。

**不变量**:
- 链表无环，以 `NULL` 终止
- `data` 指向的块大小至少为 `TRE_MEM_BLOCK_SIZE`（或更大的按需块）

---

### tre_mem_struct / tre_mem_t

**[Visibility]:** Internal — musl TRE 内部内存分配器类型，定义于 `tre.h`，标准未定义

```c
typedef struct tre_mem_struct {
  tre_list_t *blocks;    // 内存块链表头指针
  tre_list_t *current;   // 当前活跃块节点
  char *ptr;             // 当前块中下一个可分配位置
  size_t n;              // 当前块中剩余可用字节数
  int failed;            // 分配失败标志（0=正常, 1=已失败）
  void **provided;       // 外部提供的块指针（alloca 模式）
} *tre_mem_t;
```

**语义**: 基于块链表的简化内存分配器（bump-pointer allocator）。不能单独释放分配出的内存块，只能通过 `tre_mem_destroy` 一次性释放全部。

**不变量**:
- `failed == 0` 时: `blocks` 链表中的所有节点和 `current` 均有效，`ptr` 指向有效内存区域，`n` 反映当前块的剩余容量
- `failed == 1` 时: 分配器处于永久失败状态，后续所有分配请求均返回 `NULL`
- `current` 为 `NULL` ⟹ `blocks` 也为 `NULL`，`ptr` 和 `n` 无意义（初始状态）
- `current != NULL` ⟹ `current` 必然是 `blocks` 链表中的某个节点

---

## 内部宏

### TRE_MEM_BLOCK_SIZE

**[Visibility]:** Internal — musl TRE 内部常量，定义于 `tre.h`

```c
#define TRE_MEM_BLOCK_SIZE 1024
```

**语义**: 默认内存块大小（字节）。新分配的内存块至少为此大小；若单次请求超过此值的 1/8，则块大小扩大为 `size * 8`。

---

### ALIGN

**[Visibility]:** Internal — musl TRE 内部内存对齐宏，定义于 `tre.h`

```c
#define ALIGN(ptr, type) \
  ((((long)ptr) % sizeof(type)) \
   ? (sizeof(type) - (((long)ptr) % sizeof(type))) \
   : 0)
```

**语义**: 计算为使指针 `ptr` 按 `type` 类型对齐所需的额外字节数。返回值为 0（已对齐）或正数（需要跳过的字节数）。

**前置条件**: `ptr` 为可转换为 `long` 的有效指针值；`type` 为完整类型。

**后置条件**: 返回值 range 为 `[0, sizeof(type) - 1]`。

---

### tre_mem_new

**[Visibility]:** Internal — musl TRE 内部宏，定义于 `tre.h`，宏展开调用 `__tre_mem_new_impl`

```c
#define tre_mem_new()  tre_mem_new_impl(0, NULL)
```

**语义**: 创建新的堆分配内存分配器，等价于 `tre_mem_new_impl(0, NULL)`。实际由 `calloc` 分配 `tre_mem_struct`。

---

### tre_mem_alloc

**[Visibility]:** Internal — musl TRE 内部宏，定义于 `tre.h`，宏展开调用 `__tre_mem_alloc_impl`

```c
#define tre_mem_alloc(mem, size) tre_mem_alloc_impl(mem, 0, NULL, 0, size)
```

**语义**: 从分配器 `mem` 中分配 `size` 字节，不进行零初始化。等价于 `tre_mem_alloc_impl(mem, 0, NULL, 0, size)`。

---

### tre_mem_calloc

**[Visibility]:** Internal — musl TRE 内部宏，定义于 `tre.h`，宏展开调用 `__tre_mem_alloc_impl`

```c
#define tre_mem_calloc(mem, size) tre_mem_alloc_impl(mem, 0, NULL, 1, size)
```

**语义**: 从分配器 `mem` 中分配 `size` 字节并零初始化。等价于 `tre_mem_alloc_impl(mem, 0, NULL, 1, size)`。

---

## 内部函数

### tre_mem_new_impl

**[Visibility]:** Internal — 通过 `#define tre_mem_new_impl __tre_mem_new_impl` 重命名，以 `hidden` 可见性标记；仅通过内部宏 `tre_mem_new()` 间接调用，不对外导出

```c
tre_mem_t tre_mem_new_impl(int provided, void *provided_block);
```

#### 意图
创建并初始化一个新的 TRE 内存分配器实例，支持两种模式：堆分配模式（默认）和外部提供块模式（`TRE_USE_ALLOCA`）。

#### 前置条件
- `provided` 必须为 `0` 或 `1`
- 若 `provided == 1`，`provided_block` 必须指向一块至少 `sizeof(struct tre_mem_struct)` 字节的有效内存区域
- 若 `provided == 0`，`provided_block` 被忽略（传 `NULL`）

#### 后置条件
- **Case 1 (成功)**:
  - 返回指向已初始化 `tre_mem_struct` 的指针
  - 分配器处于初始状态: `blocks=NULL, current=NULL, ptr=NULL, n=0, failed=0, provided=NULL`
  - 若 `provided == 1`: 使用 `provided_block` 直接作为分配器结构体，并对其 `memset` 清零
  - 若 `provided == 0`: 通过 `calloc` 在堆上分配并清零
- **Case 2 (失败)**:
  - 返回 `NULL`
  - 仅当 `provided == 0` 且底层 `calloc` 失败时发生

#### 算法
```
if provided:
    mem = provided_block
    memset(mem, 0, sizeof(*mem))
else:
    mem = calloc(1, sizeof(*mem))
if mem == NULL:
    return NULL
return mem
```

---

### tre_mem_alloc_impl

**[Visibility]:** Internal — 通过 `#define tre_mem_alloc_impl __tre_mem_alloc_impl` 重命名，以 `hidden` 可见性标记；仅通过内部宏 `tre_mem_alloc()` / `tre_mem_calloc()` 间接调用，不对外导出

```c
void *tre_mem_alloc_impl(tre_mem_t mem, int provided, void *provided_block,
                         int zero, size_t size);
```

#### 意图
从 bump-pointer 分配器 `mem` 中分配一块内存。核心是高效的增量分配器（bump allocator），通过块链表（linked blocks）管理内存，无法单独释放分配出的内存。

#### 前置条件
- `mem` 是由 `tre_mem_new_impl` 成功返回的有效 `tre_mem_t` 指针
- `provided` 必须为 `0` 或 `1`
- 若 `provided == 1`，且当前块不足时，`provided_block` 若非 `NULL` 则指向 `TRE_MEM_BLOCK_SIZE` 字节的有效内存；若为 `NULL` 则立即失败
- `zero` 必须为 `0` 或 `1`
- `size > 0`

#### 后置条件
- **Case 1 (mem->failed == 1)**: 立即返回 `NULL`，不修改分配器状态
- **Case 2 (当前块有足够空间)**:
  - 返回指向 `mem->ptr`（对齐调整后）的指针
  - `mem->ptr` 向前推进 `size + 对齐填充` 字节
  - `mem->n` 减少相应字节数
  - 若 `zero == 1`: 通过 `memset` 将分配区域清零
- **Case 3 (当前块空间不足，需分配新块)**:
  - 计算新块大小: `block_size = max(TRE_MEM_BLOCK_SIZE, size * 8)`
  - 若 `provided == 1`:
    - 若 `provided_block == NULL` → 设 `mem->failed = 1`，返回 `NULL`
    - 否则使用 `provided_block` 作为新块，设置 `mem->ptr = provided_block`，`mem->n = TRE_MEM_BLOCK_SIZE`
  - 若 `provided == 0`:
    - 通过 `malloc` 分配 `tre_list_t` 节点和 `block_size` 字节数据块
    - 将新节点链接到 `blocks` 链表末尾，更新 `current`
    - 设置 `mem->ptr` 指向新数据块首地址，`mem->n = block_size`
    - 任何 `malloc` 失败 → 设 `mem->failed = 1`，返回 `NULL`（已分配的部分需通过 `free` 清理后再设失败标志）
  - 然后执行与 Case 2 相同的空间分配逻辑
- **失败永久性**: 一旦 `mem->failed` 被设为 `1`，所有后续对同一 `mem` 的调用均返回 `NULL`

#### 不变量
- 分配器失败后 (`failed=1`) 为不可逆状态
- `ptr` 始终在所属块的 `[data, data + block_size)` 范围内
- 返回的指针满足 `long` 类型对齐要求
- 链表中的块不会移动或释放，直到 `tre_mem_destroy`

#### 算法

```
if mem->failed:
    return NULL

if mem->n < size:
    // 当前块空间不足，分配新块
    if provided:
        if provided_block == NULL:
            mem->failed = 1; return NULL
        mem->ptr = provided_block
        mem->n = TRE_MEM_BLOCK_SIZE
    else:
        block_size = max(TRE_MEM_BLOCK_SIZE, size * 8)
        l = malloc(sizeof(tre_list_t))
        if l == NULL:
            mem->failed = 1; return NULL
        l->data = malloc(block_size)
        if l->data == NULL:
            free(l); mem->failed = 1; return NULL
        l->next = NULL
        // 链入 blocks 链表
        if mem->current: mem->current->next = l
        if mem->blocks == NULL: mem->blocks = l
        mem->current = l
        mem->ptr = l->data
        mem->n = block_size

// 对齐调整
size += ALIGN(mem->ptr + size, long)

// 从当前块分配（bump pointer）
ptr = mem->ptr
mem->ptr += size
mem->n -= size

// 可选的零初始化
if zero:
    memset(ptr, 0, size)

return ptr
```

---

### tre_mem_destroy

**[Visibility]:** Internal — 通过 `#define tre_mem_destroy __tre_mem_destroy` 重命名，以 `hidden` 可见性标记；由 `regfree` 等公开函数间接调用，不直接对外导出

```c
void tre_mem_destroy(tre_mem_t mem);
```

#### 意图
释放分配器 `mem` 及其管理的所有内存，包括所有链表块的数据区和节点本身，以及分配器结构体自身。

#### 前置条件
- `mem` 是由 `tre_mem_new_impl` 成功返回的有效 `tre_mem_t` 指针（可为 `NULL` 吗？实际实现中若 `mem == NULL` 则 `mem->blocks` 解引用将导致未定义行为，因此调用者必须确保 `mem != NULL`）

#### 后置条件
- 分配器结构体 `*mem` 被 `free` 回收
- 所有通过 `tre_mem_alloc_impl` 从该分配器分配的内存被 `free` 回收
- `blocks` 链表中的每个节点（`tre_list_t`）及其 `data` 均被 `free`
- 分配器及其所有关联内存不可再被访问

#### 算法

```
l = mem->blocks
while l != NULL:
    free(l->data)     // 释放数据块
    tmp = l->next
    free(l)           // 释放链表节点
    l = tmp
free(mem)             // 释放分配器结构体自身
```

---

## 依赖关系

### 依赖的外部资源
- `<stdlib.h>`: `malloc`, `calloc`, `free`（通过 `xmalloc`, `xcalloc`, `xfree` 宏重命名）
- `<string.h>`: `memset`
- `"tre.h"`: `tre_mem_t`, `tre_list_t`, `TRE_MEM_BLOCK_SIZE`, `ALIGN` 宏

### 被依赖
- `regcomp.c`: 通过 `tre_mem_new()` / `tre_mem_alloc()` / `tre_mem_calloc()` / `tre_mem_destroy()` 使用本模块，用于管理正则编译期间的所有临时分配
- `regexec.c`: 通过相同宏使用本模块，用于管理正则匹配期间的运行时分配

---

## 设计意图总览

TRE 内存分配器是一种 **bump-pointer 分配器**（又称 arena allocator），专为正则表达式编译/匹配期间大量小块分配的场景优化：

1. **批量释放**: 不支持单独释放，仅在 `tre_mem_destroy` 时一次性回收所有内存。这避免了每个小块单独 `free` 的开销和潜在的内存碎片
2. **块链扩展**: 当当前块耗尽时，分配新的固定大小块（1024 字节）并链入链表，而非尝试 `realloc`
3. **双模式支持**: 通过 `TRE_USE_ALLOCA` 编译时选项，支持以 `alloca`（栈分配）替代 `malloc`，允许在栈上创建分配器以降低堆分配开销
4. **失败传播**: `failed` 标志位确保一旦某次分配失败，后续所有分配请求快速失败（fail-fast），避免在 OOM 场景下反复尝试分配
