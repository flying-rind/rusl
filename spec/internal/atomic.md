# atomic.h 规约

> **源文件**: `/home/mangp/桌面/OS/musl/src/internal/atomic.h`
> **复杂度等级**: Level 3（高度优化设计 — 需要前置/后置条件 + 意图 + 显式系统算法）

---

## 依赖图

```
(外部) atomic_arch.h ──> atomic.h ──> 使用者（libc 其他模块）
                           │
                           ├── LL/SC 路径: a_pre_llsc / a_ll / a_sc / a_post_llsc
                           ├── CAS 路径:   a_cas (架构提供或 LL/SC 合成)
                           └── 软件回退:   所有未由架构提供的原语均从 a_cas 合成
```

`atomic.h` 是 musl libc 的核心原子操作抽象层。它包含 `atomic_arch.h`（架构特定原子指令），并为架构未提供的操作提供通用软件回退实现。所有函数均为 `static inline`，在编译时被内联展开为对应架构的最优指令序列。

---

## 外部依赖

| 依赖项 | 来源 | 处理方式 |
|--------|------|----------|
| `<stdint.h>` | C 标准库 | 跳过 |
| `atomic_arch.h` | musl 架构特定头文件 | **外部模块依赖** — 每个架构提供其原生原子操作子集，本 spec 不展开，仅记录接口约定 |

---

## 架构特定接口约定（atomic_arch.h 应提供）

架构头文件通过 `#define` 提供以下原语中的**一个或多个**（`#ifndef` 守卫机制：架构定义了就不会使用通用回退）。musl 支持两种策略：

### 策略 A：基于 LL/SC（Load-Linked / Store-Conditional，如 RISC-V、ARM、MIPS）

| 宏/函数 | 签名 | 语义 |
|---------|------|------|
| `a_ll` | `int a_ll(volatile int *p)` | Load-Linked：读取 `*p`，同时设置独占监视器 |
| `a_sc` | `int a_sc(volatile int *p, int v)` | Store-Conditional：若独占监视器仍有效，写入 `v` 到 `*p` 并返回 1；否则返回 0 |
| `a_pre_llsc` | `void a_pre_llsc()` | LL/SC 循环前置屏障（可选，默认为空） |
| `a_post_llsc` | `void a_post_llsc()` | LL/SC 循环后置屏障（可选，默认为空） |
| `a_ll_p` | `void *a_ll_p(volatile void *p)` | 指针版 Load-Linked（可选） |
| `a_sc_p` | `int a_sc_p(volatile void *p, void *v)` | 指针版 Store-Conditional（可选） |

### 策略 B：基于原生 CAS（如 x86_64、aarch64）

| 宏/函数 | 签名 | 语义 |
|---------|------|------|
| `a_cas` | `int a_cas(volatile int *p, int t, int s)` | 原子比较并交换：若 `*p == t` 则写入 `s`，返回 `*p` 的旧值 |
| `a_swap` | `int a_swap(volatile int *p, int v)` | 原子交换（可选，可回退到 CAS） |
| `a_fetch_add` | `int a_fetch_add(volatile int *p, int v)` | 原子加并返回旧值（可选） |
| `a_fetch_and` | `int a_fetch_and(volatile int *p, int v)` | 原子按位与并返回旧值（可选） |
| `a_fetch_or` | `int a_fetch_or(volatile int *p, int v)` | 原子按位或并返回旧值（可选） |
| `a_and` | `void a_and(volatile int *p, int v)` | 原子按位与（可选） |
| `a_or` | `void a_or(volatile int *p, int v)` | 原子按位或（可选） |
| `a_inc` | `void a_inc(volatile int *p)` | 原子自增（可选） |
| `a_dec` | `void a_dec(volatile int *p)` | 原子自减（可选） |
| `a_store` | `void a_store(volatile int *p, int v)` | 带内存屏障的原子存储（可选） |
| `a_barrier` | `void a_barrier()` | 编译器+内存屏障（可选） |
| `a_cas_p` | `void *a_cas_p(volatile void *p, void *t, void *s)` | 指针版 CAS（可选） |
| `a_and_64` | `void a_and_64(volatile uint64_t *p, uint64_t v)` | 64 位原子按位与（可选） |
| `a_or_64` | `void a_or_64(volatile uint64_t *p, uint64_t v)` | 64 位原子按位或（可选） |
| `a_ctz_32` | `int a_ctz_32(uint32_t x)` | 32 位尾随零计数（可选） |
| `a_ctz_64` | `int a_ctz_64(uint64_t x)` | 64 位尾随零计数（可选） |
| `a_clz_32` | `int a_clz_32(uint32_t x)` | 32 位前导零计数（可选） |
| `a_clz_64` | `int a_clz_64(uint64_t x)` | 64 位前导零计数（可选） |
| `a_crash` | `void a_crash()` | 触发崩溃（可选） |

