# replaced.rs -- Rust 接口归约

> **对应 C 源文件**: `src/malloc/replaced.c`
> **Rust 源文件**: `src/malloc/replaced.rs`
> **复杂度层级**: Level 1 (仅需前置/后置条件)
> **模块类别**: 内部全局状态定义 -- 无函数实现，仅定义两个全局原子变量

---

## 依赖图

```
replaced.rs (无函数调用)
  └─ 写入者 (生产者):
       ├─ ldso/dynlink (动态链接器初始化，写入两个标志位)
  └─ 读取者 (消费者):
       ├─ src/malloc/calloc.rs (读取 MALLOC_REPLACED)
       ├─ src/malloc/mallocng/aligned_alloc.rs (读取两者, 通过 DISABLE_ALIGNED_ALLOC)
       ├─ src/malloc/oldmalloc/aligned_alloc.rs (读取两者)
       └─ src/malloc/mallocng/glue.rs (读取两者, 通过 DISABLE_ALIGNED_ALLOC 宏)
```

---

## 完整依赖关系递归追踪分析

### 直接依赖（本模块定义 --> 外部消费者）

| 本模块符号 | Rust 可见性 | 写入者 | 读取者模块 |
|------------|-------------|--------|------------|
| `MALLOC_REPLACED` | `pub(crate)` | `ldso/dynlink` | `calloc.rs`, `mallocng/aligned_alloc.rs`, `oldmalloc/aligned_alloc.rs`, `mallocng/glue.rs` |
| `ALIGNED_ALLOC_REPLACED` | `pub(crate)` | `ldso/dynlink` | `mallocng/aligned_alloc.rs`, `oldmalloc/aligned_alloc.rs`, `mallocng/glue.rs` |

### 递归追踪消费者依赖链

**Layer 1 -- calloc.rs 读取 MALLOC_REPLACED**:
```
calloc.rs (对外导出 calloc + 内部 __libc_calloc)
  ├── 读取 MALLOC_REPLACED 以决定是否启用零页优化
  │    └── 若 MALLOC_REPLACED == 0: 使用 __malloc_allzerop(p) 快速零检测
  │    └── 若 MALLOC_REPLACED != 0: 跳过优化，强制显式 memset 清零
  ├── 依赖: malloc → lite_malloc.c / mallocng/malloc.c
  ├── 依赖: __malloc_allzerop → 本模块 weak_alias / oldmalloc 强覆盖
  └── 依赖: mal0_clear, memset
```

**Layer 1 -- mallocng/glue.h (glue.rs) 读取两者**:
```
glue.rs (mallocng 胶水层，全 Internal)
  ├── 定义宏 DISABLE_ALIGNED_ALLOC = (MALLOC_REPLACED && !ALIGNED_ALLOC_REPLACED)
  ├── 被 aligned_alloc.rs 引用，控制对齐分配开关
  ├── 依赖: libc.need_locks (→ MT 线程检测)
  ├── 依赖: libc.auxv (→ 随机密钥生成)
  ├── 依赖: __syscall (→ brk/mmap/madvise/mremap)
  ├── 依赖: __lock/__unlock (→ 自旋锁原语)
  └── 依赖: a_cas/a_ctz_32/a_or/a_crash (→ 原子操作)
```

**Layer 1 -- mallocng/aligned_alloc.rs 读取两者**:
```
aligned_alloc.rs (对外导出 aligned_alloc)
  ├── 读取 DISABLE_ALIGNED_ALLOC (展开为 MALLOC_REPLACED && !ALIGNED_ALLOC_REPLACED)
  │    └── 若为真: 返回 NULL + ENOMEM (对齐分配被禁用)
  ├── 依赖: malloc → 底层分配引擎
  ├── 依赖: get_meta/get_slot_index/get_stride/set_size → meta.h (内部 inline)
  ├── 依赖: struct meta/struct group → meta.h (内部类型)
  └── 依赖: UNIT/IB → meta.h (内部常量)
```

