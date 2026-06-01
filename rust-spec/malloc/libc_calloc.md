# libc_calloc 规约 (Rust 版本)

> **对应 C spec**: `src/malloc/spec/libc_calloc.md`
> **对应 C 源文件**: `src/malloc/libc_calloc.c`
> **编译机制映射**: C 版本通过 `#define calloc __libc_calloc` / `#define malloc __libc_malloc` 对 `calloc.c` 进行符号重命名后 `#include "calloc.c"`。Rust 版本使用**泛型函数指针参数化**替代预处理器符号重命名——公共 `calloc` 和内部 `__libc_calloc` 共享同一个带 `malloc_fn` 参数的通用实现 `calloc_impl`，不同调用点传入不同的分配器函数指针。

---

## 依赖图

```
libc_calloc 模块 (rusl)
  │
  ├── pub(crate) unsafe fn __libc_calloc(size_t, size_t) -> *mut c_void
  │
  │   内部依赖:
  │   ├── __libc_malloc(size_t) -> *mut c_void          [see 依赖1: mallocng/malloc 模块]
  │   ├── __malloc_replaced: AtomicBool                  [see 依赖2: replaced 模块]
  │   ├── __malloc_allzerop(*const c_void) -> bool       [see 依赖3: mallocng/malloc 模块]
  │   ├── pub(crate) fn mal0_clear(*mut u8, usize) -> usize [本模块内部]
  │   └── pub(crate) unsafe fn memset(*mut u8, i32, usize) -> *mut u8 [see 依赖4: string/memset 模块]
  │
  ├── mal0_clear(*mut u8, usize) -> usize               [pub(crate) 内部函数]
  │   内部依赖:
  │   └── memset(*mut u8, i32, usize) -> *mut u8        [see 依赖4: string/memset 模块]
  │
  └── 通用实现: calloc_impl(usize, usize, fn(usize)->*mut c_void) -> *mut c_void  [pub(crate)]
      内部依赖:
       ├── 传入的 malloc 函数指针
       ├── __malloc_replaced                            [see 依赖2]
       ├── __malloc_allzerop                            [see 依赖3]
       ├── mal0_clear                                   [本模块]
       └── memset                                       [see 依赖4]
```

---

## 设计要点：从 C 预处理器重命名到 Rust 参数化

**C 版本策略 (musl)**:
```c
// libc_calloc.c
#define calloc __libc_calloc   // 重命名对外符号
#define malloc __libc_malloc   // 重命名内部依赖
#include "calloc.c"            // 复用同一份源码
```
通过预处理器在编译期替换符号名，使得同一份 `calloc.c` 源码产生两个不同的链接符号：公开的 `calloc`（使用 `malloc`）和内部的 `__libc_calloc`（使用 `__libc_malloc`）。

**Rust 版本策略 (rusl)**:
```rust
// 通用内部实现 — 以 allocator 函数指针参数化
pub(crate) unsafe fn calloc_impl(
    m: usize,
    n: usize,
    malloc_fn: unsafe extern "C" fn(usize) -> *mut c_void,
) -> *mut c_void { /* 溢出检测 + 分配 + 清零 */ }

// 公共 calloc — 使用 public malloc
pub unsafe extern "C" fn calloc(m: usize, n: usize) -> *mut c_void {
    calloc_impl(m, n, malloc)
}

// 内部 __libc_calloc — 使用 internal __libc_malloc
pub(crate) fn __libc_calloc(m: usize, n: usize) -> *mut c_void {
    calloc_impl(m, n, __libc_malloc)
}
```

