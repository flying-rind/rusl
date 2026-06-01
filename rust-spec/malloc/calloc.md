# calloc — Rust 接口归约

> 源 C 文件: `src/malloc/calloc.c`
> 对应 C spec: `src/malloc/spec/calloc.md`
> 功能: 实现 `calloc` 及内部零填充优化辅助函数
> rusl 模块路径: `rusl/src/malloc/calloc.rs`

---

## 完整依赖链路 (递归追踪)

```
calloc (Public, extern "C")
├── malloc                          → rusl/src/malloc/  (外部模块)
│   ├── [lite_malloc.c / mallocng/ spec]
│   ├── __libc_malloc_impl         → lite_malloc weak_alias / mallocng strong symbol
│   │   └── __simple_malloc        → bump 分配器核心
│   │       ├── LOCK / UNLOCK      → __lock / __unlock (lock.h)
│   │       ├── __syscall(SYS_brk) → syscall.h (内联汇编)
│   │       ├── __mmap             → mman/mmap.c (syscall 封装)
│   │       └── libc.page_size     → libc.h (运行时页大小)
│   └── __libc_malloc              → 间接调用 __libc_malloc_impl 的内部分配入口
│
├── __malloc_replaced              → replaced.c (内部全局 int 标志)
│   └── (无更深的函数依赖 — 仅 int 全局变量, 被 ldso/dynlink.c 写入)
│
├── __malloc_allzerop(p)           → [本模块] 弱符号默认实现 (allzerop)
│   └── (无依赖 — 默认始终返回 0)
│
├── mal0_clear(p, n)              → [本模块] 内部 static 辅助函数
│   └── memset                     → rusl/src/string/memset.rs (外部模块)
│       └── core::slice::<impl [u8]>::fill  (Rust 标准库核心原语, no_std 兼容)
│
└── errno / ENOMEM                 → rusl 内部 errno 机制
    └── thread_local! { ... }     (per-thread 错误存储)
```

---

## 内部结构体/类型

| 类型 | 来源 | 用途 |
|------|------|------|
| `usize` | `core::ffi` 原语类型, 等价于 C `size_t` | 内存尺寸 |
| `u64` | Rust 内建类型, 等价于 C `uint64_t` | 快速零检测的别名类型 |
| `*mut core::ffi::c_void` | `core::ffi` | 通用内存指针 |
| `core::ffi::c_int` | `core::ffi` | C ABI int 类型 |

---

## Layer 1: 底层内部函数

---

### mal0_clear (内部函数)

**[Visibility]**: Internal -- `pub(crate)` 函数，rusl 内部零填充优化辅助函数，不对用户导出

**[Complexity]**: Level 2 -- 含优化策略，需 Intent 描述

#### Rust 内部签名 (安全重设计)

```rust
pub(crate) fn mal0_clear(p: *mut u8, n: usize) -> usize;
```

#### C ABI 兼容说明

该函数为内部函数，不出现在对外导出接口中，因此**无需保持与原 C 的 ABI 兼容**。rusl 内部可自由重新设计参数类型和调用约定。

#### Intent (意图描述)

对已分配的 `n` 字节内存块 `p` 进行高效的尾部清零。利用内核 mmap 页面初始即为零的语义特性，从块尾部向头部逐页"探测"：若连续整页已全为零，则跳过 memset，仅返回仍需清零的起始偏移量。

#### 前置条件 (Preconditions)

- `p` 指向一块有效的、可写入的 `n` 字节内存区域（来自 `malloc` 返回值或等同来源）
- `n >= 0`
- 调用者持有对 `p` 所指向内存的所有权，无并发写入者

#### 后置条件 (Postconditions)

**Case 1 — 正常路径 (n < 4096)**:
- 不做任何清零操作，原样返回 `n`
- 语义: 小块内存不满足优化阈值，交给调用者全量清零

**Case 2 — 大块优化路径 (n >= 4096)**:
- 从 `p + n` 向 `p` 方向逐页扫描
- 利用 `core::ptr::read_unaligned::<u64>` 读取每页首尾双字检测非零值（GCC 路径等价物）
- 非 GCC 场景降级为逐字节读取
- 返回值 `r`，满足 `0 <= r < 4096`
- **保证**: 从 `p + r` 到 `p + n` 的全部字节已被清零