---

## 符号规约

下文按 **拓扑排序** 排列：底层原语在前，依赖它们的上层函数在后。

---

### `a_pre_llsc` / `a_post_llsc`

```c
#ifndef a_pre_llsc
#define a_pre_llsc()
#endif

#ifndef a_post_llsc
#define a_post_llsc()
#endif
```

[Visibility]: Internal — musl 原子操作基础设施，POSIX/C 标准未定义。

#### 功能意图 (Intent)

LL/SC 循环的前后屏障。在需要显式屏障的弱内存序架构（如 ARM）上，由 `atomic_arch.h` 定义为实际的 `dmb`/`fence` 指令；在强内存序架构（如 x86_64）上保持为空宏，编译时被完全消除。

#### 不变量 (Invariants)

- **INV-1**: 若 `a_pre_llsc()` 和 `a_post_llsc()` 均为非空，则它们必须配对使用，不能在 LL/SC 循环中间单独出现。

---

### `a_cas`（从 LL/SC 合成版）

```c
#ifndef a_cas
#define a_cas a_cas
static inline int a_cas(volatile int *p, int t, int s)
{
    int old;
    a_pre_llsc();
    do old = a_ll(p);
    while (old==t && !a_sc(p, s));
    a_post_llsc();
    return old;
}
#endif
```

[Visibility]: Internal — musl 原子操作基础设施。

#### 系统算法 (System Algorithm)

使用 LL/SC 指令对实现 CAS 的标准模式：

```
a_cas(*p, t, s) {
    do {
        old = a_ll(p);          // Load-Linked: 读取并设置独占监视
        if (old != t) break;    // 值不匹配，无需尝试写入
    } while (!a_sc(p, s));      // Store-Conditional: 若独占监视有效则写入 s
    return old;                 // 返回 *p 的原始值
}
```

**Key Insight**: 当 `old != t` 时，立即退出循环而不尝试 `a_sc`，避免浪费总线周期。若 `a_sc` 因独占监视丢失而失败，重新 LL 再试。

#### 前置条件 (Preconditions)

- **PRE-1**: `p` 指向有效的、对齐到 `int` 边界的内存位置。
- **PRE-2**: `*p` 不被其他线程同时通过非原子手段访问。

#### 后置条件 (Postconditions)

- **POST-1**: 返回 `*p` 在执行 CAS 之前的旧值。
- **POST-2**: 若 `*p == t`，则 `*p` 被原子地设置为 `s`（但可能已有其他写入者抢先修改，需调用者自己检查返回值）。
- **POST-3**: 整个操作具有完全的内存顺序一致性（依赖于 `a_pre_llsc`/`a_post_llsc` 屏障）。

---

### `a_swap`（从 LL/SC 合成版）

```c
#ifndef a_swap
#define a_swap a_swap
static inline int a_swap(volatile int *p, int v)
{
    int old;
    a_pre_llsc();
    do old = a_ll(p);
    while (!a_sc(p, v));
    a_post_llsc();
    return old;
}
#endif
```

[Visibility]: Internal — musl 原子操作基础设施。

#### 系统算法 (System Algorithm)

与 CAS 版本类似，但无条件写入：循环直到 `a_sc` 成功。由于每次 LL 后总是尝试 SC，在 LL 和 SC 之间无分支，减少了流水线暂停。

#### 前置条件 (Preconditions)

- **PRE-1**: `p` 指向有效的、对齐的 `int` 内存。
- **PRE-2**: `*p` 不被其他线程同时通过非原子手段访问。

#### 后置条件 (Postconditions)

