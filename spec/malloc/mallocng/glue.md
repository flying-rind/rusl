# glue.h 规约

> **文件定位**: `glue.h` 是 mallocng 分配器与 musl 内部基础设施之间的**胶水适配层**，不包含任何对外导出的公共 API。其核心职责是：
> 1. 将 mallocng 内部符号映射到 musl 的 `__` 前缀命名空间
> 2. 封装系统调用接口（brk/mmap/madvise/mremap）
> 3. 提供统一的锁原语（读写锁/atfork 支持）
> 4. 提供线程安全检测、随机密钥生成等辅助基础设施

---

## 依赖图

```
glue.h 被包含于 meta.h → malloc.c / free.c / realloc.c / aligned_alloc.c
glue.h 依赖：
  → <stdint.h>      (标准库类型)
  → <sys/mman.h>    (mmap/madvise 等系统接口声明)
  → <pthread.h>     (线程相关)
  → <unistd.h>      (brk 声明)
  → <elf.h>         (AT_RANDOM 定义)
  → <string.h>      (memcpy)
  → "atomic.h"      (a_cas, a_crash 等原子操作 → see atomic.h spec)
  → "syscall.h"     (__syscall 宏 → see syscall.h spec)
  → "libc.h"        (libc 全局状态结构体 → see libc.h spec)
  → "lock.h"        (__lock/__unlock → see lock.h spec)
  → "dynlink.h"     (__malloc_replaced, __aligned_alloc_replaced → see dynlink.h spec)
```

---

## 宏：命名空间重映射

### size_classes
```
#define size_classes __malloc_size_classes
```

[Visibility]: Internal — musl 命名空间重映射，将内部分配器的大小类别表映射到 `__` 前缀

- **意图**: 避免与非 POSIX ISO C 保留符号命名空间之外的标识符发生符号冲突
- **说明**: 重映射后，源码中对 `size_classes` 的引用实际引用 `__malloc_size_classes`，该变量在 `malloc.c` 中定义，存储 48 个类别的大小上限（以 16 字节为单位）

---

### ctx
```
#define ctx __malloc_context
```

[Visibility]: Internal — musl 命名空间重映射

- **意图**: 将全局分配器上下文 `__malloc_context` 映射为内部分配器代码中的短名 `ctx`
- **说明**: `struct malloc_context` 类型定义在 `meta.h` 中，全局实例在 `malloc.c` 中定义，包含 secret、活跃 group 链表、使用量统计、mmap 计数器等所有全局状态

---

### alloc_meta
```
#define alloc_meta __malloc_alloc_meta
```

[Visibility]: Internal — musl 命名空间重映射

- **意图**: 重映射元数据分配函数
- **说明**: `alloc_meta()` 定义在 `malloc.c` 中（非 static），负责分配和初始化 `struct meta` 对象

---

### is_allzero
```
#define is_allzero __malloc_allzerop
```

[Visibility]: Internal — musl 命名空间重映射

- **意图**: 重映射"是否全零"检测函数
- **说明**: `is_allzero()` 定义在 `malloc.c` 中（非 static），涉及 `malloc_usable_size` 的正确性判断

---

### dump_heap
```
#define dump_heap __dump_heap
```

[Visibility]: Internal — musl 命名空间重映射，调试辅助

- **意图**: 重映射堆转储函数（调试用途）
- **说明**: 仅用于调试/诊断目的

---

### malloc / realloc / free
```
#define malloc  __libc_malloc_impl
#define realloc __libc_realloc
#define free    __libc_free
```

[Visibility]: Internal — musl 命名空间重映射

- **意图**: 在 mallocng 源码内部，`malloc`/`realloc`/`free` 实际绑定到 musl 内部实现符号而非 POSIX 公共名称
- **说明**: musl 在 `src/malloc/` 下的公共入口文件中通过 `weak_alias` 将 POSIX 名称重定向到这些 `__libc_*` 实现

---

## 宏：系统调用封装

### brk(p)
```
#define brk(p) ((uintptr_t)__syscall(SYS_brk, p))
```

[Visibility]: Internal — musl 内部系统调用封装

| 项目 | 描述 |
|------|------|
| **前置条件** | 无（直接发起系统调用） |
| **后置条件 (Case 1)** | 成功时返回新的 program break 地址（`uintptr_t` 类型） |
| **后置条件 (Case 2)** | 失败时返回不同于 `p` 的值（即旧 break 值），由调用者 `alloc_meta()` 处理 |
| **意图** | 封装 `SYS_brk` 系统调用，用于扩展进程堆（brk）区域来分配 meta_area 页面 |

