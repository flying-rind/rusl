# __reset_tls — Rust 接口归约

> **源 C spec**: `src/env/spec/__reset_tls.md`
> **来源文件**: `musl/src/env/__reset_tls.c`
> **复杂度层级**: Level 2 — 纯数据搬运（逐模块内存复制 + 尾零填充），无状态机、无分配、无系统调用
> **导出状态**: 本模块所有符号均为 Internal — musl/rusl 内部 TLS 重置函数，POSIX/C 标准未定义，用户程序不应直接访问或调用。
> **调用者**: `cleanup_fromsig()`（定时器信号处理线程）及 `fork()` 后的子进程 TLS 重置路径

---

## 依赖图

```
__reset_tls
  ├── pthread_self()                                   // 获取当前线程控制块 (*mut Pthread)
  ├── crate::internal::libc::TlsModule                 // TLS 模块描述符结构体
  ├── crate::internal::libc::libc                      // 全局运行时状态（.tls_head 字段）
  ├── crate::internal::pthread_arch::DTP_OFFSET        // 编译时常量，dtv 偏置值
  ├── core::ptr::copy_nonoverlapping::<u8>             // 替代 C 的 memcpy
  └── core::ptr::write_bytes::<u8>                     // 替代 C 的 memset
```

> **说明**: `__reset_tls` 模块自身不定义任何结构体、常量或内部辅助函数。所有依赖均来自其他 rusl 内部模块或 Rust `core` 库。

---

## 跨文件依赖速查

| 依赖项 | 来源模块 | 处理方式 |
|--------|----------|----------|
| `Pthread` (`.dtv` 字段) | `crate::internal::pthread_impl` | 已有 spec: `src/internal/rust-spec/pthread_impl.md` |
| `TlsModule` / `libc` 全局变量 | `crate::internal::libc` | 已有 spec: `src/internal/rust-spec/libc.md` |
| `pthread_self() → *mut Pthread` | `crate::internal::pthread_impl` | 已有 spec |
| `DTP_OFFSET` 常量 | `crate::internal::pthread_arch` | 架构相关常量，无独立 spec |
| `copy_nonoverlapping` / `write_bytes` | Rust `core::ptr` | `#![no_std]` 兼容零开销原语 |

---

## 模块概述

`__reset_tls` 实现了 POSIX 线程模型中关键的 TLS 重置语义：当执行上下文从父线程继承 TLS 内存（如 `fork()` 后的子进程）或从进程地址空间复用 TLS 内存（如定时器信号处理线程）时，必须将当前线程的所有 TLS 变量恢复到程序加载时的初始值。

本函数采用"逐模块全量复制 + 尾零填充"策略：遍历全局 TLS 模块链表，将每个模块的初始数据映像（`.tdata` 段）拷贝回当前线程的对应 TLS 内存区域，已初始化部分之外的内存区域清零（对应 `.tbss` 零初始语义）。

在 Rust 实现中：
- 使用 `core::ptr::copy_nonoverlapping` 替代 C 的 `memcpy`，执行 TLS 初始数据拷贝；
- 使用 `core::ptr::write_bytes` 替代 C 的 `memset`，执行 `.tbss` 段清零；
- 两者均为 Rust `core` 零开销原语，完全兼容 `#![no_std]` 环境，不依赖任何外部 libc。

---

## 函数规约

---

### `__reset_tls`

```rust
// Rust 签名
pub(crate) fn __reset_tls();
```

[Visibility]: Internal (`pub(crate)`) — rusl 内部 TLS 重置函数，POSIX/C 标准未定义。声明于 `pthread_impl` 内部模块。仅用于 `fork()` 之后或定时器信号处理线程 (`cleanup_fromsig()`) 的 TLS 重置场景。用户程序不可调用。

**C 对照**: `hidden void __reset_tls(void);` (`pthread_impl.h` 中声明为 `hidden`)

#### 意图 (Intent) — Level 2

将当前线程的所有 TLS（Thread-Local Storage）变量恢复到程序加载时的初始值。此操作对于 `fork()` 后的子进程以及与进程共享地址空间的信号处理线程是必需的：这些执行上下文继承了父线程的 TLS 内存，但其内容可能已偏离初始状态（如 `errno`、`h_errno`、区域设置等），必须重置以确保语义正确。

实现采用"逐模块复制 + 尾零填充"策略：遍历全局 TLS 模块链表，将每个模块的初始镜像拷贝回当前线程的对应 TLS 内存区域，已初始化部分之外的区域清零（对应 `.tbss` 语义）。

#### 前置条件 (Pre-condition)