- **POST-1**: 返回 `*p` 在执行交换之前的旧值。
- **POST-2**: `*p` 被原子地设置为 `v`。
- **POST-3**: 操作是原子的且有顺序一致性。

---

### `a_fetch_add`（从 LL/SC 合成版）

```c
#ifndef a_fetch_add
#define a_fetch_add a_fetch_add
static inline int a_fetch_add(volatile int *p, int v)
{
    int old;
    a_pre_llsc();
    do old = a_ll(p);
    while (!a_sc(p, (unsigned)old + v));
    a_post_llsc();
    return old;
}
#endif
```

[Visibility]: Internal — musl 原子操作基础设施。

#### 系统算法 (System Algorithm)

使用 LL/SC 实现原子的 fetch-and-add。`(unsigned)old + v` 确保加法以无符号语义进行（回绕行为定义良好，与原子性语义兼容）。

---

### `a_fetch_and`（从 LL/SC 合成版）

```c
#ifndef a_fetch_and
#define a_fetch_and a_fetch_and
static inline int a_fetch_and(volatile int *p, int v)
{
    int old;
    a_pre_llsc();
    do old = a_ll(p);
    while (!a_sc(p, old & v));
    a_post_llsc();
    return old;
}
#endif
```

[Visibility]: Internal — musl 原子操作基础设施。

---

### `a_fetch_or`（从 LL/SC 合成版）

```c
#ifndef a_fetch_or
#define a_fetch_or a_fetch_or
static inline int a_fetch_or(volatile int *p, int v)
{
    int old;
    a_pre_llsc();
    do old = a_ll(p);
    while (!a_sc(p, old | v));
    a_post_llsc();
    return old;
}
#endif
```

[Visibility]: Internal — musl 原子操作基础设施。

---

### `a_cas_p`（从 LL/SC 合成版，指针版本）

```c
#ifdef a_ll_p
#ifndef a_cas_p
#define a_cas_p a_cas_p
static inline void *a_cas_p(volatile void *p, void *t, void *s)
{
    void *old;
    a_pre_llsc();
    do old = a_ll_p(p);
    while (old==t && !a_sc_p(p, s));
    a_post_llsc();
    return old;
}
#endif
#endif
```

[Visibility]: Internal — musl 原子操作基础设施。

#### 功能意图 (Intent)

指针版本的 CAS。仅在架构提供 `a_ll_p`/`a_sc_p` 时可用（如 aarch64）。语义与 `a_cas` 完全相同，但操作对象为指针。

---

### `a_cas` — 存在性检查

```c
#ifndef a_cas
#error missing definition of a_cas
#endif
```

[Visibility]: Internal — 编译期约束。

#### 不变量 (Invariants)

- **INV-1**: 编译时保证：在 `atomic.h` 处理后，`a_cas` 必须已被定义（由架构提供或由 LL/SC 合成）。若两者均未提供，编译失败。这是 musl 原子层的基本安全约束。

---

### `a_swap`（CAS 回退版）

```c
#ifndef a_swap
#define a_swap a_swap
static inline int a_swap(volatile int *p, int v)
{
    int old;
    do old = *p;
    while (a_cas(p, old, v) != old);
    return old;
}
#endif
```

[Visibility]: Internal — musl 原子操作基础设施。

#### 系统算法 (System Algorithm)

使用 CAS 循环模拟原子交换：
```
do {
    old = *p;                 // 读取当前值
} while (a_cas(p, old, v) != old);  // CAS: 若没人修改则写入 v
```
若 CAM 中间有其他写入者修改了 `*p`，`a_cas` 返回的值将不等于 `old`，循环会重试。

#### 前置条件 (Preconditions)

- **PRE-1**: `p` 指向有效的、对齐的 `int` 内存。
- **PRE-2**: `a_cas` 已在别处定义（由架构提供或 LL/SC 合成）。

---

### `a_fetch_add`（CAS 回退版）

```c
#ifndef a_fetch_add
#define a_fetch_add a_fetch_add
static inline int a_fetch_add(volatile int *p, int v)
{
    int old;
    do old = *p;
    while (a_cas(p, old, (unsigned)old+v) != old);
    return old;
}
#endif
```

[Visibility]: Internal — musl 原子操作基础设施。

#### 系统算法 (System Algorithm)