**设计理由**:
- Rust 没有 C 预处理器（`#define` + `#include` 拼接），无法在源码级重命名符号。
- 泛型函数指针参数化是 Rust 惯用的"策略模式"等价物，比预处理器拼接更显式、更安全（类型检查覆盖函数签名）。
- 公共 `calloc` 为 `extern "C"`（保持 ABI 兼容）；内部 `__libc_calloc` 为 `pub(crate)` 普通 Rust 函数（仅 crate 内可见，不需要 C ABI）。
- musl 中内部模块通过 `#define calloc __libc_calloc` 在包含 `<stdlib.h>` 前重定向调用。rusl 中内部模块直接 `use crate::malloc::libc_calloc::__libc_calloc` 进行 Rust 原生调用，无需宏重定向。
- 加上 `#[inline]` 标注可确保 `calloc_impl` 在编译期被内联到两个调用点，消除函数指针间接调用的运行时开销（零成本抽象）。

---

## calloc_impl (通用内部实现)

```rust
pub(crate) unsafe fn calloc_impl(
    m: usize,
    n: usize,
    malloc_fn: unsafe extern "C" fn(usize) -> *mut c_void,
) -> *mut c_void;
```

[Visibility]: Internal — `pub(crate)` 可见性，仅在 rusl crate 内部使用。不对 crate 外部暴露。

### 复杂度层级: Level 3

### 意图 (Intent)

以参数化的 `malloc_fn` 指针替代 C 预处理器中的 `malloc` 符号重命名，为公共 `calloc` 和内部 `__libc_calloc` 提供统一的分配 + 溢出检测 + 清零逻辑。`malloc_fn` 在公共版本为 `malloc`（可被用户替换），在内部版本为 `__libc_malloc`（不可被替换）。

### 前置条件

- `m` 和 `n` 为任意 `usize` 值。
- `malloc_fn` 是一个有效的函数指针，其语义等价于 `malloc`：接受 `usize` 大小参数，返回指向已分配内存的指针或 NULL。

### 后置条件

- **Case 1 (成功)**: 返回指向 `m * n` 字节连续内存块的指针，所有字节初始化为 0。返回指针满足与底层 `malloc_fn` 相同的对齐要求。
- **Case 2 (溢出)**: 若 `m * n` 溢出 `usize`，`errno` 被设置为 `ENOMEM`，返回 `core::ptr::null_mut()`。
- **Case 3 (分配失败)**: 若 `malloc_fn(n)` 返回 NULL，返回 `core::ptr::null_mut()`。
- **Case 4 (malloc 已被替换)**: 若 `__malloc_replaced` 为非零（通过 `Ordering::Relaxed` 读取），禁用零页优化，强制通过 `memset`/`mal0_clear` 清零所有字节。

### 系统算法 (System Algorithm)

**阶段 1 — 溢出检测**:
```
if n != 0 && m > usize::MAX / n:
    set_errno(ENOMEM); return null_mut();
```

**阶段 2 — 分配**:
```
n = m * n;  // 已验证不溢出
let p = malloc_fn(n);
```

**阶段 3 — 清零优化**:
```
if p.is_null() || (!__malloc_replaced.load(Relaxed) && __malloc_allzerop(p)) {
    return p;
}
let remaining = mal0_clear(p as *mut u8, n);
return memset(p as *mut u8, 0, remaining) as *mut c_void;
```

### 不变量

- `calloc_impl` 始终返回零初始化内存（或 NULL），无论 `__malloc_replaced` 的状态如何。
- `calloc_impl` 不得访问 `malloc_fn` 参数之外的任何可替换分配器符号——分配器选择完全由调用者通过函数指针显式指定。
- 通过 `#[inline]` 的编译期内联保证函数指针调用在 release 构建中被优化为直接调用，无间接跳转开销。

---

## mal0_clear (内部函数)

```rust
pub(crate) fn mal0_clear(p: *mut u8, n: usize) -> usize;
```

[Visibility]: Internal — `pub(crate)` 可见性，仅 rusl crate 内部使用。与 C 版本 `static` 函数语义相同，不对 crate 外部暴露。

### 复杂度层级: Level 3

### 意图 (Intent)