1. **TLS 已初始化**: 调用线程必须已通过 TLS 初始化流程（`init_tls` 或 `copy_tls`），即 `self.dtv` 不为 null 且 `(*self.dtv)`（即 `dtv[0]`）已正确设置为已加载 TLS 模块的数量 `n`。
2. **模块链表已就绪**: 全局链表 `libc.tls_head` 已构建完毕，且其中的模块顺序与 DTV 索引 `1..n` 一一对应（即 `tls_head` 对应 i=1，`tls_head.next` 对应 i=2，依此类推）。
3. **内存区域有效**: 对于所有 `i ∈ [1, n]`：
   - `(self.dtv.add(i).read() - DTP_OFFSET) as *mut u8` 指向的 TLS 块区域大小至少为对应模块 `p.size` 字节，且该内存区域可读写。
   - `p.image` 指向至少 `p.len` 字节的有效初始数据映像（TLS `.tdata` 段在 ELF 中的原型镜像，程序生命周期内不可变）。
4. **调用上下文**: 当前应在单线程环境中执行（或调用者已保证无并发 TLS 访问），以避免 `copy_nonoverlapping`/`write_bytes` 写入 TLS 时与其它线程的 TLS 读取产生数据竞争。

#### 后置条件 (Post-condition)

- **Case 1 — 无 TLS 模块 (`n == 0`)**: 函数为空操作，直接返回。不访问 DTV 或内存。
- **Case 2 — 有 TLS 模块 (`n > 0`)**: 对于所有 `i ∈ [1, n]`，设 `p` 为 `libc.tls_head` 链表中第 `i` 个 `&TlsModule`：
  - `mem = (self.dtv.add(i).read() - DTP_OFFSET) as *mut u8` 处的前 `p.len` 字节等于 `p.image` 指向的初始数据（即 TLS 已初始化数据（`.tdata` 段）恢复到程序加载时的值）。
  - 地址 `mem.add(p.len)` 至 `mem.add(p.size - 1)` 的全部字节被置零（即 `.tbss` 段恢复为零初始状态）。
  - 所有 TLS 变量的值等同于程序刚加载时的初始值。
- **Case 3 — 失败**: 无。此函数不返回错误码，且始终成功（`copy_nonoverlapping`/`write_bytes` 在有效内存范围内不会失败——违反前置条件导致未定义行为而非可控错误）。

#### 不变量 (Invariants)

无跨此函数维持的不变量。此函数是一次性重置操作，每次调用独立；它不修改 `libc.tls_head`、`dtv` 指针或任何全局状态。

#### 系统算法 (System Algorithm) — Level 3

该函数对 rusl 的 TLS 运行时正确性至关重要，具体算法如下：

```
Input:  当前线程的 *mut Pthread self（通过 pthread_self() 获取）
        libc.tls_head 全局 TLS 模块链表

Algorithm:
1.  self := pthread_self()                         // 获取当前线程控制块裸指针
2.  dtv := (*self).dtv                              // 读取 DTV 数组指针
3.  if dtv.is_null(): return                         // 防御: DTV 未分配
4.  n := unsafe { dtv.read() }                     // dtv[0] 存储模块数量
5.  if n == 0: return                               // 无 TLS 模块，直接返回
6.  p := libc.tls_head                              // p 指向第一个 TLS 模块 (*const TlsModule)
7.  for i := 1 to n:
8.      dtv_val := unsafe { dtv.add(i).read() }     // dtv[i] = TLS块起始地址 + DTP_OFFSET
9.      mem := ((dtv_val - DTP_OFFSET) as *mut u8)  // 恢复 TLS 块真实起始地址
10.     unsafe {
11.         core::ptr::copy_nonoverlapping(
12.             (*p).image as *const u8,             // 源: TLS 初始映像
13.             mem,                                  // 目标: 当前线程的 TLS 块
14.             (*p).len                              // 拷贝已初始化数据大小
15.         )
16.     }
17.     unsafe {
18.         core::ptr::write_bytes(
19.             mem.add((*p).len),                    // 目标: .tbss 起始地址
20.             0u8,                                   // 填充值: 0
21.             (*p).size - (*p).len                  // .tbss 区域大小
22.         )
23.     }
24.     p := (*p).next                                // 前进到下一模块
25. end for
```

**时间复杂度**: O(各模块 `size` 之和)。每次调用对所有 TLS 模块做全量复制，而非增量更新。

**性能说明**: 此函数采用"全量复制"而非"增量差分"，因为：
1. 无法可靠追踪哪些 TLS 变量被修改过（TLS 变量通过 `__tls_get_addr` 动态寻址，无集中式修改记录）；
2. `fork()` 和信号处理线程的 TLS 重置属于低频操作（`fork()` 次数通常有限，信号处理线程创建亦不频繁）；
3. 正确性优先于性能——确保所有 TLS 变量恢复初始状态，避免 `fork()` 后子进程中残留父进程的脏数据（如未刷新的 stdio 缓冲区、过期的 errno 值等）。

---

## 架构差异