标准 CAS 循环实现 fetch-and-add：
```
do {
    old = *p;                          // 读取当前值
    new = (unsigned)old + v;           // 计算新值（无符号用于回绕）
} while (a_cas(p, old, new) != old);
```

---

### `a_fetch_and`（CAS 回退版）

```c
#ifndef a_fetch_and
#define a_fetch_and a_fetch_and
static inline int a_fetch_and(volatile int *p, int v)
{
    int old;
    do old = *p;
    while (a_cas(p, old, old&v) != old);
    return old;
}
#endif
```

[Visibility]: Internal — musl 原子操作基础设施。

---

### `a_fetch_or`（CAS 回退版）

```c
#ifndef a_fetch_or
#define a_fetch_or a_fetch_or
static inline int a_fetch_or(volatile int *p, int v)
{
    int old;
    do old = *p;
    while (a_cas(p, old, old|v) != old);
    return old;
}
#endif
```

[Visibility]: Internal — musl 原子操作基础设施。

---

### `a_and`

```c
#ifndef a_and
#define a_and a_and
static inline void a_and(volatile int *p, int v)
{
    a_fetch_and(p, v);
}
#endif
```

[Visibility]: Internal — musl 原子操作基础设施。

#### 功能意图 (Intent)

原子按位与，丢弃返回值。当调用者仅关心副作用（修改 `*p`），不关心旧值时使用。

#### 后置条件 (Postconditions)

- **POST-1**: `*p` 被原子地设置为 `*p & v`（执行 `a_and` 前的值与 `v` 按位与的结果）。

---

### `a_or`

```c
#ifndef a_or
#define a_or a_or
static inline void a_or(volatile int *p, int v)
{
    a_fetch_or(p, v);
}
#endif
```

[Visibility]: Internal — musl 原子操作基础设施。

#### 后置条件 (Postconditions)

- **POST-1**: `*p` 被原子地设置为 `*p | v`。

---

### `a_inc`

```c
#ifndef a_inc
#define a_inc a_inc
static inline void a_inc(volatile int *p)
{
    a_fetch_add(p, 1);
}
#endif
```

[Visibility]: Internal — musl 原子操作基础设施。

#### 功能意图 (Intent)

原子的 `++(*p)`。等价于 `a_fetch_add(p, 1)` 但丢弃返回值。用于引用计数、统计计数等高频场景。

#### 后置条件 (Postconditions)

- **POST-1**: `*p` 被原子地增加 1。
- **POST-2**: 无返回值（轻量级，避免不必要的寄存器使用）。

---

### `a_dec`

```c
#ifndef a_dec
#define a_dec a_dec
static inline void a_dec(volatile int *p)
{
    a_fetch_add(p, -1);
}
#endif
```

[Visibility]: Internal — musl 原子操作基础设施。

#### 功能意图 (Intent)

原子的 `--(*p)`。等价于 `a_fetch_add(p, -1)`。

#### 后置条件 (Postconditions)

- **POST-1**: `*p` 被原子地减少 1。

---

### `a_store`

```c
#ifndef a_store
#define a_store a_store
static inline void a_store(volatile int *p, int v)
{
#ifdef a_barrier
    a_barrier();
    *p = v;
    a_barrier();
#else
    a_swap(p, v);
#endif
}
```

[Visibility]: Internal — musl 原子操作基础设施。

#### 系统算法 (System Algorithm)

两个实现策略：

1. **屏障路径（a_barrier 存在）**: 先发内存屏障（刷新所有进行中的写操作），再执行普通赋值，再发内存屏障（确保赋值对后续读可见）。在 x86_64 上，`a_barrier()` 仅是编译器屏障，代价最低。
2. **交换路径（a_barrier 不存在）**: 使用 `a_swap` 实现，`a_swap` 隐含全内存屏障。

#### 后置条件 (Postconditions)

- **POST-1**: `*p == v` 成立。
- **POST-2**: 在所有线程看来，store 之前的写入已发生，store 之后的读取将看到新值。
- **POST-3**: 无返回值。

---

### `a_barrier`

```c
#ifndef a_barrier
#define a_barrier a_barrier
static inline void a_barrier()
{
    volatile int tmp = 0;
    a_cas(&tmp, 0, 0);
}
#endif
```

[Visibility]: Internal — musl 原子操作基础设施。

