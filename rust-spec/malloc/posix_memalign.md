# posix_memalign — Rust 接口归约

## 原始 C 接口

```c
int posix_memalign(void **res, size_t align, size_t len);
```

[Visibility]: Public — POSIX.1-2001 标准函数，`<stdlib.h>` 声明

---

## Rust 外部 ABI 接口

```rust
// [Visibility]: External — 对外导出，必须保持 C ABI 兼容
#[no_mangle]
pub unsafe extern "C" fn posix_memalign(
    res: *mut *mut core::ffi::c_void,
    align: core::ffi::c_ulong,
    len: core::ffi::c_ulong,
) -> core::ffi::c_int;
```

**ABI 兼容性说明**: 参数类型 `*mut *mut c_void`、`c_ulong` 以及返回值类型 `c_int` 在 x86_64 Linux ABI 上与 C 的 `void **`、`size_t`、`int` 完全兼容。调用约定使用 `extern "C"` 匹配。

---

## Rust 内部安全接口设计

虽然 `posix_memalign` 自身是对外导出的 `unsafe extern "C"` 包装函数，但其内部逻辑全部委托给私有内部函数，内部函数使用安全的 Rust 抽象设计。

### 内部依赖层次（递归追踪）

本模块依赖以下内部符号（按层次组织）：

```
posix_memalign (extern "C" 对外导出)

  └── aligned_alloc_inner() — 内部 Rust 对齐分配引擎
       [模块: mallocng::aligned_alloc, 可见性: pub(crate)]

       ├── malloc_inner() — 内部 Rust 基础分配器
       │    [模块: mallocng::malloc, 可见性: pub(crate)]
       │
       ├── 错误类型: AlignedAllocError
       │    [模块: mallocng::aligned_alloc, 可见性: pub(crate)]
       │    enum AlignedAllocError { InvalidAlignment, OutOfMemory }
       │
       ├── 常量: UNIT = 16, IB = 4
       │    [模块: mallocng::meta, 可见性: pub(crate)]
       │
       ├── 常量: SIZE_MAX
       │    [来源: core::usize::MAX]
       │
       ├── 内部结构体: struct Meta, struct Group
       │    [模块: mallocng::meta, 可见性: pub(crate)]
       │
       ├── 内部函数: get_meta(), get_slot_index(), get_stride(), set_size()
       │    [模块: mallocng::meta, 可见性: pub(crate)]
       │
       ├── 内部查找表: SIZE_CLASSES
       │    [模块: mallocng::malloc, 可见性: pub(crate)]
       │
       ├── 替换检测标志: MALLOC_REPLACED, ALIGNED_ALLOC_REPLACED
       │    [模块: mallocng::replaced, 可见性: pub(crate)]
       │    [类型: core::sync::atomic::AtomicBool]
       │
       └── 系统调用: mmap() (rusl 内部 syscall 封装)
            [模块: syscall, 可见性: pub(crate)]
            [实现: 通过 asm! 内联汇编直接发起 SYS_mmap]

  ├── 错误常量: EINVAL = 22, ENOMEM = 12
  │    [模块: errno, 可见性: pub(crate)]
  │
  └── 线程本地 errno 存储 (用于外部 C ABI 兼容)
       [模块: errno, 可见性: pub(crate)]
       [实现: #[thread_local] static mut ERRNO: c_int 或等效机制]
```

### 递归依赖终止说明

以下依赖在递归追踪中终止：

- **malloc_inner()** — 内部 Rust 分配器引擎，其完整依赖链包括：
  - `alloc_meta()` — 元数据分配（依赖 mmap/brk/mprotect 系统调用）
  - `size_to_class()` / `size_overflows()` — 大小类别计算
  - `try_avail()` / `alloc_slot()` / `alloc_group()` — slot 分配策略
  - `enframe()` / `set_size()` — 帧初始化和头部编码
  - `a_cas()` / `a_ctz_32()` — 原子操作（使用 `core::sync::atomic` 或自定义实现）
  - 锁原语 `rdlock()` / `wrlock()` / `unlock()` — 基于 Rust 内部 Mutex
  - `get_random_secret()` — 安全随机密钥生成（读 auxv + 栈地址混合）
  - 系统调用: `mmap`、`brk`、`mprotect`、`munmap`、`madvise`
  - **终止**: `malloc_inner` 的完整规约在 `src/malloc/mallocng/rust-spec/malloc.md` 中独立描述