`__reset_tls` 的实现不直接依赖 TLS Above/Below TP 的布局差异——它通过 `dtv[i] - DTP_OFFSET` 统一获取 TLS 块基地址，此计算对两种布局均成立。唯一需要关注的架构差异是 `DTP_OFFSET` 常量在不同架构上的值：

| 架构 | DTP_OFFSET | 说明 |
|------|-----------|------|
| x86_64, i386 | 0 | 默认值，无偏置 |
| aarch64 | 0 | 默认值，无偏置 |
| arm | 0 | 默认值，无偏置 |
| riscv64 | 0 | 默认值，无偏置 |

在 rusl 中通过 `crate::internal::pthread_arch` 模块定义：

```rust
// 定义于 crate::internal::pthread_arch
pub(crate) const DTP_OFFSET: usize = 0;
```

> **注意**: 即使未来某个架构的 `DTP_OFFSET != 0`，`__reset_tls` 的算法也保持不变——只需从 `dtv[i]` 中减去 `DTP_OFFSET`（作为 `usize` 整数运算而非指针减法）即可得到 TLS 块的真实地址。

---

## Rust 实现要点

1. **内存操作原语替换**: 使用 `core::ptr::copy_nonoverlapping` 替代 C 的 `memcpy`，使用 `core::ptr::write_bytes` 替代 C 的 `memset`。两者均为 `#![no_std]` 兼容的零开销原语，语义等价且不依赖任何外部 libc。这两个函数接受裸指针参数，调用者需自行保证指针有效性和区域不重叠——这正是 `unsafe` 职责所在。

2. **线程控制块访问**: 通过 `crate::internal::pthread_impl::pthread_self()` 获取 `*mut Pthread`，然后通过裸指针访问 `dtv` 字段。在 Rust 中，`pthread_self()` 应返回一个有效指针——本函数需防御性检查 null `dtv` 情况（对应 C 实现中宏展开后的隐式行为）。

3. **DTV 偏置处理**: `DTP_OFFSET` 是编译时常量。`dtv[i]` 存储的是"已偏置值"（`TLS块起始地址 + DTP_OFFSET`），故在解引用前需减去 `DTP_OFFSET`。在 Rust 中使用整数运算：`(dtv_val - DTP_OFFSET) as *mut u8`，而非指针 `.sub()` 方法——使用整数运算更贴近 C 原版的语义（先做 `uintptr_t` 整数减法，再转为 `char*`），避免了 `*mut u8` 指针在地址空间低位时 `sub()` 可能产生的 wrapping 歧义。

4. **unsafe 范围控制**: 按照 rusl 编码规范，每个 `unsafe` 块应尽量小——仅包裹必须的裸指针操作：
   - `dtv.read()` 和 `dtv.add(i).read()` —— DTV 是 C 分配的数组，解引用需 unsafe；
   - `core::ptr::copy_nonoverlapping` —— 源和目标均为裸指针，需 unsfae；
   - `core::ptr::write_bytes` —— 目标为裸指针，需 unsafe；
   - `(*p).image` / `(*p).len` / `(*p).size` / `(*p).next` —— 通过裸指针访问 `TlsModule` 字段，需 unsafe。
   - 循环控制和条件判断等纯逻辑代码保持在 unsafe 块之外。

5. **TlsModule 字段访问**: `TlsModule` 以 `#[repr(C)]` 布局（定义于 `crate::internal::libc`），字段 `next`、`image`、`len`、`size` 与 C 结构体布局完全一致。本函数仅读取 `next`、`image`、`len`、`size` 四个字段，不读写 `align` 和 `offset`。

6. **no_std 约束**: 本模块完全不依赖 `std`，仅使用 `core::ptr` 内存操作原语和 rusl 内部模块。无需任何第三方 crate。

7. **防御性编程**: 在 Debug 模式下建议通过 `debug_assert!` 增加以下检查，Release 模式下编译消除：
   - `pthread_self()` 返回的指针非 null
   - `dtv` 非 null
   - 每个模块满足 `p.len <= p.size`
   - 每个 `dtv[i] >= DTP_OFFSET`（防止减法下溢）

---

## 调用时序

`__reset_tls` 在两个关键场景被调用：

### 场景 1: `fork()` 后的子进程

```
fork()
  → 子进程返回路径
    → __reset_tls()         ← ★ 重置所有 TLS 变量为初始值
    → 执行 fork 处理器链
    → 返回给用户代码
```

子进程继承了父进程的完整地址空间（COW），包括 TLS 内存。但子进程是全新的执行上下文，其 TLS 变量值（如 `errno`、`locale`、stdio 缓冲区状态等）不应保留父进程的状态，必须重置。

### 场景 2: 定时器信号处理线程

```
timer_create(...)
  → [内核创建信号处理线程]
    → start(td)
      → cleanup_fromsig()
        → __reset_tls()     ← ★ 线程复用进程地址空间时重置 TLS
        → ...
```