**Layer 1 -- oldmalloc/aligned_alloc.rs 读取两者**:
```
oldmalloc/aligned_alloc.rs (对外导出 aligned_alloc, 旧分配器路径)
  ├── 读取 MALLOC_REPLACED && !ALIGNED_ALLOC_REPLACED
  │    └── 若为真: 返回 NULL + ENOMEM
  ├── 依赖: malloc → oldmalloc/malloc.c
  ├── 依赖: __bin_chunk → oldmalloc/malloc.c (内部 chunk 回收)
  ├── 依赖: struct chunk/SIZE_ALIGN/C_INUSE/IS_MMAPPED → malloc_impl.h
  └── 依赖: MEM_TO_CHUNK/NEXT_CHUNK → malloc_impl.h (内部宏)
```

**Layer 1 -- ldso/dynlink (写入者)**:
```
ldso/dynlink (动态链接器初始化)
  ├── 写入 MALLOC_REPLACED
  │    └── 在完成所有共享库符号解析后，若 malloc 符号不由 ldso 提供则置 1
  ├── 写入 ALIGNED_ALLOC_REPLACED
  │    └── 在完成所有共享库符号解析后，若 aligned_alloc 不由 ldso 提供则置 1
  └── 时机: __dls3 函数末尾，进入 runtime=1 模式之前
```

### 递归终止边界

| 终止符号 | 终止原因 |
|----------|----------|
| `malloc` / `__libc_malloc_impl` | 来自 `lite_malloc.c` 或 `mallocng/malloc.c`，已有独立 spec |
| `__libc_free` | 来自 `mallocng/free.c`，已有独立 spec |
| `__libc_realloc` | 来自 `mallocng/realloc.c`，已有独立 spec |
| `__bin_chunk` | 来自 `oldmalloc/malloc.c`，已有独立 spec |
| `__malloc_allzerop` / `is_allzero` | 来自 `calloc.c` 的 weak_alias 或 `oldmalloc/malloc.c` 的强覆盖 |
| `get_meta` / `get_slot_index` / `get_stride` / `set_size` | `meta.h` 中 `static inline`，在 aligned_alloc spec 中描述 |
| `struct meta` / `struct group` / `UNIT` / `IB` | `meta.h` 中定义，内部基础设施 |
| `__syscall` / `__lock` / `__unlock` | `syscall.h` / `lock.h`，底层系统基础设施 |
| `a_cas` / `a_or` / `a_ctz_32` / `a_crash` | `atomic.h`，底层原子操作 |
| `libc.auxv` / `libc.need_locks` / `libc.page_size` | `libc.h`，musl 全局运行时状态 |
| `errno` / `EINVAL` / `ENOMEM` / `SIZE_MAX` | C 标准库 / POSIX，外部基础设施 |
| `memset` / `memcpy` | `<string.h>`，外部 libc 函数 |
| `madvise` / `munmap` / `mremap` / `mmap` | 系统调用，由 glue 层封装 |

---

## 模块概述

本模块是 rusl 中 malloc 系列函数 **插替检测标志** (interposition detection flags) 的唯一定义点。这两个标志允许 rusl 在运行时检测用户是否通过 ELF 符号插替 (symbol interposition) 替换了 `malloc` 或 `aligned_alloc`，从而在 `calloc`、`aligned_alloc` 等依赖内部 malloc 实现细节的函数中切换到安全路径。

在 Rust 实现中，原始 C 的 `int` 类型被替换为 `core::sync::atomic::AtomicI32`，提供无 `unsafe` 的安全并发访问。由于写入仅发生在单线程的动态链接器初始化阶段（此后所有访问均为只读），所有操作均使用 `Ordering::Relaxed` 即可保证正确性。

---

## 内部全局状态

### MALLOC_REPLACED

```rust
pub(crate) static MALLOC_REPLACED: core::sync::atomic::AtomicI32;
```

[Visibility]: Internal -- rusl 内部状态变量，POSIX/C 标准未定义。`pub(crate)` 可见性，仅 rusl crate 内部模块可访问，不对用户程序暴露。

#### 语义

指示标准 `malloc` 函数是否已被外部代码插替 (interposed)。

| 值 | 含义 |
|----|------|
| `0` | `malloc` **未被**替换 -- rusl 内部实现为唯一提供者。`calloc` 可使用内部优化（如 `__malloc_allzerop` 快速清零检查），动态链接器可安全使用内部 `realloc`。 |
| `1` (非零) | `malloc` **已被**替换 -- 外部实现覆盖了 rusl 的 `malloc`。rusl 必须切换到"防御性"模式：禁用依赖内部 malloc 元数据的优化，在特定路径中避免使用 `realloc`。 |