#### 系统算法 (System Algorithm)

```
Input: p (*mut u8), n (usize), pagesz = 4096
Output: remaining_bytes (usize)

1. if n < pagesz → return n
2. pp = unsafe { p.add(n) }
3. i = (pp as usize) & (pagesz - 1)
4. loop:
   a. pp = memset(pp.sub(i), 0, i)  // 清零尾部片段
   b. if pp.offset_from(p) < pagesz → return pp.offset_from(p) as usize
   c. for i in (pagesz down to 0).step_by(2 * size_of::<T>()):
        if read_unaligned::<T>(pp.sub(size_of::<T>())) != 0
        or read_unaligned::<T>(pp.sub(2 * size_of::<T>())) != 0:
          break
   d. if i == 0: 整个页已为零，pp = pp.sub(pagesz)
      else: 进入下一次迭代清零该非零页
```

**类型 T 选择策略 (Rust 版本)**:
- 默认使用 `u64`，利用 `read_unaligned` 一次检查 8 或 16 字节
- 可使用编译时 `cfg` 条件选择最优访存宽度

#### 内部实现依赖

| 依赖 | 来源 | 说明 |
|------|------|------|
| `memset` | `rusl/src/string/memset.rs` | 显式字节清零（extern "C" 导入） |
| `core::ptr::read_unaligned` | `core` | 未对齐内存读取原语 (no_std) |

---

### allzerop (内部函数) / __malloc_allzerop (内部符号)

**[Visibility]**: Internal -- 默认实现始终返回 `0`（"非全零"）。在 rusl 中，由于不使用 musl 的弱符号覆盖机制，`__malloc_allzerop` 不再作为一个独立的外部符号存在。其功能合并到 `calloc` 内部的零检测逻辑中：rusl 内置的 malloc 实现直接通过内联检查或 trait 方法判断分配块是否已零。

**设计变更说明**: C 版本的 `weak_alias(allzerop, __malloc_allzerop)` 弱符号机制依赖 ELF 链接器对弱/强符号的覆盖语义，在纯静态 Rust 构建（`no_std` + 无动态链接器）环境下不可用。rusl 改用以下方案之一替代：

1. **方案 A (Trait 分发)**: 在 allocator trait 中定义 `fn is_all_zero(ptr: *const c_void) -> bool` 方法，默认实现返回 `false`
2. **方案 B (内联常量)**: rusl 内部始终使用自有 malloc，直接根据分配来源（brk vs mmap）在 `calloc` 内联判断

**前置条件**:
- `p` 是由 `malloc` 分配的有效指针

**后置条件**:
- 返回 `bool`：`false` 表示需要显式清零，`true` 表示已知全零可跳过

---

## Layer 2: 对外导出函数

---

### calloc (对外导出 -- C ABI)

**[Visibility]**: Public -- POSIX.1-2001 标准函数，`<stdlib.h>` 声明，用户程序可直接调用

**[Complexity]**: Level 2 -- 含溢出检测和零填充优化路径

#### C 原始签名

```c
void *calloc(size_t m, size_t n);
```

#### Rust 对外 ABI 签名

```rust
extern "C" fn calloc(m: usize, n: usize) -> *mut core::ffi::c_void;
```

#### Intent (意图描述)

分配一个包含 `m` 个元素、每个元素 `n` 字节的数组，并将分配的内存全部清零后返回指针。相比 `malloc(m*n) + memset`，`calloc` 有两项优势：

1. **乘法溢出检测**: 在分配前检查 `m * n` 是否溢出 `usize` 范围，溢出则返回 NULL 并设置 `errno = ENOMEM`
2. **零填充优化**: 对于大块内存，利用内核 mmap 页初始为零的特性，通过 `mal0_clear` 跳过已零页的显式清零

#### 前置条件 (Preconditions)

- `m` 和 `n` 为任意 `usize` 值（对应 C `size_t`）
- 无内部状态要求（线程安全，可重入）

#### 后置条件 (Postconditions)