将已分配的内存块清零，但利用**向后扫描**策略减少不必要的工作量：当内存由内核的零页映射返回时，页面可能是干净的全零页。`mal0_clear` 从内存块的**末尾**向**开头**扫描，跳过已为零的页面，仅对**非零**区域调用 `memset` 进行清零。

**Rust 设计改进**: C 版本依赖 GCC 特定的 `__attribute__((__may_alias__))` 实现 `uint64_t` 类型双关（type punning）以加速零检测。Rust 版本使用 `core::arch::x86_64` 或等价 `cfg`-gate 下的 SIMD / 宽字加载操作，无需违反严格别名规则即可安全地以 8 字节粒度扫描。非 x86 平台可降级为逐字节 `u8` 扫描。

### 系统算法 (System Algorithm)

与 C 版本算法相同——从尾到头的反向逐页扫描：

1. **起点对齐**: 以 `pagesz = 4096` 为粒度。将指针 `pp` 初始化为 `p.wrapping_add(n)`（缓冲区末尾），将 `i` 初始化为 `pp as usize & (pagesz - 1)`（页内偏移量）。

2. **循环扫描**:
   - **Step A — 清零页内尾部**: 对页面内非对齐部分执行 `memset(pp.sub(i), 0, i)` 清零。
   - **Step B — 提前终止检查**: 若 `pp.offset_from(p) < pagesz as isize`（剩余不足一页），返回剩余未处理字节数。
   - **Step C — 整页扫描**: 从当前页末尾向开头以 `2 * mem::size_of::<T>()` 步进扫描。`T` 通过 `cfg` 选择：
     - `target_arch = "x86_64"` 或 `target_arch = "aarch64"`：`T = u64`，一次检查 16 字节。
     - 其他架构：`T = u8`，逐字节扫描。
     使用 `core::ptr::read_unaligned` 读取，避免未对齐访问的 UB。
   - **Step D — 跳过零页**: 若扫描完整个页面未发现非零值，则 `pp` 跳过该页继续向前。

3. **返回**: 返回值为还需调用者额外清零的字节数。

### 前置条件

- `p` 指向一个长度为 `n` 字节的可读可写内存块（通过 `malloc` 返回）。
- `n >= 0`。

### 后置条件

- 返回值 `r` 满足 `0 <= r < pagesz` 或 `r == n`（当 `n < pagesz` 时）。
- 所有 `pp` 至 `p + n` 之间的内存已被清零，剩余 `p[0..r]` 由调用者补齐。

### 设计要点 (Rust 特定)

- **类型双关安全化**: 使用 `core::ptr::read_unaligned::<u64>()` 替代 C 的 `__attribute__((__may_alias__))`。Rust 的 `read_unaligned` 在 safe 边界外使用 `unsafe` 块，明确标注潜在的未对齐访问，同时不违反严格别名规则。
- **平台特化**: 通过 `#[cfg(target_arch = "...")]` 选择最优扫描宽度，避免 C 版本的 `#ifdef __GNUC__` 编译器检测。
- **`pagesz` 常量**: 保持 4096 字面量。在 Rust 中可定义为 `const PAGESZ: usize = 4096;`，与 C 版本策略一致（不依赖实际系统页大小）。

---

## __libc_calloc (内部符号)

```rust
pub(crate) fn __libc_calloc(m: usize, n: usize) -> *mut c_void;
```

> **注意**: 与 C 版本通过 `#define calloc __libc_calloc` 预处理产生同名 C 符号不同，Rust 版本 `__libc_calloc` 是一个独立的 `pub(crate)` Rust 函数。它调用 `calloc_impl(m, n, __libc_malloc)` ，将内部分配器函数指针传入，从而在实现层面等价于 C 版本的"不可被替换的内部 calloc"语义。

[Visibility]: Internal — `pub(crate)` 可见性，仅在 rusl crate 内部模块间可调用。相较于 C 版本的 `hidden` 可见性（通过 `__attribute__((__visibility__("hidden")))` 控制），Rust 的可见性由模块系统在编译期强制执行，更安全且无链接器依赖。

