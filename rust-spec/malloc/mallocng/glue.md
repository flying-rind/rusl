# glue 模块规约 (rusl)

> **模块定位**: `glue` 模块是 rusl mallocng 分配器与 rusl 内部基础设施之间的**胶水适配层**，不包含任何对外导出的公共 API。其核心职责是：
> 1. 封装系统调用接口（brk/mmap/madvise/mremap/munmap/mprotect）
> 2. 提供统一的锁原语（基于 futex 的自旋锁，支持 atfork）
> 3. 提供线程安全检测、随机密钥生成等辅助基础设施
>
> **设计哲学**: 在 C 原版中，`glue.h` 通过大量 `#define` 宏实现命名空间重映射、系统调用封装和编译时常量配置。Rust 版本**不需要对应的命名空间重映射宏**——Rust 的模块系统通过 `use` 语句天然支持符号重命名，无需预处理器介入。本模块聚焦于类型安全的系统调用封装和锁抽象。

---

## 依赖图

```
glue 模块 (rusl)
├── 内部依赖 (来自 rusl 其他模块)
│   ├── crate::atomic          — 原子操作原语 (a_cas, a_ctz_32, a_or, a_crash, a_clz_32)
│   │   └── 等价于 C 的 atomic.h，提供 core::sync::atomic 抽象
│   ├── crate::syscall         — 系统调用宏/函数 (__syscall → syscall! 宏)
│   │   └── 等价于 C 的 syscall.h，通过 core::arch::asm! 实现
│   ├── crate::runtime         — rusl 全局运行时状态 (libc 等价物)
│   │   └── 提供 auxv、need_locks 等全局状态访问
│   ├── crate::lock            — 底层 futex 自旋锁原语 (__lock/__unlock)
│   │   └── 等价于 C 的 lock.h
│   └── crate::dynlink         — 动态链接替换标志 (__malloc_replaced 等)
│       └── 等价于 C 的 dynlink.h
│
├── 内部依赖 (来自 mallocng 其他模块)
│   ├── super::meta            — 数据结构定义 (struct malloc_context 等)
│   │   └── 等价于 C 的 meta.h
│   ├── super::malloc          — 提供 size_classes[], ctx, alloc_meta(), is_allzero()
│   │   └── 等价于 C 的 malloc.c
│   └── super::free            — 提供 free_group() 等内部函数
│       └── 等价于 C 的 free.c
│
└── 外部依赖 (core 库, no_std 兼容)
    ├── core::sync::atomic     — AtomicI32, AtomicBool, Ordering
    ├── core::arch::asm!       — 内联汇编，发起 syscall 指令
    ├── core::ffi::c_int       — C ABI 兼容类型
    └── core::ptr::addr_of!    — 获取栈地址 (用于随机密钥生成)
```

---

## 类型别名与常量（替代 C 宏的命名空间重映射）

在 C 原版中，以下 `#define` 宏用于将内部分配器符号映射到 musl 的 `__` 前缀命名空间。在 Rust 中，这些映射通过 **`use` 语句**在消费模块中实现，无需在此模块中定义。

| C 宏 (glue.h) | Rust 等价方式 | 说明 |
|---------------|---------------|------|
| `#define size_classes __malloc_size_classes` | `use super::malloc::size_classes;` | 模块级 use 别名 |
| `#define ctx __malloc_context` | `use super::malloc::ctx;` | 模块级 use 别名 |
| `#define alloc_meta __malloc_alloc_meta` | `use super::malloc::alloc_meta;` | 模块级 use 别名 |
| `#define is_allzero __malloc_allzerop` | `use super::malloc::is_allzero;` | 模块级 use 别名 |
| `#define dump_heap __dump_heap` | `use super::malloc::dump_heap;` | 调试用，feature-gated |
| `#define malloc __libc_malloc_impl` | 不在此模块处理 | 由公共入口层通过 `#[export_name]` 处理 |
| `#define realloc __libc_realloc` | 不在此模块处理 | 同上 |
| `#define free __libc_free` | 不在此模块处理 | 同上 |

> **设计说明**: Rust 通过属性宏 `#[export_name = "__libc_malloc_impl"]` 在函数定义处直接指定导出符号名，无需通过模块层重映射。这使得 `glue` 模块的职责更加纯粹——仅提供基础设施，不参与符号导出策略。