#### 功能意图 (Intent)

全内存屏障的通用软件实现。当架构未提供原生屏障指令时，通过 `a_cas(&tmp, 0, 0)` 模拟内存屏障——CAS 操作隐含全内存序列化（在大多数架构上）。

#### 后置条件 (Postconditions)

- **POST-1**: 在执行 `a_barrier()` 之后的所有内存操作，在所有线程看来，都发生于该屏障之后。
- **POST-2**: 在执行 `a_barrier()` 之前的所有内存操作，在所有线程看来，都发生于该屏障之前。

---

### `a_spin`

```c
#ifndef a_spin
#define a_spin a_barrier
#endif
```

[Visibility]: Internal — musl 原子操作基础设施。

#### 功能意图 (Intent)

自旋等待提示。在支持 `PAUSE` 指令的架构上（如 x86_64），`a_spin` 被定义为特殊的低功耗等待指令以减少自旋循环中的功耗；在不支持的架构上，退化为 `a_barrier`，仅阻止编译器重排。

---

### `a_and_64`

```c
#ifndef a_and_64
#define a_and_64 a_and_64
static inline void a_and_64(volatile uint64_t *p, uint64_t v)
{
    union { uint64_t v; uint32_t r[2]; } u = { v };
    if (u.r[0]+1) a_and((int *)p, u.r[0]);
    if (u.r[1]+1) a_and((int *)p+1, u.r[1]);
}
#endif
```

[Visibility]: Internal — musl 原子操作基础设施。

#### 系统算法 (System Algorithm)

将 64 位按位与分解为两个 32 位原子操作：

1. 将 64 位值 `v` 按字节序表示为两个 `uint32_t`（`r[0]` 为低 32 位，`r[1]` 为高 32 位，小端序）。
2. `if (u.r[0]+1)` 检查低 32 位是否全为 1（即 `0xFFFFFFFF`）——若 `r[0] == 0xFFFFFFFF`，则 `r[0]+1 == 0`，跳过低 32 位操作（因为 `x & 0xFFFFFFFF == x`，无需操作）；否则执行 32 位 `a_and`。
3. 对高 32 位做同样的优化检查。

**Key Insight**: 此算法**不是原子 64 位操作**——两个 32 位操作之间存在竞态窗口。此函数仅用于标志位清除场景（如将指定位清零），在 musl 的使用上下文中，竞态条件是可接受的。

#### 前置条件 (Preconditions)

- **PRE-1**: `p` 指向有效的 8 字节对齐（或至少 4 字节对齐）的 `uint64_t`。

#### 后置条件 (Postconditions)

- **POST-1**: `*p` 的低 32 位被原子地按位与 `(uint32_t)v`（当 `v` 的低 32 位不是全 1 时）。
- **POST-2**: `*p` 的高 32 位被原子地按位与 `(uint32_t)(v >> 32)`（当 `v` 的高 32 位不是全 1 时）。
- **POST-3**: 低 32 位和高 32 位的修改之间**没有原子性保证**。

---

### `a_or_64`

```c
#ifndef a_or_64
#define a_or_64 a_or_64
static inline void a_or_64(volatile uint64_t *p, uint64_t v)
{
    union { uint64_t v; uint32_t r[2]; } u = { v };
    if (u.r[0]) a_or((int *)p, u.r[0]);
    if (u.r[1]) a_or((int *)p+1, u.r[1]);
}
#endif
```

[Visibility]: Internal — musl 原子操作基础设施。

#### 系统算法 (System Algorithm)

将 64 位按位或分解为两个 32 位原子操作：

1. `if (u.r[0])` 检查低 32 位是否为 0——若为 0，`x | 0 == x`，跳过；否则执行 32 位 `a_or`。
2. 对高 32 位做同样的优化检查。

与 `a_and_64` 相同，两个 32 位操作之间没有 64 位原子性。

---

### `a_cas_p`（通用回退版，指针到 int 强制转换）

```c
#ifndef a_cas_p
typedef char a_cas_p_undefined_but_pointer_not_32bit[-sizeof(char) == 0xffffffff ? 1 : -1];
#define a_cas_p a_cas_p
static inline void *a_cas_p(volatile void *p, void *t, void *s)
{
    return (void *)a_cas((volatile int *)p, (int)t, (int)s);
}
#endif
```