- **mmap 等系统调用** — rusl 必须直接通过 `asm!` 发起，不得使用 `libc` crate
  - **终止**: 系统调用层规约在 rusl 的 `syscall` 模块中独立描述

- **EINVAL / ENOMEM** — POSIX 错误码，定义为本模块级 `const`，与 Linux ABI 硬编码值一致

- **线程本地 errno** — 用于 `extern "C"` 函数间的错误传递约定
  - **终止**: errno 机制规约在 `src/errno` 的 spec 中独立描述

---

## 意图（Intent）

提供一个符合 POSIX 标准的对齐内存分配接口，与 C 实现语义完全相同：

- `aligned_alloc` (C11) 返回 `void *`，失败返回 NULL 并设置 `errno`
- `posix_memalign` 通过输出参数 `res` 返回指针，以**返回值**直接传递错误码（POSIX 惯例），不设置 `errno`

Rust 内部实现策略：所有参数校验与分配逻辑委托给内部 `aligned_alloc_inner()`（返回 `Result<NonNull<u8>, AlignedAllocError>`），`posix_memalign` 仅负责：
1. 参数转发
2. `align < sizeof(void *)` 的快速路径校验
3. `Result` 到 C 错误码的格式转换

---

## 前置条件（Preconditions）

| 条件 | 描述 |
|------|------|
| P1 | `res` 必须为非 NULL 的有效指针，指向一个 `*mut c_void` 类型的可写内存位置 |
| P2 | `align` 和 `len` 可以为任意 `size_t` 值（包括 0）；参数合法性由本函数及内部 `aligned_alloc_inner` 校验 |
| P3 | 无外部锁或全局状态依赖；函数为线程安全（依赖于底层分配器的线程安全性） |
| P4 | `rusl` 为 `#![no_std]` 环境，所有内部依赖均不涉及 `std` 或 `libc` crate |

---

## 后置条件（Postconditions）

### Case 1：分配成功（返回值 = 0）

| 条件 | 描述 |
|------|------|
| Q1.1 | `res` 解引用后指向一块大小为 `len` 字节的对齐内存区域 |
| Q1.2 | 返回地址满足对齐要求：`(*res) % align == 0`，且 `align >= sizeof(*mut c_void)` 且 `align` 为 2 的幂 |
| Q1.3 | 分配的内存未初始化（内容不确定） |
| Q1.4 | 分配的内存可安全读写 `len` 字节 |
| Q1.5 | 可通过 `free(*res)` 释放（与 `malloc`/`aligned_alloc` 共享同一堆） |

### Case 2：分配失败（返回值 != 0）

| 条件 | 描述 |
|------|------|
| Q2.1 | `res` 解引用的值**未被修改**（保持调用前的值） |
| Q2.2 | 无内存被分配，无堆状态变更 |
| Q2.3 | 返回值是以下错误码之一：`EINVAL` 或 `ENOMEM` |

---

## 错误码语义

| 返回值 | 触发条件 | 检测位置 |
|--------|----------|----------|
| `EINVAL` | `align < core::mem::size_of::<*mut c_void>()` | `posix_memalign` 自身检测 |
| `EINVAL` | `align` 不是 2 的幂 | `aligned_alloc_inner` 内部检测 |
| `ENOMEM` | `len == 0` | `aligned_alloc_inner` 内部处理 |
| `ENOMEM` | 内存不足或 `len` 溢出 (`len > SIZE_MAX - align`) | `aligned_alloc_inner` 内部检测 |
| `ENOMEM` | 底层 `malloc_inner` 分配失败 | `aligned_alloc_inner` → `malloc_inner` 链路 |