- **调用上下文**: 仅在 `alloc_meta()` 中被调用，用于在动态链接器 brk 之上扩展元数据区域
- **返回值处理**: `alloc_meta()` 中检查 `brk(new) != new` 来判断失败

---

### mmap / madvise / mremap
```
#define mmap    __mmap
#define madvise __madvise
#define mremap  __mremap
```

[Visibility]: Internal — musl 内部系统调用名称映射

| 宏 | 说明 |
|-----|------|
| `mmap` | 映射到 `__mmap` — musl 内部 mmap 封装，可能涉及取消 off_t 的符号版本控制 |
| `madvise` | 映射到 `__madvise` — musl 内部 madvise 封装 |
| `mremap` | 映射到 `__mremap` — musl 内部 mremap 封装（Linux 特定） |

- **意图**: 在 musl 内部绕过公共符号版本控制，直接使用内部 mmap/madvise/mremap 实现
- **System Algorithm**: 这些重映射确保 mallocng 使用 musl 自己的系统调用封装路径，而非系统的 libc 符号，避免递归调用风险

---

## 宏：运行时配置

### USE_MADV_FREE
```
#define USE_MADV_FREE 0
```

[Visibility]: Internal — 编译时常量

- **意图**: 控制 `free()` 中是否使用 `MADV_FREE` 归还物理页面
- **说明**: 设为 0 时禁用 MADV_FREE（保守策略，页面立即可被内核回收统计计数），设为 1 时在 `free()` 的 madvise 路径中优先使用 `MADV_FREE`（延迟回收，性能更优但 RSS 统计不精确）

---

### DISABLE_ALIGNED_ALLOC
```
#define DISABLE_ALIGNED_ALLOC (__malloc_replaced && !__aligned_alloc_replaced)
```

[Visibility]: Internal — 运行时条件宏

- **前置条件**: 依赖 `dynlink.h` 中定义的 `__malloc_replaced` 和 `__aligned_alloc_replaced` 全局标志
- **后置条件**: 当用户替换了 `malloc` 但未替换 `aligned_alloc` 时求值为真（1），此时 `aligned_alloc()` 应返回 `ENOMEM` 以保持一致性
- **意图**: 防止在交叉替换场景下的不一致行为（用户自定 malloc 与 musl 的 aligned_alloc 混用会导致崩溃）

---

### MT
```
#define MT (libc.need_locks)
```

[Visibility]: Internal — 线程安全检测宏

- **说明**: 运行时检测是否需要加锁。当进程为单线程时（`libc.need_locks == 0`），跳过所有锁操作以提升性能
- **使用场景**: 在 `rdlock()` / `wrlock()` / `free()` 等所有锁操作路径中使用

---

### RDLOCK_IS_EXCLUSIVE
```
#define RDLOCK_IS_EXCLUSIVE 1
```

[Visibility]: Internal — 锁语义配置

- **说明**: 当为 1 时，读锁和写锁使用相同的互斥锁（无读写区分，都是排他锁），简化了锁语义
- **意图**: 在 malloc 场景下，读写者并无真正的并发收益（分配操作需要修改全局状态），因此简单的排他锁即可满足需求且避免读写锁的复杂度
- **使用位置**: `malloc()` 函数 fast-path 中，若 `RDLOCK_IS_EXCLUSIVE` 则直接本地更新 `avail_mask` 而非使用 CAS

---

### assert(x)
```
#if USE_REAL_ASSERT
#include <assert.h>
#else
#undef assert
#define assert(x) do { if (!(x)) a_crash(); } while(0)
#endif
```

[Visibility]: Internal — 断言行为配置

| **条件** | **行为** |
|-----------|----------|
| `USE_REAL_ASSERT` 定义 | 使用标准 `<assert.h>`（受 `NDEBUG` 控制） |
| `USE_REAL_ASSERT` 未定义（默认） | 断言失败时调用 `a_crash()` 直接终止进程，不受 `NDEBUG` 影响 |

- **意图**: 确保 mallocng 内部的一致性检查在发布版本中也生效（默认行为）。分配器内部的不变式违反通常意味着堆损坏，比崩溃更需要立即停止
- **依赖**: `a_crash()` 定义在 `atomic.h` 中（`__builtin_trap()` 或非法指令）

---

### PAGESIZE
```
#ifndef PAGESIZE
#define PAGESIZE PAGE_SIZE
#endif
```

[Visibility]: Internal — 页大小回退定义

- **意图**: 某些架构可能未定义 `PAGESIZE`（如使用 `PAGE_SIZE`），此回退确保兼容性

---

## 宏：锁对象定义

### LOCK_OBJ_DEF
```c
#define LOCK_OBJ_DEF \
void __malloc_atfork(int who) { malloc_atfork(who); } \
int __malloc_lock[1]
```