[Visibility]: Internal — musl 原子操作基础设施。

#### 系统算法 (System Algorithm)

将指针 CAS 退化为 `int` 版 CAS，利用了 "在 32 位系统上 sizeof(void*) == sizeof(int)" 的事实。

#### 编译期安全检查

```c
typedef char a_cas_p_undefined_but_pointer_not_32bit[-sizeof(char) == 0xffffffff ? 1 : -1];
```

这行代码的作用：
- `-sizeof(char) == 0xffffffff` 在 **32 位系统**上成立（`-1 == 0xffffffff`），数组大小为 1，类型定义成功。
- 在 **64 位系统**上不成立（`-1 != 0xffffffffffffffff`），数组大小为 -1，触发**编译错误**。

**Key Insight**: 通用 `a_cas_p` 回退**仅在 32 位系统上有效**。64 位架构必须在 `atomic_arch.h` 中提供专门的 `a_cas_p` 实现。

---

### `a_or_l`

```c
#ifndef a_or_l
#define a_or_l a_or_l
static inline void a_or_l(volatile void *p, long v)
{
    if (sizeof(long) == sizeof(int)) a_or(p, v);
    else a_or_64(p, v);
}
#endif
```

[Visibility]: Internal — musl 原子操作基础设施。

#### 功能意图 (Intent)

与平台字长匹配的原子按位或。在 32 位系统上调用 `a_or`（32 位操作），在 64 位系统上调用 `a_or_64`（64 位操作，但分解为两个 32 位操作）。

---

### `a_crash`

```c
#ifndef a_crash
#define a_crash a_crash
static inline void a_crash()
{
    *(volatile char *)0=0;
}
#endif
```

[Visibility]: Internal — musl 原子操作基础设施。

#### 功能意图 (Intent)

触发立即崩溃。在架构未提供专门崩溃指令时，通过写入空指针触发 SIGSEGV。用于断言失败、不可恢复的内部错误等场景。

#### 后置条件 (Postconditions)

- **POST-1**: 程序终止（通过 SIGSEGV 或等效信号）。

---

### `a_ctz_32`

```c
#ifndef a_ctz_32
#define a_ctz_32 a_ctz_32
static inline int a_ctz_32(uint32_t x)
{
#ifdef a_clz_32
    return 31-a_clz_32(x&-x);
#else
    static const char debruijn32[32] = { ... };
    return debruijn32[(x&-x)*0x076be629 >> 27];
#endif
}
#endif
```

[Visibility]: Internal — musl 原子操作基础设施。

#### 系统算法 (System Algorithm)

计算 32 位整数尾随零（ctz = count trailing zeros）。两种实现策略：

1. **a_clz_32 存在时**: `ctz(x) = 31 - clz(x & -x)`。因为 `x & -x` 只保留了最低位的 1，其 `clz` 即从最高位到该位的距离，31 减之即为尾随零数。
2. **通用 De Bruijn 序列法**: 通过完美的哈希乘法 `(x & -x) * 0x076be629`，提取高 5 位字节作为查表索引。De Bruijn 序列保证 32 种可能的 `x & -x` 值映射到 0..31 的唯一索引。

#### 前置条件 (Preconditions)

- **PRE-1**: `x` 不为 0。调用者负责确保 `x != 0`（`a_ctz(0)` 的结果未定义）。

#### 后置条件 (Postconditions)

- **POST-1**: 返回 `x` 的二进制表示中最低位的 1 所在的位置（0 = bit 0, 31 = bit 31）。

---

### `a_ctz_64`

```c
#ifndef a_ctz_64
#define a_ctz_64 a_ctz_64
static inline int a_ctz_64(uint64_t x)
{
    static const char debruijn64[64] = { ... };
    if (sizeof(long) < 8) {
        uint32_t y = x;
        if (!y) {
            y = x>>32;
            return 32 + a_ctz_32(y);
        }
        return a_ctz_32(y);
    }
    return debruijn64[(x&-x)*0x022fdd63cc95386dull >> 58];
}
#endif
```

[Visibility]: Internal — musl 原子操作基础设施。

#### 系统算法 (System Algorithm)