**Case 1 -- 乘法溢出**:
- **触发条件**: `n != 0` 且 `m > usize::MAX / n`（即 `m * n > usize::MAX`）
- **效果**:
  - `errno = ENOMEM`
  - 返回 `core::ptr::null_mut()` (空指针)
  - 不分配任何内存
  - **保证**: 无内存泄漏

**Case 2 -- 底层分配失败**:
- **触发条件**: 乘法未溢出，但内部 `malloc(n)` 返回 `null_mut()`
- **效果**:
  - `errno` 由 `malloc` 设置 (通常为 `ENOMEM`)
  - 返回 `null_mut()`
  - 不执行清零操作

**Case 3 -- 分配成功且内存已全零**:
- **触发条件**: `malloc(n)` 成功 且 `!__malloc_replaced` 且 `is_all_zero(p) == true`
- **效果**:
  - 返回 `p`，指向 `n` 字节全零内存
  - 不执行额外的清零操作
  - **前提**: 仅当使用 rusl 内置 malloc（`__malloc_replaced == 0`）且底层分配器确认内存已零时

**Case 4 -- 分配成功但需显式清零**:
- **触发条件**: `malloc(n)` 成功 且 (用户替换了 malloc (`__malloc_replaced != 0`) 或 `is_all_zero(p) == false`)
- **效果**:
  - 调用 `mal0_clear(p, n)` 从尾部高效清零，获得剩余需清零前缀长度 `r`
  - 调用 `memset(p, 0, r)` 清零剩余前缀
  - 返回 `p`，指向 `n` 字节全零内存

#### 系统算法 (Rust 伪代码)

```rust
extern "C" fn calloc(m: usize, n: usize) -> *mut core::ffi::c_void {
    // Stage 1: Overflow detection
    if n != 0 && m > usize::MAX / n {
        unsafe { rusl_errno::set(ENOMEM); }
        return core::ptr::null_mut();
    }

    // Stage 2: Allocate
    let total = n.wrapping_mul(m);
    let p = unsafe { malloc(total) };
    if p.is_null() {
        return core::ptr::null_mut();
    }

    // Stage 3: Zero fill
    // 注: __malloc_replaced 在 rusl 中简化为常量或静态标志
    if !__malloc_replaced() && is_all_zero(p) {
        return p;
    }

    let r = mal0_clear(p, total);
    if r > 0 {
        unsafe { memset(p, 0, r) };
    }
    p
}
```

#### 外部依赖 (直接)

| 依赖 | 来源 | 角色 |
|------|------|------|
| `malloc` | `rusl/src/malloc/` (lite_malloc 或 mallocng 实现) | 底层原始内存分配 |
| `__malloc_replaced` | `rusl/src/malloc/replaced.rs` | 标记用户是否替换了 malloc 实现 |
| `memset` | `rusl/src/string/memset.rs` | 显式字节清零 (extern "C") |
| `mal0_clear` | 本模块 `pub(crate)` 内部函数 | 尾部高效清零 |
| `is_all_zero` / `allzerop` | 本模块 或 malloc trait 方法 | 检测分配块是否已全零 |
| `errno` / `ENOMEM` | rusl errno 模块 | 错误报告机制 |

#### 间接依赖 (通过 malloc)

| 间接依赖 | 来源 | 角色 |
|---------|------|------|
| `__lock` / `__unlock` | rusl 同步原语 | 分配器互斥锁 |
| `brk` syscall | 内联 asm / `core::arch` | 堆空间扩展 |
| `mmap` syscall | 内联 asm / `core::arch` | 匿名内存映射 |
| `libc.page_size` | rusl 运行时状态 | 页大小 |

#### 线程安全

- `calloc` 本身无内部静态状态
- 线程安全由底层 `malloc` 实现提供
- `errno` 的设置遵循 per-thread 语义

#### 与 malloc(0) 的兼容性

- 若 `m == 0 || n == 0`，则 `total = m * n = 0`
- 结果行为取决于底层 `malloc(0)` 的实现策略（返回 NULL 或唯一指针）
- 使用前检查返回值是否为 NULL 即可处理所有情况

---