---

## 系统调用封装函数

在 C 原版中，`brk`、`mmap`、`madvise`、`mremap` 通过 `#define` 宏映射到 musl 内部符号。在 Rust 中，这些变为直接通过 `core::arch::asm!` 发起 syscall 的 `unsafe fn`。

### brk

```rust
/// 扩展进程堆 (brk) 区域。
///
/// 通过 SYS_brk 系统调用设置新的 program break 地址。
/// 返回值: 成功时返回新的 program break 地址，失败时返回不等于 `p` 的值（旧 break 值）。
///
/// # Safety
/// - `p` 必须是有效的内存地址或 0（查询当前 brk）
/// - 调用者负责处理返回值以判断成功/失败
pub(crate) unsafe fn brk(p: usize) -> usize;
```

[Visibility]: Internal -- rusl mallocng 内部系统调用封装

| 项目 | 描述 |
|------|------|
| **前置条件** | `p` 为 0（查询）或合法的新 brk 地址 |
| **后置条件 (Case 1)** | 成功时返回新的 program break 地址 |
| **后置条件 (Case 2)** | 失败时返回旧 break 值（不等于 `p`），由调用者 `alloc_meta()` 处理 |
| **Intent** | 通过 `syscall!(SYS_brk, p)` 内联汇编发起系统调用，用于扩展 meta_area 页面 |
| **调用上下文** | 仅在 `alloc_meta()` 中被调用 |

**Rust 设计要点**:
- 使用 `usize` 替代 C 的 `uintptr_t`，Rust 原生指针宽度类型
- 通过 `crate::syscall::syscall!` 宏封装 `core::arch::asm!`
- 返回 `usize` 而非 C 宏表达式，类型安全

---

### mmap

```rust
/// 内存映射系统调用封装。
///
/// # Safety
/// - 参数语义与 POSIX mmap 一致
/// - 返回的指针可能为 `core::ptr::null_mut()` 表示 MAP_FAILED
pub(crate) unsafe fn mmap(
    addr: *mut core::ffi::c_void,
    length: usize,
    prot: c_int,
    flags: c_int,
    fd: c_int,
    offset: off_t,
) -> *mut core::ffi::c_void;
```

[Visibility]: Internal -- rusl mallocng 内部系统调用封装

| 项目 | 描述 |
|------|------|
| **前置条件** | 参数语义与 POSIX `mmap` 一致 |
| **后置条件** | 成功返回映射区域指针，失败返回 `null_mut()` |
| **Intent** | 通过 `syscall!(SYS_mmap, ...)` 内联汇编直接发起 mmap 系统调用，不经过任何外部 libc FFI |

**Rust 设计要点**:
- 参数使用 Rust 原生类型：`*mut c_void`、`usize`、`c_int`、`off_t`
- 返回 `*mut c_void` 而非 `void*`，利用 Rust 的可空指针优化
- 不使用 libc crate 的类型定义

---

### madvise

```rust
/// 内存建议系统调用封装。
///
/// # Safety
/// - 参数语义与 POSIX madvise 一致
pub(crate) unsafe fn madvise(addr: *mut core::ffi::c_void, length: usize, advice: c_int) -> c_int;
```

[Visibility]: Internal -- rusl mallocng 内部系统调用封装

| 项目 | 描述 |
|------|------|
| **前置条件** | `addr` 页对齐，`length` > 0 |
| **后置条件** | 成功返回 0，失败返回 -1（errno 由 syscall 层设置） |
| **Intent** | 通过 `syscall!(SYS_madvise, ...)` 直接发起系统调用 |

---

### mremap

```rust
/// 内存重映射系统调用封装（Linux 特定）。
///
/// # Safety
/// - 参数语义与 Linux mremap 一致
pub(crate) unsafe fn mremap(
    old_addr: *mut core::ffi::c_void,
    old_len: usize,
    new_len: usize,
    flags: c_int,
    new_addr: *mut core::ffi::c_void,
) -> *mut core::ffi::c_void;
```

[Visibility]: Internal -- rusl mallocng 内部系统调用封装