定时器信号处理线程与主进程共享地址空间，其 TLS 内存可能从前一个使用相同地址空间的上下文中继承脏数据。在信号处理开始前必须重置 TLS 以确保正确的初始语义。

---

## 内部类型引用

以下类型定义于其他模块，在此处仅做简要说明以支撑规约理解。

### `TlsModule`（定义于 `crate::internal::libc`）

[Visibility]: Internal — rusl 内部 TLS 模块描述符，不在 POSIX 标准中定义

```rust
#[repr(C)]
pub(crate) struct TlsModule {
    pub(crate) next: *const TlsModule,   // 单向链表，指向下一个已加载的 TLS 模块
    pub(crate) image: *const c_void,     // 指向 TLS 模板块初始数据的原型镜像（.tdata 段）
    pub(crate) len: usize,               // 已初始化数据段大小（.tdata 段字节数）
    pub(crate) size: usize,              // TLS 块总大小（.tdata + .tbss 段字节数）
    pub(crate) align: usize,             // 对齐要求
    pub(crate) offset: usize,            // 模块在 DTV 中的偏移编排信息
}
```

**不变量**: `len <= size` 始终成立（已初始化数据不超过总大小）。`image` 指向的初始数据在程序生命周期内不可变。

> **注意**: `__reset_tls` 仅读取 `next`、`image`、`len`、`size` 四个字段。

### `Pthread` 中的 `dtv` 字段（定义于 `crate::internal::pthread_impl`）

[Visibility]: Internal — rusl 内部线程控制块

```rust
#[repr(C)]
pub(crate) struct Pthread {
    // ...
    pub(crate) dtv: *mut usize,   // 指向 Dynamic Thread Vector 数组
    // ...
}
```

`dtv` 数组约定：
- `dtv[0]` 存储当前线程绑定的 TLS 模块数 `n`。
- 对于 `i ∈ [1, n]`，`dtv[i]` 存储模块 i 的 TLS 块"已偏置指针"；通过 `dtv[i] - DTP_OFFSET` 可得该模块 TLS 块的真实起始地址。

### `libc` 全局变量（定义于 `crate::internal::libc`）

[Visibility]: Internal — rusl 内部全局运行时状态

```rust
pub(crate) struct Libc {
    // ...
    pub(crate) tls_head: *const TlsModule,  // 指向已加载 TLS 模块的单向链表头
    // ...
}
```

`libc.tls_head` 指向已加载 TLS 模块的单向链表头，遍历顺序与 DTV 索引 i 一致（`tls_head` 对应 i=1，`tls_head.next` 对应 i=2，以此类推）。

### `DTP_OFFSET` 常量（定义于 `crate::internal::pthread_arch`）

[Visibility]: Internal — 架构相关的编译时常量

```rust
pub(crate) const DTP_OFFSET: usize = 0;
```

DTP_OFFSET 是 dtv 指针与 TLS 块起始地址之间的固定偏移量。在大多数架构上为 0。`dtv[i] = TLS块真实起始地址 + DTP_OFFSET`。

---

/* Rely */
[RELY]
Rust Core 内建原语:
  core::ptr::copy_nonoverlapping::<u8>        // 依赖1: 非重叠内存复制，替代 C 的 memcpy
  core::ptr::write_bytes::<u8>               // 依赖2: 逐字节写入常量值，替代 C 的 memset
  core::ffi::c_void                           // 依赖3: 对应 C 的 void 类型（用于 TlsModule.image 字段指针）

rusl 内部模块:
  fn pthread_self() -> *mut Pthread;         // 依赖4: 获取当前线程控制块指针 (crate::internal::pthread_impl)
  struct Pthread { dtv: *mut usize, ... }    // 依赖5: 线程控制块，通过 dtv 字段访问 DTV 数组
  struct TlsModule {                         // 依赖6: TLS 模块描述符 (crate::internal::libc)
      next: *const TlsModule,
      image: *const c_void,
      len: usize,
      size: usize,
      ...
  }
  libc: Libc { tls_head: *const TlsModule, ... }
                                             // 依赖7: 全局运行时状态，tls_head 为 TLS 模块链表头
  const DTP_OFFSET: usize;                   // 依赖8: 架构相关 DTV 指针偏置常量 (crate::internal::pthread_arch)

编译时保证:
  TlsModule 为 #[repr(C)] 布局               // 依赖9: 与 C 结构体内存布局兼容
  Pthread 为 #[repr(C)] 布局                // 依赖10: 与 C 结构体内存布局兼容

[GUARANTEE]
Internal Interface (pub(crate)):
  pub(crate) fn __reset_tls();              // 本模块保证: 将当前线程所有 TLS 变量重置为程序加载时的初始值