#### 生命周期与状态转换

```
初始值: 0 (编译期常量初始化)
  │
  │  动态链接器加载所有共享库后执行符号查找:
  │  if malloc 符号不由 ldso 自身提供:
  │       MALLOC_REPLACED.store(1, Ordering::Relaxed);
  │
  ▼
最终值: 0 或 1 (在动态链接器完成加载后确定，此后只读)
```

**不变量**: 一旦动态链接器完成所有共享库的加载和重定位，`MALLOC_REPLACED` 的值不再改变。任何后续代码仅读取此值。

#### 读写者

| 角色 | 模块 | 操作 |
|------|------|------|
| **写入者** | `ldso/dynlink` | 动态链接器初始化阶段写入一次 |
| **读取者** | `src/malloc/calloc.rs` | 若 `MALLOC_REPLACED.load(Relaxed) == 0`，启用零页优化 |
| **读取者** | `src/malloc/mallocng/aligned_alloc.rs` | 作为 `DISABLE_ALIGNED_ALLOC` 条件的一部分 |
| **读取者** | `src/malloc/oldmalloc/aligned_alloc.rs` | 直接读取判断是否禁用对齐分配 |
| **读取者** | `src/malloc/mallocng/glue.rs` | 定义 `DISABLE_ALIGNED_ALLOC` 宏 |
| **读取者** | `ldso/dynlink` | 动态链接器在特定路径中读取以决定是否使用 `realloc` |

---

### ALIGNED_ALLOC_REPLACED

```rust
pub(crate) static ALIGNED_ALLOC_REPLACED: core::sync::atomic::AtomicI32;
```

[Visibility]: Internal -- rusl 内部状态变量，POSIX/C 标准未定义。`pub(crate)` 可见性，仅 rusl crate 内部模块可访问，不对用户程序暴露。

#### 语义

指示标准 `aligned_alloc` 函数是否已被外部代码插替。

| 值 | 含义 |
|----|------|
| `0` | `aligned_alloc` **未被**替换 -- rusl 内部实现为唯一提供者。 |
| `1` (非零) | `aligned_alloc` **已被**替换 -- 外部实现覆盖了 rusl 的版本。 |

#### 生命周期与状态转换

```
初始值: 0 (编译期常量初始化)
  │
  │  动态链接器加载所有共享库后执行符号查找:
  │  if aligned_alloc 符号不由 ldso 自身提供:
  │       ALIGNED_ALLOC_REPLACED.store(1, Ordering::Relaxed);
  │
  ▼
最终值: 0 或 1 (在动态链接器完成加载后确定，此后只读)
```

**不变量**: 与 `MALLOC_REPLACED` 相同，在动态链接器进入运行时模式后不可变。

#### 读写者

| 角色 | 模块 | 操作 |
|------|------|------|
| **写入者** | `ldso/dynlink` | 动态链接器初始化阶段写入一次 |
| **读取者** | `src/malloc/mallocng/aligned_alloc.rs` | 作为 `DISABLE_ALIGNED_ALLOC` 条件的取反部分 |
| **读取者** | `src/malloc/oldmalloc/aligned_alloc.rs` | 直接读取判断是否禁用对齐分配 |
| **读取者** | `src/malloc/mallocng/glue.rs` | 定义 `DISABLE_ALIGNED_ALLOC` 宏 |

---

## Rust 设计要点

### 类型选择: `AtomicI32` vs `int`

原 C 实现使用 `int` 类型（非 `volatile`、非 `_Atomic`），依赖 BSS 段零初始化和单线程写入保证正确性。在 Rust 中：

1. **`static mut` 被排除**: Rust 的 `static mut` 要求所有访问均在 `unsafe` 块中，且缺乏形式化的并发安全保证。
2. **`AtomicI32` 是正确选择**: 提供安全的内部可变性（interior mutability），`load`/`store` 均为 safe 操作，无需 `unsafe`。
3. **`Ordering::Relaxed` 足够**: 写入仅发生一次（单线程初始化阶段），写入后的 happens-before 关系由动态链接器的同步屏障（如 `runtime` 标志的 store-release / load-acquire）保证。所有后续读取仅需 `Relaxed` —— 但保守地可在动态链接器的写入端使用 `Release`，读取端使用 `Acquire`，由 `dynlink` spec 约定具体定序。