1. **在 32 位系统上**（`sizeof(long) < 8`）: 先将 64 位数分为低 32 位和高 32 位处理。若低 32 位为 0，则尾随零在高 32 位，结果为 `32 + a_ctz_32(hi)`；否则为 `a_ctz_32(lo)`。
2. **在 64 位系统上**: 使用 De Bruijn 序列法——与 `a_ctz_32` 类似，但使用 64 位的魔数 `0x022fdd63cc95386d`，查表返回 0..63。

#### 前置条件 (Preconditions)

- **PRE-1**: `x` 不为 0。

#### 后置条件 (Postconditions)

- **POST-1**: 返回 `x` 的二进制表示中最低位的 1 所在的位置（0..63）。

---

### `a_ctz_l`

```c
static inline int a_ctz_l(unsigned long x)
{
    return (sizeof(long) < 8) ? a_ctz_32(x) : a_ctz_64(x);
}
```

[Visibility]: Internal — musl 原子操作基础设施。

> 注意：此函数**无条件定义**（无 `#ifndef` 守卫），意味着架构不能覆盖它的实现。

#### 功能意图 (Intent)

与平台字长匹配的尾随零计数。根据 `sizeof(long)` 分发到 32 位或 64 位版本。

---

### `a_clz_64`

```c
#ifndef a_clz_64
#define a_clz_64 a_clz_64
static inline int a_clz_64(uint64_t x)
{
#ifdef a_clz_32
    if (x>>32)
        return a_clz_32(x>>32);
    return a_clz_32(x) + 32;
#else
    uint32_t y;
    int r;
    if (x>>32) y=x>>32, r=0; else y=x, r=32;
    if (y>>16) y>>=16; else r |= 16;
    if (y>>8) y>>=8; else r |= 8;
    if (y>>4) y>>=4; else r |= 4;
    if (y>>2) y>>=2; else r |= 2;
    return r | !(y>>1);
#endif
}
#endif
```

[Visibility]: Internal — musl 原子操作基础设施。

#### 系统算法 (System Algorithm)

计算前导零（clz = count leading zeros）：

1. **a_clz_32 存在时**: 若高 32 位非零，`clz = clz32(hi32)`；否则 `clz = 32 + clz32(lo32)`。
2. **通用二分搜索**: 基于 2 的幂次分段确定最高位 1 的位置：
   - 先判断 64 位中的高 32 位是否有 1
   - 再依次二分判断 16/8/4/2 位范围
   - 最后通过 `!(y>>1)` 确定余下 1 位
   - 累加结果即为前导零数（0..63）

#### 前置条件 (Preconditions)

- **PRE-1**: `x` 可以为 0。当 `x == 0` 时，返回 64（此为常见约定，但调用者应注意）。

#### 后置条件 (Postconditions)

- **POST-1**: 当 `x != 0` 时，返回 `x` 的二进制表示中最高位 1 之前的前导零数量（0..63）。
- **POST-2**: 当 `x == 0` 时，返回 64（通过二分搜索的 `r` 累积逻辑自然得出）。

---

### `a_clz_32`

```c
#ifndef a_clz_32
#define a_clz_32 a_clz_32
static inline int a_clz_32(uint32_t x)
{
    x >>= 1;
    x |= x >> 1;
    x |= x >> 2;
    x |= x >> 4;
    x |= x >> 8;
    x |= x >> 16;
    x++;
    return 31-a_ctz_32(x);
}
#endif
```

[Visibility]: Internal — musl 原子操作基础设施。

#### 系统算法 (System Algorithm)

使用"涂抹" (smear) 技术将最高位 1 之后的所有位都置为 1，然后使用 `a_ctz_32`：

1. `x >>= 1`: 右移 1 位，防止 `x` 本身就是全 1 的情况（此时 `x+1` 会溢出）。
2. `x |= x >> 1; x |= x >> 2; ... x |= x >> 16`: 幂次传播，将最高位 1 之后的所有位都置为 1。
3. `x++`: 加 1，产生一个恰好为 2 的幂的数（仅最高位 1 的下一位为 1）。
4. `31 - a_ctz_32(x)`: ctz 得出的是从最低位算起的位置，31 减去即得到最高位 1 的前导零数。

#### 前置条件 (Preconditions)

- **PRE-1**: `x` 可以为 0。当 `x == 0` 时，返回 32。