### 复杂度层级: Level 2 (委托到 calloc_impl 后逻辑复杂度下降)

### 意图 (Intent)

为 rusl crate 内部提供**不可被替换的** `calloc` 实现。这是 musl 架构设计在 Rust 中的等价翻译——内部分配器函数通过"策略参数化"（传入 `__libc_malloc` 而非 `malloc`）与公共 API 隔离，确保 crate 内部代码始终使用内部分配器，即使应用程序通过 `LD_PRELOAD` 或静态链接替换了公共 `malloc`/`calloc`。

在 Rust/rusl 中，由于不存在 ELF 符号插替（symbol interposition）的概念（所有内部调用在编译期静态分派），"不可替换"由 Rust 的模块私有性和编译期单态化自然保证，无需运行时检查。

### 前置条件

- `m` 和 `n` 为任意 `usize` 值。
- 调用者不持有任何 malloc 锁。

### 后置条件

由 `calloc_impl` 的后置条件完全继承，详见 `calloc_impl` 规约。

### 不变量

- `__libc_calloc` 始终返回零初始化内存（或 NULL）。
- `__libc_calloc` 仅调用 `__libc_malloc`（内部版本），不依赖可替换的公共 `malloc`。
- 函数不带有 `unsafe` 标记，因为其所有内部 `unsafe` 操作均由 `calloc_impl` 封装。外部调用者是安全的（前提是返回值被正确使用）。

---

## 对 rusl 内部调用者的说明

rusl 内部模块通过 `use crate::malloc::libc_calloc::__libc_calloc;` 直接引入并使用内部分配器。以下 rusl 组件应始终使用内部 `__libc_calloc` 实现：

| 使用模块 | 对应 musl 源文件 | 说明 |
|---------|-----------------|------|
| atexit 处理 | `src/exit/atexit.c` | 注册退出处理函数时的内存分配 |
| 命名信号量 | `src/thread/sem_open.c` | 命名 POSIX 信号量的内部状态 |
| 异步 I/O | `src/aio/aio.c` | AIO 控制块分配 |
| 动态链接器错误处理 | `src/ldso/dlerror.c` | 动态链接器内部错误信息存储 |
| 进程 fd 操作 | `src/process/fdop.h` | 文件描述符操作的内部状态 |
| NLS / gettext | `src/locale/dcngettext.c` | 国际化消息目录的内部状态 |

在 rusl 中，这些模块导入 `__libc_calloc` 后直接以 Rust 函数调用方式使用，不再需要 C 预处理器宏重定向。

---

## 外部依赖说明

| 依赖符号 | 来源模块 | Rust 类型签名 | 说明 |
|---------|---------|--------------|------|
| `__libc_malloc` | `crate::malloc::mallocng::malloc` | `pub(crate) unsafe extern "C" fn(usize) -> *mut c_void` | rusl 内部 malloc，不可被替换。与 C 版本 `__libc_malloc` (hidden) 语义等价。 |
| `__malloc_replaced` | `crate::malloc::replaced` | `pub(crate) static __malloc_replaced: core::sync::atomic::AtomicBool` | 标记公共 malloc 是否被替换。在 rusl 中从 C 版本的 `volatile int` 升级为 `AtomicBool`，提供明确定义的内存顺序语义。 |
| `__malloc_allzerop` | `crate::malloc::mallocng::malloc` (或 `crate::malloc::oldmalloc::malloc`) | `pub(crate) fn __malloc_allzerop(p: *const c_void) -> bool` | 零页检测函数。C 版本通过 `weak_alias` 实现可覆盖默认实现；Rust 版本由 malloc 后端模块直接导出强符号。默认实现（若无 malloc 后端）返回 `false`。 |
| `memset` | `crate::string::memset` | `pub(crate) unsafe fn memset(dest: *mut u8, c: i32, n: usize) -> *mut u8` | 标准内存设置函数。rusl 内部实现，签名保持 C ABI 兼容。 |
| `errno` / `ENOMEM` | `crate::errno` | `pub(crate) fn set_errno(e: i32)` + `pub const ENOMEM: i32` | 错误报告机制。rusl 采用模块封装的 `set_errno` 函数，内部通过 `core::arch::asm!` 直接写入线程局部 errno 地址。 |