> **注**：POSIX 标准规定 `size == 0` 时的行为是实现定义的。musl（通过 `aligned_alloc`）此时返回 `ENOMEM`，rusl 保持此行为。

---

## 系统算法（System Algorithm）

```
posix_memalign(res, align, len):
1. if align < core::mem::size_of::<*mut c_void>() → return EINVAL
2. match aligned_alloc_inner(align, len) {
3.     Ok(ptr) => {
4.         unsafe { *res = ptr.as_ptr() as *mut c_void; }
5.         0
6.     }
7.     Err(AlignedAllocError::InvalidAlignment) => EINVAL,
8.     Err(AlignedAllocError::OutOfMemory)      => ENOMEM,
9. }
```

- **步骤 1** 是一个快速路径优化：当 `align` 小于指针宽度时直接拒绝，避免进入 `aligned_alloc_inner`。
- **步骤 2-8** 委托给内部 `aligned_alloc_inner`，该函数内部完成：2 的幂校验、溢出检测、替换检测、实际对齐分配。
- **错误转换**从 Rust `Result<_, AlignedAllocError>` 转换到 C 的整数错误码。

---

## 不变量（Invariants）

- **错误码一致性**：`posix_memalign` 的返回值始终是 `EINVAL` 或 `ENOMEM`，不会返回非 POSIX 定义的错误码。
- **参数保护**：失败时 `*res` 保持不变（POSIX 要求），调用者无需在失败分支中释放 `*res`。
- **ABI 不变式**：`extern "C"` 函数签名在编译为共享库后，参数布局、返回值布局与 C 实现完全一致。
- **errno 不变式**：由于内部使用 `Result` 传播错误，`posix_memalign` 本身不设置线程本地 `errno`。但若其他 `extern "C"` 函数（如 `aligned_alloc`）也需要导出 ABI，则它们之间仍通过 `errno` 协调，具体见各函数规约。

---

## 内部符号汇总

### 本模块直接依赖

| 符号 | 类型 | 来源模块 | 可见性 |
|------|------|----------|--------|
| `posix_memalign` | `extern "C" fn` | 本模块 | **Public** — POSIX `<stdlib.h>` |
| `aligned_alloc_inner` | `pub(crate) fn` | `mallocng::aligned_alloc` | Internal — 返回 `Result` |
| `AlignedAllocError` | `pub(crate) enum` | `mallocng::aligned_alloc` | Internal |
| `EINVAL` | `const c_int` | `errno` 或本模块 | Internal |
| `ENOMEM` | `const c_int` | `errno` 或本模块 | Internal |

### aligned_alloc_inner 的递归依赖（摘要）

| 符号 | 类型 | 来源模块 | 可见性 |
|------|------|----------|--------|
| `malloc_inner` | `pub(crate) fn` | `mallocng::malloc` | Internal |
| `UNIT` | `const usize` (16) | `mallocng::meta` | Internal |
| `IB` | `const usize` (4) | `mallocng::meta` | Internal |
| `SIZE_MAX` | `const usize` | `core::usize::MAX` | 标准库核心 |
| `struct Meta` | `pub(crate) struct` | `mallocng::meta` | Internal |
| `struct Group` | `pub(crate) struct` | `mallocng::meta` | Internal |
| `struct MetaArea` | `pub(crate) struct` | `mallocng::meta` | Internal |
| `struct MallocContext` | `pub(crate) struct` | `mallocng::meta` | Internal |
| `SIZE_CLASSES` | `pub(crate) static` | `mallocng::malloc` | Internal |
| `MALLOC_REPLACED` | `pub(crate) static AtomicBool` | `mallocng::replaced` | Internal |
| `ALIGNED_ALLOC_REPLACED` | `pub(crate) static AtomicBool` | `mallocng::replaced` | Internal |
| `get_meta` | `pub(crate) fn` | `mallocng::meta` | Internal |
| `get_slot_index` | `pub(crate) fn` | `mallocng::meta` | Internal |
| `get_stride` | `pub(crate) fn` | `mallocng::meta` | Internal |
| `set_size` | `pub(crate) fn` | `mallocng::meta` | Internal |
| `enframe` | `pub(crate) fn` | `mallocng::meta` | Internal |
| `size_to_class` | `pub(crate) fn` | `mallocng::meta` | Internal |
| `size_overflows` | `pub(crate) fn` | `mallocng::meta` | Internal |
| `sys_mmap` | `pub(crate) unsafe fn` | `syscall` | Internal — asm! syscall |
| `sys_munmap` | `pub(crate) unsafe fn` | `syscall` | Internal — asm! syscall |
| `sys_madvise` | `pub(crate) unsafe fn` | `syscall` | Internal — asm! syscall |
| `sys_brk` | `pub(crate) unsafe fn` | `syscall` | Internal — asm! syscall |
| `sys_mprotect` | `pub(crate) unsafe fn` | `syscall` | Internal — asm! syscall |
| `alloc_meta` | `pub(crate) fn` | `mallocng::malloc` | Internal |
| `MALLOC_LOCK` | `pub(crate) static Mutex` | `mallocng::malloc` | Internal |