| 项目 | 描述 |
|------|------|
| **前置条件** | 参数语义与 Linux `mremap` 一致 |
| **后置条件** | 成功返回新映射地址，失败返回 `null_mut()` (MAP_FAILED) |
| **Intent** | 通过 `syscall!(SYS_mremap, ...)` 直接发起系统调用，用于 realloc 的大块重映射优化 |

---

### munmap

```rust
/// 解除内存映射系统调用封装。
///
/// # Safety
/// - 参数语义与 POSIX munmap 一致
pub(crate) unsafe fn munmap(addr: *mut core::ffi::c_void, length: usize) -> c_int;
```

[Visibility]: Internal -- rusl mallocng 内部系统调用封装

| 项目 | 描述 |
|------|------|
| **前置条件** | `addr` 页对齐，`length` > 0 |
| **后置条件** | 成功返回 0，失败返回 -1 |
| **Intent** | 通过 `syscall!(SYS_munmap, ...)` 直接发起系统调用，用于释放 mmap 分配的内存 |

---

### mprotect

```rust
/// 内存保护系统调用封装。
///
/// # Safety
/// - 参数语义与 POSIX mprotect 一致
pub(crate) unsafe fn mprotect(addr: *mut core::ffi::c_void, length: usize, prot: c_int) -> c_int;
```

[Visibility]: Internal -- rusl mallocng 内部系统调用封装

| 项目 | 描述 |
|------|------|
| **前置条件** | `addr` 页对齐 |
| **后置条件** | 成功返回 0，失败返回 -1 |
| **Intent** | 通过 `syscall!(SYS_mprotect, ...)` 直接发起系统调用，用于设置 meta_area 保护页 (PROT_NONE) |

---

## 运行时配置常量

### USE_MADV_FREE

```rust
/// 控制 `free()` 中是否使用 `MADV_FREE` 归还物理页面。
///
/// 设为 `false` 时禁用 MADV_FREE（保守策略，页面立即可被内核回收统计计数）。
/// 设为 `true` 时在 `free()` 的 madvise 路径中优先使用 `MADV_FREE`（延迟回收，性能更优但 RSS 统计不精确）。
pub(crate) const USE_MADV_FREE: bool = false;
```

[Visibility]: Internal -- 编译时常量

---

### DISABLE_ALIGNED_ALLOC

```rust
/// 运行时条件判断：当用户替换了 `malloc` 但未替换 `aligned_alloc` 时返回 `true`。
///
/// 此时 `aligned_alloc()` 应返回 `ENOMEM` 以保持一致性。
///
/// # Panics / Requirements
/// - 依赖 `crate::dynlink::malloc_replaced()` 和 `crate::dynlink::aligned_alloc_replaced()` 的返回值
pub(crate) fn disable_aligned_alloc() -> bool;
```

[Visibility]: Internal -- 运行时条件函数

| 项目 | 描述 |
|------|------|
| **前置条件** | 动态链接器已完成初始化（`__malloc_replaced` / `__aligned_alloc_replaced` 已确定） |
| **后置条件** | 返回 `malloc_replaced() && !aligned_alloc_replaced()` |
| **Intent** | 防止在交叉替换场景下的不一致行为 |

**Rust 设计要点**:
- C 的 `#define DISABLE_ALIGNED_ALLOC (__malloc_replaced && !__aligned_alloc_replaced)` 是一个宏，在 Rust 中改为 `fn`，允许运行时求值
- 通过 `crate::dynlink` 模块提供原子读取的全局标志访问

---

### is_mt (原 MT 宏)

```rust
/// 运行时检测是否需要加锁。
///
/// 当进程为单线程时（`need_locks == false`），返回 `false`，跳过所有锁操作以提升性能。
pub(crate) fn is_mt() -> bool;
```

[Visibility]: Internal -- 线程安全检测函数

| 项目 | 描述 |
|------|------|
| **Intent** | 等价于 C 的 `MT` 宏 (`libc.need_locks`)，但改为函数以适配 Rust 的模块化设计 |
| **使用场景** | 在 `rdlock()` / `wrlock()` / `unlock()` 等所有锁操作路径中使用 |

**Rust 设计要点**:
- C 的 `#define MT (libc.need_locks)` 依赖全局变量直接访问
- Rust 中通过 `crate::runtime::need_locks()` 函数封装，提供更好的封装性和可测试性