### Rust 命名约定

原 C 使用 `__` 前缀标识"实现内部符号"。在 Rust 中，`pub(crate)` 可见性已提供等价的封装控制，因此采用 `SCREAMING_SNAKE_CASE`（Rust 静态变量惯用命名）替代 C 的 `__lower_case` 风格：

| C 符号名 | Rust 符号名 | 理由 |
|----------|-------------|------|
| `__malloc_replaced` | `MALLOC_REPLACED` | `pub(crate)` 已表达内部可见性，去除冗余 `__` 前缀 |
| `__aligned_alloc_replaced` | `ALIGNED_ALLOC_REPLACED` | 同上 |

### no_std 兼容性

`core::sync::atomic::AtomicI32` 和 `Ordering` 均来自 `core` 库，不依赖 `std`，满足 rusl 的 `#![no_std]` 要求。无需引入任何第三方 crate。

---

## 系统不变量 (System Invariants)

1. **写入单调性**: 这两个原子变量由编译期零初始化后，仅在动态链接器初始化阶段被写入**最多一次**。写入后永不回退为 0。

2. **读取线程安全性**: 变量仅被写入一次（在单线程启动阶段），之后所有访问均为只读，因此即使使用 `Relaxed` 定序也无需显式同步机制即可保证多线程安全。

3. **写入者唯一性**: 只有动态链接器（`ldso/dynlink`）负责写入这两个变量。rusl 中其他所有模块均为只读消费者。

4. **部分替换兼容性**: rusl 通过两个独立标志处理"部分替换"场景：
   - `malloc` 被替换但 `aligned_alloc` 未被替换 (`MALLOC_REPLACED=1, ALIGNED_ALLOC_REPLACED=0`)：对齐分配功能被禁用，因为 rusl 的 `aligned_alloc` 依赖内部 `malloc` 实现细节。
   - 两者均被替换 (`MALLOC_REPLACED=1, ALIGNED_ALLOC_REPLACED=1`)：对齐分配委托给替换实现，`calloc` 跳过内部优化。
   - 仅 `aligned_alloc` 被替换而 `malloc` 未替换 (`MALLOC_REPLACED=0, ALIGNED_ALLOC_REPLACED=1`)：理论上可能但实际极少发生；此时内部 `aligned_alloc` 仍正常工作，但替换实现不会收到调用。

---

## 设计意图 (Intent)

本模块体现了 rusl 对 **ELF 符号插替兼容性** 的精心设计。标准 C 库规范允许用户替换 `malloc` 系列函数，但替换不需要覆盖全部变体（如仅替换 `malloc`/`free` 而不替换 `calloc`）。若 rusl 的 `calloc` 在内部直接操作 `malloc` 返回的 chunk 元数据，而用户替换的 `malloc` 使用了不兼容的内部布局，则会导致内存损坏。

本模块通过两个全局原子标志将"是否有外部插替"的信息从动态链接器传递到 malloc 子系统，使 `calloc`、`aligned_alloc` 等函数在检测到插替时自动降级为安全路径（放弃依赖内部元数据的优化），从而在不牺牲默认性能的前提下保证替换兼容性。

---

/* Rely */
[RELY]
Predefined Structures/Functions:
  core::sync::atomic::AtomicI32  // 依赖1: 原子整数类型，提供安全的内部可变性
  core::sync::atomic::Ordering   // 依赖2: 内存定序语义（Relaxed/Acquire/Release）

Predefined Macros/Traits:
  (none)

[GUARANTEE]
Exported Interface:
  (none) — 本模块所有符号均为 Internal，无对外导出 C ABI 接口。
            musl 用户通过 POSIX 标准接口（malloc/calloc/free/aligned_alloc）间接使用
            本模块的功能，不直接引用这些标志变量。

Internal Interface:
  pub(crate) static MALLOC_REPLACED: core::sync::atomic::AtomicI32;          // malloc 插替检测标志
  pub(crate) static ALIGNED_ALLOC_REPLACED: core::sync::atomic::AtomicI32;   // aligned_alloc 插替检测标志