#### 后置条件 (Postconditions)

- **POST-1**: 当 `x != 0` 时，返回 `x` 的最高位 1 之前的前导零数（0..31）。
- **POST-2**: 当 `x == 0` 时，返回 32。

---

## 全局不变量 (Global Invariants)

适用于 `atomic.h` 中所有符号：

- **GINV-1 (最小基元保证)**: 在此头文件的末尾，`a_cas` **必须**已被定义。若架构既不提供原生 `a_cas` 也不提供 `a_ll`/`a_sc`，编译将失败。
- **GINV-2 (同步保证)**: 所有返回旧值的原子操作（`a_cas`、`a_swap`、`a_fetch_add` 等）具有顺序一致性，即操作之间全局线序，且与程序序一致。
- **GINV-3 (volatile 正确性)**: 所有原子操作的目标指针均为 `volatile` 限定，确保编译器不优化掉或重排操作。
- **GINV-4 (无锁保证)**: 所有 `atomic.h` 中的原语均为无锁实现（lock-free），依赖于硬件原子指令或 LL/SC 指令对，不依赖于内核互斥锁。

---

## 跨模块依赖

| 符号 | 定义位置 | 关系 |
|------|----------|------|
| `a_ll`, `a_sc`, `a_ll_p`, `a_sc_p` | `arch/*/atomic_arch.h` | LL/SC 基元，由架构提供 |
| `a_cas` (原生版) | `arch/*/atomic_arch.h` | CAS 基元，由架构提供 |
| `a_and`, `a_or`, `a_inc`, `a_dec` (原生版) | `arch/*/atomic_arch.h` | 架构可能提供更优的原生实现 |
| `a_store` (原生版) | `arch/*/atomic_arch.h` | 架构可能提供带屏障的存储指令 |
| `a_barrier` (原生版) | `arch/*/atomic_arch.h` | 架构可能提供原生屏障指令 |
| `a_spin` (原生版) | `arch/*/atomic_arch.h` | 架构可能提供 PAUSE 类指令 |
| `a_clz_32`, `a_clz_64`, `a_ctz_32`, `a_ctz_64` (原生版) | `arch/*/atomic_arch.h` | 架构可能提供原生位扫描指令 |
| `a_crash` (原生版) | `arch/*/atomic_arch.h` | 架构可能提供专门的崩溃指令 |

所有 `atomic.h` 中的原子原语是整个 musl libc 的**同步基础设施**，被锁实现、信号处理、线程管理、内存分配器等所有子系统依赖。

---

## Rust 实现提示 (`#![no_std]`)

在 `rusl` 中，此模块应完全使用 `core::sync::atomic` 重建：

| musl C 原语 | Rust 等价 |
|-------------|-----------|
| `a_cas(p, t, s)` | `(*p).compare_exchange(t, s, Ordering::SeqCst, Ordering::SeqCst)` |
| `a_swap(p, v)` | `(*p).swap(v, Ordering::SeqCst)` |
| `a_fetch_add(p, v)` | `(*p).fetch_add(v, Ordering::SeqCst)` |
| `a_fetch_and(p, v)` | `(*p).fetch_and(v, Ordering::SeqCst)` |
| `a_fetch_or(p, v)` | `(*p).fetch_or(v, Ordering::SeqCst)` |
| `a_and(p, v)` | `(*p).fetch_and(v, Ordering::SeqCst);` |
| `a_or(p, v)` | `(*p).fetch_or(v, Ordering::SeqCst);` |
| `a_inc(p)` | `(*p).fetch_add(1, Ordering::SeqCst);` |
| `a_dec(p)` | `(*p).fetch_add(-1, Ordering::SeqCst);` |
| `a_store(p, v)` | `(*p).store(v, Ordering::SeqCst);` |
| `a_barrier()` | `core::sync::atomic::fence(Ordering::SeqCst);` |
| `a_spin()` | `core::sync::atomic::spin_loop_hint();` |
| `a_crash()` | `core::intrinsics::abort();` |
| `a_ctz_32/64` | `(x).trailing_zeros()` (内建方法) |
| `a_clz_32/64` | `(x).leading_zeros()` (内建方法) |

> Rust 的 `core::sync::atomic` 是 `#![no_std]` 兼容的，无需使用 `std`。