---

### RDLOCK_IS_EXCLUSIVE

```rust
/// 锁语义配置：当为 `true` 时，读锁和写锁使用相同的互斥锁（无读写区分，都是排他锁）。
pub(crate) const RDLOCK_IS_EXCLUSIVE: bool = true;
```

[Visibility]: Internal -- 锁语义配置常量

| 项目 | 描述 |
|------|------|
| **Intent** | 简化锁语义。在 malloc 场景下，读写者并无真正的并发收益 |
| **使用位置** | `malloc()` 函数 fast-path 中，若 `RDLOCK_IS_EXCLUSIVE` 则直接本地更新 `avail_mask` |

---

## 断言行为

在 C 原版中，`assert(x)` 通过条件编译在 `a_crash()` 和标准 `assert.h` 之间选择。在 Rust 中：

```rust
/// mallocng 内部一致性检查断言。
///
/// 默认行为（非 test 模式）：断言失败时调用 `crate::atomic::crash()` 直接终止进程。
/// 此行为不受 `debug_assert!` 的 `debug-only` 限制——分配器内部的不变式违反意味着堆损坏。
///
/// 在 test 模式下：使用标准 `assert!`，可通过 `#[cfg(test)]` 控制行为。
macro_rules! malloc_assert {
    ($cond:expr) => {
        if cfg!(test) {
            assert!($cond);
        } else {
            if !$cond {
                crate::atomic::crash();
            }
        }
    };
    ($cond:expr, $($arg:tt)*) => {
        if cfg!(test) {
            assert!($cond, $($arg)*);
        } else {
            if !$cond {
                crate::atomic::crash();
            }
        }
    };
}
```

[Visibility]: Internal -- 断言行为配置

| 条件 | 行为 |
|------|------|
| `cfg(test)` | 使用标准 `assert!`（受 `debug_assertions` 或 test 配置控制） |
| 非 test（默认） | 断言失败时调用 `crate::atomic::crash()` 直接终止进程，不受 `debug_assertions` 影响 |

**Rust 设计要点**:
- Rust 的 `assert!` 在 release 构建下被优化掉（除非 `debug_assertions` 启用）
- mallocng 的断言不能依赖 `debug_assertions`，因此在 release 构建中使用 `crash()` 保证检查生效
- 使用 `macro_rules!` 而非函数，以保留源位置信息（通过 `file!()` / `line!()` 可在 crash 时报告位置）

---

## 页大小常量

```rust
/// 系统页大小常量。
///
/// 若编译时可确定（通过 target 配置），使用编译常量；
/// 否则在运行时从 `ctx.pagesize` 读取。
#[cfg(target_os = "linux")]
pub(crate) const PAGE_SIZE: usize = 4096;

/// 运行时页大小获取函数（用于编译时无法确定页大小的架构）。
///
/// 从 `ctx.pagesize` 读取，该值在分配器初始化阶段由 auxv 获取。
pub(crate) fn page_size() -> usize;
```

[Visibility]: Internal -- 页大小定义

| 项目 | 描述 |
|------|------|
| **Intent** | 替代 C 的 `#define PAGESIZE` 回退逻辑 |
| **编译时路径** | `PAGE_SIZE` 常量 = 4096（Linux x86-64 / aarch64 几乎总是 4K） |
| **运行时路径** | `page_size()` 在编译时无法确定页大小时使用（如某些 RISC-V 变体） |

---

## 锁类型与实现

在 C 原版中，锁通过 `LOCK_OBJ_DEF` 宏展开生成：
1. `__malloc_atfork()` 全局函数
2. `__malloc_lock[1]` 全局 int 数组

加上 `rdlock()`/`wrlock()`/`unlock()`/`upgradelock()`/`resetlock()` 内联函数。

在 Rust 中，这些被统一设计为一个 **MallocLock** 类型：

### MallocLock