[Visibility]: Internal — musl 内部分配器锁对象声明宏

- **意图**: 在一个宏中展开生成：
  1. `__malloc_atfork()` — POSIX `pthread_atfork` 所需的回调函数符号
  2. `__malloc_lock[1]` — 分配器的全局互斥锁变量（单元素数组用于取地址语义）
- **使用位置**: 在 `malloc.c` 顶部通过 `LOCK_OBJ_DEF;` 展开为全局定义
- **说明**: `__malloc_lock` 声明为 `__visibility__("hidden")`，仅 musl 内部可见

---

## get_random_secret (内联函数)
```
static inline uint64_t get_random_secret()
```

[Visibility]: Internal — musl mallocng 内部辅助函数

| 项目 | 描述 |
|------|------|
| **前置条件** | `libc.auxv` 已初始化（动态链接器设置的辅助向量） |
| **后置条件** | 返回一个 64 位无符号随机值 |
| **Intent** | 为分配器生成一个进程生命期内**固定的随机密钥**，用于 `meta_area.check` 字段防止元数据伪造 |
| **System Algorithm** | 采用两步混合：1) 先取栈地址（`&secret`）乘以常数 `1103515245`（经典 LCG 乘数）作为基础熵源；2) 遍历 `libc.auxv[]` 查找 `AT_RANDOM` 条目，将内核提供的 16 字节随机种子中的高 8 字节 `memcpy` 到 secret 中 |

- **调用者**: 仅在 `alloc_meta()` 初始化路径中被调用一次，结果存入 `ctx.secret`
- **安全属性**: 结合了 ASLR 栈地址和内核随机种子两个熵源，降低了可预测性风险
- **说明**: 此密钥用于保护 `meta_area.check` 字段（分配器内部安全校验），使攻击者难以通过堆溢出伪造元数据区域

---

## 锁原语 (内联函数)

### rdlock()
```
static inline void rdlock()
{
    if (MT) LOCK(__malloc_lock);
}
```

[Visibility]: Internal — musl 内部分配器读锁

| 项目 | 描述 |
|------|------|
| **前置条件** | `libc.need_locks` 反映当前线程数状态 |
| **后置条件** | 若多线程模式：`__malloc_lock` 被持有，调用者获得排他访问权；单线程模式：无操作 |
| **Intent** | "读锁"实质上与写锁相同（`RDLOCK_IS_EXCLUSIVE=1`），仅在命名上区分分配路径的优化语义 |
| **System Algorithm** | 使用简单的自旋锁（`__lock` + `__unlock`），基于 futex 实现，不区分读写者 |

---

### wrlock()
```
static inline void wrlock()
{
    if (MT) LOCK(__malloc_lock);
}
```

[Visibility]: Internal — musl 内部分配器写锁

| 项目 | 描述 |
|------|------|
| **前置条件** | 同上 |
| **后置条件** | 若多线程模式：`__malloc_lock` 被持有 |
| **Intent** | 与 `rdlock()` 实现相同，语义上用于 `free()` 等需要修改全局状态的操作 |
| **说明** | 由于 `RDLOCK_IS_EXCLUSIVE=1`，实质上与 `rdlock()` 等价；代码中保留名称区分以便将来可改为真正的读写锁 |

---

### unlock()
```
static inline void unlock()
{
    UNLOCK(__malloc_lock);
}
```

[Visibility]: Internal — musl 内部分配器解锁

| 项目 | 描述 |
|------|------|
| **前置条件** | `__malloc_lock` 被当前线程持有（若多线程模式） |
| **后置条件** | `__malloc_lock` 被释放，等待者之一可获得锁 |
| **说明** | 调用 `__unlock()` → 基于 futex 的锁释放 |

---

### upgradelock()
```
static inline void upgradelock()
{
}
```

[Visibility]: Internal — musl 内部分配器锁升级（当前为空操作）

- **Intent**: 设计为将来在真正的读写锁实现中将读锁升级为写锁
- **说明**: 由于当前 `RDLOCK_IS_EXCLUSIVE=1`，读锁已是排他的，无需升级。此函数保留接口以备将来区分读写锁

---

### resetlock()
```
static inline void resetlock()
{
    __malloc_lock[0] = 0;
}
```

[Visibility]: Internal — musl 内部分配器锁重置

| 项目 | 描述 |
|------|------|
| **前置条件** | 在 `fork()` 的子进程中调用（单线程上下文，父进程的锁状态无效） |
| **后置条件** | `__malloc_lock` 被强制归零，清除父进程的遗留锁状态 |
| **Intent** | `fork()` 后子进程继承父进程的内存状态但只有一个线程，任何被父进程持有的锁必须在子进程中重置 |
| **使用场景** | 由 `malloc_atfork()` 在 `who > 0`（子进程分支）时调用 |