---

## [RELY]

```rust
Predefined Structures/Functions:
  // 依赖1: malloc 后端模块提供的内部分配器函数
  crate::malloc::mallocng::malloc::__libc_malloc:
      pub(crate) unsafe extern "C" fn(usize) -> *mut c_void;
  
  // 依赖2: replaced 模块提供的全局替换标志
  crate::malloc::replaced::__malloc_replaced:
      pub(crate) static __malloc_replaced: core::sync::atomic::AtomicBool;
  
  // 依赖3: malloc 后端模块提供的零页检测函数
  crate::malloc::mallocng::malloc::__malloc_allzerop:
      pub(crate) fn __malloc_allzerop(p: *const c_void) -> bool;
  
  // 依赖4: string 模块提供的 memset 函数
  crate::string::memset::memset:
      pub(crate) unsafe fn memset(dest: *mut u8, c: i32, n: usize) -> *mut u8;
  
  // 依赖5: errno 模块提供的错误报告机制
  crate::errno::set_errno:
      pub(crate) fn set_errno(e: i32);
  crate::errno::ENOMEM:
      pub(crate) const ENOMEM: i32;

Predefined Types:
  core::ffi::c_void;     // 通用指针类型 (等价于 C 的 void *)
  usize;                 // 标准无符号整数类型 (等价于 C 的 size_t)
  u8;                    // 8位无符号字节类型
  u64;                   // 64位无符号整数类型 (用于零页宽字扫描)
  bool;                  // Rust 布尔类型 (替代 C 中 int 返回 0/1 的惯例)
  core::sync::atomic::AtomicBool;  // 原子布尔类型
  core::sync::atomic::Ordering;    // 原子操作内存顺序枚举

Predefined Macros/Attributes:
  #[inline];             // 编译期内联标注，消除函数指针间接调用开销
  #[cfg(target_arch = "...")];  // 平台条件编译，选择最优扫描宽度
```

## [GUARANTEE]

```rust
Exported Interface (pub(crate) — crate 内部可见):

  // 内部 calloc 接口 — 不可被用户替换
  pub(crate) fn __libc_calloc(m: usize, n: usize) -> *mut c_void;
  // 语义: 分配 m*n 字节零初始化内存
  // 行为: 等价于 calloc(m, n)，但始终使用内部分配器 __libc_malloc
  // 溢出: m*n 溢出 usize 时返回 null_mut() 并设置 errno = ENOMEM
  // 清零: 内部使用 mal0_clear + memset 确保全零结果
  // 线程安全: 由底层 __libc_malloc 保证

Internal Implementation Details (pub(crate) — 不对外保证稳定性):

  // 通用 calloc 实现 — 以函数指针参数化
  pub(crate) unsafe fn calloc_impl(
      m: usize,
      n: usize,
      malloc_fn: unsafe extern "C" fn(usize) -> *mut c_void,
  ) -> *mut c_void;
  // 语义: 可参数化的 calloc 核心逻辑，传入 malloc_fn 区分公共/内部版本
  // unsafe: 调用者保证 malloc_fn 函数指针有效且行为符合 malloc 规约

  // 反向扫描清零辅助函数
  pub(crate) fn mal0_clear(p: *mut u8, n: usize) -> usize;
  // 语义: 从尾部向头部扫描，跳过已为零的页面，仅清零非零区域
  // 返回: 仍需调用者补齐清零的字节数
```