```rust
/// musl mallocng 分配器的全局互斥锁。
///
/// 基于 futex 的自旋锁实现。在单线程模式下（`is_mt() == false`），所有锁操作退化为空操作。
///
/// 字段 `lock` 为 `AtomicI32`，使用 futex(2) 系统调用实现阻塞等待。
/// `0` = 未锁定，`1` = 已锁定（有竞争时通过 futex 等待）。
pub(crate) struct MallocLock {
    lock: core::sync::atomic::AtomicI32,
}

impl MallocLock {
    /// 创建初始未锁定状态的锁实例。
    pub(crate) const fn new() -> Self;

    /// 读锁（排他锁，与写锁实现相同）。
    ///
    /// 多线程模式下获取排他锁，单线程模式下为空操作。
    pub(crate) fn rdlock(&self);

    /// 写锁（排他锁，与读锁实现相同）。
    ///
    /// 多线程模式下获取排他锁，单线程模式下为空操作。
    pub(crate) fn wrlock(&self);

    /// 释放锁。
    ///
    /// 多线程模式下释放排他锁，单线程模式下为空操作。
    ///
    /// # Safety
    /// - 必须在持有锁的同一线程中调用
    pub(crate) fn unlock(&self);

    /// 锁升级（当前为空操作）。
    ///
    /// 保留接口以备将来区分读写锁实现。
    /// 由于 `RDLOCK_IS_EXCLUSIVE=true`，读锁已是排他的，无需升级。
    pub(crate) fn upgradelock(&self);

    /// 重置锁状态，将锁强制归零。
    ///
    /// 仅在 `fork()` 后的子进程中调用（单线程上下文，父进程的锁状态无效）。
    ///
    /// # Safety
    /// - 必须在确认单线程环境（子进程 fork 后）中调用
    pub(crate) unsafe fn resetlock(&self);

    /// atfork 回调处理。
    ///
    /// 根据 `who` 参数执行相应操作：
    /// - `who < 0` (prepare): 获取锁，阻止其他线程在 fork 期间修改堆
    /// - `who == 0` (parent): 释放 prepare 阶段获取的锁
    /// - `who > 0` (child): 强制重置锁状态
    pub(crate) fn atfork(&self, who: c_int);
}
```

[Visibility]: Internal -- rusl mallocng 内部锁类型

**Rust 设计要点**:
- 使用 `AtomicI32` 替代 C 的 `volatile int[1]`，通过 `Ordering::Acquire` / `Ordering::Release` 保证内存序
- 将 C 的独立全局函数（`rdlock`、`wrlock`、`unlock` 等）封装为 `impl MallocLock` 方法，增强内聚性
- futex 等待/唤醒通过 `crate::syscall` 模块封装的 `futex_wait` / `futex_wake` 实现
- 单线程模式优化：方法内部检查 `is_mt()`，若为单线程则跳过原子操作
- `resetlock` 标记为 `unsafe`，因为它在语义上绕过了锁协议（仅 fork 子进程可调用）

---

## 全局锁实例

```rust
/// musl mallocng 全局互斥锁实例。
///
/// 由 `MallocLock::new()` 初始化，存储为模块级静态变量。
/// 使用 `#[link_section]` 或 `#[export_name]` 确保 C ABI 兼容的可见性。
pub(crate) static MALLOC_LOCK: MallocLock = MallocLock::new();
```

[Visibility]: Internal -- 模块级静态变量

| 项目 | 描述 |
|------|------|
| **类型** | `MallocLock` |
| **Intent** | 等价于 C 的 `int __malloc_lock[1]`，但类型安全 |
| **跨模块可见性** | `pub(crate)`：整个 rusl mallocng 子系统内可见 |

**Rust 设计要点**:
- 使用 `static` 而非 `static mut`——`MallocLock` 内部使用 `AtomicI32`，通过 `&self`（不可变引用）安全地进行内部可变性操作
- 避免了对 `unsafe` 块的依赖，符合 Rust 安全编码原则

---

## atfork 对外回调函数

在 C 原版中，`LOCK_OBJ_DEF` 宏展开生成全局函数 `__malloc_atfork()`。在 Rust 中，该函数需保持与 C ABI 兼容的导出签名，因为 musl 的 `pthread_atfork()` 机制通过符号名查找此回调。

```rust
/// musl atfork 回调函数。由 `pthread_atfork()` 机制在 `fork()` 前后调用。
///
/// 此函数必须保持 C ABI 兼容签名，因为外部 C 代码通过符号名调用。
/// 内部委托给 `MALLOC_LOCK.atfork(who)`。
///
/// # C ABI 兼容性
/// - 函数签名: `extern "C" fn(c_int)`
/// - 参数语义: `who < 0` = prepare, `who == 0` = parent, `who > 0` = child
#[no_mangle]
extern "C" fn __malloc_atfork(who: c_int) {
    MALLOC_LOCK.atfork(who);
}
```

[Visibility]: Internal -- musl 内部 atfork 回调，通过 `#[no_mangle]` 导出为 C 兼容符号