## 递归依赖追踪汇总

以下是根据 C spec 依赖图递归追踪的所有符号，按层级汇总：

### Layer 0: 本模块符号

| 符号 | 可见性 | 类型 | Rust 设计策略 |
|------|--------|------|---------------|
| `calloc` | Public | extern "C" fn | ABI 兼容封装 |
| `mal0_clear` | Internal | pub(crate) fn | 安全 Rust 重设计，使用 `unsafe` 块明确标记危险操作 |
| `allzerop` | Internal | pub(crate) fn → Trait 方法 | 合并入 allocator trait 的 `is_all_zero` 方法 |

### Layer 1: 直接依赖

| 符号 | 来源 C 文件 | 可见性 | C spec 文件 |
|------|------------|--------|------------|
| `malloc` | `lite_malloc.c` / `mallocng/` | Public | `src/malloc/spec/lite_malloc.md` |
| `__malloc_replaced` | `replaced.c` | Internal | `src/malloc/spec/replaced.md` |
| `__malloc_allzerop` | `calloc.c` (weak_alias) | Internal | `src/malloc/spec/calloc.md` (same file) |
| `memset` | `string/memset.c` | Public | `src/string/spec/memset.md` |
| `ENOMEM` / `errno` | errno 机制 | Public (宏) | rusl 自建 errno 模块 |

### Layer 2: 间接依赖 (通过 malloc)

| 符号 | 来源 C 文件 | 可见性 |
|------|------------|--------|
| `__lock` / `__unlock` | `thread/__lock.c` | Internal |
| `__syscall` / `SYS_brk` | `internal/syscall.h` | Internal |
| `__mmap` | `mman/mmap.c` | Internal |
| `libc.page_size` | `internal/libc.h` (struct `__libc`) | Internal |
| `libc.auxv` | `internal/libc.h` | Internal |

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  extern "C" fn malloc(size: usize) -> *mut core::ffi::c_void;
                                  // 依赖1: 底层内存分配 (rusl/src/malloc/, 对外导出, 弱符号)
  extern "C" fn memset(dest: *mut core::ffi::c_void, c: core::ffi::c_int, n: usize) -> *mut core::ffi::c_void;
                                  // 依赖2: 内存设置函数 (rusl/src/string/memset.rs, 对外导出)
  __malloc_replaced: bool / AtomicBool;
                                  // 依赖3: malloc 替换检测标志 (rusl/src/malloc/replaced.rs, Internal)
                                  // rusl 中可简化为编译时常量或 AtomicBool
  is_all_zero(p: *const core::ffi::c_void) -> bool;
                                  // 依赖4: 零页检测 (本模块或 allocator trait, Internal)

Predefined Macros/Traits:
  core::ptr::read_unaligned::<T>; // 未对齐内存读取原语 (core, no_std)
  core::ptr::null_mut::<T>;       // 空指针常量
  usize::MAX;                     // 等价于 SIZE_MAX
  ENOMEM;                         // errno 常量 (rusl 自建)

[GUARANTEE]
Exported Interface:
  extern "C" fn calloc(m: usize, n: usize) -> *mut core::ffi::c_void;
                                  // 本模块保证对外提供的接口签名:
                                  // ABI 与 C 标准 calloc 完全兼容
                                  // 调用约定: extern "C"
                                  // 参数: m (元素个数), n (每元素字节数)
                                  // 返回: 零初始化内存指针, 失败返回 null_mut()
                                  // 前置: m, n 为任意 usize 值
                                  // 后置: 成功时返回全零内存, 溢出/失败时返回 null 并设 errno=ENOMEM

Internal Interface:
  pub(crate) fn mal0_clear(p: *mut u8, n: usize) -> usize;
                                  // 尾部高效清零辅助函数
                                  // 返回剩余需清零的前缀字节数
  pub(crate) fn allzerop(p: *const core::ffi::c_void) -> bool;
                                  // 默认零页检测 (始终返回 false)
                                  // 可被 allocator trait 实现覆盖
  pub(crate) fn __malloc_replaced() -> bool;
                                  // 获取 malloc 替换状态标志
                                  // rusl 默认实现始终返回 false