---

## malloc_atfork (内联函数)
```
static inline void malloc_atfork(int who)
```

[Visibility]: Internal — musl 内部分配器 atfork 回调

| 项目 | 描述 |
|------|------|
| **前置条件** | 由 `pthread_atfork()` 机制在 `fork()` 前后调用 |
| **后置条件** | 根据 `who` 参数执行相应操作 |
| **Intent** | 确保 `fork()` 期间分配器状态的一致性 |

**参数语义**:

| `who` 值 | 含义 | 执行操作 |
|-----------|------|----------|
| `who < 0` | prepare（fork 前） | `rdlock()` — 获取分配器锁，阻止其他线程在 fork 期间修改堆 |
| `who == 0` | parent（父进程 post-fork） | `unlock()` — 释放 prepare 阶段获取的锁 |
| `who > 0` | child（子进程 post-fork） | `resetlock()` — 将锁强制归零，清除父进程遗留的锁状态 |

- **System Algorithm**: 遵循标准"三阶段 atfork"模式。prepare 阶段加锁阻塞所有分配操作，确保 fork 时堆状态一致；父进程解锁恢复正常；子进程重置锁（因为子进程中锁的实际持有线程不存在）
- **调用链**: `LOCK_OBJ_DEF` 展开生成全局 `__malloc_atfork()` 函数，musl 的 `pthread_atfork()` 注册机制通过该符号找到回调

---

## 全局变量

### __malloc_lock
```
__attribute__((__visibility__("hidden")))
extern int __malloc_lock[1];
```

[Visibility]: Internal (不导出) — `__visibility__("hidden")`，仅在 musl 内部可见

- **类型**: `int[1]` — 单元素数组，用于取地址传递到 `__lock`/`__unlock`（需要 `volatile int *` 参数）
- **Intent**: musl mallocng 分配器的全局互斥锁。由 `LOCK_OBJ_DEF` 宏在 `malloc.c` 中定义
- **锁语义**: 自旋锁实现（基于 futex），在 `MT`（多线程）时为排他锁。`RDLOCK_IS_EXCLUSIVE=1` 时读写锁退化为此单一锁

---

## 跨文件依赖汇总

| 依赖符号 | 来源 | 类别 |
|----------|------|------|
| `__syscall` | `syscall.h` (src/internal) | 系统调用层 |
| `__lock` / `__unlock` | `lock.h` (src/internal) | 自旋锁原语 |
| `a_cas` / `a_ctz_32` / `a_or` / `a_crash` / `a_clz_32` | `atomic.h` (src/internal) | 原子操作层 |
| `libc.auxv` / `libc.need_locks` | `libc.h` (src/internal) | musl 全局运行时状态 |
| `__malloc_replaced` / `__aligned_alloc_replaced` | `dynlink.h` (src/internal) | 动态链接替换标志 |
| `size_classes[]` | `malloc.c` | 大小类别表定义 |
| `struct malloc_context ctx` | `malloc.c` → `meta.h` | 全局分配器上下文 |
| `alloc_meta()` | `malloc.c` | 元数据分配函数 |
| `get_page_size()` | libc 内部 | 运行时页大小获取 |
| `mprotect` | libc | 内存保护系统调用封装 |

---

## 不变式 (Invariants)

1. **锁一致性不变量**: `__malloc_lock` 的任何操作（加锁/解锁/重置）必须成对出现。每个 `rdlock()`/`wrlock()` 必须有对应的 `unlock()`，且在任何 `fork()` 子进程中 `resetlock()` 必须在首次锁操作前被调用。

2. **安全不变量**: `ctx.secret` 在进程生命期内保持不变，且 `meta_area.check` 必须始终等于 `ctx.secret`。此不变量由 `get_meta()` 中的断言检查保证（`assert(area->check == ctx.secret)`）。

3. **初始化顺序不变量**: `get_random_secret()` 必须在任何 `alloc_meta()` 调用后被调用，而 `alloc_meta()` 的使用必须发生在任何 `malloc()` / `free()` / `realloc()` 操作之前。该不变量由 `ctx.init_done` 标志 + `alloc_meta()` 中的惰性初始化保证。

4. **线程安全不变量**: 任何修改 `ctx` 全局状态（活跃链表、使用量计数、mmap 计数器、序列号）的操作必须在持有 `__malloc_lock` 时进行。fast-path 中的 `avail_mask` CAS 操作是唯一例外（原子操作隐含的锁自由语义）。