| 项目 | 描述 |
|------|------|
| **导出符号名** | `__malloc_atfork` |
| **Intent** | 保持与 musl `pthread_atfork()` 的 ABI 兼容性 |
| **实现** | 委托给 `MALLOC_LOCK.atfork(who)` |

**Rust 设计要点**:
- 这是 glue 模块中**唯一需要 `extern "C"` 导出的符号**
- 使用 `#[no_mangle]` 而非 `#[export_name]`，因为函数名恰好就是所需的符号名
- 函数体为薄封装，实际逻辑在 `MallocLock::atfork()` 中

---

## 随机密钥生成

### get_random_secret

```rust
/// 为分配器生成一个进程生命期内**固定的随机密钥**。
///
/// 用于 `meta_area.check` 字段，防止元数据伪造攻击。
///
/// # Requirements
/// - `auxv` 必须已初始化（动态链接器设置的辅助向量可通过 `crate::runtime::auxv()` 访问）
///
/// # Returns
/// - 一个 64 位无符号随机值，在进程生命期内保持不变
pub(crate) fn get_random_secret() -> u64;
```

[Visibility]: Internal -- rusl mallocng 内部辅助函数

| 项目 | 描述 |
|------|------|
| **前置条件** | `crate::runtime::auxv()` 已初始化 |
| **后置条件** | 返回一个 64 位无符号随机值 |
| **Intent** | 结合 ASLR 栈地址和内核随机种子两个熵源，降低可预测性风险 |

**系统算法** (分两步混合):

**Step 1 — 栈地址熵源**:
```rust
let stack_var: u64 = 0;
let stack_addr = core::ptr::addr_of!(stack_var) as u64;
let mut secret = stack_addr.wrapping_mul(1103515245); // 经典 LCG 乘数
```

**Step 2 — 内核随机种子熵源**:
```rust
// 遍历 auxv 查找 AT_RANDOM 条目
if let Some(random_seed) = crate::runtime::auxv().find(AT_RANDOM) {
    // 读取内核提供的 16 字节随机种子中的高 8 字节
    let kernel_secret = unsafe {
        core::ptr::read_unaligned(random_seed as *const u64)
    };
    secret = kernel_secret; // 直接覆盖（内核熵源质量更高）
}
```

**Rust 设计要点**:
- 使用 `core::ptr::addr_of!` 获取栈变量地址，避免创建引用
- 通过 `crate::runtime::auxv()` 封装对 AT_RANDOM 的访问
- 不使用 `memcpy`（C 原版的 `memcpy` 在 Rust 中由 `core::ptr::read_unaligned` 替代）
- 调用者：仅在 `alloc_meta()` 初始化路径中被调用一次，结果存入 `ctx.secret`

---

## 跨模块依赖汇总

| 依赖符号 | 来源模块 | 类别 | Rust 等价物 |
|----------|----------|------|-------------|
| `syscall!` | `crate::syscall` | 系统调用层 | `macro_rules! syscall!` 通过 `core::arch::asm!` |
| `futex_wait` / `futex_wake` | `crate::syscall` | futex 封装 | 对 `SYS_futex` 的薄封装 |
| `a_cas` / `a_ctz_32` / `a_or` / `a_crash` / `a_clz_32` | `crate::atomic` | 原子操作层 | `AtomicI32::compare_exchange` / `u32::trailing_zeros` / `AtomicI32::fetch_or` 等 |
| `need_locks()` / `auxv()` | `crate::runtime` | rusl 全局运行时状态 | 函数封装替代全局变量直接访问 |
| `malloc_replaced()` / `aligned_alloc_replaced()` | `crate::dynlink` | 动态链接替换标志 | 原子读取的函数封装 |
| `size_classes[]` | `super::malloc` | 大小类别表 | 常量数组 |
| `struct malloc_context` / `ctx` | `super::meta` / `super::malloc` | 全局分配器上下文 | 类型定义 + 全局实例 |
| `alloc_meta()` | `super::malloc` | 元数据分配函数 | `pub(crate) fn alloc_meta()` |
| `MallocLock` / `MALLOC_LOCK` | 本模块 | 锁类型与实例 | 取代 C 的分散锁原语 |
| `get_random_secret()` | 本模块 | 随机密钥生成 | 取代 C 的内联函数 |