---

/* Rely */
[RELY]
Predefined Structures/Functions:

  // === 直接内部依赖 ===

  /// 内部对齐分配函数，返回 Result 代替 C 的 NULL+errno 惯例
  pub(crate) fn aligned_alloc_inner(align: usize, len: usize) -> Result<core::ptr::NonNull<core::ffi::c_void>, AlignedAllocError>;
                                   // 依赖1: 内部 Rust 对齐分配引擎
                                   // 来源: rusl::mallocng::aligned_alloc

  pub(crate) enum AlignedAllocError { InvalidAlignment, OutOfMemory }
                                   // 依赖2: 分配错误类型
                                   // 来源: rusl::mallocng::aligned_alloc

  // === 间接递归依赖（通过 aligned_alloc_inner） ===

  /// 内部基础分配函数
  pub(crate) fn malloc_inner(n: usize) -> Option<core::ptr::NonNull<core::ffi::c_void>>;
                                   // 依赖3: 内部 Rust 基础分配器
                                   // 来源: rusl::mallocng::malloc

  // === 内部常量和数据结构（mallocng 引擎） ===

  pub(crate) const UNIT: usize = 16;
                                   // 依赖4: 最小分配对齐单位
                                   // 来源: rusl::mallocng::meta

  pub(crate) const IB: usize = 4;
                                   // 依赖5: 带内头部大小
                                   // 来源: rusl::mallocng::meta

  pub(crate) struct Meta { /* prev/next/mem/avail_mask/freed_mask/last_idx/... */ }
                                   // 依赖6: 组元数据结构体
                                   // 来源: rusl::mallocng::meta

  pub(crate) struct Group { /* meta/active_idx/storage[] */ }
                                   // 依赖7: 内存组结构体
                                   // 来源: rusl::mallocng::meta

  pub(crate) struct MallocContext { /* secret/active[48]/usage_by_class[48]/... */ }
                                   // 依赖8: 全局分配器上下文
                                   // 来源: rusl::mallocng::meta

  pub(crate) static SIZE_CLASSES: [u16; 48];
                                   // 依赖9: 大小类别查找表
                                   // 来源: rusl::mallocng::malloc

  pub(crate) static MALLOC_REPLACED: core::sync::atomic::AtomicBool;
                                   // 依赖10: malloc 替换检测标志
                                   // 来源: rusl::mallocng::replaced

  pub(crate) static ALIGNED_ALLOC_REPLACED: core::sync::atomic::AtomicBool;
                                   // 依赖11: aligned_alloc 替换检测标志
                                   // 来源: rusl::mallocng::replaced

  // === 内部辅助函数（meta 模块） ===

  pub(crate) unsafe fn get_meta(p: *const u8) -> &Meta;
                                   // 依赖12: 指针反查元数据（含安全校验）
                                   // 来源: rusl::mallocng::meta

  pub(crate) fn get_slot_index(p: *const u8) -> usize;
                                   // 依赖13: 提取槽位索引
                                   // 来源: rusl::mallocng::meta

  pub(crate) fn get_stride(g: &Meta) -> usize;
                                   // 依赖14: 获取槽位跨距
                                   // 来源: rusl::mallocng::meta

  pub(crate) fn set_size(p: *mut u8, end: *const u8, n: usize);
                                   // 依赖15: 写入分配大小头部
                                   // 来源: rusl::mallocng::meta

  pub(crate) fn enframe(g: &Meta, idx: usize, n: usize, ctr: u32) -> *mut u8;
                                   // 依赖16: 初始化分配帧
                                   // 来源: rusl::mallocng::meta

  // === 系统调用封装（rusl 直接通过 asm! 发起） ===

  pub(crate) unsafe fn sys_mmap(addr: *mut c_void, len: usize, prot: c_int, flags: c_int, fd: c_int, offset: off_t) -> *mut c_void;
                                   // 依赖17: mmap 系统调用
                                   // 来源: rusl::syscall

  pub(crate) unsafe fn sys_munmap(addr: *mut c_void, len: usize) -> c_int;
                                   // 依赖18: munmap 系统调用
                                   // 来源: rusl::syscall

  pub(crate) unsafe fn sys_madvise(addr: *mut c_void, len: usize, advice: c_int) -> c_int;
                                   // 依赖19: madvise 系统调用
                                   // 来源: rusl::syscall

  pub(crate) unsafe fn sys_brk(addr: *mut c_void) -> *mut c_void;
                                   // 依赖20: brk 系统调用
                                   // 来源: rusl::syscall

  pub(crate) unsafe fn sys_mprotect(addr: *mut c_void, len: usize, prot: c_int) -> c_int;
                                   // 依赖21: mprotect 系统调用
                                   // 来源: rusl::syscall