---

## 不变式 (Invariants)

### INV-LOCK-01: 锁配对不变量

`MallocLock` 的任何操作（`rdlock`/`wrlock` 与 `unlock`）必须成对出现。每个 `rdlock()`/`wrlock()` 必须有对应的 `unlock()`。在任何 `fork()` 子进程中，`resetlock()` 必须在首次锁操作前被调用。

### INV-SECRET-01: 安全不变量

`ctx.secret` 在进程生命期内保持不变，且 `meta_area.check` 必须始终等于 `ctx.secret`。此不变量由 `get_meta()` 中的断言检查保证。

### INV-INIT-01: 初始化顺序不变量

`get_random_secret()` 必须在任何 `alloc_meta()` 调用后被调用，而 `alloc_meta()` 的使用必须发生在任何 `malloc()` / `free()` / `realloc()` 操作之前。该不变量由 `ctx.init_done` 标志 + `alloc_meta()` 中的惰性初始化保证。

### INV-LOCK-02: 线程安全不变量

任何修改 `ctx` 全局状态的操作必须在持有 `MALLOC_LOCK` 时进行。fast-path 中的 `avail_mask` CAS 操作是唯一例外（原子操作隐含的锁自由语义）。

### INV-LOCK-03: 零大小锁不变量

`sizeof(MallocLock) == sizeof(AtomicI32)` (4 字节)，与 C 原版的 `int[1]` 大小一致，确保 ABI 兼容性。

---

## 符号导出状态总览

| 符号 | 可见性 | 说明 |
|------|--------|------|
| `brk()` | `pub(crate)` | 模块内部可见 |
| `mmap()` | `pub(crate)` | 模块内部可见 |
| `madvise()` | `pub(crate)` | 模块内部可见 |
| `mremap()` | `pub(crate)` | 模块内部可见 |
| `munmap()` | `pub(crate)` | 模块内部可见 |
| `mprotect()` | `pub(crate)` | 模块内部可见 |
| `USE_MADV_FREE` | `pub(crate)` | 模块内部可见常量 |
| `disable_aligned_alloc()` | `pub(crate)` | 模块内部可见函数 |
| `is_mt()` | `pub(crate)` | 模块内部可见函数 |
| `RDLOCK_IS_EXCLUSIVE` | `pub(crate)` | 模块内部可见常量 |
| `PAGE_SIZE` | `pub(crate)` | 模块内部可见常量 |
| `page_size()` | `pub(crate)` | 模块内部可见函数 |
| `malloc_assert!` | `pub(crate)` | 模块内部宏 |
| `MallocLock` | `pub(crate)` | 模块内部可见类型 |
| `MALLOC_LOCK` | `pub(crate)` | 模块内部可见静态实例 |
| `__malloc_atfork` | `#[no_mangle] extern "C"` | **对外导出** (C ABI 兼容) -- 供 `pthread_atfork` 回调 |
| `get_random_secret()` | `pub(crate)` | 模块内部可见函数 |

---

## C spec 对照表

以下列出 C spec (glue.h) 中每个符号对应的 Rust 设计，标注哪些因模块系统自然消解而无需对应的 Rust 构造：

| C spec 符号 | Rust 设计 | 变更说明 |
|-------------|-----------|----------|
| `#define size_classes __malloc_size_classes` | 无对应（由消费模块的 `use` 语句替代） | Rust 模块系统天然支持 |
| `#define ctx __malloc_context` | 无对应 | 同上 |
| `#define alloc_meta __malloc_alloc_meta` | 无对应 | 同上 |
| `#define is_allzero __malloc_allzerop` | 无对应 | 同上 |
| `#define dump_heap __dump_heap` | 无对应 | 同上 |
| `#define malloc / realloc / free` | 无对应（由 `#[export_name]` 在定义处处理） | Rust 属性替代宏 |
| `#define brk(p)` | `pub(crate) unsafe fn brk(p: usize) -> usize` | 宏变函数，类型安全 |
| `#define mmap / madvise / mremap` | `pub(crate) unsafe fn mmap(...)` 等 | 宏变函数，类型安全 |
| `#define USE_MADV_FREE 0` | `pub(crate) const USE_MADV_FREE: bool = false` | 宏变常量 |
| `#define DISABLE_ALIGNED_ALLOC` | `pub(crate) fn disable_aligned_alloc() -> bool` | 宏变函数 |
| `#define MT (libc.need_locks)` | `pub(crate) fn is_mt() -> bool` | 宏变函数，封装全局状态 |
| `#define RDLOCK_IS_EXCLUSIVE 1` | `pub(crate) const RDLOCK_IS_EXCLUSIVE: bool = true` | 宏变常量 |
| `#define assert(x)` | `macro_rules! malloc_assert` | 宏变宏（保留宏形式以获取源位置） |
| `#define PAGESIZE PAGE_SIZE` | `pub(crate) const PAGE_SIZE: usize` + `pub(crate) fn page_size() -> usize` | 宏变常量+函数 |
| `#define LOCK_OBJ_DEF` | `static MALLOC_LOCK: MallocLock` + `impl MallocLock` | 宏展开变为类型定义 |
| `__malloc_lock[1]` (extern) | `static MALLOC_LOCK: MallocLock` | 全局变量变为静态实例 |
| `get_random_secret()` (static inline) | `pub(crate) fn get_random_secret() -> u64` | inline 函数变普通函数 |
| `rdlock()` / `wrlock()` (static inline) | `MallocLock::rdlock()` / `MallocLock::wrlock()` | 独立函数变为方法 |
| `unlock()` (static inline) | `MallocLock::unlock()` | 独立函数变为方法 |
| `upgradelock()` (static inline) | `MallocLock::upgradelock()` | 独立函数变为方法 |
| `resetlock()` (static inline) | `MallocLock::resetlock()` (unsafe) | 独立函数变为 unsafe 方法 |
| `malloc_atfork()` (static inline) | `MallocLock::atfork()` + `extern "C" fn __malloc_atfork()` | 分拆为类型方法 + ABI 导出桩 |
| `__malloc_atfork()` (LOCK_OBJ_DEF 生成) | `extern "C" fn __malloc_atfork()` | 保持 C ABI 兼容 |

---

## 递归依赖终止说明

本模块的递归依赖向上追踪终止于以下节点：

1. **`crate::syscall`**: 通过 `core::arch::asm!` 直接发起 Linux syscall。不依赖任何外部 libc。其自身 spec 应在 `src/internal/rust-spec/syscall.md` 中描述。

2. **`crate::atomic`**: 封装 `core::sync::atomic` 提供原子操作。其自身 spec 应在 `src/internal/rust-spec/atomic.md` 中描述。

3. **`crate::runtime`**: 管理 rusl 全局运行时状态（`auxv`、`need_locks` 等）。其自身 spec 应在 `src/internal/rust-spec/runtime.md` 中描述。

4. **`crate::lock`**: 提供底层 futex 自旋锁原语（`__lock`/`__unlock` 的 Rust 等价物）。其自身 spec 应在 `src/internal/rust-spec/lock.md` 中描述。

5. **`crate::dynlink`**: 提供动态链接符号替换检测标志。其自身 spec 应在 `src/ldso/rust-spec/dynlink.md` 中描述。

6. **`super::meta`**: 定义 `struct malloc_context`、`struct meta`、`struct group` 等核心数据结构。其自身 spec 见 `src/malloc/mallocng/rust-spec/meta.md`。

7. **`super::malloc`**: 定义 `size_classes[]`、`ctx`、`alloc_meta()` 等全局符号。其自身 spec 见 `src/malloc/mallocng/rust-spec/malloc.md`。

8. **`super::free`**: 提供 `free_group()` 等内部函数。其自身 spec 见 `src/malloc/mallocng/rust-spec/free.md`。