Predefined Macros:

  pub(crate) const EINVAL: core::ffi::c_int = 22;
                                   // 依赖22: POSIX EINVAL 错误码
                                   // 来源: rusl::errno (或本模块内联)

  pub(crate) const ENOMEM: core::ffi::c_int = 12;
                                   // 依赖23: POSIX ENOMEM 错误码
                                   // 来源: rusl::errno (或本模块内联)

[GUARANTEE]
Exported Interface:

  // === 对外导出符号（C ABI 兼容） ===

  /// POSIX posix_memalign — 对齐内存分配
  ///
  /// C ABI 签名: int posix_memalign(void **res, size_t align, size_t len);
  ///
  /// 本函数为 `rusl` 对外导出的符号，编译为共享库后可被外部 C 代码透明调用。
  /// 调用约定: extern "C"
  /// 参数布局: 与 C 的 void**, size_t, size_t 完全一致
  /// 返回布局: c_int (i32)
  ///
  /// 规约约束:
  /// - 前置条件 P1-P4 全部满足时，后置条件 Q1.1-Q1.5 或 Q2.1-Q2.3 成立
  /// - 返回值仅为 0 (成功), EINVAL, ENOMEM
  /// - 成功时 *res 指向对齐内存，可通过 free(*res) 释放
  /// - 失败时 *res 保持不变（POSIX 要求）
  #[no_mangle]
  pub unsafe extern "C" fn posix_memalign(
      res: *mut *mut core::ffi::c_void,
      align: core::ffi::c_ulong,
      len: core::ffi::c_ulong,
  ) -> core::ffi::c_int;

Internal Interface:

  // === 内部辅助函数（不对外导出） ===

  /// 将内部 aligned_alloc_inner 的 Result 转换为 C 错误码
  #[inline]
  fn convert_aligned_alloc_result(
      result: Result<core::ptr::NonNull<core::ffi::c_void>, AlignedAllocError>,
      res: *mut *mut core::ffi::c_void,
  ) -> core::ffi::c_int;
  // 内部工具函数，不对外导出